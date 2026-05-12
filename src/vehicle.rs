// Raycast suspension vehicle model.
//
// TUNING CONSTANTS (physical rationale):
//   CHASSIS_MASS   = 1500 kg   — Jeep Wrangler TJ curb weight
//   SPRING_K       = 50_000    — ~0.07 m static sag per wheel at 1500 kg / 4
//   DAMPING_C      = 4_000     — near-critical per wheel; kills bounce within 1-2 cycles
//   SUSPENSION_LEN = 0.60 m    — axle-to-ground distance at natural rest
//   DRIVE_FORCE    = 700 N/whl — 2800 N total; adequate for off-road traction
//   LATERAL_GRIP   = 8_000     — N/(m/s) per wheel; prevents sideways slide
//   BRAKE_FORCE    = 3000 N/whl— overcomes ~11° slope gravity + stops from 4 m/s in ~0.5 s
//   MAX_STEER_DEG  = 30°       — typical off-road steering angle
//   ANG_DAMP       = 16.0      — raised from 12 to damp roll oscillations faster
//   ANTI_ROLL_K    = 18_000    — N/m per axle; tuned to keep max_tilt < 20° on forward 3

use bevy::prelude::*;
use avian3d::prelude::*;
use crate::terrain::terrain_height_at;
use crate::vehicle_mods::{BumperKind, VehicleModsState};

pub struct VehiclePlugin;
impl Plugin for VehiclePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DriveInput>()
           .init_resource::<RespawnRequest>()
           .add_systems(Startup, spawn_vehicle)
           .add_systems(Update, (drive_input_keyboard, respawn_on_request).chain())
           .add_systems(PhysicsSchedule, suspension_system
               .after(PhysicsStepSystems::NarrowPhase)
               .before(PhysicsStepSystems::Solver))
           .add_systems(Update, update_wheel_visuals);
    }
}

/// Public flag any plugin can set to request a full chassis respawn next frame.
/// vehicle_mods.rs sets this on `VehicleModsState` change so swapping tire size,
/// long-arm kit, bumpers, or winch is instantly visible without `cargo run`-ing.
#[derive(Resource, Default)]
pub struct RespawnRequest(pub bool);

fn respawn_on_request(
    mut commands: Commands,
    mut request: ResMut<RespawnRequest>,
    vehicle: Option<Res<VehicleRoot>>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
    quality: Res<crate::graphics_quality::GraphicsQuality>,
    mods_opt: Option<Res<VehicleModsState>>,
) {
    if !request.0 {
        return;
    }
    request.0 = false;

    // Despawn the previous chassis entity tree (chassis + body + wheels +
    // mods + variant skin children all under the same root).
    if let Some(v) = vehicle.as_deref() {
        if let Ok(mut e) = commands.get_entity(v.chassis) {
            e.despawn();
        }
    }

    // Re-run the same spawn logic so the new chassis reflects the latest
    // mods + quality settings.
    spawn_vehicle(commands, meshes, materials, quality, mods_opt);
}

pub struct VehiclePluginHeadless;
impl Plugin for VehiclePluginHeadless {
    fn build(&self, app: &mut App) {
        app.init_resource::<DriveInput>()
           .add_systems(Startup, spawn_vehicle)
           .add_systems(PhysicsSchedule, suspension_system
               .after(PhysicsStepSystems::NarrowPhase)
               .before(PhysicsStepSystems::Solver))
           .add_systems(Update, update_wheel_visuals);
    }
}

// ---- Components / Resources ----

#[derive(Component)]
pub struct Chassis;

/// Marker on the 7 default Jeep-silhouette mesh children of the chassis. Used
/// by `variants.rs` to identify and despawn the default skin when cycling to
/// a different vehicle variant.
#[derive(Component)]
pub struct DefaultSkin;

#[derive(Component)]
pub struct Wheel {
    pub index: usize,
    pub current_compression: f32,
    pub spin: f32,
    pub is_grounded: bool,
}

#[derive(Resource)]
pub struct VehicleRoot { pub chassis: Entity }

#[derive(Resource, Default)]
pub struct DriveInput {
    pub drive: f32,
    pub steer: f32,
    pub brake: bool,
}

// ---- Constants ----

const CHASSIS_HALF: Vec3         = Vec3::new(1.0, 0.4, 2.0);
#[allow(dead_code)]
const WHEEL_RADIUS: f32          = 0.35;
const WHEEL_HALF_WIDTH: f32      = 0.18;
const RIM_RADIUS: f32            = 0.20;
const CHASSIS_MASS: f32          = 1500.0;
const SPRING_K: f32              = 50_000.0;
const DAMPING_C: f32             = 5_000.0;
#[allow(dead_code)]
const SUSPENSION_LEN: f32        = 0.60;
// Bumped 700 → 1800. With 4 wheels = 7200 N, mass 1500 = 4.8 m/s² flat-
// ground accel, enough to climb ~28° slopes and push out of water under
// boost. The original 700 (2800 N total) couldn't climb anything beyond
// ~10° before gravity overpowered it.
const DRIVE_FORCE_PER_WHEEL: f32 = 5500.0;
const LATERAL_GRIP: f32          = 8_000.0;
const BRAKE_FORCE_PER_WHEEL: f32 = 3_000.0;
const MAX_STEER_ANGLE: f32       = 30_f32 * std::f32::consts::PI / 180.0;
const ANG_DAMP: f32              = 25.0;
const ANTI_ROLL_K: f32           = 30_000.0;
const ANTI_PITCH_K: f32          = 30_000.0;

// FL, FR, RL, RR anchor offsets in chassis local space.
const WHEEL_OFFSETS: [Vec3; 4] = [
    Vec3::new(-1.1, -0.35, -1.4),
    Vec3::new( 1.1, -0.35, -1.4),
    Vec3::new(-1.1, -0.35,  1.4),
    Vec3::new( 1.1, -0.35,  1.4),
];

// ---- Spawn ----

fn spawn_vehicle(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    quality: Res<crate::graphics_quality::GraphicsQuality>,
    // Sprint 48: Option<Res> so the headless test harness (which doesn't
    // register VehicleModsPlugin) still works — falls back to all-stock defaults,
    // which are identical to the pre-Sprint-48 constants.
    mods_opt: Option<Res<VehicleModsState>>,
) {
    let _default_mods = VehicleModsState::default();
    let mods: &VehicleModsState = mods_opt.as_deref().unwrap_or(&_default_mods);

    // Sprint 48: read active mod values.  All defaults equal the pre-mod
    // constants so drive_test results are unchanged when mods are stock.
    let susp_len    = mods.suspension_len();       // 0.60 when stock
    let wheel_r     = mods.wheel_radius();          // 0.35 when stock
    let spawn_lift  = mods.spawn_y_lift();          // 0.0 when stock
    let extra_mass  = mods.extra_mass();            // 0.0 when stock
    // Sprint 43: Medium+ uses a glossier "car paint" material on the chassis.
    // Metal flakes + low roughness + bumped reflectance read as clearcoat
    // even without Bevy's optional clearcoat feature. Low keeps the matte
    // sRGB(0.8, 0.2, 0.1) original to stay consistent with the legacy look.
    let body_mat = if quality.vehicle_clearcoat() {
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.75, 0.12, 0.10),
            perceptual_roughness: 0.32,
            metallic: 0.55,
            reflectance: 0.65,
            ..default()
        })
    } else {
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.75, 0.12, 0.10),
            perceptual_roughness: 0.6,
            ..default()
        })
    };
    let bumper_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.18, 0.18),
        perceptual_roughness: 0.9,
        ..default()
    });
    let glass_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.55, 0.75, 0.85, 0.45),
        perceptual_roughness: 0.1,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let headlight_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 1.0, 0.9),
        emissive: LinearRgba::rgb(4.0, 4.0, 3.0),
        perceptual_roughness: 0.05,
        ..default()
    });
    let wheel_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.12, 0.12),
        perceptual_roughness: 0.9,
        ..default()
    });
    // Sprint 44: chrome rims on Medium+; matte aluminium on Low.
    let rim_mat = if quality.vehicle_clearcoat() {
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.78, 0.78, 0.80),
            perceptual_roughness: 0.18,
            metallic: 0.95,
            reflectance: 0.85,
            ..default()
        })
    } else {
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.22, 0.22, 0.22),
            perceptual_roughness: 0.7,
            ..default()
        })
    };

    let body_mesh        = meshes.add(Cuboid::new(CHASSIS_HALF.x*2.0, CHASSIS_HALF.y*2.0, CHASSIS_HALF.z*2.0));
    let hood_mesh        = meshes.add(Cuboid::new(1.9, 0.22, 1.2));
    let windshield_mesh  = meshes.add(Cuboid::new(1.8, 0.05, 0.8));
    let headlight_mesh   = meshes.add(Sphere::new(0.10));
    // Sprint 48: wheel radius driven by TireSize mod (stock = WHEEL_RADIUS = 0.35).
    let wheel_mesh       = meshes.add(Cylinder::new(wheel_r, WHEEL_HALF_WIDTH * 2.0));
    let rim_mesh         = meshes.add(Cylinder::new(RIM_RADIUS, WHEEL_HALF_WIDTH * 1.4));

    // Sprint 45 — Skrambler detail meshes:
    // 7-slot grille slats, roll cage bars/cross-bars, fender flares, doors,
    // side mirrors, roof light bar with LED spots, tailgate-mounted spare.
    let grille_slat_mesh = meshes.add(Cuboid::new(0.04, 0.32, 0.05));
    let cage_bar_mesh    = meshes.add(Cylinder::new(0.045, 0.95));
    let cage_topbar_x    = meshes.add(Cylinder::new(0.045, 1.92));
    let cage_topbar_z    = meshes.add(Cylinder::new(0.045, 0.86));
    let fender_mesh      = meshes.add(Cuboid::new(0.18, 0.10, 0.92));
    let door_mesh        = meshes.add(Cuboid::new(0.06, 0.46, 1.10));
    let mirror_mesh      = meshes.add(Cuboid::new(0.12, 0.06, 0.18));
    let light_bar_mesh   = meshes.add(Cuboid::new(1.50, 0.07, 0.10));
    // Sprint 48: spare tire scales with active tire size.
    let spare_tire_mesh  = meshes.add(Cylinder::new(wheel_r * 0.95, WHEEL_HALF_WIDTH * 1.7));

    // Matte-black roll-cage / grille / mirror material.
    let cage_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.05, 0.05),
        perceptual_roughness: 0.75,
        metallic: 0.10,
        ..default()
    });

    // Sprint 48: use active suspension length and long-arm spawn lift.
    let spawn_y = {
        let sum: f32 = WHEEL_OFFSETS.iter()
            .map(|o| terrain_height_at(o.x, o.z) + susp_len - o.y).sum();
        sum / WHEEL_OFFSETS.len() as f32 + spawn_lift
    };

    // Chassis: full-size collider, centred at the rigid-body origin. Earlier
    // attempts to offset the collider via a child entity broke the chassis
    // because Avian distributes mass across the collider AABB — an offset
    // collider means an off-centre COM, which made the chassis top-heavy
    // and flip immediately. Sticking with the centred collider; reverse
    // asymmetry is now compensated by giving reverse 1.6x drive force
    // (see compute_input drive shaping below — see Round 13 fix in
    // suspension_system).
    // Sprint 48: bumper kit adds up to 60 kg for steel front+rear.
    let chassis_id = commands.spawn((
        Chassis,
        Transform::from_translation(Vec3::new(0.0, spawn_y, 0.0)),
        Visibility::default(),
        RigidBody::Dynamic,
        Collider::cuboid(CHASSIS_HALF.x, CHASSIS_HALF.y, CHASSIS_HALF.z),
        Mass(CHASSIS_MASS + extra_mass),
        LinearDamping(0.5),
        AngularDamping(ANG_DAMP),
        SleepingDisabled,
    )).id();

    // Jeep silhouette: 7 child meshes (body, hood, windshield, 2 bumpers, 2 headlights).
    // Use add_child on the chassis (same pattern as wheels below) — set_parent_in_place
    // does not attach the child in Bevy 0.18, leaving the body parts as orphan root
    // entities at world-space "child-local" positions (i.e. buried near the origin).
    let body = commands.spawn((DefaultSkin, Mesh3d(body_mesh), MeshMaterial3d(body_mat.clone()),
        Transform::IDENTITY)).id();
    let hood = commands.spawn((DefaultSkin, Mesh3d(hood_mesh), MeshMaterial3d(body_mat.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.12, -1.6)))).id();
    let windshield = commands.spawn((DefaultSkin, Mesh3d(windshield_mesh), MeshMaterial3d(glass_mat),
        Transform::from_translation(Vec3::new(0.0, 0.32, -0.88))
            .with_rotation(Quat::from_rotation_x(-25_f32.to_radians())))).id();
    let hl_l = commands.spawn((DefaultSkin, Mesh3d(headlight_mesh.clone()),
        MeshMaterial3d(headlight_mat.clone()),
        Transform::from_translation(Vec3::new(-0.75, -0.12, -2.10)))).id();
    let hl_r = commands.spawn((DefaultSkin, Mesh3d(headlight_mesh.clone()), MeshMaterial3d(headlight_mat.clone()),
        Transform::from_translation(Vec3::new( 0.75, -0.12, -2.10)))).id();

    // Sprint 48: bumper meshes depend on active BumperKind mod.
    //   Stock         → thin plastic-look cuboid (original).
    //   SteelFront    → chunky steel front + stock rear.
    //   SteelFrontRear → chunky steel both ends.
    // Steel bumpers are 10 cm thicker (0.22 depth vs 0.12) and include
    // two D-ring spheres on each end face.
    let steel_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.28, 0.28),
        perceptual_roughness: 0.55,
        metallic: 0.70,
        ..default()
    });
    let dring_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.50, 0.50, 0.52),
        perceptual_roughness: 0.35,
        metallic: 0.90,
        ..default()
    });

    let front_bp = spawn_bumper_front(&mut commands, &mut meshes, &mods.bumper,
        bumper_mat.clone(), steel_mat.clone(), dring_mat.clone());
    let rear_bp  = spawn_bumper_rear(&mut commands, &mut meshes, &mods.bumper,
        bumper_mat.clone(), steel_mat.clone(), dring_mat.clone());

    commands.entity(chassis_id).add_children(&[body, hood, windshield, front_bp, rear_bp, hl_l, hl_r]);

    // ----- Sprint 45: Skrambler detail layer -----------------------------
    //
    // 7-slot grille between the headlights.
    let mut details: Vec<Entity> = Vec::new();
    for i in 0..7 {
        let x = -0.42 + i as f32 * 0.14;
        let slat = commands.spawn((
            DefaultSkin,
            Mesh3d(grille_slat_mesh.clone()),
            MeshMaterial3d(cage_mat.clone()),
            Transform::from_translation(Vec3::new(x, -0.18, -2.04)),
        )).id();
        details.push(slat);
    }

    // Roll cage — 4 vertical bars + front/rear/top crossbars across the cabin.
    // Cylinder axis is +Y, so vertical bars need no rotation; the X-axis cross
    // bar gets a Z rotation; the Z-axis side bars get an X rotation.
    let rotz_90 = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    let rotx_90 = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
    for &(x, z) in &[(-0.95_f32, -0.30_f32), (0.95, -0.30), (-0.95, 0.50), (0.95, 0.50)] {
        let bar = commands.spawn((
            DefaultSkin,
            Mesh3d(cage_bar_mesh.clone()),
            MeshMaterial3d(cage_mat.clone()),
            Transform::from_translation(Vec3::new(x, 0.85, z)),
        )).id();
        details.push(bar);
    }
    // Top X-axis cross bars (front-of-cabin and back-of-cabin).
    for &z in &[-0.30_f32, 0.50] {
        let bar = commands.spawn((
            DefaultSkin,
            Mesh3d(cage_topbar_x.clone()),
            MeshMaterial3d(cage_mat.clone()),
            Transform::from_translation(Vec3::new(0.0, 1.30, z))
                .with_rotation(rotz_90),
        )).id();
        details.push(bar);
    }
    // Top Z-axis side rails.
    for &x in &[-0.95_f32, 0.95] {
        let bar = commands.spawn((
            DefaultSkin,
            Mesh3d(cage_topbar_z.clone()),
            MeshMaterial3d(cage_mat.clone()),
            Transform::from_translation(Vec3::new(x, 1.30, 0.10))
                .with_rotation(rotx_90),
        )).id();
        details.push(bar);
    }

    // Fender flares above each wheel.
    for &offset in WHEEL_OFFSETS.iter() {
        let outer = if offset.x > 0.0 { 1.10 } else { -1.10 };
        let flare = commands.spawn((
            DefaultSkin,
            Mesh3d(fender_mesh.clone()),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_translation(Vec3::new(outer, -0.10, offset.z)),
        )).id();
        details.push(flare);
    }

    // Driver / passenger door panels (back half of the cabin only — open-top).
    for &x in &[-1.04_f32, 1.04] {
        let door = commands.spawn((
            DefaultSkin,
            Mesh3d(door_mesh.clone()),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_translation(Vec3::new(x, 0.05, 0.30)),
        )).id();
        details.push(door);
    }

    // Side mirrors at the front of each door.
    for &x in &[-1.16_f32, 1.16] {
        let mirror = commands.spawn((
            DefaultSkin,
            Mesh3d(mirror_mesh.clone()),
            MeshMaterial3d(cage_mat.clone()),
            Transform::from_translation(Vec3::new(x, 0.36, -0.30)),
        )).id();
        details.push(mirror);
    }

    // Roof light bar above the windshield + 4 LED spots.
    let light_bar = commands.spawn((
        DefaultSkin,
        Mesh3d(light_bar_mesh.clone()),
        MeshMaterial3d(cage_mat.clone()),
        Transform::from_translation(Vec3::new(0.0, 1.32, -1.05)),
    )).id();
    details.push(light_bar);
    for i in 0..4 {
        let lx = -0.55 + i as f32 * 0.36;
        let led = commands.spawn((
            DefaultSkin,
            Mesh3d(headlight_mesh.clone()),
            MeshMaterial3d(headlight_mat.clone()),
            Transform::from_translation(Vec3::new(lx, 1.32, -1.13))
                .with_scale(Vec3::splat(0.55)),
        )).id();
        details.push(led);
    }

    // Tailgate-mounted spare tire (rotated to face rear).
    let spare_rot = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)
        * Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    let spare = commands.spawn((
        DefaultSkin,
        Mesh3d(spare_tire_mesh.clone()),
        MeshMaterial3d(wheel_mat.clone()),
        Transform::from_translation(Vec3::new(0.0, 0.10, 2.32))
            .with_rotation(spare_rot),
    )).id();
    details.push(spare);

    // ----- Solid axles + (optional) long-arm control arms -------------------
    //
    // Both axles hang at the wheel-hub Y in chassis-local space. The hub Y
    // accounts for the long-arm/body-lift visual drop so the axle visibly
    // sits at the tires' centerline, not buried inside the body.
    // Axle Y in chassis-local: wheel hub height with an extra 0.18 m drop so
    // the tube hangs clearly BELOW the chassis bottom (which sits at y = -0.4
    // = -CHASSIS_HALF.y). Without the drop the stock axle was buried inside
    // the body; now it pokes out and the diff bulge is visible from any angle.
    let axle_y = WHEEL_OFFSETS[0].y - mods.suspension_len() + crate::vehicle_mods::BASE_SUSPENSION_LEN - mods.body_lift_y() - 0.18;
    let axle_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.32, 0.32, 0.36),       // lighter steel so it reads against the dark underbody
        perceptual_roughness: 0.45,
        metallic: 0.90,
        ..default()
    });
    let diff_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.42, 0.46),       // even lighter — diff catches the eye
        perceptual_roughness: 0.35,
        metallic: 0.95,
        ..default()
    });
    // Axle tube: cylinder spanning left ↔ right wheel hub, slightly inset so
    // it tucks behind the chrome rim. Cylinder axis is Y by default; we rotate
    // 90° around Z so it lies along world X.
    // Slightly chunkier so it reads from chase-camera distance.
    let axle_tube_len = (WHEEL_OFFSETS[1].x - WHEEL_OFFSETS[0].x) * 0.92;
    let axle_tube_mesh = meshes.add(Cylinder::new(0.10, axle_tube_len));
    // Transfer-case output height — used by driveshaft spawn below.
    let tc_y = axle_y + 0.10;  // just under chassis floor
    for &z in &[WHEEL_OFFSETS[0].z, WHEEL_OFFSETS[2].z] {
        // Front axle (Dana 60): diff offset to the DRIVER side (x = -0.30).
        // Real Jeep Dana 60 front: diff is on the driver (left) side so the
        // driveshaft has a straighter run from the driver-side transfer-case output.
        // Rear axle (Dana 44): diff is centred on the axle tube.
        let is_front = z < 0.0;
        let diff_x = if is_front { -0.30 } else { 0.0 };

        // Tube
        details.push(commands.spawn((
            DefaultSkin,
            Mesh3d(axle_tube_mesh.clone()),
            MeshMaterial3d(axle_mat.clone()),
            Transform::from_translation(Vec3::new(0.0, axle_y, z))
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
        )).id());
        // Differential pumpkin — offset per real Dana geometry.
        details.push(commands.spawn((
            DefaultSkin,
            Mesh3d(meshes.add(Sphere::new(0.22))),  // bigger diff pumpkin so it's actually visible
            MeshMaterial3d(diff_mat.clone()),
            Transform::from_translation(Vec3::new(diff_x, axle_y - 0.02, z)),
        )).id());
        // Pinion stub pointing toward the transfer case.
        // Front (diff at x=-0.30): pinion shifts toward x=-0.24 (inboard toward
        //   chassis center) and angles forward (toward -Z nose).
        // Rear (diff centred): pinion points straight rearward toward TC.
        let pinion_z_offset = if is_front { 0.18 } else { -0.18 };
        let pinion_x_offset = if is_front { 0.06 } else { 0.0 }; // inboard toward center when diff is at -0.30
        details.push(commands.spawn((
            DefaultSkin,
            Mesh3d(meshes.add(Cylinder::new(0.06, 0.22))),
            MeshMaterial3d(diff_mat.clone()),
            Transform::from_translation(Vec3::new(
                diff_x + pinion_x_offset,
                axle_y - 0.02,
                z + pinion_z_offset,
            )).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
        )).id());
    }

    // Long-arm control arms — visible diagonal bars running from the chassis
    // frame mount (mid-chassis) back/down to the axle attach point near each
    // wheel hub. Real long-arm kits have two per wheel (lower + upper); we draw
    // one prominent lower arm per wheel for clarity.
    if mods.long_arm {
        let arm_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.78, 0.42, 0.12),  // bright off-road copper — pops against dark underbody
            perceptual_roughness: 0.45,
            metallic: 0.75,
            ..default()
        });
        for &wheel_offset in WHEEL_OFFSETS.iter() {
            // Frame mount: mid-chassis, slightly inboard of the wheel.
            // For front wheels (z < 0): mount points are BACK from the wheel (positive z).
            // For rear  wheels (z > 0): mount points are FORWARD from the wheel (negative z).
            let mount_z = if wheel_offset.z < 0.0 { wheel_offset.z + 0.95 } else { wheel_offset.z - 0.95 };
            let frame_mount = Vec3::new(wheel_offset.x * 0.55, -CHASSIS_HALF.y + 0.05, mount_z);
            // Axle attach: just inboard of the wheel hub, at the axle Y.
            let axle_attach = Vec3::new(wheel_offset.x * 0.85, axle_y, wheel_offset.z);

            let arm_vec    = axle_attach - frame_mount;
            let arm_len    = arm_vec.length();
            let arm_dir    = arm_vec / arm_len.max(1e-4);
            let arm_mid    = (frame_mount + axle_attach) * 0.5;
            // Map cylinder +Y to arm_dir.
            let arm_rot    = Quat::from_rotation_arc(Vec3::Y, arm_dir);
            details.push(commands.spawn((
                DefaultSkin,
                Mesh3d(meshes.add(Cylinder::new(0.070, arm_len))),  // thicker so it reads from chase view
                MeshMaterial3d(arm_mat.clone()),
                Transform::from_translation(arm_mid).with_rotation(arm_rot),
            )).id());
            // Mount bushing at the frame end (small sphere, darker)
            details.push(commands.spawn((
                DefaultSkin,
                Mesh3d(meshes.add(Sphere::new(0.09))),  // bushings — slightly bigger so the joints read
                MeshMaterial3d(axle_mat.clone()),
                Transform::from_translation(frame_mount),
            )).id());
            // Mount bushing at the axle end
            details.push(commands.spawn((
                DefaultSkin,
                Mesh3d(meshes.add(Sphere::new(0.09))),  // bushings — slightly bigger so the joints read
                MeshMaterial3d(axle_mat.clone()),
                Transform::from_translation(axle_attach),
            )).id());
        }
    }

    // Winch assembly: housing + motor + drum + end plates + fairlead + cable.
    // Mounted to the front bumper face. Only spawned when bumper != Stock.
    if mods.winch && mods.bumper != BumperKind::Stock {
        let housing_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.20, 0.20, 0.22),
            perceptual_roughness: 0.45,
            metallic: 0.85,
            ..default()
        });
        let drum_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.10, 0.10, 0.10),
            perceptual_roughness: 0.55,
            metallic: 0.90,
            ..default()
        });
        let cable_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.45, 0.40, 0.32),
            perceptual_roughness: 0.85,
            ..default()
        });
        let fairlead_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.65, 0.65, 0.68),
            perceptual_roughness: 0.30,
            metallic: 0.95,
            ..default()
        });

        let z_front = -2.10;
        let y       = -0.18;

        // Main housing block (motor + gearbox shell).
        details.push(commands.spawn((
            DefaultSkin,
            Mesh3d(meshes.add(Cuboid::new(0.62, 0.22, 0.28))),
            MeshMaterial3d(housing_mat.clone()),
            Transform::from_translation(Vec3::new(0.0, y, z_front)),
        )).id());

        // Motor cap on the right end (small protrusion).
        details.push(commands.spawn((
            DefaultSkin,
            Mesh3d(meshes.add(Cylinder::new(0.10, 0.18))),
            MeshMaterial3d(housing_mat.clone()),
            Transform::from_translation(Vec3::new(0.40, y, z_front))
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
        )).id());

        // Drum (the spool the cable winds around) — visible between the end caps.
        details.push(commands.spawn((
            DefaultSkin,
            Mesh3d(meshes.add(Cylinder::new(0.085, 0.34))),
            MeshMaterial3d(drum_mat),
            Transform::from_translation(Vec3::new(0.0, y, z_front))
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
        )).id());

        // Two end plates (the round flanges that cap the drum).
        for &x in &[-0.18_f32, 0.18] {
            details.push(commands.spawn((
                DefaultSkin,
                Mesh3d(meshes.add(Cylinder::new(0.115, 0.04))),
                MeshMaterial3d(housing_mat.clone()),
                Transform::from_translation(Vec3::new(x, y, z_front))
                    .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
            )).id());
        }

        // Fairlead — the chrome four-roller frame the cable feeds through, mounted
        // ~0.18 m forward of the drum.
        let fl_z = z_front - 0.22;
        details.push(commands.spawn((
            DefaultSkin,
            Mesh3d(meshes.add(Cuboid::new(0.34, 0.18, 0.04))),
            MeshMaterial3d(fairlead_mat.clone()),
            Transform::from_translation(Vec3::new(0.0, y, fl_z)),
        )).id());
        // Four small rollers (top, bottom, left, right) inside the fairlead frame.
        let r_radius = 0.025_f32;
        let r_len    = 0.20_f32;
        for (pos, axis) in [
            (Vec3::new( 0.0, y + 0.07, fl_z), Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
            (Vec3::new( 0.0, y - 0.07, fl_z), Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
            (Vec3::new( 0.14, y, fl_z),       Quat::IDENTITY),
            (Vec3::new(-0.14, y, fl_z),       Quat::IDENTITY),
        ] {
            details.push(commands.spawn((
                DefaultSkin,
                Mesh3d(meshes.add(Cylinder::new(r_radius, r_len))),
                MeshMaterial3d(fairlead_mat.clone()),
                Transform::from_translation(pos).with_rotation(axis),
            )).id());
        }
        // Cable: sags across the bumper from the fairlead exit to the right
        // D-ring, where it terminates in a J-hook clipped to the ring. This
        // matches how real off-roaders stow a winch line — the hook is
        // attached to one of the bumper's own shackles so it doesn't dangle.
        //
        // The cable is modeled as 5 short cylinder segments along a quadratic
        // bezier that dips below the chord midpoint to give the catenary sag.
        let p_start = Vec3::new(0.0, y, fl_z);              // fairlead exit
        let p_end   = Vec3::new(0.85, -0.32, -2.24);        // right D-ring
        let chord_mid = (p_start + p_end) * 0.5;
        // Pull the bezier control point ~6 cm lower (in -Y) than the chord
        // midpoint so the cable visibly sags. The bezier passes through this
        // displaced control point twice as strongly as a single sample, so the
        // visual mid-sag ends up around half that depth — about 3 cm.
        let control = chord_mid + Vec3::new(0.0, -0.12, 0.0);
        let segs    = 5;
        let mut prev = p_start;
        for i in 1..=segs {
            let t = i as f32 / segs as f32;
            let one_t = 1.0 - t;
            // Quadratic bezier B(t) = (1-t)²·p0 + 2·(1-t)·t·c + t²·p1
            let pt = p_start * (one_t * one_t)
                   + control * (2.0 * one_t * t)
                   + p_end   * (t * t);
            let mid = (prev + pt) * 0.5;
            let seg = pt - prev;
            let len = seg.length();
            if len > 1e-4 {
                let rot = Quat::from_rotation_arc(Vec3::Y, seg / len);
                details.push(commands.spawn((
                    DefaultSkin,
                    Mesh3d(meshes.add(Cylinder::new(0.014, len))),
                    MeshMaterial3d(cable_mat.clone()),
                    Transform::from_translation(mid).with_rotation(rot),
                )).id());
            }
            prev = pt;
        }

        // J-hook at the D-ring end: a vertical shank (cylinder going down from
        // just above the ring) and a small horizontal curl at the bottom that
        // wraps around the D-ring sphere.
        let hook_shank_top = p_end + Vec3::new(0.0, 0.05, 0.0);
        details.push(commands.spawn((
            DefaultSkin,
            Mesh3d(meshes.add(Cylinder::new(0.022, 0.10))),
            MeshMaterial3d(housing_mat.clone()),
            Transform::from_translation(hook_shank_top + Vec3::new(0.0, -0.05, 0.0)),
        )).id());
        // Hook curl — a short horizontal cylinder cradling the D-ring.
        details.push(commands.spawn((
            DefaultSkin,
            Mesh3d(meshes.add(Cylinder::new(0.022, 0.10))),
            MeshMaterial3d(housing_mat.clone()),
            Transform::from_translation(p_end + Vec3::new(-0.04, -0.045, 0.0))
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
        )).id());
        // Hook eye — small dark ring/cap where the cable meets the shank.
        details.push(commands.spawn((
            DefaultSkin,
            Mesh3d(meshes.add(Sphere::new(0.025))),
            MeshMaterial3d(housing_mat),
            Transform::from_translation(hook_shank_top),
        )).id());
    }
    // -------------------------------------------------------------------------

    commands.entity(chassis_id).add_children(&details);
    // ---------------------------------------------------------------------

    let tire_rot = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    for (i, &offset) in WHEEL_OFFSETS.iter().enumerate() {
        let wheel_id = commands.spawn((
            Wheel { index: i, current_compression: 0.0, spin: 0.0, is_grounded: false },
            Mesh3d(wheel_mesh.clone()),
            MeshMaterial3d(wheel_mat.clone()),
            Transform::from_translation(offset).with_rotation(tire_rot),
        )).id();
        let rim = commands.spawn((Mesh3d(rim_mesh.clone()), MeshMaterial3d(rim_mat.clone()),
            Transform::IDENTITY)).id();
        commands.entity(wheel_id).add_child(rim);
        commands.entity(chassis_id).add_child(wheel_id);
    }

    commands.insert_resource(VehicleRoot { chassis: chassis_id });
}

// ---- Bumper helpers (Sprint 48) ----
//
// Returns a single DefaultSkin entity whose mesh depends on the active BumperKind.
// Stock → original thin cuboid.
// Steel → 10 cm thicker cuboid + two D-ring spheres on each end face.
//
// Splitting front / rear into separate functions keeps things readable.

fn spawn_bumper_front(
    commands:   &mut Commands,
    meshes:     &mut Assets<Mesh>,
    bumper:     &BumperKind,
    stock_mat:  Handle<StandardMaterial>,
    steel_mat:  Handle<StandardMaterial>,
    dring_mat:  Handle<StandardMaterial>,
) -> Entity {
    match bumper {
        BumperKind::Stock => {
            let m = meshes.add(Cuboid::new(2.1, 0.15, 0.12));
            commands.spawn((DefaultSkin, Mesh3d(m), MeshMaterial3d(stock_mat),
                Transform::from_translation(Vec3::new(0.0, -0.30, -2.10)))).id()
        }
        _ => {
            // Chunky steel front bumper: wider, thicker, slightly lower.
            let m = meshes.add(Cuboid::new(2.2, 0.22, 0.22));
            let bp = commands.spawn((DefaultSkin, Mesh3d(m), MeshMaterial3d(steel_mat),
                Transform::from_translation(Vec3::new(0.0, -0.32, -2.12)))).id();
            // Two D-ring shackles on each end.
            let ring_mesh = meshes.add(Sphere::new(0.06));
            for &rx in &[-0.85_f32, 0.85] {
                commands.spawn((DefaultSkin,
                    Mesh3d(ring_mesh.clone()),
                    MeshMaterial3d(dring_mat.clone()),
                    Transform::from_translation(Vec3::new(rx, -0.32, -2.24))));
            }
            bp
        }
    }
}

fn spawn_bumper_rear(
    commands:   &mut Commands,
    meshes:     &mut Assets<Mesh>,
    bumper:     &BumperKind,
    stock_mat:  Handle<StandardMaterial>,
    steel_mat:  Handle<StandardMaterial>,
    dring_mat:  Handle<StandardMaterial>,
) -> Entity {
    match bumper {
        BumperKind::Stock | BumperKind::SteelFront => {
            // Stock rear for both Stock and SteelFront kit.
            let m = meshes.add(Cuboid::new(2.1, 0.15, 0.12));
            commands.spawn((DefaultSkin, Mesh3d(m), MeshMaterial3d(stock_mat),
                Transform::from_translation(Vec3::new(0.0, -0.30, 2.10)))).id()
        }
        BumperKind::SteelFrontRear => {
            let m = meshes.add(Cuboid::new(2.2, 0.22, 0.22));
            let bp = commands.spawn((DefaultSkin, Mesh3d(m), MeshMaterial3d(steel_mat),
                Transform::from_translation(Vec3::new(0.0, -0.32, 2.12)))).id();
            let ring_mesh = meshes.add(Sphere::new(0.06));
            for &rx in &[-0.85_f32, 0.85] {
                commands.spawn((DefaultSkin,
                    Mesh3d(ring_mesh.clone()),
                    MeshMaterial3d(dring_mat.clone()),
                    Transform::from_translation(Vec3::new(rx, -0.32, 2.24))));
            }
            bp
        }
    }
}

// ---- Input ----

pub fn drive_input_keyboard(keys: Res<ButtonInput<KeyCode>>, mut input: ResMut<DriveInput>) {
    input.drive = 0.0;
    input.steer = 0.0;
    input.brake = keys.pressed(KeyCode::Space);
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp)    { input.drive += 1.0; }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown)  { input.drive -= 1.0; }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft)  { input.steer += 1.0; }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) { input.steer -= 1.0; }
}

pub fn apply_drive_input() {} // stub — kept for external references

// ---- Suspension + drive (PhysicsSchedule, after narrow phase, before solver) ----

fn suspension_system(
    input:    Res<DriveInput>,
    vehicle:  Option<Res<VehicleRoot>>,
    mods_opt: Option<Res<VehicleModsState>>,
    mut chassis_q: Query<(Forces, &Transform), With<Chassis>>,
    mut wheel_q: Query<&mut Wheel>,
    spatial: SpatialQuery,
) {
    let Some(vehicle) = vehicle else { return };
    let Ok((mut forces, transform)) = chassis_q.get_mut(vehicle.chassis) else { return };

    // Sprint 48: read effective suspension length from active mods.
    // Falls back to stock default (0.60) when VehicleModsPlugin is absent (headless tests).
    let _default_mods = VehicleModsState::default();
    let mods: &VehicleModsState = mods_opt.as_deref().unwrap_or(&_default_mods);
    let susp_len = mods.suspension_len();

    let chassis_pos = transform.translation;
    let chassis_rot = transform.rotation;
    let chassis_fwd = (chassis_rot * Vec3::NEG_Z).normalize();
    let chassis_up  = (chassis_rot * Vec3::Y).normalize();
    let lin_vel_v   = forces.linear_velocity();
    let ang_vel_v   = forces.angular_velocity();

    // Speed-sensitive steering: full angle at rest, reduced linearly at speed
    let effective_steer = MAX_STEER_ANGLE * (1.0 / (1.0 + 0.1 * lin_vel_v.length()));

    let filter  = SpatialQueryFilter::from_excluded_entities([vehicle.chassis]);
    let ray_len = susp_len + 0.5;

    let mut compressions  = [0.0_f32; 4];
    let mut world_anchors = [Vec3::ZERO; 4];
    let mut normals       = [Vec3::Y; 4];
    let mut contacts      = [false; 4];

    for (i, &local_anchor) in WHEEL_OFFSETS.iter().enumerate() {
        let world_anchor = chassis_pos + chassis_rot * local_anchor;
        world_anchors[i] = world_anchor;
        if let Some(hit) = spatial.cast_ray(world_anchor, Dir3::NEG_Y, ray_len, true, &filter) {
            let c = (susp_len - hit.distance).max(0.0);
            compressions[i] = c;
            normals[i]  = Vec3::new(hit.normal.x, hit.normal.y, hit.normal.z);
            contacts[i] = c > 0.0;
        }
    }

    // Propagate grounded/compression state to Wheel components (read by particles agent)
    for mut wheel in wheel_q.iter_mut() {
        let i = wheel.index;
        wheel.current_compression = compressions[i];
        wheel.is_grounded = contacts[i];
    }

    for i in 0..4 {
        if !contacts[i] { continue; }

        let world_anchor = world_anchors[i];
        let normal       = normals[i];
        let r            = world_anchor - chassis_pos;
        let v_anchor     = lin_vel_v + ang_vel_v.cross(r);
        let comp_vel     = -v_anchor.dot(normal);

        let f_damp = DAMPING_C * comp_vel.clamp(-10.0, 10.0);
        let mut f_susp = (SPRING_K * compressions[i] + f_damp).max(0.0);

        // Weight transfer on braking: front loads up, rear unloads (nose-dive feel)
        if input.brake { f_susp *= if i < 2 { 1.2 } else { 0.8 }; }

        forces.apply_force_at_point(normal * f_susp, world_anchor);

        let steer_fwd = if i < 2 {
            (Quat::from_axis_angle(chassis_up, input.steer * effective_steer) * chassis_fwd).normalize()
        } else { chassis_fwd };

        let fwd_ground   = (steer_fwd - steer_fwd.dot(normal) * normal).normalize_or_zero();
        let right_ground = fwd_ground.cross(normal).normalize_or_zero();

        if input.brake {
            let v_long  = v_anchor.dot(fwd_ground);
            let brake_f = (-BRAKE_FORCE_PER_WHEEL * v_long.signum())
                .clamp(-BRAKE_FORCE_PER_WHEEL, BRAKE_FORCE_PER_WHEEL);
            forces.apply_force_at_point(fwd_ground * brake_f, world_anchor);
        } else if input.drive.abs() > 0.0 {
            // Throttle curve: powf(1.5) gives gentle low end, full power at full input.
            // Symmetric force in both directions; the empirically tuned reverse
            // boost from earlier sprints over-compensated after Sprint 38–40
            // (forward felt much weaker than reverse).
            let shaped = input.drive.signum() * input.drive.abs().powf(1.5);
            forces.apply_force_at_point(
                fwd_ground * shaped * DRIVE_FORCE_PER_WHEEL,
                world_anchor,
            );
        } else {
            let v_long   = v_anchor.dot(fwd_ground);
            let resist_f = (-LATERAL_GRIP * v_long).clamp(-f_susp * 1.2, f_susp * 1.2);
            forces.apply_force_at_point(fwd_ground * resist_f, world_anchor);
        }

        let v_lat = v_anchor.dot(right_ground);
        let f_lat = (-LATERAL_GRIP * v_lat).clamp(-f_susp * 1.2, f_susp * 1.2);
        forces.apply_force_at_point(right_ground * f_lat, world_anchor);
    }

    // Anti-roll bar: resist differential compression between left/right on each axle.
    // diff > 0 (left more compressed) → push left UP to resist, push right DOWN.
    for &(l, r) in &[(0usize, 1usize), (2usize, 3usize)] {
        if !contacts[l] && !contacts[r] { continue; }
        let arb = ANTI_ROLL_K * (compressions[l] - compressions[r]);
        forces.apply_force_at_point(Vec3::Y *   arb,  world_anchors[l]);
        forces.apply_force_at_point(Vec3::Y * (-arb), world_anchors[r]);
    }

    // Anti-pitch: resist differential compression between front and rear axles.
    // Reduces nose-dive on braking and rear-squat on acceleration.
    let front_avg = 0.5 * (compressions[0] + compressions[1]);
    let rear_avg  = 0.5 * (compressions[2] + compressions[3]);
    let any_front = contacts[0] || contacts[1];
    let any_rear  = contacts[2] || contacts[3];
    if any_front && any_rear {
        let pitch = ANTI_PITCH_K * (front_avg - rear_avg);
        // Front more compressed → lift front, push rear down.
        forces.apply_force_at_point(Vec3::Y *  pitch, world_anchors[0]);
        forces.apply_force_at_point(Vec3::Y *  pitch, world_anchors[1]);
        forces.apply_force_at_point(Vec3::Y * -pitch, world_anchors[2]);
        forces.apply_force_at_point(Vec3::Y * -pitch, world_anchors[3]);
    }
}

// ---- Visual wheel update ----

fn update_wheel_visuals(
    vehicle:  Option<Res<VehicleRoot>>,
    chassis_q: Query<(&Transform, &LinearVelocity), With<Chassis>>,
    mut wheel_q: Query<(&mut Transform, &mut Wheel), Without<Chassis>>,
    input:    Res<DriveInput>,
    mods_opt: Option<Res<VehicleModsState>>,
    time:     Res<Time>,
) {
    let Some(vehicle) = vehicle else { return };
    let Ok((c_transform, lin_vel)) = chassis_q.get(vehicle.chassis) else { return };

    // Sprint 48: spin rate uses active wheel radius so large tires roll
    // at the correct angular speed. Falls back to stock default in headless mode.
    let _default_mods = VehicleModsState::default();
    let mods: &VehicleModsState = mods_opt.as_deref().unwrap_or(&_default_mods);
    let wheel_r = mods.wheel_radius();

    let fwd   = *c_transform.forward();
    let speed = Vec3::new(lin_vel.x, lin_vel.y, lin_vel.z).dot(fwd);
    let dt    = time.delta_secs();

    // Front wheels (index 0=FL, 1=FR) yaw with steering input. Same speed-
    // sensitive shape used in the suspension drive logic so the visual
    // matches the physics.
    let speed_mps = Vec3::new(lin_vel.x, lin_vel.y, lin_vel.z).length();
    let effective_steer = MAX_STEER_ANGLE * (1.0 / (1.0 + 0.1 * speed_mps));
    let steer_yaw = input.steer * effective_steer;

    // Visual wheel slip: when the driver mashes throttle but the chassis is
    // moving slowly (or stuck), spin the wheels faster than ground speed so
    // the player sees they're "trying". slip_rpm peaks at ~1200 rpm visual.
    let slip_factor = (1.0 - (speed_mps / 8.0).clamp(0.0, 1.0)) * input.drive.abs();
    let slip_omega  = slip_factor * 25.0;  // rad/s extra spin

    // When the suspension is extended (long-arm kit) or the body is lifted on
    // spacers (body lift kit), the chassis sits higher off the ground but the
    // wheels stay on the ground. So we drop the wheel mesh in chassis-local
    // space by the same amount the chassis floated up, so the wheels visually
    // remain grounded while the body has noticeable fender clearance.
    let extra_suspension = mods.suspension_len() - crate::vehicle_mods::BASE_SUSPENSION_LEN;
    let body_lift_y      = mods.body_lift_y();
    let visual_drop      = extra_suspension + body_lift_y;

    for (mut transform, mut wheel) in wheel_q.iter_mut() {
        wheel.spin += (speed * dt / wheel_r) + slip_omega * dt * input.drive.signum();
        let base_offset    = WHEEL_OFFSETS[wheel.index];
        let compress_delta = Vec3::new(0.0, -wheel.current_compression - visual_drop, 0.0);
        // base_rot aligns Cylinder Y-axis → chassis X (lateral). spin_rot around local Y
        // then becomes rotation around chassis X: wheel rolls forward correctly.
        let base_rot = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
        let spin_rot = Quat::from_rotation_y(wheel.spin);
        // Front wheels also yaw with steering input — applied LAST so the
        // steer rotation happens around the chassis-local Y axis (vertical).
        let steer_rot = if wheel.index < 2 {
            Quat::from_rotation_y(steer_yaw)
        } else {
            Quat::IDENTITY
        };
        transform.translation = base_offset + compress_delta;
        transform.rotation    = steer_rot * base_rot * spin_rot;
    }
}
