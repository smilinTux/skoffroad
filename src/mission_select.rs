// mission_select.rs — Sprint 63
//
// Unified Mission Select full-screen overlay toggled by Shift+Tab.
//
// Layout:
//   GlobalZIndex(900) — above HUD (z 42–50), below title screen.
//
// Cards shown:
//   ── Hillclimb Tiers ──
//     Beginner / Intermediate / Expert (from hillclimb_tiers::TierLayout)
//   ── Rock Crawl ──
//     Boulder Stairs / Two-Log Bridge / Off-Camber Traverse
//   ── Trail Rides ──
//     One sub-card per entry in TrailManifest (loaded by TrailRidesPlugin)
//   ── Obstacle Course ──
//     Single placeholder "Coming in Sprint 64" (greyed out)
//
// Each non-greyed card has a "FAST TRAVEL" button that:
//   • Hillclimb / Rock Crawl: teleports chassis to the mission start position.
//   • Trail Rides: sets TrailRideRequest to trigger TrailRidesPlugin.
//
// Personal bests are read from platform_storage["missions.json"] and displayed
// on each card.  Bests are written by the individual game mode plugins (hillclimb
// tiers, rock crawl) into their own storage keys; we read those here for display
// and also write a canonical "missions.json" summary on behalf of each mode.
//
// Hotkey: Shift + Tab (ShiftLeft or ShiftRight held while Tab just pressed).
//
// Public API:
//   MissionSelectPlugin
//   MissionSelectOpen (Resource — bool, true while overlay is visible)

use bevy::prelude::*;

use crate::hillclimb_tiers::{HillclimbTiersState, TierLayout, NUM_TIERS, TIER_NAMES};
use crate::obstacle_course::{
    ObstacleCourseLayout, ObstacleCourseState,
    LEVEL_NAMES as OC_LEVEL_NAMES,
    NUM_LEVELS as OC_NUM_LEVELS,
};
use crate::platform_storage;
use crate::rock_crawl_trail::RockCrawlTrailState;
use crate::terrain::terrain_height_at;
use crate::trail_rides::{TrailManifest, TrailRideRequest};
use crate::vehicle::{Chassis, VehicleRoot};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Rock Crawl section names and spawn centres (copied from rock_crawl_trail.rs
/// public constants — we can't use private consts directly).
const RC_SECTION_NAMES: [&str; 3] = [
    "Boulder Stairs",
    "Two-Log Bridge",
    "Off-Camber Traverse",
];

/// World-space X/Z centres of the Rock Crawl start gate (cx − corridor_half).
/// These match rock_crawl_trail::SECTION_CX/CZ/CORRIDOR_HALF.
const RC_START_X: [f32; 3] = [102.0, -90.0, 38.0]; // cx - half
const RC_START_Z: [f32; 3] = [0.0, 80.0, -120.0];

const RC_SECTION_DESC: [&str; 3] = [
    "8 stair-step boulders ascending to the finish.",
    "Two narrow logs spanning a ravine — precision is everything.",
    "Alternating banked slabs demand constant steering correction.",
];

/// Storage key for the unified mission personal-best summary.
const MISSIONS_STORAGE_KEY: &str = "missions.json";

// ---------------------------------------------------------------------------
// Resources & events
// ---------------------------------------------------------------------------

/// True while the Mission Select overlay is open.
#[derive(Resource, Default)]
pub struct MissionSelectOpen(pub bool);

// ---------------------------------------------------------------------------
// Component markers
// ---------------------------------------------------------------------------

#[derive(Component)]
struct MissionSelectRoot;

/// Spawned per fast-travel button with the target encoded as a variant.
#[derive(Component, Clone, Debug)]
enum FastTravelTarget {
    HillclimbTier(usize),
    RockCrawlSection(usize),
    TrailRide(usize),
    ObstacleCourse(usize),
}

/// Marker for a personal-best text node; carries mission id for update.
#[derive(Component)]
struct PbLabel {
    mission_id: String,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct MissionSelectPlugin;

impl Plugin for MissionSelectPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MissionSelectOpen::default())
            .add_systems(Startup, spawn_ui)
            .add_systems(
                Update,
                (
                    toggle_mission_select,
                    handle_fast_travel_buttons,
                    refresh_pb_labels,
                )
                    .chain(),
            );
    }
}

// ---------------------------------------------------------------------------
// Startup: build the full overlay UI
// ---------------------------------------------------------------------------

fn spawn_ui(
    mut commands: Commands,
    manifest:   Res<TrailManifest>,
    hc_state:   Res<HillclimbTiersState>,
    rc_state:   Res<RockCrawlTrailState>,
    oc_state:   Res<ObstacleCourseState>,
) {
    // Background scrim — covers the whole screen.
    let root = commands.spawn((
        MissionSelectRoot,
        Node {
            position_type:   PositionType::Absolute,
            top:             Val::Px(0.0),
            left:            Val::Px(0.0),
            width:           Val::Percent(100.0),
            height:          Val::Percent(100.0),
            flex_direction:  FlexDirection::Column,
            align_items:     AlignItems::Center,
            padding:         UiRect::axes(Val::Px(0.0), Val::Px(24.0)),
            row_gap:         Val::Px(0.0),
            overflow:        Overflow::scroll_y(),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.02, 0.06, 0.93)),
        GlobalZIndex(900),
        Visibility::Hidden,
    )).id();

    // Title bar
    let title = commands.spawn((
        Text::new("MISSION SELECT  [Shift+Tab to close]"),
        TextFont { font_size: 20.0, ..default() },
        TextColor(Color::srgb(1.0, 0.82, 0.18)),
        Node {
            margin: UiRect::all(Val::Px(16.0)),
            ..default()
        },
    )).id();
    commands.entity(root).add_children(&[title]);

    // ── Section: Hillclimb Tiers ────────────────────────────────────────────
    let hc_header = section_header(&mut commands, "HILLCLIMB TIERS", Color::srgb(0.95, 0.70, 0.15));
    commands.entity(root).add_children(&[hc_header]);

    let tier_colors = [
        Color::srgb(0.30, 0.80, 0.30),
        Color::srgb(0.90, 0.65, 0.15),
        Color::srgb(0.90, 0.30, 0.20),
    ];
    let tier_descs = [
        "Gentle grades (20°–35°), good for beginners.",
        "Moderate grades (25°–45°), intermediate challenge.",
        "Steep grades (30°–55°), expert drivers only.",
    ];

    for tier in 0..NUM_TIERS {
        let pb = hc_state.records[tier].best_s;
        let mission_id = format!("hillclimb_tier_{}", tier);
        let card = mission_card(
            &mut commands,
            tier_colors[tier],
            TIER_NAMES[tier],
            tier_descs[tier],
            pb,
            &mission_id,
            Some(FastTravelTarget::HillclimbTier(tier)),
        );
        commands.entity(root).add_children(&[card]);
    }

    // ── Section: Rock Crawl ─────────────────────────────────────────────────
    let rc_header = section_header(&mut commands, "ROCK CRAWL TRAIL", Color::srgb(0.55, 0.80, 0.95));
    commands.entity(root).add_children(&[rc_header]);

    for sec in 0..3_usize {
        let pb = rc_state.records[sec].best_s;
        let mission_id = format!("rock_crawl_{}", sec);
        let card = mission_card(
            &mut commands,
            Color::srgb(0.45, 0.65, 0.85),
            RC_SECTION_NAMES[sec],
            RC_SECTION_DESC[sec],
            pb,
            &mission_id,
            Some(FastTravelTarget::RockCrawlSection(sec)),
        );
        commands.entity(root).add_children(&[card]);
    }

    // ── Section: Trail Rides ────────────────────────────────────────────────
    let tr_header = section_header(&mut commands, "TRAIL RIDES", Color::srgb(0.40, 0.90, 0.55));
    commands.entity(root).add_children(&[tr_header]);

    if manifest.trails.is_empty() {
        let placeholder = commands.spawn((
            Text::new("No trails found. Add a trail entry to assets/trails/manifest.json."),
            TextFont { font_size: 13.0, ..default() },
            TextColor(Color::srgb(0.5, 0.5, 0.5)),
            Node {
                margin: UiRect::axes(Val::Px(24.0), Val::Px(6.0)),
                ..default()
            },
        )).id();
        commands.entity(root).add_children(&[placeholder]);
    }

    for (idx, trail) in manifest.trails.iter().enumerate() {
        let pb = read_pb_for_mission(&trail.id);
        let desc = format!(
            "{}  ·  {:.1} km  ·  {}",
            trail.description, trail.length_km, trail.difficulty
        );
        let mission_id = trail.id.clone();
        let card = mission_card(
            &mut commands,
            Color::srgb(0.22, 0.60, 0.38),
            &trail.title,
            &desc,
            pb,
            &mission_id,
            Some(FastTravelTarget::TrailRide(idx)),
        );
        commands.entity(root).add_children(&[card]);
    }

    // ── Section: Obstacle Course ────────────────────────────────────────────
    let oc_header = section_header(&mut commands, "OBSTACLE COURSE", Color::srgb(0.85, 0.60, 0.20));
    commands.entity(root).add_children(&[oc_header]);

    let oc_colors = [
        Color::srgb(0.30, 0.80, 0.30), // Beginner: green
        Color::srgb(0.90, 0.65, 0.15), // Intermediate: amber
        Color::srgb(0.90, 0.30, 0.20), // Expert: red
    ];
    let oc_descs = [
        "8 obstacles · ~10 m spacing · 15° ramps. North of spawn.",
        "12 obstacles · ~7 m spacing · 25° ramps, boulders & chicane gates.",
        "16 obstacles · ~5 m spacing · 35° ramps, boulder clusters & mud crossings.",
    ];

    for lvl in 0..OC_NUM_LEVELS {
        let pb = oc_state.records[lvl].best_s;
        let mission_id = format!("obstacle_course_{}", lvl);
        let card = mission_card(
            &mut commands,
            oc_colors[lvl],
            OC_LEVEL_NAMES[lvl],
            oc_descs[lvl],
            pb,
            &mission_id,
            Some(FastTravelTarget::ObstacleCourse(lvl)),
        );
        commands.entity(root).add_children(&[card]);
    }

}

// ---------------------------------------------------------------------------
// Helpers: build UI node trees
// ---------------------------------------------------------------------------

fn section_header(commands: &mut Commands, label: &str, color: Color) -> Entity {
    commands.spawn((
        Text::new(label),
        TextFont { font_size: 14.0, ..default() },
        TextColor(color),
        Node {
            margin: UiRect {
                top:    Val::Px(18.0),
                bottom: Val::Px(4.0),
                left:   Val::Px(24.0),
                right:  Val::Px(24.0),
            },
            ..default()
        },
    )).id()
}

/// Build a single mission card row.  Returns the root entity.
/// `fast_travel` = None → card is greyed out (no button).
fn mission_card(
    commands:    &mut Commands,
    swatch:      Color,
    title:       &str,
    description: &str,
    pb:          Option<f32>,
    mission_id:  &str,
    fast_travel: Option<FastTravelTarget>,
) -> Entity {
    let greyed = fast_travel.is_none();
    let text_color = if greyed {
        Color::srgb(0.40, 0.40, 0.40)
    } else {
        Color::srgb(0.90, 0.90, 0.90)
    };
    let card_bg = if greyed {
        Color::srgba(0.08, 0.08, 0.08, 0.70)
    } else {
        Color::srgba(0.06, 0.10, 0.16, 0.85)
    };

    // Card row
    let card = commands.spawn((
        Node {
            flex_direction:  FlexDirection::Row,
            align_items:     AlignItems::Center,
            width:           Val::Percent(90.0),
            margin:          UiRect::axes(Val::Px(0.0), Val::Px(4.0)),
            padding:         UiRect::all(Val::Px(10.0)),
            column_gap:      Val::Px(12.0),
            ..default()
        },
        BackgroundColor(card_bg),
    )).id();

    // Colour swatch
    let swatch_ent = commands.spawn((
        Node {
            width:  Val::Px(10.0),
            height: Val::Percent(100.0),
            min_height: Val::Px(50.0),
            ..default()
        },
        BackgroundColor(if greyed { Color::srgb(0.25, 0.25, 0.25) } else { swatch }),
    )).id();

    // Text column
    let text_col = commands.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            flex_grow:      1.0,
            row_gap:        Val::Px(2.0),
            ..default()
        },
    )).id();

    let title_ent = commands.spawn((
        Text::new(title.to_string()),
        TextFont { font_size: 15.0, ..default() },
        TextColor(text_color),
    )).id();

    let desc_ent = commands.spawn((
        Text::new(description.to_string()),
        TextFont { font_size: 11.0, ..default() },
        TextColor(if greyed {
            Color::srgb(0.30, 0.30, 0.30)
        } else {
            Color::srgb(0.60, 0.65, 0.70)
        }),
    )).id();

    let pb_str = pb.map(|s| format_time(s))
        .unwrap_or_else(|| "Best: --".to_string());
    let pb_ent = commands.spawn((
        PbLabel { mission_id: mission_id.to_string() },
        Text::new(format!("PB  {}", pb_str)),
        TextFont { font_size: 11.0, ..default() },
        TextColor(if greyed {
            Color::srgb(0.30, 0.30, 0.30)
        } else {
            Color::srgb(0.50, 0.80, 0.50)
        }),
    )).id();

    commands.entity(text_col).add_children(&[title_ent, desc_ent, pb_ent]);

    // Fast-Travel button (omitted for greyed-out cards)
    if let Some(target) = fast_travel {
        let btn = commands.spawn((
            target,
            Button,
            Node {
                padding:         UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                justify_content: JustifyContent::Center,
                align_items:     AlignItems::Center,
                min_width:       Val::Px(110.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.35, 0.18)),
        )).id();

        let btn_label = commands.spawn((
            Text::new("FAST TRAVEL"),
            TextFont { font_size: 11.0, ..default() },
            TextColor(Color::srgb(0.70, 1.00, 0.75)),
        )).id();

        commands.entity(btn).add_children(&[btn_label]);
        commands.entity(card).add_children(&[swatch_ent, text_col, btn]);
    } else {
        // Greyed card: "Coming Soon" label instead of button.
        let coming_soon = commands.spawn((
            Text::new("Coming Soon"),
            TextFont { font_size: 10.0, ..default() },
            TextColor(Color::srgb(0.35, 0.35, 0.35)),
            Node {
                min_width: Val::Px(90.0),
                ..default()
            },
        )).id();
        commands.entity(card).add_children(&[swatch_ent, text_col, coming_soon]);
    }

    card
}

// ---------------------------------------------------------------------------
// System: toggle overlay on Shift+Tab
// ---------------------------------------------------------------------------

fn toggle_mission_select(
    keys:     Res<ButtonInput<KeyCode>>,
    mut open: ResMut<MissionSelectOpen>,
    mut vis_q: Query<&mut Visibility, With<MissionSelectRoot>>,
) {
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if !(shift && keys.just_pressed(KeyCode::Tab)) {
        return;
    }

    open.0 = !open.0;

    for mut vis in vis_q.iter_mut() {
        *vis = if open.0 { Visibility::Visible } else { Visibility::Hidden };
    }

    info!(
        "mission_select: overlay {}",
        if open.0 { "opened" } else { "closed" }
    );
}

// ---------------------------------------------------------------------------
// System: handle FAST TRAVEL button clicks
// ---------------------------------------------------------------------------

fn handle_fast_travel_buttons(
    interaction_q: Query<(&Interaction, &FastTravelTarget), (Changed<Interaction>, With<Button>)>,
    layout:        Res<TierLayout>,
    oc_layout:     Res<ObstacleCourseLayout>,
    manifest:      Res<TrailManifest>,
    mut open:      ResMut<MissionSelectOpen>,
    mut vis_q:     Query<&mut Visibility, With<MissionSelectRoot>>,
    vehicle_opt:   Option<Res<VehicleRoot>>,
    mut chassis_q: Query<(&mut Transform, &mut avian3d::prelude::LinearVelocity,
                          &mut avian3d::prelude::AngularVelocity), With<Chassis>>,
    mut trail_req: ResMut<TrailRideRequest>,
) {
    for (interaction, target) in interaction_q.iter() {
        if *interaction != Interaction::Pressed { continue; }

        info!("mission_select: fast-travel to {:?}", target);

        match target {
            FastTravelTarget::HillclimbTier(tier) => {
                let pos = layout.start_pos[*tier];
                teleport_chassis(vehicle_opt.as_deref(), &mut chassis_q, pos);
            }
            FastTravelTarget::RockCrawlSection(sec) => {
                let cx = RC_START_X[*sec];
                let cz = RC_START_Z[*sec];
                let spawn_y = terrain_height_at(cx, cz) + 1.5;
                teleport_chassis(
                    vehicle_opt.as_deref(),
                    &mut chassis_q,
                    Vec3::new(cx, spawn_y, cz),
                );
            }
            FastTravelTarget::TrailRide(idx) => {
                trail_req.trail_idx = Some(*idx);
                // Teleport is handled by TrailRidesPlugin; we just set the request.
                if let Some(trail) = manifest.trails.get(*idx) {
                    info!(
                        "mission_select: trail ride '{}' requested via TrailRideRequest",
                        trail.title
                    );
                }
            }
            FastTravelTarget::ObstacleCourse(lvl) => {
                let pos = oc_layout.start_pos[*lvl];
                teleport_chassis(vehicle_opt.as_deref(), &mut chassis_q, pos);
            }
        }

        // Close the overlay after any fast-travel.
        open.0 = false;
        for mut vis in vis_q.iter_mut() {
            *vis = Visibility::Hidden;
        }
    }
}

fn teleport_chassis(
    vehicle: Option<&VehicleRoot>,
    chassis_q: &mut Query<(&mut Transform, &mut avian3d::prelude::LinearVelocity,
                           &mut avian3d::prelude::AngularVelocity), With<Chassis>>,
    destination: Vec3,
) {
    let Some(veh) = vehicle else {
        warn!("mission_select: fast-travel attempted but VehicleRoot not present");
        return;
    };

    if let Ok((mut tf, mut linvel, mut angvel)) = chassis_q.get_mut(veh.chassis) {
        tf.translation = destination;
        tf.rotation = Quat::IDENTITY;
        linvel.0 = Vec3::ZERO;
        angvel.0 = Vec3::ZERO;
        info!("mission_select: chassis teleported to {:.1?}", destination);
    }
}

// ---------------------------------------------------------------------------
// System: refresh PB label text (runs every frame, cheap string update only
// when the overlay is open)
// ---------------------------------------------------------------------------

fn refresh_pb_labels(
    open:      Res<MissionSelectOpen>,
    hc_state:  Res<HillclimbTiersState>,
    rc_state:  Res<RockCrawlTrailState>,
    oc_state:  Res<ObstacleCourseState>,
    mut pb_q:  Query<(&PbLabel, &mut Text)>,
) {
    if !open.0 { return; }

    for (label, mut text) in pb_q.iter_mut() {
        let pb = resolve_pb_for_mission(&label.mission_id, &hc_state, &rc_state, &oc_state);
        let new_str = format!(
            "PB  {}",
            pb.map(format_time).unwrap_or_else(|| "--".to_string())
        );
        if text.0 != new_str {
            text.0 = new_str;
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers: personal-best resolution
// ---------------------------------------------------------------------------

/// Resolve a personal best for a given mission_id by inspecting live state.
/// Falls back to platform_storage for Trail Ride entries.
fn resolve_pb_for_mission(
    mission_id: &str,
    hc_state:   &HillclimbTiersState,
    rc_state:   &RockCrawlTrailState,
    oc_state:   &ObstacleCourseState,
) -> Option<f32> {
    // Hillclimb tiers
    for tier in 0..NUM_TIERS {
        if mission_id == format!("hillclimb_tier_{}", tier) {
            return hc_state.records[tier].best_s;
        }
    }
    // Rock crawl sections
    for sec in 0..3_usize {
        if mission_id == format!("rock_crawl_{}", sec) {
            return rc_state.records[sec].best_s;
        }
    }
    // Obstacle course levels
    for lvl in 0..OC_NUM_LEVELS {
        if mission_id == format!("obstacle_course_{}", lvl) {
            return oc_state.records[lvl].best_s;
        }
    }
    // Trail rides and others: read from platform_storage missions.json
    read_pb_for_mission(mission_id)
}

/// Read a personal best from platform_storage["missions.json"] for a given key.
fn read_pb_for_mission(mission_id: &str) -> Option<f32> {
    let text = platform_storage::read_string(MISSIONS_STORAGE_KEY)?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    v.get(mission_id)?.as_f64().map(|x| x as f32)
}

/// Write a personal best to platform_storage["missions.json"].
#[allow(dead_code)]
pub fn save_mission_pb(mission_id: &str, best_s: f32) {
    let mut map: serde_json::Map<String, serde_json::Value> = {
        platform_storage::read_string(MISSIONS_STORAGE_KEY)
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    };
    map.insert(
        mission_id.to_string(),
        serde_json::Value::Number(
            serde_json::Number::from_f64(best_s as f64)
                .unwrap_or_else(|| serde_json::Number::from(0)),
        ),
    );
    let json = serde_json::to_string(&map).unwrap_or_default();
    if let Err(e) = platform_storage::write_string(MISSIONS_STORAGE_KEY, &json) {
        warn!("mission_select: could not save missions.json: {}", e);
    }
}

// ---------------------------------------------------------------------------
// Time formatting helper (MM:SS.cc)
// ---------------------------------------------------------------------------

fn format_time(s: f32) -> String {
    let s    = s.max(0.0);
    let mins = (s / 60.0) as u32;
    let rem  = s - (mins as f32) * 60.0;
    let sec  = rem as u32;
    let cs   = ((rem % 1.0) * 100.0) as u32;
    format!("{:02}:{:02}.{:02}", mins, sec, cs)
}
