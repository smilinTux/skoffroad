// Vehicle detail upgrades: additive geometry that all variants share —
// translucent windshield + side windows, 2 side mirrors, 2 mud flaps behind
// rear wheels, 2 tail lights (red emissive), 2 headlight chrome reflectors,
// 2 door handles. Distinct from variants.rs which adds variant-specific bling
// (TJ grille, Bronco letters).
//
// Sprint 81: visual polish pass.
//   - Glass (windshield + mirror faces): believable tint, low roughness,
//     increased reflectance so it reads as real glass at Medium+.
//   - Body-paint clearcoat upgrade: a runtime system (`upgrade_paint_clearcoat`)
//     scans DefaultSkin entities each frame (debounced) and bumps paint
//     materials to high-reflectance, low-roughness clearcoat on Medium+. This
//     is the same runtime-find-and-upgrade approach used by vehicle_textures.rs,
//     so it survives chassis RespawnRequest (new DefaultSkin entities will be
//     found on subsequent scans as the cooldown clears).
//
// Every detail mesh carries the `VehicleDetail` marker component so future
// systems can identify, hide, or swap them without touching DefaultSkin /
// VariantSkin children.
//
// Public API:
//   VehicleDetailPlugin
//   VehicleDetail          (component marker on spawned detail meshes)
//   VehicleDetailAttached  (marker on Chassis once details have been attached —
//                           respawn safe: new chassis won't carry it)

use bevy::prelude::*;
use std::f32::consts::PI;
use crate::vehicle::{VehicleRoot, Chassis, DefaultSkin};
use crate::graphics_quality::GraphicsQuality;

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct VehicleDetailPlugin;

impl Plugin for VehicleDetailPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (attach_details, upgrade_paint_clearcoat));
    }
}

// ── Component markers ─────────────────────────────────────────────────────────

/// Marker placed on every additive detail mesh child (windshield, mirrors,
/// mud flaps, tail lights, headlight reflectors, door handles). Allows future
/// systems to query or despawn detail geometry independently of DefaultSkin /
/// VariantSkin children.
#[derive(Component)]
pub struct VehicleDetail;

/// Placed on the Chassis entity once its detail children have been attached.
/// Because a chassis respawn creates a fresh Chassis entity, new chassis
/// entities won't carry this marker and `attach_details` will re-fire for them.
#[derive(Component)]
pub struct VehicleDetailAttached;

// ── Detail attachment system ──────────────────────────────────────────────────

/// Runs every Update frame. For each Chassis entity that does NOT yet carry
/// `VehicleDetailAttached`, waits for VehicleRoot and then spawns all additive
/// detail children. Inserts VehicleDetailAttached so subsequent frames skip
/// that chassis. Re-fires on chassis respawn (fresh Chassis entity has no
/// marker).
fn attach_details(
    vehicle: Option<Res<VehicleRoot>>,
    chassis_q: Query<Entity, (With<Chassis>, Without<VehicleDetailAttached>)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    quality: Res<GraphicsQuality>,
) {
    let Some(vehicle) = vehicle else { return };

    // Only process a chassis that hasn't had details attached yet.
    let Ok(chassis) = chassis_q.get(vehicle.chassis) else { return };

    // ── Glass material ────────────────────────────────────────────────────────
    // Sprint 81: believable tinted glass on Medium+.
    //   - Lower roughness (0.03) → sharper reflections.
    //   - Higher reflectance (0.8) → glass-like specular highlight.
    //   - Slightly richer tint while staying translucent (alpha 0.35).
    // Low keeps the simple original glass (alpha 0.4, roughness 0.05) to avoid
    // blend-mode overdraw on weaker GPUs.
    let windshield_mat = if quality.vehicle_clearcoat() {
        materials.add(StandardMaterial {
            base_color: Color::srgba(0.30, 0.55, 0.80, 0.35),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.03,
            reflectance: 0.80,
            ..default()
        })
    } else {
        materials.add(StandardMaterial {
            base_color: Color::srgba(0.4, 0.6, 0.8, 0.4),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.05,
            ..default()
        })
    };

    // ── 1. Windshield ────────────────────────────────────────────────────────
    let windshield_mesh = meshes.add(Cuboid::new(1.6, 0.7, 0.05));
    let windshield = commands.spawn((
        VehicleDetail,
        Mesh3d(windshield_mesh),
        MeshMaterial3d(windshield_mat),
        Transform::from_translation(Vec3::new(0.0, 0.6, -1.0))
            .with_rotation(Quat::from_rotation_x(-0.4)),
    )).id();

    // ── 2. Side mirrors (LH + RH) ────────────────────────────────────────────
    let mirror_housing_mesh = meshes.add(Cuboid::new(0.15, 0.18, 0.10));
    let mirror_face_mesh    = meshes.add(Cuboid::new(0.13, 0.16, 0.02));

    let mirror_housing_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.12, 0.14),
        perceptual_roughness: 0.70,
        ..default()
    });
    // Mirror glass face: true chrome on Medium+, matte on Low.
    let mirror_face_mat = if quality.vehicle_clearcoat() {
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.88, 0.88, 0.94),
            perceptual_roughness: 0.05,
            metallic: 0.95,
            reflectance: 0.95,
            ..default()
        })
    } else {
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.75, 0.75, 0.82),
            perceptual_roughness: 0.20,
            metallic: 0.70,
            ..default()
        })
    };

    let mut mirror_ids: Vec<Entity> = Vec::with_capacity(2);
    for side in [-1.0_f32, 1.0_f32] {
        let housing = commands.spawn((
            VehicleDetail,
            Mesh3d(mirror_housing_mesh.clone()),
            MeshMaterial3d(mirror_housing_mat.clone()),
            Transform::from_translation(Vec3::new(side * 1.10, 0.45, -1.4)),
        )).id();
        let face = commands.spawn((
            VehicleDetail,
            Mesh3d(mirror_face_mesh.clone()),
            MeshMaterial3d(mirror_face_mat.clone()),
            Transform::from_translation(Vec3::new(0.0, 0.0, -0.04)),
        )).id();
        commands.entity(housing).add_child(face);
        mirror_ids.push(housing);
    }

    // ── 3. Mud flaps (rear LH + RH) ─────────────────────────────────────────
    let mudflap_mesh = meshes.add(Cuboid::new(0.30, 0.40, 0.04));
    let mudflap_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.06, 0.06, 0.06),
        perceptual_roughness: 0.95,
        ..default()
    });

    let mut mudflap_ids: Vec<Entity> = Vec::with_capacity(2);
    for side in [-1.0_f32, 1.0_f32] {
        let flap = commands.spawn((
            VehicleDetail,
            Mesh3d(mudflap_mesh.clone()),
            MeshMaterial3d(mudflap_mat.clone()),
            Transform::from_translation(Vec3::new(side * 1.10, -0.10, 1.85)),
        )).id();
        mudflap_ids.push(flap);
    }

    // ── 4. Tail lights (rear LH + RH) ────────────────────────────────────────
    let taillight_mesh = meshes.add(Cuboid::new(0.20, 0.10, 0.03));
    let taillight_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.10, 0.10),
        emissive: LinearRgba::rgb(0.8, 0.1, 0.1),
        perceptual_roughness: 0.2,
        ..default()
    });

    let mut taillight_ids: Vec<Entity> = Vec::with_capacity(2);
    for side in [-1.0_f32, 1.0_f32] {
        let tl = commands.spawn((
            VehicleDetail,
            Mesh3d(taillight_mesh.clone()),
            MeshMaterial3d(taillight_mat.clone()),
            Transform::from_translation(Vec3::new(side * 0.70, 0.10, 2.01)),
        )).id();
        taillight_ids.push(tl);
    }

    // ── 5. Headlight chrome reflectors (front LH + RH) ───────────────────────
    let reflector_mesh = meshes.add(Cylinder::new(0.12, 0.08));
    // Sprint 81: true chrome on Medium+.
    let reflector_mat = if quality.vehicle_clearcoat() {
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.92, 0.92, 0.96),
            metallic: 0.95,
            perceptual_roughness: 0.04,
            reflectance: 0.95,
            ..default()
        })
    } else {
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.80, 0.80, 0.86),
            metallic: 0.80,
            perceptual_roughness: 0.10,
            ..default()
        })
    };

    let mut reflector_ids: Vec<Entity> = Vec::with_capacity(2);
    for side in [-1.0_f32, 1.0_f32] {
        let ref_e = commands.spawn((
            VehicleDetail,
            Mesh3d(reflector_mesh.clone()),
            MeshMaterial3d(reflector_mat.clone()),
            Transform::from_translation(Vec3::new(side * 0.70, 0.10, -2.01))
                .with_rotation(Quat::from_rotation_x(-PI / 2.0)),
        )).id();
        reflector_ids.push(ref_e);
    }

    // ── 6. Door handles (LH + RH) ────────────────────────────────────────────
    let handle_mesh = meshes.add(Cuboid::new(0.20, 0.04, 0.04));
    // Sprint 81: true chrome on Medium+.
    let handle_mat = if quality.vehicle_clearcoat() {
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.88, 0.88, 0.94),
            metallic: 0.95,
            perceptual_roughness: 0.06,
            reflectance: 0.95,
            ..default()
        })
    } else {
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.75, 0.75, 0.80),
            metallic: 0.70,
            perceptual_roughness: 0.20,
            ..default()
        })
    };

    let mut handle_ids: Vec<Entity> = Vec::with_capacity(2);
    for side in [-1.0_f32, 1.0_f32] {
        let handle = commands.spawn((
            VehicleDetail,
            Mesh3d(handle_mesh.clone()),
            MeshMaterial3d(handle_mat.clone()),
            Transform::from_translation(Vec3::new(side * 1.005, 0.10, 0.4)),
        )).id();
        handle_ids.push(handle);
    }

    // ── Attach everything to the chassis ────────────────────────────────────
    commands.entity(chassis).add_child(windshield);
    for &id in &mirror_ids    { commands.entity(chassis).add_child(id); }
    for &id in &mudflap_ids   { commands.entity(chassis).add_child(id); }
    for &id in &taillight_ids { commands.entity(chassis).add_child(id); }
    for &id in &reflector_ids { commands.entity(chassis).add_child(id); }
    for &id in &handle_ids    { commands.entity(chassis).add_child(id); }

    // Mark this chassis so we don't re-attach next frame.
    commands.entity(chassis).insert(VehicleDetailAttached);
}

// ── Clearcoat paint upgrade system ───────────────────────────────────────────

/// Sprint 81: Runtime clearcoat upgrade on Medium+.
///
/// Scans DefaultSkin entities (body paint) at ~2 Hz and upgrades any paint
/// material that has not yet been given clearcoat treatment.  Mirrors the
/// approach of vehicle_textures.rs (find-and-upgrade, not one-shot Local<bool>)
/// so it correctly re-runs when a chassis RespawnRequest replaces old DefaultSkin
/// entities with fresh ones whose materials haven't been upgraded yet.
///
/// Clearcoat emulation (no Bevy clearcoat feature required):
///   - High reflectance (0.70) → strong specular lobe.
///   - Low-ish perceptual roughness (0.28) → tight, glossy highlight.
///   - Metallic 0.55 → slight metallic tint (same as vehicle.rs Medium path).
/// This is the same technique as Sprint 43's body_mat (vehicle.rs line ~165)
/// but applied as a post-spawn material edit so it works on all variants and
/// survives respawn without touching vehicle.rs's frozen spawn code.
///
/// On Low quality this system does nothing — the matte material set by
/// vehicle.rs is intentionally preserved.
///
/// The `polished` set tracks AssetIds already upgraded so each material is
/// touched at most once. After a respawn the new vehicle's materials get fresh
/// AssetIds, so they pass the guard and get upgraded on the next tick.
fn upgrade_paint_clearcoat(
    quality: Res<GraphicsQuality>,
    time: Res<Time>,
    mut cooldown: Local<f32>,
    mut polished: Local<std::collections::HashSet<AssetId<StandardMaterial>>>,
    skin_q: Query<&MeshMaterial3d<StandardMaterial>, With<DefaultSkin>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Low tier: skip entirely — keep matte look consistent with Low's other
    // material choices (no bloom, no SSAO, matte body paint).
    if !quality.vehicle_clearcoat() { return; }

    // Debounce: run at most once every 0.5 s.
    *cooldown -= time.delta_secs();
    if *cooldown > 0.0 { return; }
    *cooldown = 0.5;

    let mut upgraded = 0usize;
    for mat_handle in skin_q.iter() {
        let id = mat_handle.id();
        if polished.contains(&id) { continue; }

        let Some(mat) = materials.get_mut(id) else { continue };

        // Only upgrade body-paint materials:
        //   metallic < 0.7          — excludes chrome, axles, steel
        //   emissive near zero      — excludes headlights
        //   alpha Opaque            — excludes glass
        //   roughness > 0.20        — already clearcoated materials skip this
        let is_paint = mat.metallic < 0.70
            && mat.emissive.red   < 0.1
            && mat.emissive.green < 0.1
            && mat.emissive.blue  < 0.1
            && matches!(mat.alpha_mode, AlphaMode::Opaque)
            && mat.perceptual_roughness > 0.20;

        if !is_paint { continue; }

        // Apply clearcoat emulation.
        mat.perceptual_roughness = mat.perceptual_roughness.min(0.28);
        mat.metallic             = mat.metallic.max(0.55);
        mat.reflectance          = 0.70;

        polished.insert(id);
        upgraded += 1;
    }

    if upgraded > 0 {
        info!("vehicle_detail: clearcoat upgrade applied to {} paint materials", upgraded);
    }
}
