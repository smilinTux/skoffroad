// Hauler SK — mid-size pickup truck.
//
// Silhouette hallmarks:
//   • Two-tier silhouette: short cab (cuboid) + open flatbed (box frame) behind it
//   • Cab taller than bed walls — classic pickup step-down look
//   • Round-stacked dual headlights (stacked vertically, 2 per side)
//   • Wide horizontal grille with chrome surround
//   • Larger rear tires than front (same physics, just visual scale note in comments)
//   • Drop-down tailgate (small flat cuboid at bed rear, slightly offset = open position)
//   • Side toolboxes on bed rails (cuboids above the bed side walls)
//   • Exhaust pipe under rear, driver side
//   • Front tow hooks (small cuboids at bumper corners)
//   • Mud flaps behind rear wheels
//
// All shapes are Bevy primitives only (Cuboid / Cylinder / Sphere).
// Call `spawn_hauler` to get a Vec<Entity> ready for `add_children`.

use bevy::prelude::*;
use crate::variants::VariantSkin;

// ---------------------------------------------------------------------------
// Internal material helpers
// ---------------------------------------------------------------------------

fn body_mat(mats: &mut Assets<StandardMaterial>, c: Color) -> Handle<StandardMaterial> {
    mats.add(StandardMaterial {
        base_color: c,
        perceptual_roughness: 0.52,
        metallic: 0.12,
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

fn chrome_mat(mats: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
    mats.add(StandardMaterial {
        base_color: Color::srgb(0.82, 0.82, 0.88),
        metallic: 0.95,
        perceptual_roughness: 0.14,
        ..default()
    })
}

fn glass_mat(mats: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
    mats.add(StandardMaterial {
        base_color: Color::srgba(0.55, 0.75, 0.85, 0.40),
        perceptual_roughness: 0.10,
        alpha_mode: AlphaMode::Blend,
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

/// Spawn a Hauler SK body and return its child entities.
/// Attach them to the chassis: `commands.entity(chassis).add_children(&kids)`.
pub fn spawn_hauler(
    cmds: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
) -> Vec<Entity> {
    let mut ids: Vec<Entity> = Vec::new();

    // Palette: deep forest green cab, matte grey bed, dark bumpers/trim
    let cab_m    = body_mat(mats, Color::srgb(0.18, 0.38, 0.22)); // forest green
    let bed_m    = body_mat(mats, Color::srgb(0.28, 0.28, 0.30)); // dark pewter
    let bumper_m = dark_mat(mats);
    let chrome_m = chrome_mat(mats);
    let glass_m  = glass_mat(mats);
    let hl_m     = headlight_mat(mats);

    // ========================================================================
    // Cab section — forward half of vehicle, Z ≈ -2.2 to 0.2
    // The cab is taller than the Skrambler body to read as a full-size truck.
    // ========================================================================

    // Main cab body (short but wide and tall)
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(2.10, 1.10, 2.40))),
        MeshMaterial3d(cab_m.clone()),
        Transform::from_translation(Vec3::new(0.0, 0.08, -0.90)),
    )).id());

    // Hood — long, wide, slightly lower than cab top
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(2.10, 0.22, 1.60))),
        MeshMaterial3d(cab_m.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.10, -2.10)),
    )).id());

    // Windshield — angled more aggressively than the Highland
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(1.98, 0.05, 0.88))),
        MeshMaterial3d(glass_m.clone()),
        Transform::from_translation(Vec3::new(0.0, 0.62, -1.42))
            .with_rotation(Quat::from_rotation_x(-20_f32.to_radians())),
    )).id());

    // Rear cab window (smaller, sits at cabin rear)
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(1.96, 0.05, 0.52))),
        MeshMaterial3d(glass_m.clone()),
        Transform::from_translation(Vec3::new(0.0, 0.54, 0.08))
            .with_rotation(Quat::from_rotation_x(-8_f32.to_radians())),
    )).id());

    // ========================================================================
    // Bed section — rear half of vehicle, Z ≈ 0.2 to 2.3
    // Cab and bed are separate cuboids — the step-down is the silhouette key.
    // ========================================================================

    // Bed floor — low, wide, flat
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(2.10, 0.10, 2.10))),
        MeshMaterial3d(bed_m.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.44, 1.25)),
    )).id());

    // Bed left side wall
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(0.08, 0.52, 2.10))),
        MeshMaterial3d(bed_m.clone()),
        Transform::from_translation(Vec3::new(-1.01, -0.13, 1.25)),
    )).id());

    // Bed right side wall
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(0.08, 0.52, 2.10))),
        MeshMaterial3d(bed_m.clone()),
        Transform::from_translation(Vec3::new( 1.01, -0.13, 1.25)),
    )).id());

    // Bed front wall (cab-to-bed divider)
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(2.10, 0.52, 0.08))),
        MeshMaterial3d(bed_m.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.13, 0.21)),
    )).id());

    // Tailgate — slightly protruding from bed rear, same width as bed.
    // Positioned as "slightly dropped" (small downward tilt = visual interest).
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(2.10, 0.08, 0.52))),
        MeshMaterial3d(cab_m.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.44, 2.36))
            .with_rotation(Quat::from_rotation_x(15_f32.to_radians())),
    )).id());

    // Tailgate handle — small horizontal cylinder
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cylinder::new(0.035, 0.50))),
        MeshMaterial3d(chrome_m.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.30, 2.34))
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
    )).id());

    // ---- Toolboxes on bed rails (both sides, just inside the bed walls) ----
    // Classic truck bed toolboxes: flat-topped cuboids sitting on the rails.
    let toolbox = meshes.add(Cuboid::new(0.26, 0.28, 1.20));
    let toolbox_m = dark_mat(mats);
    for sx in [-0.88_f32, 0.88] {
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(toolbox.clone()),
            MeshMaterial3d(toolbox_m.clone()),
            Transform::from_translation(Vec3::new(sx, 0.00, 1.25)),
        )).id());
    }

    // ========================================================================
    // Front end — grille, bumper, headlights
    // ========================================================================

    // Front bumper — thick, full width, with a slight step profile
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(2.30, 0.26, 0.16))),
        MeshMaterial3d(bumper_m.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.36, -2.94)),
    )).id());

    // Chrome bumper guard bar (sits proud of bumper face)
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(2.26, 0.08, 0.10))),
        MeshMaterial3d(chrome_m.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.28, -2.96)),
    )).id());

    // Front tow hooks — two small cuboids at bumper outer corners
    let hook = meshes.add(Cuboid::new(0.16, 0.12, 0.22));
    for sx in [-1.00_f32, 1.00] {
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(hook.clone()),
            MeshMaterial3d(chrome_m.clone()),
            Transform::from_translation(Vec3::new(sx, -0.42, -2.92)),
        )).id());
    }

    // Grille background
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(1.60, 0.52, 0.10))),
        MeshMaterial3d(dark_mat(mats)),
        Transform::from_translation(Vec3::new(0.0, -0.10, -2.90)),
    )).id());

    // Chrome grille surround (top and bottom bars)
    let gsurround = meshes.add(Cuboid::new(1.64, 0.06, 0.08));
    let gsurr_m = chrome_mat(mats);
    for gy in [-0.36_f32, 0.14] {
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(gsurround.clone()),
            MeshMaterial3d(gsurr_m.clone()),
            Transform::from_translation(Vec3::new(0.0, gy, -2.88)),
        )).id());
    }
    // Vertical chrome centre bar
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(0.06, 0.52, 0.08))),
        MeshMaterial3d(chrome_mat(mats)),
        Transform::from_translation(Vec3::new(0.0, -0.10, -2.88)),
    )).id());

    // Stacked dual headlights — 2 per side, large rectangular
    // Upper lamp: taller/narrower; lower: wider. Both on the outer edges.
    let hl_upper = meshes.add(Cuboid::new(0.28, 0.20, 0.06));
    let hl_lower = meshes.add(Cuboid::new(0.28, 0.16, 0.06));
    for sx in [-1.00_f32, 1.00] {
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(hl_upper.clone()),
            MeshMaterial3d(hl_m.clone()),
            Transform::from_translation(Vec3::new(sx, 0.10, -2.92)),
        )).id());
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(hl_lower.clone()),
            MeshMaterial3d(hl_m.clone()),
            Transform::from_translation(Vec3::new(sx, -0.10, -2.92)),
        )).id());
    }
    // Chrome bezel frame around each headlight cluster
    let hl_bezel = meshes.add(Cuboid::new(0.32, 0.40, 0.05));
    let bezel_m = chrome_mat(mats);
    for sx in [-1.00_f32, 1.00] {
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(hl_bezel.clone()),
            MeshMaterial3d(bezel_m.clone()),
            Transform::from_translation(Vec3::new(sx, 0.0, -2.93)),
        )).id());
    }

    // ========================================================================
    // Exterior trim
    // ========================================================================

    // Side mirrors — large flat units at front A-pillar
    let mirror = meshes.add(Cuboid::new(0.09, 0.22, 0.18));
    let mirror_m = dark_mat(mats);
    for sx in [-1.10_f32, 1.10] {
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(mirror.clone()),
            MeshMaterial3d(mirror_m.clone()),
            Transform::from_translation(Vec3::new(sx, 0.42, -1.35)),
        )).id());
    }

    // Fender flares — boxy truck-style, front and rear
    let fflare = meshes.add(Cuboid::new(0.12, 0.16, 1.05));
    let fflare_m = dark_mat(mats);
    for (sx, fz) in [
        (-1.08_f32, -1.50_f32), (1.08, -1.50),  // front
        (-1.08,      1.50),     (1.08,  1.50),   // rear
    ] {
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(fflare.clone()),
            MeshMaterial3d(fflare_m.clone()),
            Transform::from_translation(Vec3::new(sx, -0.20, fz)),
        )).id());
    }

    // Rear mud flaps — flat thin cuboids hanging behind rear wheels
    let mudflap = meshes.add(Cuboid::new(0.06, 0.38, 0.40));
    let mudflap_m = dark_mat(mats);
    for sx in [-1.00_f32, 1.00] {
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(mudflap.clone()),
            MeshMaterial3d(mudflap_m.clone()),
            Transform::from_translation(Vec3::new(sx, -0.30, 1.65)),
        )).id());
    }

    // Step bars (below cab doors on each side)
    let step = meshes.add(Cuboid::new(0.10, 0.09, 2.00));
    let step_m = dark_mat(mats);
    for sx in [-1.08_f32, 1.08] {
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(step.clone()),
            MeshMaterial3d(step_m.clone()),
            Transform::from_translation(Vec3::new(sx, -0.62, -0.90)),
        )).id());
    }

    // Rear bumper
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(2.20, 0.20, 0.14))),
        MeshMaterial3d(bumper_m.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.38, 2.30)),
    )).id());

    // Exhaust pipe — single cylinder protruding under rear on driver side
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cylinder::new(0.055, 0.36))),
        MeshMaterial3d(chrome_mat(mats)),
        Transform::from_translation(Vec3::new(0.60, -0.54, 2.42))
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
    )).id());

    // Fuel cap (passenger side rear quarter)
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cylinder::new(0.06, 0.05))),
        MeshMaterial3d(chrome_mat(mats)),
        Transform::from_translation(Vec3::new(-1.06, 0.08, 1.80))
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
    )).id());

    ids
}
