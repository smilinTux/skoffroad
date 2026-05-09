// Highland SK — Bronco-style full-size SUV.
//
// Silhouette hallmarks:
//   • Wide, tall boxy body — squarer than the Skrambler, more upright than the Bronco
//   • Separate hardtop section (cuboid sitting flush on the cabin roof)
//   • Square headlights with chrome bezels (4 lamps total, stacked 2×2 each side)
//   • Flat vertical grille with horizontal bars across the full width
//   • Thick C-pillar cladding (rear quarter panels) — two-tone paint vibe
//   • Roof rack + roof-mounted spare tire
//   • Heavy front skid plate and side rock sliders
//
// All shapes are Bevy primitives only (Cuboid / Cylinder / Sphere).
// Call `spawn_highland` to get a Vec<Entity> of children ready to be
// `add_children`-ed onto the chassis entity.

use bevy::prelude::*;
use crate::variants::VariantSkin;

// ---------------------------------------------------------------------------
// Internal material helpers (mirror the style in variants.rs)
// ---------------------------------------------------------------------------

fn body_mat(mats: &mut Assets<StandardMaterial>, c: Color) -> Handle<StandardMaterial> {
    mats.add(StandardMaterial {
        base_color: c,
        perceptual_roughness: 0.55,
        metallic: 0.10,
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
        base_color: Color::srgb(0.82, 0.82, 0.86),
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
// Spawn function — returns a list of child entities to attach to the chassis
// ---------------------------------------------------------------------------

/// Spawn a Highland SK body and return its child entities.
/// Attach them to the chassis: `commands.entity(chassis).add_children(&kids)`.
pub fn spawn_highland(
    cmds: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
) -> Vec<Entity> {
    let mut ids: Vec<Entity> = Vec::new();

    // Two-tone palette: sand/khaki lower body, white upper cabin + hardtop
    let lower_m  = body_mat(mats, Color::srgb(0.72, 0.62, 0.42)); // sand
    let upper_m  = body_mat(mats, Color::srgb(0.93, 0.92, 0.88)); // off-white
    let bumper_m = dark_mat(mats);
    let chrome_m = chrome_mat(mats);
    let glass_m  = glass_mat(mats);
    let hl_m     = headlight_mat(mats);
    let rack_m   = dark_mat(mats);

    // ---- Main lower body (wide, tall, boxy) --------------------------------
    // Wider and taller than Skrambler (2.0×0.8×4.0 vs 2.0×0.8×4.0), but we
    // explicitly make it feel bigger: 2.2 wide, 0.95 tall, 4.4 long.
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(2.2, 0.95, 4.4))),
        MeshMaterial3d(lower_m.clone()),
        Transform::IDENTITY,
    )).id());

    // ---- Hardtop (sits on top of lower body — the defining feature) --------
    // A solid flat-roofed cap, flush with the body sides, covering the full
    // cabin (front half). The rear half is open like a short-bed SUV.
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(2.20, 0.55, 2.60))),
        MeshMaterial3d(upper_m.clone()),
        Transform::from_translation(Vec3::new(0.0, 0.75, -0.70)),
    )).id());

    // ---- Hood — wide, flat, slightly forward of body -----------------------
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(2.10, 0.22, 1.50))),
        MeshMaterial3d(lower_m.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.10, -2.00)),
    )).id());

    // ---- Windshield — near-vertical (very upright Bronco/G-Wagen style) ----
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(2.05, 0.05, 0.92))),
        MeshMaterial3d(glass_m.clone()),
        Transform::from_translation(Vec3::new(0.0, 0.55, -1.22))
            .with_rotation(Quat::from_rotation_x(-12_f32.to_radians())),
    )).id());

    // ---- Rear glass (liftgate window, behind the hardtop) ------------------
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(2.05, 0.05, 0.55))),
        MeshMaterial3d(glass_m.clone()),
        Transform::from_translation(Vec3::new(0.0, 0.88, 0.62))
            .with_rotation(Quat::from_rotation_x(-8_f32.to_radians())),
    )).id());

    // ---- C-pillar cladding — dark contrasting trim on rear quarters --------
    // Two thick vertical panel stripes, one per side, at the B/C pillar.
    let cladding = meshes.add(Cuboid::new(0.08, 0.55, 0.60));
    for sx in [-1.12_f32, 1.12] {
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(cladding.clone()),
            MeshMaterial3d(bumper_m.clone()),
            Transform::from_translation(Vec3::new(sx, 0.70, 0.65)),
        )).id());
    }

    // ---- Front bumper — thick box, full width ------------------------------
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(2.40, 0.28, 0.18))),
        MeshMaterial3d(bumper_m.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.38, -2.38)),
    )).id());

    // ---- Rear bumper -------------------------------------------------------
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(2.30, 0.22, 0.15))),
        MeshMaterial3d(bumper_m.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.38, 2.28)),
    )).id());

    // ---- Grille — wide horizontal-bar style (5 bars across full width) -----
    // Grille background fill
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(1.80, 0.50, 0.10))),
        MeshMaterial3d(dark_mat(mats)),
        Transform::from_translation(Vec3::new(0.0, -0.10, -2.34)),
    )).id());
    // Horizontal chrome grille bars (5 slats)
    let bar_mesh = meshes.add(Cuboid::new(1.78, 0.05, 0.08));
    let bar_m = chrome_mat(mats);
    for i in 0..5_i32 {
        let y = -0.30 + i as f32 * 0.10;
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(bar_mesh.clone()),
            MeshMaterial3d(bar_m.clone()),
            Transform::from_translation(Vec3::new(0.0, y, -2.32)),
        )).id());
    }

    // ---- Headlights — square, 2×2 stack each side (4-lamp cluster) ---------
    // Each cluster has a top and bottom lamp, separated by a thin chrome strip.
    let sq_hl = meshes.add(Cuboid::new(0.32, 0.18, 0.06));
    for (sx, offset_y) in [
        (-0.94_f32, 0.10_f32),
        (-0.94,    -0.10),
        ( 0.94,     0.10),
        ( 0.94,    -0.10),
    ] {
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(sq_hl.clone()),
            MeshMaterial3d(hl_m.clone()),
            Transform::from_translation(Vec3::new(sx, offset_y, -2.36)),
        )).id());
    }
    // Chrome bezel strip between lamp rows
    let bezel = meshes.add(Cuboid::new(0.34, 0.03, 0.05));
    for sx in [-0.94_f32, 0.94] {
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(bezel.clone()),
            MeshMaterial3d(chrome_mat(mats)),
            Transform::from_translation(Vec3::new(sx, 0.0, -2.35)),
        )).id());
    }

    // ---- Front skid plate — heavy flat plate under the front bumper --------
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(2.30, 0.08, 0.60))),
        MeshMaterial3d(bumper_m.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.56, -2.10)),
    )).id());

    // ---- Rock sliders — long sturdy steps below doors each side ------------
    let slider = meshes.add(Cuboid::new(0.14, 0.12, 3.20));
    let slider_m = dark_mat(mats);
    for sx in [-1.14_f32, 1.14] {
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(slider.clone()),
            MeshMaterial3d(slider_m.clone()),
            Transform::from_translation(Vec3::new(sx, -0.60, 0.0)),
        )).id());
    }

    // ---- Fender flares — boxy, not rounded (Bronco / Defender style) -------
    let flare = meshes.add(Cuboid::new(0.16, 0.14, 1.10));
    for sx in [-1.12_f32, 1.12] {
        // Front fenders
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(flare.clone()),
            MeshMaterial3d(bumper_m.clone()),
            Transform::from_translation(Vec3::new(sx, -0.18, -1.50)),
        )).id());
        // Rear fenders
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(flare.clone()),
            MeshMaterial3d(bumper_m.clone()),
            Transform::from_translation(Vec3::new(sx, -0.18, 1.50)),
        )).id());
    }

    // ---- Large side mirrors — tall square units at front door corners -------
    let mirror = meshes.add(Cuboid::new(0.09, 0.24, 0.18));
    let mirror_m = dark_mat(mats);
    for sx in [-1.16_f32, 1.16] {
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(mirror.clone()),
            MeshMaterial3d(mirror_m.clone()),
            Transform::from_translation(Vec3::new(sx, 0.42, -0.80)),
        )).id());
    }

    // ---- Roof rack — 4 posts + 2 longitudinal rails + 3 cross-bars ---------
    // Sits on top of the hardtop section.
    let rpost = meshes.add(Cylinder::new(0.035, 0.20));
    for (px, pz) in [(-0.90_f32, -1.00), (0.90, -1.00), (-0.90, 0.10), (0.90, 0.10)] {
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(rpost.clone()),
            MeshMaterial3d(rack_m.clone()),
            Transform::from_translation(Vec3::new(px, 1.12, pz)),
        )).id());
    }
    let rlong = meshes.add(Cuboid::new(0.04, 0.04, 1.15));
    for rx in [-0.90_f32, 0.90] {
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(rlong.clone()),
            MeshMaterial3d(rack_m.clone()),
            Transform::from_translation(Vec3::new(rx, 1.22, -0.45)),
        )).id());
    }
    let rlat = meshes.add(Cuboid::new(1.84, 0.04, 0.04));
    for rz in [-1.00_f32, -0.45, 0.10] {
        ids.push(cmds.spawn((
            VariantSkin,
            Mesh3d(rlat.clone()),
            MeshMaterial3d(rack_m.clone()),
            Transform::from_translation(Vec3::new(0.0, 1.22, rz)),
        )).id());
    }

    // ---- Roof-mounted spare tire (on the hardtop rear edge) ----------------
    // Cylinder lying flat (axis X, rotated 90° on Z then 90° on X) centered
    // on the rear of the hardtop.
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cylinder::new(0.40, 0.20))),
        MeshMaterial3d(dark_mat(mats)),
        Transform::from_translation(Vec3::new(0.0, 1.22, 0.22))
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
    )).id());

    // ---- Tow hitch receiver — small square nub on rear centre --------------
    ids.push(cmds.spawn((
        VariantSkin,
        Mesh3d(meshes.add(Cuboid::new(0.14, 0.12, 0.20))),
        MeshMaterial3d(chrome_m.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.45, 2.38)),
    )).id());

    ids
}
