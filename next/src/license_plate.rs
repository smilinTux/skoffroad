// License plate: a small white rectangle attached to the rear of the chassis
// with 6 procedural "characters" rendered as colored cuboid stripes.
// No actual text rendering — stripes are deterministically colored each session.
//
// Public API:
//   LicensePlatePlugin

use bevy::prelude::*;
use crate::vehicle::VehicleRoot;

// ---- Constants --------------------------------------------------------------

/// Plate background dimensions (width x height x depth).
const PLATE_W: f32 = 0.6;
const PLATE_H: f32 = 0.25;
const PLATE_D: f32 = 0.04;

/// Plate position in chassis local space:
/// z=+2.02 places it just beyond the rear face (chassis half-depth = 2.0),
/// y=-0.1 drops it slightly below chassis centre.
const PLATE_LOCAL: Vec3 = Vec3::new(0.0, -0.1, 2.02);

/// Character stripe dimensions.
const STRIPE_W: f32 = 0.06;
const STRIPE_H: f32 = 0.18;
const STRIPE_D: f32 = 0.02;

/// Number of character stripes.
const NUM_STRIPES: usize = 6;

/// Horizontal margin from plate edge to first/last stripe centre (plate coords).
const STRIPE_MARGIN: f32 = 0.06;

// ---- Accent color palette ---------------------------------------------------
// 4 accent colors + black (black at 50%, each accent at 12.5%).
const ACCENT_COLORS: [Color; 4] = [
    Color::srgb(0.80, 0.10, 0.10), // red
    Color::srgb(0.10, 0.45, 0.80), // blue
    Color::srgb(0.90, 0.75, 0.10), // yellow
    Color::srgb(0.10, 0.65, 0.25), // green
];
const BLACK: Color = Color::srgb(0.05, 0.05, 0.05);
const WHITE_PLATE: Color = Color::srgb(0.95, 0.95, 0.92);

// ---- Plugin -----------------------------------------------------------------

pub struct LicensePlatePlugin;

impl Plugin for LicensePlatePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, attach_plate_once);
    }
}

// ---- Marker component -------------------------------------------------------

#[derive(Component)]
struct LicensePlate;

// ---- System -----------------------------------------------------------------

/// Runs every frame until VehicleRoot exists, then attaches the plate once.
/// Uses a `Local<bool>` guard so subsequent frames are a no-op.
fn attach_plate_once(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    vehicle:       Option<Res<VehicleRoot>>,
    time:          Res<Time>,
    mut done:      Local<bool>,
) {
    if *done { return; }
    let Some(vehicle) = vehicle else { return };

    let seed = time.elapsed_secs();

    // ---- Background plate ---------------------------------------------------

    let plate_mesh = meshes.add(Cuboid::new(PLATE_W, PLATE_H, PLATE_D));
    let plate_mat  = materials.add(StandardMaterial {
        base_color: WHITE_PLATE,
        perceptual_roughness: 0.6,
        ..default()
    });

    let bg = commands.spawn((
        LicensePlate,
        Mesh3d(plate_mesh),
        MeshMaterial3d(plate_mat),
        Transform::from_translation(PLATE_LOCAL),
    )).id();
    commands.entity(vehicle.chassis).add_child(bg);

    // ---- 6 character stripes ------------------------------------------------

    let stripe_mesh = meshes.add(Cuboid::new(STRIPE_W, STRIPE_H, STRIPE_D));

    // Evenly space NUM_STRIPES stripes across the plate width minus margins.
    let usable_width = PLATE_W - 2.0 * STRIPE_MARGIN;
    let gap = if NUM_STRIPES > 1 {
        usable_width / (NUM_STRIPES as f32 - 1.0)
    } else {
        0.0
    };
    let left_x = -(PLATE_W * 0.5) + STRIPE_MARGIN;

    for i in 0..NUM_STRIPES {
        let color = stripe_color(seed, i);
        let mat   = materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: 0.5,
            ..default()
        });

        let x = left_x + gap * i as f32;
        // Stripe sits flush on the front face of the plate background.
        // Plate is centred at z=0 in its own space; its front face is at z = -PLATE_D/2.
        // Stripe extends STRIPE_D/2 further forward.
        let z = -(PLATE_D * 0.5) - (STRIPE_D * 0.5);

        let stripe = commands.spawn((
            LicensePlate,
            Mesh3d(stripe_mesh.clone()),
            MeshMaterial3d(mat),
            Transform::from_translation(Vec3::new(x, 0.0, z)),
        )).id();
        commands.entity(bg).add_child(stripe);
    }

    *done = true;
}

// ---- Helpers ----------------------------------------------------------------

/// Deterministically pick a stripe color from `seed` and stripe `index`.
/// 50% chance of black, 50% chance of one of the 4 accent colors.
fn stripe_color(seed: f32, index: usize) -> Color {
    // Cheap LCG-style hash; no external crate needed.
    let raw = (seed * 1337.7 + index as f32 * 479.3).sin() * 43758.5453;
    let frac = raw - raw.floor(); // in [0, 1)

    if frac < 0.5 {
        BLACK
    } else {
        // Map frac in [0.5, 1.0) to one of 4 accent colors.
        let idx = ((frac - 0.5) * 8.0) as usize % ACCENT_COLORS.len();
        ACCENT_COLORS[idx]
    }
}
