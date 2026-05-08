// Event log: detects notable gameplay events and renders a scrolling ticker.
//
// Detection runs each Update frame. Emitted events go into EventLog (ring buffer,
// cap 8). The UI ticker shows the most recent 5, fading each over 8 seconds.
// Press E to toggle the panel.

use std::collections::VecDeque;
use bevy::prelude::*;
use avian3d::prelude::LinearVelocity;
use crate::terrain::terrain_height_at;
use crate::vehicle::{Chassis, VehicleRoot};

// ---- Public types -----------------------------------------------------------

#[derive(Clone, Debug)]
pub enum GameEvent {
    HardImpact { v: f32 },
    BigTilt { tilt_deg: f32 },
    DistanceMilestone { km: u32 },
    SpeedMilestone { mph: u32 },
    BrakeStop { from_mph: f32 },
    Airtime { duration_s: f32 },
}

#[derive(Resource, Default)]
pub struct EventLog {
    pub events: VecDeque<(f32, GameEvent)>, // (timestamp, event)
}

impl EventLog {
    const CAP: usize = 8;

    fn push(&mut self, t: f32, ev: GameEvent) {
        if self.events.len() >= Self::CAP {
            self.events.pop_front();
        }
        self.events.push_back((t, ev));
    }
}

// ---- Plugin -----------------------------------------------------------------

pub struct EventLogPlugin;

impl Plugin for EventLogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EventLog>()
            .init_resource::<EventDetectorState>()
            .init_resource::<EventPanelVisible>()
            .add_systems(Startup, spawn_event_panel)
            .add_systems(
                Update,
                (
                    detect_events,
                    update_event_panel,
                    toggle_event_panel,
                )
                    .run_if(resource_exists::<VehicleRoot>),
            );
    }
}

// ---- Detection state --------------------------------------------------------

// How many velocity samples to keep for impact detection.
const VEL_RING: usize = 6;
// Chassis height above terrain at which we consider it airborne.
const AIRBORNE_H: f32 = 1.5;
// Minimum airborne time to emit an Airtime event.
const AIRTIME_THRESHOLD: f32 = 0.4;
// Hard-impact threshold: downward speed (positive = downward in our convention).
const IMPACT_THRESHOLD: f32 = 5.0;
// Big tilt threshold in degrees.
const TILT_THRESHOLD: f32 = 45.0;
// Distance milestone in metres.
const DIST_MILESTONE_M: f32 = 500.0;
// Speed milestone granularity (mph).
const SPEED_MILESTONE_STEP: u32 = 5;
// BrakeStop: from > this mph, to < 1 mph within 5 s.
const BRAKE_STOP_FROM_MPH: f32 = 10.0;
const BRAKE_STOP_WINDOW_S: f32 = 5.0;

#[derive(Resource, Default)]
struct EventDetectorState {
    // Ring of recent vertical velocities (vy, positive = up).
    vy_ring: [f32; VEL_RING],
    vy_head: usize,
    // Was falling last frame?
    was_falling: bool,

    // Tilt: are we currently in a tilt warning?
    in_tilt: bool,

    // Distance tracking (independent of hud.rs SessionStats).
    last_pos_xz: Option<Vec2>,
    distance_m: f32,
    last_milestone_m: f32,

    // Speed milestone: highest 5-mph boundary crossed.
    max_speed_milestone_mph: u32,

    // BrakeStop: track high-speed entry and time.
    brake_peak_mph: f32,
    brake_peak_time: f32,

    // Airtime tracking.
    airborne_start: Option<f32>,
}

// ---- Detection system -------------------------------------------------------

fn detect_events(
    vehicle: Res<VehicleRoot>,
    chassis_q: Query<(&Transform, &LinearVelocity), With<Chassis>>,
    time: Res<Time>,
    mut state: ResMut<EventDetectorState>,
    mut log: ResMut<EventLog>,
) {
    let Ok((transform, lin_vel)) = chassis_q.get(vehicle.chassis) else {
        return;
    };

    let t = time.elapsed_secs();
    let dt = time.delta_secs();
    let pos = transform.translation;
    let vy = lin_vel.y; // positive = moving up

    // ---- HardImpact ---------------------------------------------------------
    // Store current vy in ring (read index into local first to satisfy borrow checker).
    let head = state.vy_head;
    state.vy_ring[head] = vy;
    state.vy_head = (head + 1) % VEL_RING;

    // Min vy over the ring = peak downward velocity this window.
    let min_vy = state.vy_ring.iter().cloned().fold(f32::INFINITY, f32::min);

    if state.was_falling && min_vy < -IMPACT_THRESHOLD && vy > -1.0 {
        // Just transitioned from hard downward to slow/upward: impact.
        log.push(t, GameEvent::HardImpact { v: min_vy });
    }
    state.was_falling = vy < -IMPACT_THRESHOLD;

    // ---- BigTilt ------------------------------------------------------------
    let chassis_up = transform.up();
    let dot = chassis_up.dot(Vec3::Y).clamp(-1.0, 1.0);
    let tilt_deg = dot.acos().to_degrees();

    if tilt_deg > TILT_THRESHOLD && !state.in_tilt {
        log.push(t, GameEvent::BigTilt { tilt_deg });
        state.in_tilt = true;
    } else if tilt_deg <= TILT_THRESHOLD {
        state.in_tilt = false;
    }

    // ---- DistanceMilestone --------------------------------------------------
    let pos_xz = Vec2::new(pos.x, pos.z);
    if let Some(prev) = state.last_pos_xz {
        let delta = pos_xz.distance(prev);
        // Gate large jumps (teleports / first frame).
        if delta <= 5.0 {
            state.distance_m += delta;
            let crossed = (state.distance_m / DIST_MILESTONE_M).floor() as u32;
            let prev_crossed = (state.last_milestone_m / DIST_MILESTONE_M).floor() as u32;
            if crossed > prev_crossed {
                let km_half = crossed; // each unit = 0.5 km
                log.push(t, GameEvent::DistanceMilestone { km: km_half });
                state.last_milestone_m = state.distance_m;
            }
        }
    }
    state.last_pos_xz = Some(pos_xz);

    // ---- SpeedMilestone -----------------------------------------------------
    let speed_mps = Vec3::from(lin_vel.0).length();
    let speed_mph = speed_mps * 2.237;
    // Which 5-mph boundary has been reached?
    let current_bucket = (speed_mph / SPEED_MILESTONE_STEP as f32).floor() as u32 * SPEED_MILESTONE_STEP;
    if current_bucket > 0 && current_bucket > state.max_speed_milestone_mph {
        // Fire for each new bucket crossed (usually just one at a time).
        let mut bucket = state.max_speed_milestone_mph + SPEED_MILESTONE_STEP;
        while bucket <= current_bucket {
            log.push(t, GameEvent::SpeedMilestone { mph: bucket });
            bucket += SPEED_MILESTONE_STEP;
        }
        state.max_speed_milestone_mph = current_bucket;
    }

    // ---- BrakeStop ----------------------------------------------------------
    if speed_mph > BRAKE_STOP_FROM_MPH {
        // Record the highest speed seen in recent high-speed window.
        state.brake_peak_mph = state.brake_peak_mph.max(speed_mph);
        state.brake_peak_time = t;
    } else if speed_mph < 1.0
        && state.brake_peak_mph > BRAKE_STOP_FROM_MPH
        && (t - state.brake_peak_time) < BRAKE_STOP_WINDOW_S
    {
        log.push(t, GameEvent::BrakeStop { from_mph: state.brake_peak_mph });
        // Reset so we don't fire again until we speed up again.
        state.brake_peak_mph = 0.0;
    } else if speed_mph < 1.0 {
        // Vehicle has been slow too long or never sped up — reset.
        if (t - state.brake_peak_time) >= BRAKE_STOP_WINDOW_S {
            state.brake_peak_mph = 0.0;
        }
    }

    // ---- Airtime ------------------------------------------------------------
    let terrain_y = terrain_height_at(pos.x, pos.z);
    let height_above = pos.y - terrain_y;
    let is_airborne = height_above > AIRBORNE_H;

    if is_airborne {
        if state.airborne_start.is_none() {
            state.airborne_start = Some(t);
        }
    } else if let Some(start) = state.airborne_start.take() {
        let duration_s = t - start;
        // Ignore the very first tick (dt) to avoid a spurious event at startup.
        if duration_s > AIRTIME_THRESHOLD && duration_s > dt * 2.0 {
            log.push(t, GameEvent::Airtime { duration_s });
        }
    }

    // Unused: suppress lint on dt used only in airtime guard above.
    let _ = dt;
}

// ---- UI ---------------------------------------------------------------------

const FADE_DURATION: f32 = 8.0;
const MAX_DISPLAYED: usize = 5;
const BG: Color = Color::srgba(0.05, 0.05, 0.07, 0.75);
const COLOR_IMPACT: Color = Color::srgb(0.95, 0.35, 0.25);
const COLOR_MILESTONE: Color = Color::srgb(0.4, 0.9, 0.4);
const COLOR_AIRTIME: Color = Color::srgb(0.95, 0.85, 0.2);

fn event_color(ev: &GameEvent) -> Color {
    match ev {
        GameEvent::HardImpact { .. } | GameEvent::BigTilt { .. } => COLOR_IMPACT,
        GameEvent::DistanceMilestone { .. } | GameEvent::SpeedMilestone { .. } => COLOR_MILESTONE,
        GameEvent::BrakeStop { .. } => Color::WHITE,
        GameEvent::Airtime { .. } => COLOR_AIRTIME,
    }
}

fn event_text(ev: &GameEvent) -> String {
    match ev {
        GameEvent::HardImpact { v } => format!("IMPACT! {:.1} m/s vertical", v),
        GameEvent::BigTilt { tilt_deg } => format!("TILT WARNING: {:.1}\u{b0}", tilt_deg),
        GameEvent::DistanceMilestone { km } => {
            // km here is actually the count of 500 m intervals crossed.
            let dist_km = *km as f32 * 0.5;
            format!("+{:.1} km traveled", dist_km)
        }
        GameEvent::SpeedMilestone { mph } => format!("{} mph reached!", mph),
        GameEvent::BrakeStop { from_mph } => format!("Stopped from {:.0} mph", from_mph),
        GameEvent::Airtime { duration_s } => format!("Airtime: {:.1} s", duration_s),
    }
}

#[derive(Resource)]
struct EventPanelVisible(bool);

impl Default for EventPanelVisible {
    fn default() -> Self { Self(true) }
}

#[derive(Component)]
struct EventPanelRoot;

/// Each text row in the panel. Index 0 = oldest visible, 4 = newest.
#[derive(Component)]
struct EventRow(usize);

fn spawn_event_panel(mut commands: Commands) {
    let root = commands
        .spawn((
            EventPanelRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(12.0),
                bottom: Val::Px(12.0),
                width: Val::Px(340.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(10.0)),
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(BG),
        ))
        .id();

    for i in 0..MAX_DISPLAYED {
        let row = commands
            .spawn((
                EventRow(i),
                Text::new(""),
                TextFont { font_size: 13.0, ..default() },
                TextColor(Color::NONE),
            ))
            .id();
        commands.entity(root).add_child(row);
    }
}

fn update_event_panel(
    log: Res<EventLog>,
    time: Res<Time>,
    mut rows: Query<(&EventRow, &mut Text, &mut TextColor)>,
) {
    let now = time.elapsed_secs();

    // Collect events still within fade window, newest-last.
    let visible: Vec<_> = log
        .events
        .iter()
        .filter(|(ts, _)| now - ts < FADE_DURATION)
        .collect();

    // We display at most MAX_DISPLAYED, taking the tail (most recent).
    let start = if visible.len() > MAX_DISPLAYED {
        visible.len() - MAX_DISPLAYED
    } else {
        0
    };
    let slice = &visible[start..];

    for (row, mut text, mut color) in &mut rows {
        let idx = row.0;
        // slot 0 = oldest of the visible slice, slot MAX_DISPLAYED-1 = newest.
        let slot_in_slice = idx as isize - (MAX_DISPLAYED as isize - slice.len() as isize);
        if slot_in_slice >= 0 && (slot_in_slice as usize) < slice.len() {
            let (ts, ev) = slice[slot_in_slice as usize];
            let age = now - ts;
            let alpha = (1.0 - age / FADE_DURATION).clamp(0.0, 1.0);
            let base = event_color(ev);
            // Apply alpha by extracting components and rebuilding with faded alpha.
            let lin = base.to_linear();
            color.0 = Color::linear_rgba(lin.red, lin.green, lin.blue, alpha);
            text.0 = event_text(ev);
        } else {
            text.0 = String::new();
            color.0 = Color::NONE;
        }
    }
}

fn toggle_event_panel(
    keys: Res<ButtonInput<KeyCode>>,
    mut visible: ResMut<EventPanelVisible>,
    mut root_q: Query<&mut Node, With<EventPanelRoot>>,
) {
    if keys.just_pressed(KeyCode::KeyE) {
        visible.0 = !visible.0;
        for mut node in &mut root_q {
            node.display = if visible.0 { Display::Flex } else { Display::None };
        }
    }
}
