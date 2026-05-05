// Vehicle silhouette variants — cycle with Backslash (\).
//
// Despawn rules:
//   Leaving JeepTJ   → despawn DefaultSkin children of chassis
//   Leaving non-Jeep → despawn VariantSkin children of chassis
//   Arriving JeepTJ  → spawn_jeep_default (tagged DefaultSkin + VariantSkin)
//
// Mesh children are added via add_children (the only reliable parent API in 0.18).

use bevy::prelude::*;
use crate::vehicle::{DefaultSkin, VehicleRoot};

// ---- Plugin -----------------------------------------------------------------

pub struct VariantsPlugin;
impl Plugin for VariantsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VehicleVariant>()
           .add_systems(Startup, spawn_variant_hud)
           .add_systems(Update, (cycle_variant, update_variant_hud)
               .run_if(resource_exists::<VehicleRoot>));
    }
}

// ---- Public API -------------------------------------------------------------

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub enum VehicleVariant { #[default] JeepTJ, FordBronco, Pickup, Hummer, Buggy }

impl VehicleVariant {
    pub fn next(self) -> Self {
        match self {
            Self::JeepTJ => Self::FordBronco, Self::FordBronco => Self::Pickup,
            Self::Pickup => Self::Hummer,     Self::Hummer     => Self::Buggy,
            Self::Buggy  => Self::JeepTJ,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::JeepTJ => "Jeep TJ", Self::FordBronco => "Ford Bronco",
            Self::Pickup => "Pickup",  Self::Hummer     => "Hummer",
            Self::Buggy  => "Buggy",
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
        VehicleVariant::JeepTJ     => spawn_jeep_default(&mut commands, &mut meshes, &mut materials),
        VehicleVariant::FordBronco => spawn_bronco(&mut commands, &mut meshes, &mut materials),
        VehicleVariant::Pickup     => spawn_pickup(&mut commands, &mut meshes, &mut materials),
        VehicleVariant::Hummer     => spawn_hummer(&mut commands, &mut meshes, &mut materials),
        VehicleVariant::Buggy      => spawn_buggy(&mut commands, &mut meshes, &mut materials),
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

/// Push a simple mesh+material entity into `ids`.
fn push(
    ids: &mut Vec<Entity>, cmds: &mut Commands,
    meshes: &mut Assets<Mesh>, mats: &mut Assets<StandardMaterial>,
    extra: impl Bundle,
    mesh: impl Into<Mesh>,
    mat: Handle<StandardMaterial>,
    t: Transform,
) {
    let m = meshes.add(mesh.into());
    ids.push(cmds.spawn((VariantSkin, extra, Mesh3d(m), MeshMaterial3d(mat), t)).id());
}

// ---- Jeep TJ (re-spawn after cycling back) ----------------------------------
// Mirrors what vehicle.rs spawns at startup, also tagged DefaultSkin.

fn spawn_jeep_default(cmds: &mut Commands, meshes: &mut Assets<Mesh>, mats: &mut Assets<StandardMaterial>) -> Vec<Entity> {
    let body   = mat(mats, Color::srgb(0.8, 0.2, 0.1));
    let bumper = dark(mats);
    let glass  = glass(mats);
    let hl     = hl_mat(mats);
    let hm     = meshes.add(Sphere::new(0.10));
    let mut ids = Vec::new();
    // body 2×0.8×4
    ids.push(cmds.spawn((VariantSkin, DefaultSkin,
        Mesh3d(meshes.add(Cuboid::new(2.0, 0.8, 4.0))),
        MeshMaterial3d(body.clone()), Transform::IDENTITY)).id());
    // hood
    ids.push(cmds.spawn((VariantSkin, DefaultSkin,
        Mesh3d(meshes.add(Cuboid::new(1.9, 0.22, 1.2))),
        MeshMaterial3d(body.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.12, -1.6)))).id());
    // windshield
    ids.push(cmds.spawn((VariantSkin, DefaultSkin,
        Mesh3d(meshes.add(Cuboid::new(1.8, 0.05, 0.8))),
        MeshMaterial3d(glass),
        Transform::from_translation(Vec3::new(0.0, 0.32, -0.88))
            .with_rotation(Quat::from_rotation_x(-25_f32.to_radians())))).id());
    // front bumper
    ids.push(cmds.spawn((VariantSkin, DefaultSkin,
        Mesh3d(meshes.add(Cuboid::new(2.1, 0.15, 0.12))),
        MeshMaterial3d(bumper.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.30, -2.10)))).id());
    // rear bumper
    ids.push(cmds.spawn((VariantSkin, DefaultSkin,
        Mesh3d(meshes.add(Cuboid::new(2.1, 0.15, 0.12))),
        MeshMaterial3d(bumper),
        Transform::from_translation(Vec3::new(0.0, -0.30, 2.10)))).id());
    // headlights
    ids.push(cmds.spawn((VariantSkin, DefaultSkin,
        Mesh3d(hm.clone()), MeshMaterial3d(hl.clone()),
        Transform::from_translation(Vec3::new(-0.75, -0.12, -2.10)))).id());
    ids.push(cmds.spawn((VariantSkin, DefaultSkin,
        Mesh3d(hm), MeshMaterial3d(hl),
        Transform::from_translation(Vec3::new( 0.75, -0.12, -2.10)))).id());
    ids
}

// ---- Ford Bronco ------------------------------------------------------------
// Longer/taller body (2.0×1.0×4.5), roof rack on 4 posts, dark front grill.
// Body color deep blue.

fn spawn_bronco(cmds: &mut Commands, meshes: &mut Assets<Mesh>, mats: &mut Assets<StandardMaterial>) -> Vec<Entity> {
    let body   = mat(mats, Color::srgb(0.20, 0.40, 0.85));
    let bumper = dark(mats);
    let grill  = mats.add(StandardMaterial { base_color: Color::srgb(0.08, 0.08, 0.08),
        perceptual_roughness: 1.0, ..default() });
    let glass  = glass(mats);
    let hl     = hl_mat(mats);
    let rack   = mat(mats, Color::srgb(0.20, 0.20, 0.20));
    let mut ids = Vec::new();

    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.0, 1.0, 4.5),
        body.clone(), Transform::from_translation(Vec3::new(0.0, 0.0, 0.15)));
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(1.9, 0.20, 1.4),
        body.clone(), Transform::from_translation(Vec3::new(0.0, -0.10, -1.85)));
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(1.85, 0.05, 0.9),
        glass, Transform::from_translation(Vec3::new(0.0, 0.55, -0.85))
            .with_rotation(Quat::from_rotation_x(-20_f32.to_radians())));
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.2, 0.18, 0.14),
        bumper.clone(), Transform::from_translation(Vec3::new(0.0, -0.30, -2.36)));
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.2, 0.15, 0.12),
        bumper, Transform::from_translation(Vec3::new(0.0, -0.30, 2.40)));
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(1.30, 0.38, 0.08),
        grill, Transform::from_translation(Vec3::new(0.0, -0.12, -2.33)));

    let hm = meshes.add(Sphere::new(0.11));
    ids.push(cmds.spawn((VariantSkin, Mesh3d(hm.clone()), MeshMaterial3d(hl.clone()),
        Transform::from_translation(Vec3::new(-0.80, -0.12, -2.34)))).id());
    ids.push(cmds.spawn((VariantSkin, Mesh3d(hm), MeshMaterial3d(hl),
        Transform::from_translation(Vec3::new( 0.80, -0.12, -2.34)))).id());

    // Roof rack: 4 posts + 2 long rails + 2 cross-bars
    let post  = meshes.add(Cylinder::new(0.03, 0.18));
    let rlong = meshes.add(Cuboid::new(0.03, 0.03, 1.36));
    let rlat  = meshes.add(Cuboid::new(1.74, 0.03, 0.03));
    for (px, pz) in [(-0.85_f32, -0.50), (0.85, -0.50), (-0.85, 0.80), (0.85, 0.80)] {
        ids.push(cmds.spawn((VariantSkin, Mesh3d(post.clone()), MeshMaterial3d(rack.clone()),
            Transform::from_translation(Vec3::new(px, 0.59, pz)))).id());
    }
    for rx in [-0.85_f32, 0.85] {
        ids.push(cmds.spawn((VariantSkin, Mesh3d(rlong.clone()), MeshMaterial3d(rack.clone()),
            Transform::from_translation(Vec3::new(rx, 0.68, 0.15)))).id());
    }
    for rz in [-0.50_f32, 0.80] {
        ids.push(cmds.spawn((VariantSkin, Mesh3d(rlat.clone()), MeshMaterial3d(rack.clone()),
            Transform::from_translation(Vec3::new(0.0, 0.68, rz)))).id());
    }
    ids
}

// ---- Pickup -----------------------------------------------------------------
// Separated cabin + open bed (floor + 4 walls), long hood. Body color silver.

fn spawn_pickup(cmds: &mut Commands, meshes: &mut Assets<Mesh>, mats: &mut Assets<StandardMaterial>) -> Vec<Entity> {
    let body   = mat(mats, Color::srgb(0.72, 0.74, 0.76));
    let bumper = dark(mats);
    let grill  = mats.add(StandardMaterial { base_color: Color::srgb(0.10, 0.10, 0.10),
        perceptual_roughness: 1.0, ..default() });
    let glass  = glass(mats);
    let hl     = hl_mat(mats);
    let mut ids = Vec::new();

    // Cabin (forward), hood, windshield
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.0, 0.9, 2.2),
        body.clone(), Transform::from_translation(Vec3::new(0.0, 0.0, -0.70)));
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.0, 0.18, 1.5),
        body.clone(), Transform::from_translation(Vec3::new(0.0, -0.18, -2.05)));
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(1.85, 0.05, 0.75),
        glass, Transform::from_translation(Vec3::new(0.0, 0.50, -1.52))
            .with_rotation(Quat::from_rotation_x(-22_f32.to_radians())));

    // Bed: floor + left/right/front/tailgate walls
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.0, 0.08, 2.0),
        body.clone(), Transform::from_translation(Vec3::new(0.0, -0.40, 1.20)));
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(0.07, 0.65, 2.0),
        body.clone(), Transform::from_translation(Vec3::new(-0.96, -0.08, 1.20)));
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(0.07, 0.65, 2.0),
        body.clone(), Transform::from_translation(Vec3::new( 0.96, -0.08, 1.20)));
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.0, 0.65, 0.07),
        body.clone(), Transform::from_translation(Vec3::new(0.0, -0.08, 0.21)));
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.0, 0.65, 0.07),
        body.clone(), Transform::from_translation(Vec3::new(0.0, -0.08, 2.19)));

    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(1.40, 0.36, 0.08),
        grill, Transform::from_translation(Vec3::new(0.0, -0.15, -2.80)));
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.25, 0.18, 0.13),
        bumper.clone(), Transform::from_translation(Vec3::new(0.0, -0.30, -2.82)));
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.10, 0.15, 0.12),
        bumper, Transform::from_translation(Vec3::new(0.0, -0.38, 2.26)));

    let hm = meshes.add(Sphere::new(0.10));
    ids.push(cmds.spawn((VariantSkin, Mesh3d(hm.clone()), MeshMaterial3d(hl.clone()),
        Transform::from_translation(Vec3::new(-0.78, -0.15, -2.80)))).id());
    ids.push(cmds.spawn((VariantSkin, Mesh3d(hm), MeshMaterial3d(hl),
        Transform::from_translation(Vec3::new( 0.78, -0.15, -2.80)))).id());
    ids
}

// ---- Hummer -----------------------------------------------------------------
// Wide (2.4) squat body, brush guard (3 thick vertical bars). Body desert tan.

fn spawn_hummer(cmds: &mut Commands, meshes: &mut Assets<Mesh>, mats: &mut Assets<StandardMaterial>) -> Vec<Entity> {
    let body   = mat(mats, Color::srgb(0.82, 0.75, 0.50));
    let bumper = dark(mats);
    let guard  = mat(mats, Color::srgb(0.18, 0.18, 0.18));
    let glass  = glass(mats);
    let hl     = hl_mat(mats);
    let mut ids = Vec::new();

    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.4, 0.8, 4.2),
        body.clone(), Transform::IDENTITY);
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.3, 0.05, 0.85),
        glass, Transform::from_translation(Vec3::new(0.0, 0.43, -0.90))
            .with_rotation(Quat::from_rotation_x(-15_f32.to_radians())));
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.5, 0.22, 0.15),
        bumper.clone(), Transform::from_translation(Vec3::new(0.0, -0.28, -2.18)));
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(2.5, 0.20, 0.13),
        bumper, Transform::from_translation(Vec3::new(0.0, -0.28, 2.18)));

    // Brush guard: 3 vertical bars + horizontal cross-member
    let bar = meshes.add(Cuboid::new(0.10, 0.60, 0.12));
    for bx in [-0.65_f32, 0.0, 0.65] {
        ids.push(cmds.spawn((VariantSkin, Mesh3d(bar.clone()), MeshMaterial3d(guard.clone()),
            Transform::from_translation(Vec3::new(bx, -0.10, -2.26)))).id());
    }
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(1.55, 0.08, 0.10),
        guard, Transform::from_translation(Vec3::new(0.0, 0.10, -2.26)));

    let hm = meshes.add(Sphere::new(0.12));
    ids.push(cmds.spawn((VariantSkin, Mesh3d(hm.clone()), MeshMaterial3d(hl.clone()),
        Transform::from_translation(Vec3::new(-0.95, -0.10, -2.18)))).id());
    ids.push(cmds.spawn((VariantSkin, Mesh3d(hm), MeshMaterial3d(hl),
        Transform::from_translation(Vec3::new( 0.95, -0.10, -2.18)))).id());
    ids
}

// ---- Buggy ------------------------------------------------------------------
// No body panels. Exposed roll cage: 4 vertical poles, top hoop, X front
// braces, rear bar. Tiny seat. Vivid orange.

fn spawn_buggy(cmds: &mut Commands, meshes: &mut Assets<Mesh>, mats: &mut Assets<StandardMaterial>) -> Vec<Entity> {
    let cage = mat(mats, Color::srgb(0.92, 0.40, 0.05));
    let seat = mat(mats, Color::srgb(0.12, 0.12, 0.14));
    let hl   = hl_mat(mats);
    let mut ids = Vec::new();

    // 4 vertical posts
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

    // Front X-braces (two rotated Cuboids in the frontal plane)
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

    // Seat
    push(&mut ids, cmds, meshes, mats, (), Cuboid::new(0.55, 0.30, 0.60),
        seat, Transform::from_translation(Vec3::new(-0.20, -0.25, 0.0)));

    // Headlights (small, front-low)
    let hm = meshes.add(Sphere::new(0.09));
    ids.push(cmds.spawn((VariantSkin, Mesh3d(hm.clone()), MeshMaterial3d(hl.clone()),
        Transform::from_translation(Vec3::new(-0.50, -0.18, -1.05)))).id());
    ids.push(cmds.spawn((VariantSkin, Mesh3d(hm), MeshMaterial3d(hl),
        Transform::from_translation(Vec3::new( 0.50, -0.18, -1.05)))).id());
    ids
}
