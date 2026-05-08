// Notification queue: centralized toast popups in top-right corner.
// Other modules push messages via NotificationQueue::push(text, color).
// Each toast slides up from below, displays for 3s, then fades.
//
// Public API:
//   NotificationsPlugin
//   NotificationQueue (resource with push method)
//   Notification struct

use bevy::prelude::*;

// ---- Public plugin ----------------------------------------------------------

pub struct NotificationsPlugin;

impl Plugin for NotificationsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NotificationQueue>()
            .add_systems(Startup, spawn_toast_slots)
            .add_systems(Update, (pop_pending, tick_active));
    }
}

// ---- Public resource & notification struct ----------------------------------

/// Centralized queue other systems use to display ephemeral toast messages.
/// Add an `active` field here so lifetime state lives in one resource.
#[derive(Resource, Default)]
pub struct NotificationQueue {
    pub pending: Vec<Notification>,
    active: Vec<ActiveToast>,
}

#[derive(Clone, Debug)]
pub struct Notification {
    pub text: String,
    pub color: Color,
    pub duration_s: f32,
}

impl NotificationQueue {
    /// Push a new notification with the default 3-second lifetime.
    pub fn push(&mut self, text: impl Into<String>, color: Color) {
        self.pending.push(Notification {
            text: text.into(),
            color,
            duration_s: 3.0,
        });
    }
}

// ---- Internal active-toast state --------------------------------------------

#[derive(Clone, Debug)]
struct ActiveToast {
    text: String,
    color: Color,
    age_s: f32,
    total_s: f32,
}

// ---- Slot component markers -------------------------------------------------

/// Marks the panel Node for slot `index` (0 = bottom/newest, 2 = top/oldest).
#[derive(Component)]
struct ToastSlot(usize);

/// Marks the Text child for slot `index`.
#[derive(Component)]
struct ToastSlotText(usize);

// ---- Layout constants -------------------------------------------------------

const MAX_SLOTS: usize = 3;
const SLOT_HEIGHT: f32 = 36.0;
const SLOT_GAP: f32 = 6.0;
const SLOT_WIDTH: f32 = 260.0;
const SLOT_RIGHT: f32 = 14.0;
/// Top edge of the bottom-most (newest) slot.  Leaderboard sits above ~350 px.
const STACK_TOP: f32 = 350.0;
const TOAST_BG: Color = Color::srgba(0.05, 0.05, 0.07, 0.85);
const TEXT_SIZE: f32 = 14.0;

// ---- Startup: spawn 3 pre-allocated slot panels ----------------------------

fn spawn_toast_slots(mut commands: Commands) {
    // Slot 0 is bottommost (newest), slot 2 is topmost (oldest).
    // We position them stacked downward from STACK_TOP.
    for i in 0..MAX_SLOTS {
        let top_offset = STACK_TOP + i as f32 * (SLOT_HEIGHT + SLOT_GAP);

        let panel = commands
            .spawn((
                ToastSlot(i),
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(SLOT_RIGHT),
                    top: Val::Px(top_offset),
                    width: Val::Px(SLOT_WIDTH),
                    height: Val::Px(SLOT_HEIGHT),
                    display: Display::None,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(TOAST_BG),
                ZIndex(250),
            ))
            .id();

        let text_child = commands
            .spawn((
                ToastSlotText(i),
                Text::new(""),
                TextFont {
                    font_size: TEXT_SIZE,
                    ..default()
                },
                TextColor(Color::WHITE),
            ))
            .id();

        commands.entity(panel).add_children(&[text_child]);
    }

    // Push a startup "Game ready!" toast so the system is exercised immediately.
    // We do this via direct resource mutation in a PostStartup system instead,
    // but since we only have Startup here we add it inline via a helper command.
    commands.queue(|world: &mut World| {
        if let Some(mut queue) = world.get_resource_mut::<NotificationQueue>() {
            queue.push("Game ready!", Color::srgb(0.3, 0.95, 0.4));
        }
    });
}

// ---- pop_pending: drain front of pending into active ------------------------

fn pop_pending(mut queue: ResMut<NotificationQueue>) {
    while !queue.pending.is_empty() && queue.active.len() < MAX_SLOTS {
        let notif = queue.pending.remove(0);
        queue.active.push(ActiveToast {
            text: notif.text,
            color: notif.color,
            age_s: 0.0,
            total_s: notif.duration_s,
        });
    }
}

// ---- tick_active: advance age, compute alpha, update slots ------------------

fn tick_active(
    time: Res<Time>,
    mut queue: ResMut<NotificationQueue>,
    mut panel_q: Query<(&ToastSlot, &mut Node, &mut BackgroundColor)>,
    mut text_q: Query<(&ToastSlotText, &mut Text, &mut TextColor)>,
) {
    let dt = time.delta_secs();

    // Advance all active toast ages; remove expired ones.
    for toast in &mut queue.active {
        toast.age_s += dt;
    }
    queue.active.retain(|t| t.age_s < t.total_s);

    // Build a snapshot to avoid borrow conflicts.
    let active_snapshot: Vec<ActiveToast> = queue.active.clone();
    let active_count = active_snapshot.len();

    // Update each slot panel.
    for (slot, mut node, mut bg) in &mut panel_q {
        let idx = slot.0;

        if idx < active_count {
            let toast = &active_snapshot[idx];
            let alpha = compute_alpha(toast.age_s, toast.total_s);
            node.display = Display::Flex;

            let bg_lin = TOAST_BG.to_linear();
            bg.0 = Color::linear_rgba(bg_lin.red, bg_lin.green, bg_lin.blue, bg_lin.alpha * alpha);
        } else {
            node.display = Display::None;
            bg.0 = Color::linear_rgba(0.0, 0.0, 0.0, 0.0);
        }
    }

    // Update each slot text child.
    for (slot_text, mut text, mut tc) in &mut text_q {
        let idx = slot_text.0;

        if idx < active_count {
            let toast = &active_snapshot[idx];
            let alpha = compute_alpha(toast.age_s, toast.total_s);
            text.0 = toast.text.clone();
            let lin = toast.color.to_linear();
            tc.0 = Color::linear_rgba(lin.red, lin.green, lin.blue, alpha);
        } else {
            text.0 = String::new();
        }
    }
}

/// Compute opacity for a toast given its current age and total lifetime.
///
/// - First 0.3 s: fade in 0 → 1
/// - 0.3 s .. (total − 0.6 s): fully opaque
/// - Last 0.6 s: fade out 1 → 0
#[inline]
fn compute_alpha(age: f32, total: f32) -> f32 {
    const FADE_IN: f32 = 0.3;
    const FADE_OUT: f32 = 0.6;

    if age < FADE_IN {
        age / FADE_IN
    } else if age < total - FADE_OUT {
        1.0
    } else {
        let remaining = total - age;
        (remaining / FADE_OUT).clamp(0.0, 1.0)
    }
}
