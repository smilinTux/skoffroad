// Truck sponsor liveries — Sprint 72.
//
// Applies parody-brand decal plates to body panels, selectable as named livery
// sets.  Each set is a curated list of (BodyPanel, brand_index) placements.
//
// KEY DESIGN:
//   • BodyPanel enum — maps each panel to a chassis-local position + outward
//     facing normal so the thin decal plate can sit 0.01 m proud of the body.
//   • 5 livery presets — "Stock" (none) through "Expo Demo" (everything).
//   • Respawn guard pattern identical to wheel_detail.rs: a LiveryAttached
//     marker on the chassis is checked each frame; decals are (re)spawned
//     whenever the chassis exists without the marker.  Chassis despawn on
//     RespawnRequest removes the marker, so decals re-attach automatically.
//   • Hotkey Shift+L — cycles through livery presets and fires RespawnRequest.
//   • Persistence to platform_storage["livery.json"].
//
// Public API:
//   SponsorLiveryPlugin
//   SponsorLiveryState   (resource — current livery index)
//   BodyPanel            (enum)
//   LiveryAttached       (marker component — exported for querying)
//
// READ-ONLY deps:
//   crate::parody_brands::{ParodyBrands, brand_primary_color, brand_secondary_color}
//   crate::vehicle::{Chassis, DefaultSkin, RespawnRequest, VehicleRoot}
//   crate::platform_storage::{read_string, write_string}

use bevy::prelude::*;
use crate::parody_brands::{ParodyBrands, brand_primary_color, brand_secondary_color};
use crate::vehicle::{Chassis, DefaultSkin, RespawnRequest, VehicleRoot};
use crate::platform_storage;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct SponsorLiveryPlugin;

impl Plugin for SponsorLiveryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SponsorLiveryState>()
           .add_systems(Startup, load_livery_state)
           .add_systems(Update, (livery_hotkey, attach_livery_decals).chain());
    }
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Which livery preset is active.
#[derive(Resource, Default)]
pub struct SponsorLiveryState {
    pub current: usize,
}

/// Placed on a chassis entity once its livery decals have been attached.
/// Cleared when the chassis is despawned (on respawn) so decals re-attach
/// on the fresh chassis — same guard pattern as WheelDetailAttached.
#[derive(Component)]
pub struct LiveryAttached;

// ---------------------------------------------------------------------------
// Body panels
// ---------------------------------------------------------------------------

/// Named panel faces on the truck body.
/// `local_pos`  — chassis-local center of the decal plate.
/// `normal`     — outward-facing chassis-local normal (used to offset proud).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyPanel {
    LeftDoor,
    RightDoor,
    Hood,
    Tailgate,
    LeftFender,
    RightFender,
    Cowl,
}

/// Chassis half-extents (mirrors vehicle.rs constant, which is private).
/// CHASSIS_HALF = Vec3::new(1.0, 0.4, 2.0)
const CHASSIS_HALF_X: f32 = 1.0;
const CHASSIS_HALF_Y: f32 = 0.4;
const CHASSIS_HALF_Z: f32 = 2.0;

/// How far proud of the body surface each decal plate sits (avoids z-fight).
const DECAL_OFFSET: f32 = 0.01;

/// Thickness of the decal plate (very thin slab).
const DECAL_DEPTH: f32 = 0.02;

struct PanelDef {
    /// Chassis-local center of the decal plate (before normal offset).
    local_pos: Vec3,
    /// Outward normal in chassis space; used to push the plate proud.
    normal: Vec3,
    /// Width × height of the decal plate face (not depth).
    size: Vec2,
}

impl BodyPanel {
    fn def(self) -> PanelDef {
        match self {
            // Left door: flush with left body side face (x = -CHASSIS_HALF_X).
            // Door mesh in vehicle.rs sits at x=-1.06, z=0.30, half-size ~(0.06,0.46,1.10).
            // Place the decal centred on the door face.
            BodyPanel::LeftDoor => PanelDef {
                local_pos: Vec3::new(
                    -(CHASSIS_HALF_X + DECAL_OFFSET + DECAL_DEPTH * 0.5),
                    0.05,   // door mid-height
                    0.30,   // door mid-Z (matches vehicle.rs door spawn)
                ),
                normal: Vec3::NEG_X,
                size: Vec2::new(1.00, 0.42),
            },
            // Right door: mirror of left.
            BodyPanel::RightDoor => PanelDef {
                local_pos: Vec3::new(
                    CHASSIS_HALF_X + DECAL_OFFSET + DECAL_DEPTH * 0.5,
                    0.05,
                    0.30,
                ),
                normal: Vec3::X,
                size: Vec2::new(1.00, 0.42),
            },
            // Hood: flat on top, toward the nose.  The hood mesh in vehicle.rs
            // is at y=-0.12 relative to chassis, z=-1.6.  Hood top face is
            // chassis_y + CHASSIS_HALF_Y.
            BodyPanel::Hood => PanelDef {
                local_pos: Vec3::new(
                    0.0,
                    CHASSIS_HALF_Y + DECAL_OFFSET + DECAL_DEPTH * 0.5,
                    -1.30, // on the hood, forward of centre
                ),
                normal: Vec3::Y,
                size: Vec2::new(1.60, 0.90),
            },
            // Tailgate: rear face of the chassis (z = +CHASSIS_HALF_Z).
            BodyPanel::Tailgate => PanelDef {
                local_pos: Vec3::new(
                    0.0,
                    0.10,
                    CHASSIS_HALF_Z + DECAL_OFFSET + DECAL_DEPTH * 0.5,
                ),
                normal: Vec3::Z,
                size: Vec2::new(1.60, 0.60),
            },
            // Left fender: just behind the left front wheel arch.
            // Fender mesh in vehicle.rs at x=-1.12, z=front_wheel_z~-1.4.
            BodyPanel::LeftFender => PanelDef {
                local_pos: Vec3::new(
                    -(CHASSIS_HALF_X + DECAL_OFFSET + DECAL_DEPTH * 0.5),
                    -0.10,
                    -1.40,
                ),
                normal: Vec3::NEG_X,
                size: Vec2::new(0.72, 0.20),
            },
            // Right fender: mirror.
            BodyPanel::RightFender => PanelDef {
                local_pos: Vec3::new(
                    CHASSIS_HALF_X + DECAL_OFFSET + DECAL_DEPTH * 0.5,
                    -0.10,
                    -1.40,
                ),
                normal: Vec3::X,
                size: Vec2::new(0.72, 0.20),
            },
            // Cowl: the nose/front face of the chassis (z = -CHASSIS_HALF_Z).
            // Placed slightly high to clear the bumper.
            BodyPanel::Cowl => PanelDef {
                local_pos: Vec3::new(
                    0.0,
                    0.0,
                    -(CHASSIS_HALF_Z + DECAL_OFFSET + DECAL_DEPTH * 0.5),
                ),
                normal: Vec3::NEG_Z,
                size: Vec2::new(1.20, 0.50),
            },
        }
    }

    /// Build a Transform for the decal plate.  The plate is a thin Cuboid with
    /// its depth along `normal`; we rotate so the plate face is perpendicular to
    /// the normal — i.e. the cuboid's local +Z axis points outward along `normal`.
    fn decal_transform(self) -> Transform {
        let def = self.def();
        let rot = Quat::from_rotation_arc(Vec3::Z, def.normal);
        Transform::from_translation(def.local_pos).with_rotation(rot)
    }

    /// Mesh for the decal plate: thin Cuboid, width × height × DECAL_DEPTH.
    fn decal_mesh(self) -> Cuboid {
        let def = self.def();
        Cuboid::new(def.size.x, def.size.y, DECAL_DEPTH)
    }
}

// ---------------------------------------------------------------------------
// Livery presets
// ---------------------------------------------------------------------------

/// A single decal placement: which panel, which brand (index into ParodyBrands).
#[derive(Debug, Clone, Copy)]
pub struct Placement {
    pub panel: BodyPanel,
    pub brand_id: usize,
}

impl Placement {
    const fn new(panel: BodyPanel, brand_id: usize) -> Self {
        Self { panel, brand_id }
    }
}

/// A named livery preset.
pub struct LiveryPreset {
    pub name: &'static str,
    pub placements: &'static [Placement],
}

/// The five curated livery presets.
///
/// Brand index reference (from parody_brands.rs):
///   0 = apexis        (Tires)
///   1 = bouldergrip   (Tires)
///   2 = greatyear     (Tires)
///   3 = warden        (Winches)
///   4 = smithybuilt   (Winches)
///   5 = fawx_racing   (Shocks)
///   6 = crown_shox    (Shocks)
///   7 = bilsteen      (Shocks)
///   8 = ridge_led     (Lights)
///   9 = mesa_designs  (Lights)
///  10 = modus_wheels  (Wheels)
///  11 = ark_4x4       (Recovery)
pub static LIVERY_PRESETS: &[LiveryPreset] = &[
    // 0 — Stock: no decals (default).
    LiveryPreset {
        name: "Stock",
        placements: &[],
    },
    // 1 — Trail Team: door tire brand, hood shock brand, tailgate winch brand.
    // Tire on the doors (most visible from the side), shocks on the hood, winch
    // recovery brand on the rear — a typical sponsored trail-built rig loadout.
    LiveryPreset {
        name: "Trail Team",
        placements: &[
            Placement::new(BodyPanel::LeftDoor,  0), // APEXIS tires
            Placement::new(BodyPanel::RightDoor, 0), // APEXIS tires
            Placement::new(BodyPanel::Hood,      5), // FAWX RACING shocks
            Placement::new(BodyPanel::Tailgate,  3), // WARDEN winch
        ],
    },
    // 2 — Race Spec: bigger branding, shock + light house livery.
    // Shocks dominate the doors (they pay more on race rigs), lights on the cowl,
    // tire brand on the fender.
    LiveryPreset {
        name: "Race Spec",
        placements: &[
            Placement::new(BodyPanel::LeftDoor,   7), // BILSTEEN shocks
            Placement::new(BodyPanel::RightDoor,  7), // BILSTEEN shocks
            Placement::new(BodyPanel::Cowl,       8), // RIDGE LED lights
            Placement::new(BodyPanel::LeftFender, 1), // BOULDERGRIP tires
            Placement::new(BodyPanel::RightFender,1), // BOULDERGRIP tires
        ],
    },
    // 3 — Mud Series: tire + recovery brand; weathered / chunky feel.
    // Two complementary sponsors — tires on the door, recovery armor on the tailgate.
    LiveryPreset {
        name: "Mud Series",
        placements: &[
            Placement::new(BodyPanel::LeftDoor,  2),  // GREATYEAR tires
            Placement::new(BodyPanel::RightDoor, 2),  // GREATYEAR tires
            Placement::new(BodyPanel::Tailgate,  11), // ARK 4X4 recovery
            Placement::new(BodyPanel::Hood,       6), // CROWN SHOX
        ],
    },
    // 4 — Expo Demo: all five panel types filled — the "show vendors everything" loud livery.
    LiveryPreset {
        name: "Expo Demo",
        placements: &[
            Placement::new(BodyPanel::LeftDoor,   0),  // APEXIS tires
            Placement::new(BodyPanel::RightDoor,  5),  // FAWX RACING shocks
            Placement::new(BodyPanel::Hood,        9), // MESA DESIGNS lights
            Placement::new(BodyPanel::Tailgate,   11), // ARK 4X4 recovery
            Placement::new(BodyPanel::LeftFender,  4), // SMITHYBUILT winch
            Placement::new(BodyPanel::RightFender, 4), // SMITHYBUILT winch
            Placement::new(BodyPanel::Cowl,        8), // RIDGE LED lights
        ],
    },
];

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

const STORAGE_KEY: &str = "livery.json";

fn load_livery_state(mut state: ResMut<SponsorLiveryState>) {
    if let Some(json) = platform_storage::read_string(STORAGE_KEY) {
        if let Ok(idx) = json.trim().parse::<usize>() {
            state.current = idx.min(LIVERY_PRESETS.len().saturating_sub(1));
        }
    }
}

fn save_livery_state(state: &SponsorLiveryState) {
    let _ = platform_storage::write_string(STORAGE_KEY, &state.current.to_string());
}

// ---------------------------------------------------------------------------
// Hotkey — Shift+L cycles liveries
// ---------------------------------------------------------------------------

fn livery_hotkey(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<SponsorLiveryState>,
    mut respawn: ResMut<RespawnRequest>,
) {
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if shift && keys.just_pressed(KeyCode::KeyL) {
        state.current = (state.current + 1) % LIVERY_PRESETS.len();
        save_livery_state(&state);
        respawn.0 = true;
    }
}

// ---------------------------------------------------------------------------
// Decal attachment — fires once per fresh chassis (same pattern as wheel_detail.rs)
// ---------------------------------------------------------------------------

fn attach_livery_decals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    vehicle: Option<Res<VehicleRoot>>,
    chassis_q: Query<Entity, (With<Chassis>, Without<LiveryAttached>)>,
    state: Res<SponsorLiveryState>,
    brands: Option<Res<ParodyBrands>>,
) {
    // Wait until VehicleRoot and ParodyBrands exist.
    let Some(_vehicle) = vehicle else { return };
    let Some(brands) = brands else { return };

    let preset = &LIVERY_PRESETS[state.current.min(LIVERY_PRESETS.len() - 1)];

    for chassis_entity in chassis_q.iter() {
        let mut decals: Vec<Entity> = Vec::new();

        for placement in preset.placements.iter() {
            let panel = placement.panel;
            let brand_idx = placement.brand_id;

            let primary_color   = brand_primary_color(&brands, brand_idx);
            let secondary_color = brand_secondary_color(&brands, brand_idx);

            // Primary (main brand color) decal plate.
            let primary_mesh = meshes.add(panel.decal_mesh());
            let primary_mat = materials.add(StandardMaterial {
                base_color: primary_color,
                perceptual_roughness: 0.65,
                ..default()
            });

            // Secondary accent stripe: a narrow strip across the bottom third
            // of the panel face, same panel but slightly smaller and forward offset.
            let def = panel.def();
            let stripe_normal_offset = 0.005; // extra 0.5 mm so stripe sits proud of plate
            let stripe_rot = Quat::from_rotation_arc(Vec3::Z, def.normal);
            let stripe_pos = def.local_pos
                + def.normal * stripe_normal_offset
                - Vec3::new(0.0, def.size.y * 0.30, 0.0)
                    .rotate_by(stripe_rot); // push toward panel bottom in local-facing space

            let stripe_mesh = meshes.add(Cuboid::new(
                def.size.x,
                def.size.y * 0.18,  // bottom-stripe height
                DECAL_DEPTH,
            ));
            let stripe_mat = materials.add(StandardMaterial {
                base_color: secondary_color,
                perceptual_roughness: 0.55,
                ..default()
            });

            let plate = commands.spawn((
                DefaultSkin,
                Mesh3d(primary_mesh),
                MeshMaterial3d(primary_mat),
                panel.decal_transform(),
            )).id();
            decals.push(plate);

            let stripe_transform = Transform::from_translation(stripe_pos)
                .with_rotation(stripe_rot);
            let stripe = commands.spawn((
                DefaultSkin,
                Mesh3d(stripe_mesh),
                MeshMaterial3d(stripe_mat),
                stripe_transform,
            )).id();
            decals.push(stripe);
        }

        // Attach all decal plates as children of the chassis and mark done.
        commands.entity(chassis_entity)
            .add_children(&decals)
            .insert(LiveryAttached);
    }
}

// ---------------------------------------------------------------------------
// Vec3 extension helper — rotate_by for a Vec3 by a Quat
// ---------------------------------------------------------------------------

trait RotateBy {
    fn rotate_by(self, q: Quat) -> Self;
}

impl RotateBy for Vec3 {
    fn rotate_by(self, q: Quat) -> Self {
        q * self
    }
}
