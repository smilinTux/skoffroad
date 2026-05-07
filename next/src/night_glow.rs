// Night glow: emissive accents on landmarks, gas stations, and billboards
// that activate when TimeOfDay enters the night window (t < 0.20 or t > 0.80).
//
// Strategy: spawn NEW glow entities at known world positions.  Each entity
// carries a NightGlow component that drives visibility and emissive intensity.
// A companion GlowLight entity (PointLight + GlowLightLink) is spawned
// alongside each glow sphere so it actually illuminates nearby geometry.
//
// System order:
//   update_glow_visibility  — lerp NightGlow.intensity, toggle Visibility,
//                             write emissive for non-blinking glows
//   pulse_blinking_lights   — square-wave blink on beacons, drive ALL point lights
//
// Public API:
//   NightGlowPlugin

use std::f32::consts::TAU;

use bevy::prelude::*;

use crate::sky::TimeOfDay;
use crate::terrain::terrain_height_at;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct NightGlowPlugin;

impl Plugin for NightGlowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_glow_entities)
           .add_systems(Update, (update_glow_visibility, pulse_blinking_lights).chain());
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Real-time seconds to fully fade between day and night intensity.
const FADE_SECS: f32 = 2.0;

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Attached to every glow sphere spawned by this module.
#[derive(Component)]
pub struct NightGlow {
    /// Base emissive colour when fully "on".
    pub color: Color,
    /// Whether this beacon blinks at 1 Hz.
    pub blinking: bool,
    /// Current 0..=1 fade intensity (lerped each frame).
    pub intensity: f32,
}

/// Marks the PointLight companion of a NightGlow sphere.
#[derive(Component)]
struct GlowLight {
    base_lumens: f32,
}

/// Links a GlowLight PointLight entity back to its NightGlow sphere entity.
#[derive(Component)]
struct GlowLightLink(Entity);

// ---------------------------------------------------------------------------
// Glow descriptor table (compile-time)
// ---------------------------------------------------------------------------

struct GlowDesc {
    /// World X, Z — Y resolved via terrain_height_at at startup.
    xz: (f32, f32),
    /// Metres above the terrain surface.
    y_offset: f32,
    /// Emissive / sphere colour [r, g, b].
    color: [f32; 3],
    /// 1 Hz square-wave blink?
    blinking: bool,
    /// Visual sphere radius (metres).
    sphere_radius: f32,
    /// Point-light range (metres).
    light_range: f32,
    /// Peak point-light lumens (at full-night intensity).
    light_lumens: f32,
    /// Point-light colour [r, g, b].
    light_color: [f32; 3],
}

const GLOW_TABLE: &[GlowDesc] = &[
    // ── Lighthouse (90, _, 95) ── large yellow beacon on top of mast ─────
    GlowDesc {
        xz: (90.0, 95.0),
        y_offset: 22.0,
        color: [1.0, 0.90, 0.30],
        blinking: false,
        sphere_radius: 0.55,
        light_range: 80.0,
        light_lumens: 350_000.0,
        light_color: [1.0, 0.88, 0.45],
    },
    // ── Water tower (-80, _, -80) ── small red blink ─────────────────────
    GlowDesc {
        xz: (-80.0, -80.0),
        y_offset: 18.0,
        color: [1.0, 0.12, 0.12],
        blinking: true,
        sphere_radius: 0.25,
        light_range: 18.0,
        light_lumens: 12_000.0,
        light_color: [1.0, 0.10, 0.10],
    },
    // ── Radio tower (-95, _, 75) ── top red blink ─────────────────────────
    GlowDesc {
        xz: (-95.0, 75.0),
        y_offset: 28.0,
        color: [1.0, 0.12, 0.12],
        blinking: true,
        sphere_radius: 0.25,
        light_range: 20.0,
        light_lumens: 12_000.0,
        light_color: [1.0, 0.10, 0.10],
    },
    // ── Radio tower (-95, _, 75) ── mid-mast second beacon + subtle glow ─
    GlowDesc {
        xz: (-95.0, 75.0),
        y_offset: 18.0,
        color: [0.80, 0.10, 0.10],
        blinking: true,
        sphere_radius: 0.20,
        light_range: 12.0,
        light_lumens: 6_000.0,
        light_color: [1.0, 0.08, 0.08],
    },
    // ── Gas station sign (20, _, 35) ─────────────────────────────────────
    GlowDesc {
        xz: (20.0, 35.0),
        y_offset: 5.5,
        color: [1.0, 0.95, 0.72],
        blinking: false,
        sphere_radius: 0.40,
        light_range: 20.0,
        light_lumens: 28_000.0,
        light_color: [1.0, 0.93, 0.65],
    },
    // ── Gas station sign (-30, _, -25) ───────────────────────────────────
    GlowDesc {
        xz: (-30.0, -25.0),
        y_offset: 5.5,
        color: [1.0, 0.95, 0.72],
        blinking: false,
        sphere_radius: 0.40,
        light_range: 20.0,
        light_lumens: 28_000.0,
        light_color: [1.0, 0.93, 0.65],
    },
    // ── Gas station sign (60, _, -65) ────────────────────────────────────
    GlowDesc {
        xz: (60.0, -65.0),
        y_offset: 5.5,
        color: [1.0, 0.95, 0.72],
        blinking: false,
        sphere_radius: 0.40,
        light_range: 20.0,
        light_lumens: 28_000.0,
        light_color: [1.0, 0.93, 0.65],
    },
    // ── Rock garden post (110, _, 0) ─────────────────────────────────────
    GlowDesc {
        xz: (110.0, 0.0),
        y_offset: 3.5,
        color: [1.0, 0.80, 0.20],
        blinking: false,
        sphere_radius: 0.30,
        light_range: 14.0,
        light_lumens: 10_000.0,
        light_color: [1.0, 0.78, 0.30],
    },
    // ── Hillclimb sign (-160, _, -150) ───────────────────────────────────
    GlowDesc {
        xz: (-160.0, -150.0),
        y_offset: 4.0,
        color: [1.0, 0.80, 0.20],
        blinking: false,
        sphere_radius: 0.30,
        light_range: 14.0,
        light_lumens: 10_000.0,
        light_color: [1.0, 0.78, 0.30],
    },
];

// ---------------------------------------------------------------------------
// Night predicate
// ---------------------------------------------------------------------------

#[inline]
fn is_night(t: f32) -> bool {
    t < 0.20 || t > 0.80
}

// ---------------------------------------------------------------------------
// Startup: spawn_glow_entities
// ---------------------------------------------------------------------------

fn spawn_glow_entities(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for desc in GLOW_TABLE {
        let (fx, fz) = desc.xz;
        let fy = terrain_height_at(fx, fz) + desc.y_offset;
        let world_pos = Vec3::new(fx, fy, fz);

        let [r, g, b] = desc.color;

        let sphere_mesh = meshes.add(Sphere::new(desc.sphere_radius).mesh().ico(1).unwrap());
        // Start emissive at zero — update_glow_visibility will ramp it up at
        // night.  We bake the full colour here so pulse_blinking_lights can
        // scale it.
        let sphere_mat = materials.add(StandardMaterial {
            base_color: Color::srgba(r, g, b, 1.0),
            emissive:   LinearRgba::rgb(0.0, 0.0, 0.0),
            unlit:      true,
            ..default()
        });

        // Spawn hidden; starts at noon (default TimeOfDay).
        let glow_entity = commands.spawn((
            NightGlow {
                color:     Color::srgb(r, g, b),
                blinking:  desc.blinking,
                intensity: 0.0,
            },
            Mesh3d(sphere_mesh),
            MeshMaterial3d(sphere_mat),
            Transform::from_translation(world_pos),
            Visibility::Hidden,
        )).id();

        // Companion PointLight sibling (not a child — no transform inheritance
        // issues and no need to query the parent to drive intensity).
        let [lr, lg, lb] = desc.light_color;
        commands.spawn((
            GlowLight { base_lumens: desc.light_lumens },
            GlowLightLink(glow_entity),
            PointLight {
                intensity:       0.0,
                range:           desc.light_range,
                color:           Color::srgb(lr, lg, lb),
                shadows_enabled: false,
                ..default()
            },
            Transform::from_translation(world_pos),
        ));
    }
}

// ---------------------------------------------------------------------------
// Update: update_glow_visibility
// ---------------------------------------------------------------------------

/// Lerps NightGlow.intensity toward the day/night target over FADE_SECS.
/// Toggles Visibility on the sphere mesh.
/// Writes emissive colour for **non-blinking** glows; pulse_blinking_lights
/// handles blinkers in the next system.
fn update_glow_visibility(
    tod:  Res<TimeOfDay>,
    time: Res<Time>,
    mut glows: Query<(
        &mut NightGlow,
        &mut Visibility,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let dt = time.delta_secs();
    let target = if is_night(tod.t) { 1.0_f32 } else { 0.0_f32 };
    let step = if FADE_SECS > 0.0 { dt / FADE_SECS } else { 1.0 };

    for (mut glow, mut vis, mat_handle) in glows.iter_mut() {
        // Move intensity one step toward target.
        if glow.intensity < target {
            glow.intensity = (glow.intensity + step).min(target);
        } else if glow.intensity > target {
            glow.intensity = (glow.intensity - step).max(target);
        }

        // Toggle mesh visibility.
        *vis = if glow.intensity > 0.001 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };

        // For steady (non-blinking) glows write emissive now.
        // Blinking glows are written each frame by pulse_blinking_lights so
        // we skip them here to avoid fighting over the same material handle.
        if !glow.blinking {
            if let Some(mat) = materials.get_mut(mat_handle) {
                let c = glow.color.to_srgba();
                mat.emissive = LinearRgba::rgb(
                    c.red   * glow.intensity,
                    c.green * glow.intensity,
                    c.blue  * glow.intensity,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Update: pulse_blinking_lights
// ---------------------------------------------------------------------------

/// 1 Hz square-wave for water-tower and radio-tower beacons.
/// Also drives ALL companion PointLight intensities (blinking and steady).
fn pulse_blinking_lights(
    time:  Res<Time>,
    glows: Query<(&NightGlow, &MeshMaterial3d<StandardMaterial>)>,
    mut lights: Query<(&mut PointLight, &GlowLightLink, &GlowLight)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let t = time.elapsed_secs();
    // Square wave at 1 Hz: on in the positive half-cycle.
    let blink_on = (t * TAU).sin() >= 0.0;

    for (mut light, link, glow_light) in lights.iter_mut() {
        let Ok((glow, mat_handle)) = glows.get(link.0) else { continue };

        let effective = if glow.blinking {
            if blink_on { glow.intensity } else { 0.0 }
        } else {
            glow.intensity
        };

        light.intensity = effective * glow_light.base_lumens;

        // For blinking glows, also pulse the sphere emissive colour.
        if glow.blinking {
            if let Some(mat) = materials.get_mut(mat_handle) {
                let c = glow.color.to_srgba();
                mat.emissive = LinearRgba::rgb(
                    c.red   * effective,
                    c.green * effective,
                    c.blue  * effective,
                );
            }
        }
    }
}
