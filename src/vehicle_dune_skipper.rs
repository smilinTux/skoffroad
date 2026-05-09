// Dune Skipper — open-frame desert buggy.
//
// Silhouette hallmarks:
//   • No body panels — all exposed tube frame (Cylinders throughout)
//   • Very low chassis stance; long-travel suspension implied by exposed shocks
//   • Giant knobby tires (scaled up relative to Skrambler)
//   • Rear-mounted air-cooled engine (big cuboid behind the seat)
//   • Minimal fairing at front — just a looped bumper tube + tiny headlamp pods
//   • Bucket seat (small cuboid), roll hoop behind it
//   • Diagonal X-braces in the frontal and rear bays for stiffness detail
//   • Exposed exhaust stack rising on the right side
//
// All shapes are Bevy primitives only (Cuboid / Cylinder / Sphere).
// Call `spawn_dune_skipper` to get a Vec<Entity> ready for `add_children`.

use bevy::prelude::*;
use crate::variants::VariantSkin;

// ---------------------------------------------------------------------------
// Internal material helpers
// ---------------------------------------------------------------------------

fn tube_mat(mats: &mut Assets<StandardMaterial>, c: Color) -> Handle<StandardMaterial> {
    mats.add(StandardMaterial {
        base_color: c,
        perceptual_roughness: 0.65,
        metallic: 0.30,
        ..default()
    })
}

fn dark_mat(mats: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
    mats.add(StandardMaterial {
        base_color: Color::srgb(0.10, 0.10, 0.10),
        perceptual_roughness: 0.90,
        ..default()
    })
}

fn engine_mat(mats: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
    mats.add(StandardMaterial {
        base_color: Color::srgb(0.24, 0.20, 0.16),
        perceptual_roughness: 0.80,
        ..default()
    })
}

fn exhaust_mat(mats: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
    mats.add(StandardMaterial {
        base_color: Color::srgb(0.32, 0.30, 0.28),
        perceptual_roughness: 0.70,
        metallic: 0.50,
        ..default()
    })
}

fn shock_mat(mats: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
    mats.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.55, 0.58),
        perceptual_roughness: 0.40,
        metallic: 0.80,
        ..default()
    })
}

fn headlight_mat(mats: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
    mats.add(StandardMaterial {
        base_color: Color::srgb(1.0, 1.0, 0.9),
        emissive: LinearRgba::rgb(4.0, 4.0, 3.0),
        perceptual_roughness: 0.05,
        ..default()
    })
}

// ---------------------------------------------------------------------------
// Spawn function
// ---------------------------------------------------------------------------

/// Spawn a Dune Skipper body and return its child entities.
/// Attach them to the chassis: `commands.entity(chassis).add_children(&kids)`.
pub fn spawn_dune_skipper(
    cmds: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
) -> Vec<Entity> {
    let mut ids: Vec<Entity> = Vec::new();

    // Palette: safety-orange frame tubes, dark seat, raw-metal engine/exhaust
    let frame_m   = tube_mat(mats, Color::srgb(0.95, 0.42, 0.04)); // vivid orange
    let seat_m    = dark_mat(mats);
    let engine_m  = engine_mat(mats);
    let exhaust_m = exhaust_mat(mats);
    let shock_m   = shock_mat(mats);
    let hl_m      = headlight_mat(mats);

    // ========================================================================
    // Main cage frame — all cylinders, radius 0.048 m
    // ========================================================================
    //
    // Layout (top-down, Z= forward/rear):
    //   4 corner nodes at ±0.72 X, Z = ±1.10
    //   Frame sits low: bottom rail at Y = -0.32, roll hoop apex at Y = 0.85
    // ========================================================================

    let r = 0.048_f32; // standard tube radius
    let rotx_90 = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
    let rotz_90 = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);

    // ---- Bottom longitudinal side rails (2 per side, Z-axis) ---------------
    // Each side rail runs the full length; split into front and rear sections
    // so they connect at the seat crossmember.
    let side_rail = meshes.add(Cylinder::new(r, 2.24)); // full length
    for sx in [-0.72_f32, 0.72] {
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(side_rail.clone()),
            MeshMaterial3d(frame_m.clone()),
            Transform::from_translation(Vec3::new(sx, -0.32, 0.0))
                .with_rotation(rotx_90),
        )).id());
    }

    // ---- Bottom front cross-bar (lateral, X-axis) --------------------------
    let front_cross = meshes.add(Cylinder::new(r, 1.48));
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(front_cross.clone()),
        MeshMaterial3d(frame_m.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.32, -1.10))
            .with_rotation(rotz_90),
    )).id());

    // ---- Bottom rear cross-bar (lateral, X-axis) ---------------------------
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(front_cross.clone()),
        MeshMaterial3d(frame_m.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.32, 1.10))
            .with_rotation(rotz_90),
    )).id());

    // ---- Bottom mid cross-bar (at seat bulkhead) ----------------------------
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(front_cross.clone()),
        MeshMaterial3d(frame_m.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.32, 0.10))
            .with_rotation(rotz_90),
    )).id());

    // ---- Vertical corner posts (4) — front low posts, rear tall roll hoop --
    // Front posts: short, angled forward-down for the nose cage
    let front_post = meshes.add(Cylinder::new(r, 0.68));
    for sx in [-0.72_f32, 0.72] {
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(front_post.clone()),
            MeshMaterial3d(frame_m.clone()),
            Transform::from_translation(Vec3::new(sx, 0.02, -1.10))
                .with_rotation(Quat::from_euler(EulerRot::XYZ, 0.0, 0.0, 0.0)),
        )).id());
    }
    // Rear roll hoop posts: tall
    let rear_post = meshes.add(Cylinder::new(r, 1.20));
    for sx in [-0.72_f32, 0.72] {
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(rear_post.clone()),
            MeshMaterial3d(frame_m.clone()),
            Transform::from_translation(Vec3::new(sx, 0.28, 0.10)),
        )).id());
    }

    // ---- Roll hoop top arch (X-axis cross-bar at apex) ---------------------
    let hoop_top = meshes.add(Cylinder::new(r, 1.48));
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(hoop_top.clone()),
        MeshMaterial3d(frame_m.clone()),
        Transform::from_translation(Vec3::new(0.0, 0.88, 0.10))
            .with_rotation(rotz_90),
    )).id());

    // ---- Diagonal gussets on rear roll hoop (X) ----------------------------
    // Two crossing braces from base corners to top centre — gives a classic
    // sand-rail "X brace" look.
    for sign in [-1_f32, 1.0] {
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(meshes.add(Cylinder::new(r * 0.85, 1.28))),
            MeshMaterial3d(frame_m.clone()),
            Transform::from_translation(Vec3::new(0.0, 0.28, 0.10))
                .with_rotation(Quat::from_euler(EulerRot::XYZ,
                    0.0, 0.0, sign * 48_f32.to_radians())),
        )).id());
    }

    // ---- Front nose cage — looped bumper tube + two diagonal struts --------
    // The bumper is a horizontal cylinder spanning the front of the frame.
    let nose_tube = meshes.add(Cylinder::new(r, 1.50));
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(nose_tube.clone()),
        MeshMaterial3d(frame_m.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.20, -1.28))
            .with_rotation(rotz_90),
    )).id());
    // Two angled nose struts connecting bumper tube to front post tops
    let nose_strut = meshes.add(Cylinder::new(r * 0.85, 0.62));
    for sx in [-0.72_f32, 0.72] {
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(nose_strut.clone()),
            MeshMaterial3d(frame_m.clone()),
            Transform::from_translation(Vec3::new(sx * 0.45, -0.10, -1.18))
                .with_rotation(Quat::from_euler(EulerRot::XYZ,
                    20_f32.to_radians(), 0.0, sx.signum() * -22_f32.to_radians())),
        )).id());
    }

    // ---- Seat — single bucket (this buggy is a one-seater) -----------------
    // Small cuboid, set slightly back from centre.
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(0.50, 0.26, 0.48))),
        MeshMaterial3d(seat_m.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.18, -0.28)),
    )).id());
    // Seat back
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(0.50, 0.42, 0.08))),
        MeshMaterial3d(seat_m.clone()),
        Transform::from_translation(Vec3::new(0.0, 0.04, 0.00)),
    )).id());

    // ---- Steering wheel — thin cylinder angled toward driver ---------------
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cylinder::new(0.16, 0.04))),
        MeshMaterial3d(seat_m.clone()),
        Transform::from_translation(Vec3::new(0.0, 0.22, -0.62))
            .with_rotation(Quat::from_rotation_x(55_f32.to_radians())),
    )).id());

    // ---- Rear engine block — big air-cooled flat-four box ------------------
    // Sits behind the driver's seat, between the rear posts.
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(0.88, 0.50, 0.65))),
        MeshMaterial3d(engine_m.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.08, 0.72)),
    )).id());
    // Air-cooling fins — 4 thin horizontal slabs across the engine face
    let fin = meshes.add(Cuboid::new(0.90, 0.06, 0.06));
    let fin_m = dark_mat(mats);
    for i in 0..4_i32 {
        let y = -0.18 + i as f32 * 0.12;
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(fin.clone()),
            MeshMaterial3d(fin_m.clone()),
            Transform::from_translation(Vec3::new(0.0, y, 0.40)),
        )).id());
    }

    // ---- Exhaust stack — single upright pipe on the right side -------------
    // Rises from the engine up through the roll hoop area.
    let ex_lower = meshes.add(Cylinder::new(0.052, 0.55));
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(ex_lower.clone()),
        MeshMaterial3d(exhaust_m.clone()),
        Transform::from_translation(Vec3::new(0.68, 0.14, 0.68)),
    )).id());
    // Exit elbow (short horizontal cap at top)
    let ex_elbow = meshes.add(Cylinder::new(0.052, 0.22));
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(ex_elbow.clone()),
        MeshMaterial3d(exhaust_m.clone()),
        Transform::from_translation(Vec3::new(0.68, 0.42, 0.52))
            .with_rotation(rotx_90),
    )).id());

    // ---- Exposed shock absorbers (4 — one per corner) ----------------------
    // Long angled cylinders representing coilover shocks.
    let shock = meshes.add(Cylinder::new(0.042, 0.58));
    for (sx, sz, angle_z) in [
        (-0.65_f32, -1.05,  18_f32),
        ( 0.65,     -1.05, -18_f32),
        (-0.65,      1.05,  18_f32),
        ( 0.65,      1.05, -18_f32),
    ] {
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(shock.clone()),
            MeshMaterial3d(shock_m.clone()),
            Transform::from_translation(Vec3::new(sx, -0.14, sz))
                .with_rotation(Quat::from_rotation_z(angle_z.to_radians())),
        )).id());
    }

    // ---- Headlamp pods — two small spheres low on the nose cage ------------
    let hm = meshes.add(Sphere::new(0.09));
    for sx in [-0.40_f32, 0.40] {
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(hm.clone()),
            MeshMaterial3d(hl_m.clone()),
            Transform::from_translation(Vec3::new(sx, -0.16, -1.36)),
        )).id());
    }

    ids
}
