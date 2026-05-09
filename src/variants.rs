// Vehicle silhouette variants — cycle with Backslash (\).
//
// Despawn rules:
//   Leaving JeepTJ   → despawn DefaultSkin children of chassis
//   Leaving non-Jeep → despawn VariantSkin children of chassis
//   Arriving JeepTJ  → spawn_jeep_default (tagged DefaultSkin + VariantSkin)
//
// Mesh children are added via add_children (the only reliable parent API in 0.18).
//
// Sprint 47 additions — three new procedural vehicles:
//   HighlandSK   (src/vehicle_highland.rs)     — Bronco-style full-size SUV
//   DuneSkipper  (src/vehicle_dune_skipper.rs) — open-frame desert buggy
//   HaulerSK     (src/vehicle_hauler.rs)       — cab+bed pickup truck

use bevy::prelude::*;
use crate::vehicle::{DefaultSkin, VehicleRoot};
use crate::vehicle_highland::spawn_highland;
use crate::vehicle_dune_skipper::spawn_dune_skipper;
use crate::vehicle_hauler::spawn_hauler;

// ---- Plugin -----------------------------------------------------------------

pub struct VariantsPlugin;
impl Plugin for VariantsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VehicleVariant>()
           .init_resource::<VariantHudTimer>()
           .add_systems(Startup, spawn_variant_hud)
           .add_systems(Update, (cycle_variant, update_variant_hud)
               .run_if(resource_exists::<VehicleRoot>));
    }
}

// ---- Public API -------------------------------------------------------------

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub enum VehicleVariant {
    #[default] JeepTJ,
    FordBronco,
    Pickup,
    Hummer,
    Buggy,
    /// Sprint 47: Bronco-style full-size SUV with hardtop
    HighlandSK,
    /// Sprint 47: open-frame desert buggy with rear engine
    DuneSkipper,
    /// Sprint 47: cab + open bed pickup truck
    HaulerSK,
}

impl VehicleVariant {
    pub fn next(self) -> Self {
        match self {
            Self::JeepTJ      => Self::FordBronco,
            Self::FordBronco  => Self::Pickup,
            Self::Pickup      => Self::Hummer,
            Self::Hummer      => Self::Buggy,
            Self::Buggy       => Self::HighlandSK,
            Self::HighlandSK  => Self::DuneSkipper,
            Self::DuneSkipper => Self::HaulerSK,
            Self::HaulerSK    => Self::JeepTJ,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::JeepTJ      => "Jeep TJ",
            Self::FordBronco  => "Ford Bronco",
            Self::Pickup      => "Pickup",
            Self::Hummer      => "Hummer",
            Self::Buggy       => "Buggy",
            Self::HighlandSK  => "Highland SK",
            Self::DuneSkipper => "Dune Skipper",
            Self::HaulerSK    => "Hauler SK",
        }
    }
}

/// Marks mesh children that belong to a spawned variant skin (including the
/// re-spawned Jeep default, which also carries DefaultSkin).
#[derive(Component)]
pub struct VariantSkin;

// ---- HUD --------------------------------------------------------------------

#[derive(Resource, Default)] struct VariantHudTimer(f32);
#[derive(Component)]         struct VariantHud;
#[derive(Component)]         struct VariantHudText;

fn spawn_variant_hud(mut commands: Commands) {
    commands.spawn((
        VariantHud,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(35.0), top: Val::Px(60.0),
            width: Val::Px(220.0), padding: UiRect::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.07, 0.80)),
        Visibility::Hidden,
    )).with_children(|p| {
        p.spawn((
            VariantHudText, Text::new("VEHICLE: Jeep TJ"),
            TextFont { font_size: 14.0, ..default() },
            TextColor(Color::srgb(0.85, 0.90, 0.60)),
        ));
    });
}

fn update_variant_hud(
    time: Res<Time>, mut timer: ResMut<VariantHudTimer>,
    mut hud_q: Query<&mut Visibility, With<VariantHud>>,
) {
    timer.0 = (timer.0 - time.delta_secs()).max(0.0);
    let v = if timer.0 > 0.0 { Visibility::Inherited } else { Visibility::Hidden };
    for mut vis in &mut hud_q { *vis = v; }
}

// ---- Cycling ----------------------------------------------------------------

fn cycle_variant(
    keys: Res<ButtonInput<KeyCode>>,
    vehicle: Res<VehicleRoot>,
    mut variant: ResMut<VehicleVariant>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    children_q: Query<&Children>,
    default_skin_q: Query<Entity, With<DefaultSkin>>,
    variant_skin_q: Query<Entity, With<VariantSkin>>,
    mut text_q: Query<&mut Text, With<VariantHudText>>,
    mut timer: ResMut<VariantHudTimer>,
) {
    if !keys.just_pressed(KeyCode::Backslash) { return }

    let old = *variant;
    let new = variant.next();
    *variant = new;

    let chassis = vehicle.chassis;
    // Collect chassis children once to avoid touching non-chassis skin entities.
    let cc: std::collections::HashSet<Entity> = children_q
        .get(chassis).map(|c| c.iter().collect()).unwrap_or_default();

    if old == VehicleVariant::JeepTJ {
        for e in default_skin_q.iter() { if cc.contains(&e) { commands.entity(e).despawn(); } }
    } else {
        for e in variant_skin_q.iter() { if cc.contains(&e) { commands.entity(e).despawn(); } }
    }

    let kids = match new {
        VehicleVariant::JeepTJ      => spawn_jeep_default(&mut commands, &mut meshes, &mut materials),
        VehicleVariant::FordBronco  => spawn_bronco(&mut commands, &mut meshes, &mut materials),
        VehicleVariant::Pickup      => spawn_pickup(&mut commands, &mut meshes, &mut materials),
        VehicleVariant::Hummer      => spawn_hummer(&mut commands, &mut meshes, &mut materials),
        VehicleVariant::Buggy       => spawn_buggy(&mut commands, &mut meshes, &mut materials),
        // Sprint 47 — three new procedural vehicles
        VehicleVariant::HighlandSK  => spawn_highland(&mut commands, &mut meshes, &mut materials),
        VehicleVariant::DuneSkipper => spawn_dune_skipper(&mut commands, &mut meshes, &mut materials),
        VehicleVariant::HaulerSK    => spawn_hauler(&mut commands, &mut meshes, &mut materials),
    };
    commands.entity(chassis).add_children(&kids);

    for mut text in &mut text_q { text.0 = format!("VEHICLE: {}", new.name()); }
    timer.0 = 2.0;
}

// ---- Skin helpers -----------------------------------------------------------

fn mat(mats: &mut Assets<StandardMaterial>, c: Color) -> Handle<StandardMaterial> {
    mats.add(StandardMaterial { base_color: c, perceptual_roughness: 0.6, ..default() })
}
fn glass(mats: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
    mats.add(StandardMaterial {
        base_color: Color::srgba(0.55, 0.75, 0.85, 0.42),
        perceptual_roughness: 0.1, alpha_mode: AlphaMode::Blend, ..default() })
}
fn hl_mat(mats: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
    mats.add(StandardMaterial {
        base_color: Color::srgb(1.0, 1.0, 0.9),
        emissive: LinearRgba::rgb(4.0, 4.0, 3.0),
        perceptual_roughness: 0.05, ..default() })
}
fn dark(mats: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
    mats.add(StandardMaterial { base_color: Color::srgb(0.12, 0.12, 0.12),
        perceptual_roughness: 0.9, ..default() })
}
fn chrome(mats: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
    mats.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.85, 0.90),
        metallic: 0.95, perceptual_roughness: 0.15, ..default() })
}

/// Push a simple mesh+material entity into `ids`.
fn push(
    ids: &mut Vec<Entity>, cmds: &mut Commands,
    meshes: &mut Assets<Mesh>, _mats: &mut Assets<StandardMaterial>,
    extra: impl Bundle,
    mesh: impl Into<Mesh>,
    mat: Handle<StandardMaterial>,
    t: Transform,
) {
    let m = meshes.add(mesh.into());
    ids.push(cmds.spawn((VariantSkin, extra, Mesh3d(m), MeshMaterial3d(mat), t)).id());
}

// ---- Jeep TJ (re-spawn after cycling back) ----------------------------------
// Iconic short-wheelbase Wrangler. 7-slot vertical grille is the defining
// feature. Round headlights, squared fender flares, roll bar, spare on rear.

fn spawn_jeep_default(cmds: &mut Commands, meshes: &mut Assets<Mesh>, mats: &mut Assets<StandardMaterial>) -> Vec<Entity> {
    let body      = mat(mats, Color::srgb(0.8, 0.2, 0.1));
    let bumper_m  = dark(mats);
    let glass_m   = glass(mats);
    let hl        = hl_mat(mats);
    let roll_m    = mat(mats, Color::srgb(0.15, 0.15, 0.15));
    let mut ids   = Vec::new();

    // Main body
    ids.push(cmds.spawn((VariantSkin, DefaultSkin,
        Mesh3d(meshes.add(Cuboid::new(2.0, 0.8, 4.0))),
        MeshMaterial3d(body.clone()), Transform::IDENTITY)).id());
    // Hood (slightly lower, forward)
    ids.push(cmds.spawn((VariantSkin, DefaultSkin,
        Mesh3d(meshes.add(Cuboid::new(1.9, 0.22, 1.2))),
        MeshMaterial3d(body.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.12, -1.6)))).id());
    // Windshield
    ids.push(cmds.spawn((VariantSkin, DefaultSkin,
        Mesh3d(meshes.add(Cuboid::new(1.8, 0.05, 0.8))),
        MeshMaterial3d(glass_m),
        Transform::from_translation(Vec3::new(0.0, 0.32, -0.88))
            .with_rotation(Quat::from_rotation_x(-25_f32.to_radians())))).id());
    // Front bumper
    ids.push(cmds.spawn((VariantSkin, DefaultSkin,
        Mesh3d(meshes.add(Cuboid::new(2.1, 0.15, 0.12))),
        MeshMaterial3d(bumper_m.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.30, -2.10)))).id());
    // Rear bumper
    ids.push(cmds.spawn((VariantSkin, DefaultSkin,
        Mesh3d(meshes.add(Cuboid::new(2.1, 0.15, 0.12))),
        MeshMaterial3d(bumper_m.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.30, 2.10)))).id());
    // Round headlights — THE Jeep TJ look
    let hm = meshes.add(Sphere::new(0.10));
    ids.push(cmds.spawn((VariantSkin, DefaultSkin,
        Mesh3d(hm.clone()), MeshMaterial3d(hl.clone()),
        Transform::from_translation(Vec3::new(-0.75, -0.12, -2.10)))).id());
    ids.push(cmds.spawn((VariantSkin, DefaultSkin,
        Mesh3d(hm), MeshMaterial3d(hl),
        Transform::from_translation(Vec3::new( 0.75, -0.12, -2.10)))).id());

    // 7-slot vertical grille (the most recognizable Jeep feature)
    let slat_m = dark(mats);
    let slat   = meshes.add(Cuboid::new(0.07, 0.28, 0.06));
    for i in 0..7_i32 {
        let x = -0.42 + i as f32 * 0.14;
        ids.push(cmds.spawn((VariantSkin, DefaultSkin,
            Mesh3d(slat.clone()), MeshMaterial3d(slat_m.clone()),
            Transform::from_translation(Vec3::new(x, -0.08, -2.09)))).id());
    }

    // Squared fender flares — 1 cuboid per wheel arch corner (4 arches × 2 pieces)
    let flare_m = dark(mats);
    let flare_h = meshes.add(Cuboid::new(0.12, 0.10, 0.55)); // front/rear arch cap
    let flare_v = meshes.add(Cuboid::new(0.10, 0.18, 0.12)); // side vertical lip
    for (sx, sz) in [(-1.0_f32, -1.4), (1.0, -1.4), (-1.0, 1.4), (1.0, 1.4)] {
        ids.push(cmds.spawn((VariantSkin, DefaultSkin,
            Mesh3d(flare_h.clone()), MeshMaterial3d(flare_m.clone()),
            Transform::from_translation(Vec3::new(sx * 1.06, -0.25, sz)))).id());
        ids.push(cmds.spawn((VariantSkin, DefaultSkin,
            Mesh3d(flare_v.clone()), MeshMaterial3d(flare_m.clone()),
            Transform::from_translation(Vec3::new(sx * 1.05, -0.18, sz)))).id());
    }

    // Roll bar: 2 vertical uprights + 1 horizontal top
    let rb_post = meshes.add(Cylinder::new(0.05, 0.62));
    let rb_top  = meshes.add(Cuboid::new(1.55, 0.05, 0.05));
    ids.push(cmds.spawn((VariantSkin, DefaultSkin,
        Mesh3d(rb_post.clone()), MeshMaterial3d(roll_m.clone()),
        Transform::from_translation(Vec3::new(-0.72, 0.71, -0.35)))).id());
    ids.push(cmds.spawn((VariantSkin, DefaultSkin,
        Mesh3d(rb_post), MeshMaterial3d(roll_m.clone()),
        Transform::from_translation(Vec3::new( 0.72, 0.71, -0.35)))).id());
    ids.push(cmds.spawn((VariantSkin, DefaultSkin,
        Mesh3d(rb_top), MeshMaterial3d(roll_m.clone()),
        Transform::from_translation(Vec3::new(0.0, 1.02, -0.35)))).id());

    // Tow hook — small black cuboid centered on front bumper
    push(&mut ids, cmds, meshes, mats, DefaultSkin,
        Cuboid::new(0.18, 0.12, 0.14), bumper_m.clone(),
        Transform::from_translation(Vec3::new(0.0, -0.32, -2.20)));

    // Spare tire on rear (cylinder lying flat against rear panel)
    let spare_m = dark(mats);
    ids.push(cmds.spawn((VariantSkin, DefaultSkin,
        Mesh3d(meshes.add(Cylinder::new(0.38, 0.16))),
        MeshMaterial3d(spare_m),
        Transform::from_translation(Vec3::new(0.0, 0.10, 2.22))
            .with_rotation(Quat::from_rotation_x(90_f32.to_radians())))).id());

    ids
}

// ---- Ford Bronco ------------------------------------------------------------
// Classic 1980s squared SUV. Square headlights, thick chrome grille bar,
// FORD letter blocks across grille face, side step rails, large door mirrors.

fn spawn_bronco(cmds: &mut Commands, meshes: &mut Assets<Mesh>, mats: &mut Assets<StandardMaterial>) -> Vec<Entity> {
    let body    = mat(mats, Color::srgb(0.20, 0.40, 0.85));
    let bumper  = dark(mats);
    let grill_m = mats.add(StandardMaterial { base_color: Color::srgb(0.08, 0.08, 0.08),
        perceptual_roughness: 1.0, ..default() });
    let glass_m = glass(mats);
    let hl      = hl_mat(mats);
    let rack_m  = mat(mats, Color::srgb(0.20, 0.20, 0.20));
    let chrome_m = chrome(mats);
    let mut ids = Vec::new();

    // Main body — taller and longer than Jeep
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.0, 1.0, 4.5),
        body.clone(), Transform::from_translation(Vec3::new(0.0, 0.0, 0.15)));
    // Hood
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(1.9, 0.20, 1.4),
        body.clone(), Transform::from_translation(Vec3::new(0.0, -0.10, -1.85)));
    // Windshield
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(1.85, 0.05, 0.9),
        glass_m, Transform::from_translation(Vec3::new(0.0, 0.55, -0.85))
            .with_rotation(Quat::from_rotation_x(-20_f32.to_radians())));
    // Front bumper
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.2, 0.18, 0.14),
        bumper.clone(), Transform::from_translation(Vec3::new(0.0, -0.30, -2.36)));
    // Rear bumper
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.2, 0.15, 0.12),
        bumper, Transform::from_translation(Vec3::new(0.0, -0.30, 2.40)));
    // Grille opening (dark fill)
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(1.30, 0.38, 0.08),
        grill_m, Transform::from_translation(Vec3::new(0.0, -0.12, -2.33)));

    // Square headlights (replace round) — Bronco's signature box lamps
    let sq_hl = meshes.add(Cuboid::new(0.22, 0.18, 0.06));
    ids.push(cmds.spawn((VariantSkin, Mesh3d(sq_hl.clone()), MeshMaterial3d(hl.clone()),
        Transform::from_translation(Vec3::new(-0.82, -0.12, -2.36)))).id());
    ids.push(cmds.spawn((VariantSkin, Mesh3d(sq_hl), MeshMaterial3d(hl),
        Transform::from_translation(Vec3::new( 0.82, -0.12, -2.36)))).id());

    // Thick chrome grille bar spanning width of grille
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(1.32, 0.07, 0.10),
        chrome_m.clone(), Transform::from_translation(Vec3::new(0.0, -0.05, -2.30)));

    // "FORD" letter blocks across grille face (4 small raised cuboids)
    let letter_m = chrome_m.clone();
    let letter   = meshes.add(Cuboid::new(0.13, 0.10, 0.05));
    for (i, lx) in [-0.30_f32, -0.10, 0.10, 0.30].iter().enumerate() {
        let _ = i;
        ids.push(cmds.spawn((VariantSkin, Mesh3d(letter.clone()), MeshMaterial3d(letter_m.clone()),
            Transform::from_translation(Vec3::new(*lx, -0.20, -2.34)))).id());
    }

    // Side step rails — long thin cuboids at lower body on each side
    let step_m = mat(mats, Color::srgb(0.18, 0.18, 0.18));
    let step   = meshes.add(Cuboid::new(0.10, 0.08, 3.20));
    for sx in [-1.02_f32, 1.02] {
        ids.push(cmds.spawn((VariantSkin, Mesh3d(step.clone()), MeshMaterial3d(step_m.clone()),
            Transform::from_translation(Vec3::new(sx, -0.55, 0.15)))).id());
    }

    // Large square door mirrors
    let mirror_m = mat(mats, Color::srgb(0.22, 0.22, 0.22));
    let mirror   = meshes.add(Cuboid::new(0.08, 0.18, 0.14));
    for sx in [-1.06_f32, 1.06] {
        ids.push(cmds.spawn((VariantSkin, Mesh3d(mirror.clone()), MeshMaterial3d(mirror_m.clone()),
            Transform::from_translation(Vec3::new(sx, 0.30, -0.55)))).id());
    }

    // Roof rack: 4 posts + 2 long rails + 2 cross-bars
    let post  = meshes.add(Cylinder::new(0.03, 0.18));
    let rlong = meshes.add(Cuboid::new(0.03, 0.03, 1.36));
    let rlat  = meshes.add(Cuboid::new(1.74, 0.03, 0.03));
    for (px, pz) in [(-0.85_f32, -0.50), (0.85, -0.50), (-0.85, 0.80), (0.85, 0.80)] {
        ids.push(cmds.spawn((VariantSkin, Mesh3d(post.clone()), MeshMaterial3d(rack_m.clone()),
            Transform::from_translation(Vec3::new(px, 0.59, pz)))).id());
    }
    for rx in [-0.85_f32, 0.85] {
        ids.push(cmds.spawn((VariantSkin, Mesh3d(rlong.clone()), MeshMaterial3d(rack_m.clone()),
            Transform::from_translation(Vec3::new(rx, 0.68, 0.15)))).id());
    }
    for rz in [-0.50_f32, 0.80] {
        ids.push(cmds.spawn((VariantSkin, Mesh3d(rlat.clone()), MeshMaterial3d(rack_m.clone()),
            Transform::from_translation(Vec3::new(0.0, 0.68, rz)))).id());
    }
    ids
}

// ---- Pickup -----------------------------------------------------------------
// Long-bed classic pickup. Separated cabin + open bed. Round headlights,
// horizontal grille slats, door mirrors, tailgate handle, fuel cap, exhaust pipe.

fn spawn_pickup(cmds: &mut Commands, meshes: &mut Assets<Mesh>, mats: &mut Assets<StandardMaterial>) -> Vec<Entity> {
    let body    = mat(mats, Color::srgb(0.72, 0.74, 0.76));
    let bumper  = dark(mats);
    let grill_m = mats.add(StandardMaterial { base_color: Color::srgb(0.10, 0.10, 0.10),
        perceptual_roughness: 1.0, ..default() });
    let glass_m = glass(mats);
    let hl      = hl_mat(mats);
    let detail  = mat(mats, Color::srgb(0.18, 0.18, 0.18));
    let mut ids = Vec::new();

    // Cabin (forward half of vehicle)
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.0, 0.9, 2.2),
        body.clone(), Transform::from_translation(Vec3::new(0.0, 0.0, -0.70)));
    // Hood
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.0, 0.18, 1.5),
        body.clone(), Transform::from_translation(Vec3::new(0.0, -0.18, -2.05)));
    // Windshield
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(1.85, 0.05, 0.75),
        glass_m, Transform::from_translation(Vec3::new(0.0, 0.50, -1.52))
            .with_rotation(Quat::from_rotation_x(-22_f32.to_radians())));

    // Bed: floor + left / right / front / tailgate walls
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.0, 0.08, 2.0),
        body.clone(), Transform::from_translation(Vec3::new(0.0, -0.40, 1.20)));
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(0.07, 0.65, 2.0),
        body.clone(), Transform::from_translation(Vec3::new(-0.96, -0.08, 1.20)));
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(0.07, 0.65, 2.0),
        body.clone(), Transform::from_translation(Vec3::new( 0.96, -0.08, 1.20)));
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.0, 0.65, 0.07),
        body.clone(), Transform::from_translation(Vec3::new(0.0, -0.08, 0.21)));
    // Tailgate (rear bed wall) — slightly thicker, same color
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.0, 0.65, 0.09),
        body.clone(), Transform::from_translation(Vec3::new(0.0, -0.08, 2.19)));

    // Taller grille with 3 horizontal slats
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(1.40, 0.40, 0.08),
        grill_m, Transform::from_translation(Vec3::new(0.0, -0.14, -2.80)));
    let slat_m = mat(mats, Color::srgb(0.25, 0.25, 0.27));
    let slat   = meshes.add(Cuboid::new(1.38, 0.05, 0.06));
    for sz in [-0.28_f32, -0.14, 0.0] {
        ids.push(cmds.spawn((VariantSkin, Mesh3d(slat.clone()), MeshMaterial3d(slat_m.clone()),
            Transform::from_translation(Vec3::new(0.0, sz, -2.78)))).id());
    }

    // Front bumper
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.25, 0.18, 0.13),
        bumper.clone(), Transform::from_translation(Vec3::new(0.0, -0.30, -2.82)));
    // Rear bumper
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.10, 0.15, 0.12),
        bumper, Transform::from_translation(Vec3::new(0.0, -0.38, 2.26)));

    // Large round headlights (classic pickup style)
    let hm = meshes.add(Sphere::new(0.13));
    ids.push(cmds.spawn((VariantSkin, Mesh3d(hm.clone()), MeshMaterial3d(hl.clone()),
        Transform::from_translation(Vec3::new(-0.78, -0.10, -2.80)))).id());
    ids.push(cmds.spawn((VariantSkin, Mesh3d(hm), MeshMaterial3d(hl),
        Transform::from_translation(Vec3::new( 0.78, -0.10, -2.80)))).id());

    // Tailgate handle — small horizontal cylinder across tailgate center
    ids.push(cmds.spawn((VariantSkin,
        Mesh3d(meshes.add(Cylinder::new(0.03, 0.55))),
        MeshMaterial3d(detail.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.05, 2.24))
            .with_rotation(Quat::from_rotation_z(90_f32.to_radians())))).id());

    // Side mirrors on cabin doors
    let mirror_m = mat(mats, Color::srgb(0.20, 0.20, 0.20));
    let mirror   = meshes.add(Cuboid::new(0.07, 0.14, 0.12));
    for sx in [-1.05_f32, 1.05] {
        ids.push(cmds.spawn((VariantSkin, Mesh3d(mirror.clone()), MeshMaterial3d(mirror_m.clone()),
            Transform::from_translation(Vec3::new(sx, 0.30, -1.05)))).id());
    }

    // Fuel cap — small cylinder on rear quarter panel (driver side)
    ids.push(cmds.spawn((VariantSkin,
        Mesh3d(meshes.add(Cylinder::new(0.06, 0.04))),
        MeshMaterial3d(detail.clone()),
        Transform::from_translation(Vec3::new(-1.01, 0.10, 1.60))
            .with_rotation(Quat::from_rotation_z(90_f32.to_radians())))).id());

    // Exhaust tailpipe — cylinder protruding from rear underside
    ids.push(cmds.spawn((VariantSkin,
        Mesh3d(meshes.add(Cylinder::new(0.05, 0.28))),
        MeshMaterial3d(detail),
        Transform::from_translation(Vec3::new(0.55, -0.50, 2.38))
            .with_rotation(Quat::from_rotation_x(90_f32.to_radians())))).id());

    ids
}

// ---- Hummer -----------------------------------------------------------------
// Wide squat H1/H2 style. 8-slot vertical grille, tall headlights, roof
// antenna, front winch, side-mounted spare tires, running boards.

fn spawn_hummer(cmds: &mut Commands, meshes: &mut Assets<Mesh>, mats: &mut Assets<StandardMaterial>) -> Vec<Entity> {
    let body    = mat(mats, Color::srgb(0.82, 0.75, 0.50));
    let bumper  = dark(mats);
    let guard_m = mat(mats, Color::srgb(0.18, 0.18, 0.18));
    let glass_m = glass(mats);
    let hl      = hl_mat(mats);
    let detail  = mat(mats, Color::srgb(0.14, 0.14, 0.14));
    let mut ids = Vec::new();

    // Wide squat body — the Hummer is notably wider than tall
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.4, 0.8, 4.2),
        body.clone(), Transform::IDENTITY);
    // Windshield
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.3, 0.05, 0.85),
        glass_m, Transform::from_translation(Vec3::new(0.0, 0.43, -0.90))
            .with_rotation(Quat::from_rotation_x(-15_f32.to_radians())));
    // Front bumper
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.5, 0.22, 0.15),
        bumper.clone(), Transform::from_translation(Vec3::new(0.0, -0.28, -2.18)));
    // Rear bumper
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.5, 0.20, 0.13),
        bumper, Transform::from_translation(Vec3::new(0.0, -0.28, 2.18)));

    // 8 prominent vertical grille slats (defining Hummer look)
    let slat_m = dark(mats);
    let slat   = meshes.add(Cuboid::new(0.09, 0.52, 0.10));
    for i in 0..8_i32 {
        let x = -0.63 + i as f32 * 0.18;
        ids.push(cmds.spawn((VariantSkin, Mesh3d(slat.clone()), MeshMaterial3d(slat_m.clone()),
            Transform::from_translation(Vec3::new(x, -0.08, -2.24)))).id());
    }

    // Tall rectangular headlights (Hummer runs them tall and proud)
    let sq_hl = meshes.add(Cuboid::new(0.26, 0.32, 0.06));
    ids.push(cmds.spawn((VariantSkin, Mesh3d(sq_hl.clone()), MeshMaterial3d(hl.clone()),
        Transform::from_translation(Vec3::new(-1.00, -0.05, -2.18)))).id());
    ids.push(cmds.spawn((VariantSkin, Mesh3d(sq_hl), MeshMaterial3d(hl),
        Transform::from_translation(Vec3::new( 1.00, -0.05, -2.18)))).id());

    // Brush guard: 3 vertical bars + horizontal cross-member
    let bar = meshes.add(Cuboid::new(0.10, 0.60, 0.12));
    for bx in [-0.65_f32, 0.0, 0.65] {
        ids.push(cmds.spawn((VariantSkin, Mesh3d(bar.clone()), MeshMaterial3d(guard_m.clone()),
            Transform::from_translation(Vec3::new(bx, -0.10, -2.26)))).id());
    }
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(1.55, 0.08, 0.10),
        guard_m, Transform::from_translation(Vec3::new(0.0, 0.10, -2.26)));

    // Roof-mounted antenna — long thin cylinder on driver-side rear
    ids.push(cmds.spawn((VariantSkin,
        Mesh3d(meshes.add(Cylinder::new(0.025, 0.70))),
        MeshMaterial3d(detail.clone()),
        Transform::from_translation(Vec3::new(-0.90, 0.75, 0.80)))).id());

    // Recovery winch: spool cylinder + mounting box on front bumper center
    ids.push(cmds.spawn((VariantSkin,
        Mesh3d(meshes.add(Cylinder::new(0.08, 0.40))),
        MeshMaterial3d(detail.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.32, -2.22))
            .with_rotation(Quat::from_rotation_z(90_f32.to_radians())))).id());
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(0.46, 0.16, 0.14),
        detail.clone(), Transform::from_translation(Vec3::new(0.0, -0.38, -2.16)));

    // Side-mounted spare tires (mounted high on flanks, both sides)
    let spare_m = dark(mats);
    for sx in [-1.28_f32, 1.28] {
        ids.push(cmds.spawn((VariantSkin,
            Mesh3d(meshes.add(Cylinder::new(0.36, 0.18))),
            MeshMaterial3d(spare_m.clone()),
            Transform::from_translation(Vec3::new(sx, 0.20, 0.60))
                .with_rotation(Quat::from_rotation_z(90_f32.to_radians())))).id());
    }

    // Running boards — long thin cuboids at lower body each side
    let board_m = mat(mats, Color::srgb(0.16, 0.16, 0.16));
    let board   = meshes.add(Cuboid::new(0.18, 0.07, 3.40));
    for sx in [-1.26_f32, 1.26] {
        ids.push(cmds.spawn((VariantSkin, Mesh3d(board.clone()), MeshMaterial3d(board_m.clone()),
            Transform::from_translation(Vec3::new(sx, -0.52, 0.0)))).id());
    }

    ids
}

// ---- Buggy ------------------------------------------------------------------
// No body panels — all exposed tubular cage. Visible engine in rear, exhaust
// headers, shock absorbers at each corner, bucket seats, steering wheel,
// front bumper tube.

fn spawn_buggy(cmds: &mut Commands, meshes: &mut Assets<Mesh>, mats: &mut Assets<StandardMaterial>) -> Vec<Entity> {
    let cage    = mat(mats, Color::srgb(0.92, 0.40, 0.05));
    let seat_m  = mat(mats, Color::srgb(0.12, 0.12, 0.14));
    let engine_m = mat(mats, Color::srgb(0.22, 0.18, 0.14));
    let exhaust_m = mat(mats, Color::srgb(0.30, 0.28, 0.26));
    let shock_m  = mat(mats, Color::srgb(0.55, 0.55, 0.60));
    let hl      = hl_mat(mats);
    let mut ids = Vec::new();

    // 4 vertical cage posts
    let post = meshes.add(Cylinder::new(0.05, 1.40));
    for (px, pz) in [(-0.75_f32, -0.80), (0.75, -0.80), (-0.75, 0.80), (0.75, 0.80)] {
        ids.push(cmds.spawn((VariantSkin, Mesh3d(post.clone()), MeshMaterial3d(cage.clone()),
            Transform::from_translation(Vec3::new(px, 0.30, pz)))).id());
    }

    // Top hoop: 2 longitudinal rails + 2 lateral cross-bars
    let rlong = meshes.add(Cuboid::new(0.05, 0.05, 1.65));
    let rlat  = meshes.add(Cuboid::new(1.55, 0.05, 0.05));
    for rx in [-0.75_f32, 0.75] {
        ids.push(cmds.spawn((VariantSkin, Mesh3d(rlong.clone()), MeshMaterial3d(cage.clone()),
            Transform::from_translation(Vec3::new(rx, 1.00, 0.0)))).id());
    }
    for rz in [-0.80_f32, 0.80] {
        ids.push(cmds.spawn((VariantSkin, Mesh3d(rlat.clone()), MeshMaterial3d(cage.clone()),
            Transform::from_translation(Vec3::new(0.0, 1.00, rz)))).id());
    }

    // Front X-braces (two rotated cuboids in the frontal plane)
    let brace = meshes.add(Cuboid::new(0.05, 0.05, 1.65));
    for sign in [-1_f32, 1.0] {
        ids.push(cmds.spawn((VariantSkin, Mesh3d(brace.clone()), MeshMaterial3d(cage.clone()),
            Transform::from_translation(Vec3::new(0.0, 0.60, -0.80))
                .with_rotation(Quat::from_euler(EulerRot::XYZ,
                    0.0, 0.0, sign * 42_f32.to_radians())))).id());
    }
    // Rear cross-bar
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(1.55, 0.05, 0.05),
        cage.clone(), Transform::from_translation(Vec3::new(0.0, 0.60, 0.80)));

    // Front bumper as an exposed horizontal tube (no hood/grille)
    ids.push(cmds.spawn((VariantSkin,
        Mesh3d(meshes.add(Cylinder::new(0.05, 1.50))),
        MeshMaterial3d(cage.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.30, -1.00))
            .with_rotation(Quat::from_rotation_z(90_f32.to_radians())))).id());

    // Bucket seats — 2 short cuboids side by side
    let seat_shape = meshes.add(Cuboid::new(0.48, 0.28, 0.52));
    for sx in [-0.30_f32, 0.30] {
        ids.push(cmds.spawn((VariantSkin, Mesh3d(seat_shape.clone()), MeshMaterial3d(seat_m.clone()),
            Transform::from_translation(Vec3::new(sx, -0.26, 0.05)))).id());
    }

    // Steering wheel — thin flat cylinder
    ids.push(cmds.spawn((VariantSkin,
        Mesh3d(meshes.add(Cylinder::new(0.18, 0.03))),
        MeshMaterial3d(seat_m.clone()),
        Transform::from_translation(Vec3::new(-0.28, 0.18, -0.38))
            .with_rotation(Quat::from_rotation_x(60_f32.to_radians())))).id());

    // Visible engine block in rear (air-cooled buggy engine sits behind seats)
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(0.70, 0.45, 0.55),
        engine_m, Transform::from_translation(Vec3::new(0.0, -0.10, 0.78)));

    // Exhaust headers — 4 small cylinders fanning out from engine
    let hdr = meshes.add(Cylinder::new(0.035, 0.32));
    for (hx, hz, angle) in [
        (-0.20_f32, 0.95,  15_f32),
        ( 0.20,     0.95, -15_f32),
        (-0.28,     1.02,  30_f32),
        ( 0.28,     1.02, -30_f32),
    ] {
        ids.push(cmds.spawn((VariantSkin, Mesh3d(hdr.clone()), MeshMaterial3d(exhaust_m.clone()),
            Transform::from_translation(Vec3::new(hx, -0.05, hz))
                .with_rotation(Quat::from_rotation_z(angle.to_radians())))).id());
    }

    // Exposed shock absorbers at each wheel corner (4 small cylinders)
    let shock = meshes.add(Cylinder::new(0.04, 0.50));
    for (sx, sz) in [(-0.80_f32, -0.85), (0.80, -0.85), (-0.80, 0.85), (0.80, 0.85)] {
        ids.push(cmds.spawn((VariantSkin, Mesh3d(shock.clone()), MeshMaterial3d(shock_m.clone()),
            Transform::from_translation(Vec3::new(sx, -0.12, sz))
                .with_rotation(Quat::from_rotation_x(20_f32.to_radians())))).id());
    }

    // Headlights (small, front-low, no bezel — just the lamp)
    let hm = meshes.add(Sphere::new(0.09));
    ids.push(cmds.spawn((VariantSkin, Mesh3d(hm.clone()), MeshMaterial3d(hl.clone()),
        Transform::from_translation(Vec3::new(-0.50, -0.18, -1.05)))).id());
    ids.push(cmds.spawn((VariantSkin, Mesh3d(hm), MeshMaterial3d(hl),
        Transform::from_translation(Vec3::new( 0.50, -0.18, -1.05)))).id());

    ids
}
