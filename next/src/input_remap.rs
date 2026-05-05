// Configurable keybindings: shows a binding panel toggled via a slash key,
// lets player rebind core controls (forward/back/steer/brake/handbrake/boost).
// Default mappings mirror WASD; persisted in ~/.sandk-offroad/keybindings.json.
//
// Public API:
//   InputRemapPlugin
//   KeyBindings (resource)

use bevy::prelude::*;
use std::fs;
use std::io::Write as IoWrite;
use std::path::PathBuf;

use crate::vehicle::{DriveInput, drive_input_keyboard};

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct InputRemapPlugin;

impl Plugin for InputRemapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KeyBindings>()
            .init_resource::<InputRemapState>()
            .init_resource::<BindingSaveDebounce>()
            .add_systems(Startup, (load_keybindings, spawn_remap_panel).chain())
            .add_systems(
                Update,
                (
                    toggle_with_slash,
                    cycle_cursor,
                    capture_new_key,
                    apply_bindings_to_drive_input.after(drive_input_keyboard),
                    update_panel_view,
                    save_keybindings_on_change,
                ),
            );
    }
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

#[derive(Resource, Clone)]
pub struct KeyBindings {
    pub forward:   KeyCode,
    pub back:      KeyCode,
    pub left:      KeyCode,
    pub right:     KeyCode,
    pub brake:     KeyCode,
    pub handbrake: KeyCode,
    pub boost:     KeyCode,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            forward:   KeyCode::KeyW,
            back:      KeyCode::KeyS,
            left:      KeyCode::KeyA,
            right:     KeyCode::KeyD,
            brake:     KeyCode::Space,
            handbrake: KeyCode::ShiftLeft,
            boost:     KeyCode::KeyB,
        }
    }
}

/// UI state for the remap panel.
#[derive(Resource, Default)]
pub struct InputRemapState {
    /// Whether the panel is open.
    pub open:         bool,
    /// Index of the currently highlighted action row (0‥6).
    pub cursor_idx:   usize,
    /// When Some, we are waiting for the player to press a new key for that slot.
    pub awaiting_bind: Option<usize>,
}

/// Debounce for disk saves: we only write after KeyBindings hasn't changed for
/// 0.5 s, to avoid a write every frame.
#[derive(Resource, Default)]
struct BindingSaveDebounce {
    pending:   bool,
    elapsed_s: f32,
}

// ---------------------------------------------------------------------------
// Action metadata helpers
// ---------------------------------------------------------------------------

const ACTION_COUNT: usize = 7;

fn action_label(idx: usize) -> &'static str {
    match idx {
        0 => "Forward",
        1 => "Back",
        2 => "Left",
        3 => "Right",
        4 => "Brake",
        5 => "Handbrake",
        6 => "Boost",
        _ => "Unknown",
    }
}

fn get_binding(bindings: &KeyBindings, idx: usize) -> KeyCode {
    match idx {
        0 => bindings.forward,
        1 => bindings.back,
        2 => bindings.left,
        3 => bindings.right,
        4 => bindings.brake,
        5 => bindings.handbrake,
        6 => bindings.boost,
        _ => KeyCode::KeyW,
    }
}

fn set_binding(bindings: &mut KeyBindings, idx: usize, key: KeyCode) {
    match idx {
        0 => bindings.forward   = key,
        1 => bindings.back      = key,
        2 => bindings.left      = key,
        3 => bindings.right     = key,
        4 => bindings.brake     = key,
        5 => bindings.handbrake = key,
        6 => bindings.boost     = key,
        _ => {}
    }
}

/// Return a short printable label for a KeyCode.
fn keycode_label(k: KeyCode) -> &'static str {
    match k {
        KeyCode::KeyA => "A",
        KeyCode::KeyB => "B",
        KeyCode::KeyC => "C",
        KeyCode::KeyD => "D",
        KeyCode::KeyE => "E",
        KeyCode::KeyF => "F",
        KeyCode::KeyG => "G",
        KeyCode::KeyH => "H",
        KeyCode::KeyI => "I",
        KeyCode::KeyJ => "J",
        KeyCode::KeyK => "K",
        KeyCode::KeyL => "L",
        KeyCode::KeyM => "M",
        KeyCode::KeyN => "N",
        KeyCode::KeyO => "O",
        KeyCode::KeyP => "P",
        KeyCode::KeyQ => "Q",
        KeyCode::KeyR => "R",
        KeyCode::KeyS => "S",
        KeyCode::KeyT => "T",
        KeyCode::KeyU => "U",
        KeyCode::KeyV => "V",
        KeyCode::KeyW => "W",
        KeyCode::KeyX => "X",
        KeyCode::KeyY => "Y",
        KeyCode::KeyZ => "Z",
        KeyCode::Digit0 => "0",
        KeyCode::Digit1 => "1",
        KeyCode::Digit2 => "2",
        KeyCode::Digit3 => "3",
        KeyCode::Digit4 => "4",
        KeyCode::Digit5 => "5",
        KeyCode::Digit6 => "6",
        KeyCode::Digit7 => "7",
        KeyCode::Digit8 => "8",
        KeyCode::Digit9 => "9",
        KeyCode::Space        => "Space",
        KeyCode::Enter        => "Enter",
        KeyCode::Backspace    => "Backspace",
        KeyCode::Tab          => "Tab",
        KeyCode::Escape       => "Escape",
        KeyCode::ArrowUp      => "Up",
        KeyCode::ArrowDown    => "Down",
        KeyCode::ArrowLeft    => "Left",
        KeyCode::ArrowRight   => "Right",
        KeyCode::ShiftLeft    => "LShift",
        KeyCode::ShiftRight   => "RShift",
        KeyCode::ControlLeft  => "LCtrl",
        KeyCode::ControlRight => "RCtrl",
        KeyCode::AltLeft      => "LAlt",
        KeyCode::AltRight     => "RAlt",
        KeyCode::F1  => "F1",  KeyCode::F2  => "F2",  KeyCode::F3  => "F3",
        KeyCode::F4  => "F4",  KeyCode::F5  => "F5",  KeyCode::F6  => "F6",
        KeyCode::F7  => "F7",  KeyCode::F8  => "F8",  KeyCode::F9  => "F9",
        KeyCode::F10 => "F10", KeyCode::F11 => "F11", KeyCode::F12 => "F12",
        KeyCode::Minus        => "-",
        KeyCode::Equal        => "=",
        KeyCode::BracketLeft  => "[",
        KeyCode::BracketRight => "]",
        KeyCode::Semicolon    => ";",
        KeyCode::Quote        => "'",
        KeyCode::Comma        => ",",
        KeyCode::Period       => ".",
        KeyCode::Slash        => "/",
        KeyCode::Backslash    => "\\",
        KeyCode::Backquote    => "`",
        _ => "?",
    }
}

/// Returns true if a key should be treated as a modifier-only key and not
/// accepted as a primary binding target.
fn is_modifier_only(k: KeyCode) -> bool {
    matches!(
        k,
        KeyCode::ShiftLeft
            | KeyCode::ShiftRight
            | KeyCode::ControlLeft
            | KeyCode::ControlRight
            | KeyCode::AltLeft
            | KeyCode::AltRight
            | KeyCode::SuperLeft
            | KeyCode::SuperRight
    )
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

fn keybindings_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let mut p = PathBuf::from(home);
    p.push(".sandk-offroad");
    p.push("keybindings.json");
    p
}

fn keycode_to_str(k: KeyCode) -> &'static str {
    keycode_label(k)
}

/// Hand-rolled JSON for keybindings (avoids adding Serialize to KeyCode).
fn keybindings_to_json(b: &KeyBindings) -> String {
    format!(
        "{{\n  \"forward\": \"{}\",\n  \"back\": \"{}\",\n  \"left\": \"{}\",\n  \"right\": \"{}\",\n  \"brake\": \"{}\",\n  \"handbrake\": \"{}\",\n  \"boost\": \"{}\"\n}}",
        keycode_to_str(b.forward),
        keycode_to_str(b.back),
        keycode_to_str(b.left),
        keycode_to_str(b.right),
        keycode_to_str(b.brake),
        keycode_to_str(b.handbrake),
        keycode_to_str(b.boost),
    )
}

fn str_to_keycode(s: &str) -> Option<KeyCode> {
    match s {
        "A" => Some(KeyCode::KeyA), "B" => Some(KeyCode::KeyB),
        "C" => Some(KeyCode::KeyC), "D" => Some(KeyCode::KeyD),
        "E" => Some(KeyCode::KeyE), "F" => Some(KeyCode::KeyF),
        "G" => Some(KeyCode::KeyG), "H" => Some(KeyCode::KeyH),
        "I" => Some(KeyCode::KeyI), "J" => Some(KeyCode::KeyJ),
        "K" => Some(KeyCode::KeyK), "L" => Some(KeyCode::KeyL),
        "M" => Some(KeyCode::KeyM), "N" => Some(KeyCode::KeyN),
        "O" => Some(KeyCode::KeyO), "P" => Some(KeyCode::KeyP),
        "Q" => Some(KeyCode::KeyQ), "R" => Some(KeyCode::KeyR),
        "S" => Some(KeyCode::KeyS), "T" => Some(KeyCode::KeyT),
        "U" => Some(KeyCode::KeyU), "V" => Some(KeyCode::KeyV),
        "W" => Some(KeyCode::KeyW), "X" => Some(KeyCode::KeyX),
        "Y" => Some(KeyCode::KeyY), "Z" => Some(KeyCode::KeyZ),
        "0" => Some(KeyCode::Digit0), "1" => Some(KeyCode::Digit1),
        "2" => Some(KeyCode::Digit2), "3" => Some(KeyCode::Digit3),
        "4" => Some(KeyCode::Digit4), "5" => Some(KeyCode::Digit5),
        "6" => Some(KeyCode::Digit6), "7" => Some(KeyCode::Digit7),
        "8" => Some(KeyCode::Digit8), "9" => Some(KeyCode::Digit9),
        "Space"    => Some(KeyCode::Space),
        "Enter"    => Some(KeyCode::Enter),
        "Backspace" => Some(KeyCode::Backspace),
        "Tab"      => Some(KeyCode::Tab),
        "Escape"   => Some(KeyCode::Escape),
        "Up"       => Some(KeyCode::ArrowUp),
        "Down"     => Some(KeyCode::ArrowDown),
        "Left"     => Some(KeyCode::ArrowLeft),
        "Right"    => Some(KeyCode::ArrowRight),
        "LShift"   => Some(KeyCode::ShiftLeft),
        "RShift"   => Some(KeyCode::ShiftRight),
        "LCtrl"    => Some(KeyCode::ControlLeft),
        "RCtrl"    => Some(KeyCode::ControlRight),
        "LAlt"     => Some(KeyCode::AltLeft),
        "RAlt"     => Some(KeyCode::AltRight),
        "F1"  => Some(KeyCode::F1),  "F2"  => Some(KeyCode::F2),
        "F3"  => Some(KeyCode::F3),  "F4"  => Some(KeyCode::F4),
        "F5"  => Some(KeyCode::F5),  "F6"  => Some(KeyCode::F6),
        "F7"  => Some(KeyCode::F7),  "F8"  => Some(KeyCode::F8),
        "F9"  => Some(KeyCode::F9),  "F10" => Some(KeyCode::F10),
        "F11" => Some(KeyCode::F11), "F12" => Some(KeyCode::F12),
        "-"   => Some(KeyCode::Minus),
        "="   => Some(KeyCode::Equal),
        "["   => Some(KeyCode::BracketLeft),
        "]"   => Some(KeyCode::BracketRight),
        ";"   => Some(KeyCode::Semicolon),
        "'"   => Some(KeyCode::Quote),
        ","   => Some(KeyCode::Comma),
        "."   => Some(KeyCode::Period),
        "/"   => Some(KeyCode::Slash),
        "\\"  => Some(KeyCode::Backslash),
        "`"   => Some(KeyCode::Backquote),
        _     => None,
    }
}

fn keybindings_from_json(src: &str) -> Option<KeyBindings> {
    let v: serde_json::Value = serde_json::from_str(src).ok()?;
    let obj = v.as_object()?;

    let mut b = KeyBindings::default();
    if let Some(s) = obj.get("forward").and_then(|x| x.as_str()) {
        if let Some(k) = str_to_keycode(s) { b.forward = k; }
    }
    if let Some(s) = obj.get("back").and_then(|x| x.as_str()) {
        if let Some(k) = str_to_keycode(s) { b.back = k; }
    }
    if let Some(s) = obj.get("left").and_then(|x| x.as_str()) {
        if let Some(k) = str_to_keycode(s) { b.left = k; }
    }
    if let Some(s) = obj.get("right").and_then(|x| x.as_str()) {
        if let Some(k) = str_to_keycode(s) { b.right = k; }
    }
    if let Some(s) = obj.get("brake").and_then(|x| x.as_str()) {
        if let Some(k) = str_to_keycode(s) { b.brake = k; }
    }
    if let Some(s) = obj.get("handbrake").and_then(|x| x.as_str()) {
        if let Some(k) = str_to_keycode(s) { b.handbrake = k; }
    }
    if let Some(s) = obj.get("boost").and_then(|x| x.as_str()) {
        if let Some(k) = str_to_keycode(s) { b.boost = k; }
    }
    Some(b)
}

fn write_keybindings_to_disk(bindings: &KeyBindings) {
    let path = keybindings_path();
    if let Some(parent) = path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            warn!("input_remap: could not create directory {}: {}", parent.display(), e);
            return;
        }
    }
    let json = keybindings_to_json(bindings);
    match fs::File::create(&path) {
        Err(e) => warn!("input_remap: could not open {} for writing: {}", path.display(), e),
        Ok(mut f) => {
            if let Err(e) = f.write_all(json.as_bytes()) {
                warn!("input_remap: write failed for {}: {}", path.display(), e);
            } else {
                info!("input_remap: keybindings saved to {}", path.display());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Startup: load keybindings from disk
// ---------------------------------------------------------------------------

fn load_keybindings(mut bindings: ResMut<KeyBindings>) {
    let path = keybindings_path();
    match fs::read_to_string(&path) {
        Err(e) => {
            info!(
                "input_remap: no saved keybindings at {} ({}); using defaults",
                path.display(),
                e
            );
        }
        Ok(text) => match keybindings_from_json(&text) {
            None => {
                info!(
                    "input_remap: could not parse {}; using defaults",
                    path.display()
                );
            }
            Some(loaded) => {
                *bindings = loaded;
                info!("input_remap: loaded keybindings from {}", path.display());
            }
        },
    }
}

// ---------------------------------------------------------------------------
// UI component markers
// ---------------------------------------------------------------------------

#[derive(Component)]
struct RemapPanelRoot;

/// One row in the binding list — stores which action index it represents.
#[derive(Component)]
struct BindingRow(usize);

/// The single Text node inside a BindingRow.
#[derive(Component)]
struct BindingRowText(usize);

/// Footer text node.
#[derive(Component)]
struct RemapFooterText;

// ---------------------------------------------------------------------------
// Colour constants
// ---------------------------------------------------------------------------

const PANEL_BG:       Color = Color::srgba(0.04, 0.04, 0.06, 0.92);
const COLOR_TITLE:    Color = Color::srgb(1.0, 0.9, 0.3);
const COLOR_NORMAL:   Color = Color::srgb(0.82, 0.82, 0.82);
const COLOR_SELECTED: Color = Color::srgb(1.0, 1.0, 0.0);
const COLOR_AWAITING: Color = Color::srgb(1.0, 0.25, 0.25);
const COLOR_FOOTER:   Color = Color::srgb(0.52, 0.52, 0.52);
const ROW_BG_NORMAL:  Color = Color::NONE;
const ROW_BG_SELECTED: Color = Color::srgba(0.12, 0.12, 0.0, 0.6);

// ---------------------------------------------------------------------------
// Startup: spawn the hidden remap panel
// ---------------------------------------------------------------------------

fn spawn_remap_panel(mut commands: Commands) {
    // Full-screen overlay root (hidden until `/` is pressed).
    let root = commands
        .spawn((
            RemapPanelRoot,
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Percent(100.0),
                position_type:   PositionType::Absolute,
                align_items:     AlignItems::Center,
                justify_content: JustifyContent::Center,
                display:         Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
        ))
        .id();

    // Centred modal panel: 460 × 320.
    let panel = commands
        .spawn((
            Node {
                width:           Val::Px(460.0),
                height:          Val::Px(320.0),
                flex_direction:  FlexDirection::Column,
                padding:         UiRect::all(Val::Px(20.0)),
                row_gap:         Val::Px(6.0),
                ..default()
            },
            BackgroundColor(PANEL_BG),
        ))
        .id();

    // Title row.
    let title = commands
        .spawn((
            Text::new("KEYBINDINGS"),
            TextFont { font_size: 26.0, ..default() },
            TextColor(COLOR_TITLE),
        ))
        .id();
    commands.entity(panel).add_child(title);

    // One row per action.
    for i in 0..ACTION_COUNT {
        let row = commands
            .spawn((
                BindingRow(i),
                Node {
                    width:        Val::Percent(100.0),
                    padding:      UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(ROW_BG_NORMAL),
            ))
            .id();

        let label = commands
            .spawn((
                BindingRowText(i),
                Text::new(format!("{:<12} [?]", action_label(i))),
                TextFont { font_size: 16.0, ..default() },
                TextColor(COLOR_NORMAL),
            ))
            .id();

        commands.entity(row).add_child(label);
        commands.entity(panel).add_child(row);
    }

    // Footer.
    let footer = commands
        .spawn((
            RemapFooterText,
            Text::new("\u{2191}\u{2193} select   ENTER rebind   / close"),
            TextFont { font_size: 13.0, ..default() },
            TextColor(COLOR_FOOTER),
        ))
        .id();
    commands.entity(panel).add_child(footer);

    commands.entity(root).add_child(panel);
}

// ---------------------------------------------------------------------------
// System: toggle_with_slash — open / close the panel with `/`
// ---------------------------------------------------------------------------

fn toggle_with_slash(
    keys:      Res<ButtonInput<KeyCode>>,
    mut state: ResMut<InputRemapState>,
) {
    if keys.just_pressed(KeyCode::Slash) {
        state.open = !state.open;
        // Cancel any pending rebind when closing.
        if !state.open {
            state.awaiting_bind = None;
        }
    }
}

// ---------------------------------------------------------------------------
// System: cycle_cursor — ArrowUp / ArrowDown moves the highlighted row.
//         Enter starts awaiting a new key for the selected row.
//         (Only active when the panel is open and not awaiting a bind.)
// ---------------------------------------------------------------------------

fn cycle_cursor(
    keys:      Res<ButtonInput<KeyCode>>,
    mut state: ResMut<InputRemapState>,
) {
    if !state.open || state.awaiting_bind.is_some() {
        return;
    }

    if keys.just_pressed(KeyCode::ArrowUp) {
        if state.cursor_idx > 0 {
            state.cursor_idx -= 1;
        } else {
            state.cursor_idx = ACTION_COUNT - 1;
        }
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        state.cursor_idx = (state.cursor_idx + 1) % ACTION_COUNT;
    }
    if keys.just_pressed(KeyCode::Enter) {
        state.awaiting_bind = Some(state.cursor_idx);
    }
}

// ---------------------------------------------------------------------------
// System: capture_new_key — while awaiting_bind, grab first non-modifier key.
// ---------------------------------------------------------------------------

fn capture_new_key(
    keys:         Res<ButtonInput<KeyCode>>,
    mut state:    ResMut<InputRemapState>,
    mut bindings: ResMut<KeyBindings>,
    mut debounce: ResMut<BindingSaveDebounce>,
) {
    let Some(slot) = state.awaiting_bind else { return };

    // Accept the first non-modifier key pressed.
    for key in keys.get_just_pressed() {
        // Slash would re-toggle the panel — skip it.
        if *key == KeyCode::Slash { continue; }
        if is_modifier_only(*key) { continue; }

        set_binding(&mut bindings, slot, *key);
        state.awaiting_bind = None;

        // Arm the save debounce.
        debounce.pending   = true;
        debounce.elapsed_s = 0.0;
        return;
    }
}

// ---------------------------------------------------------------------------
// System: apply_bindings_to_drive_input — overwrite DriveInput with remapped keys.
// Runs AFTER drive_input_keyboard (last-writer-wins strategy).
// Only overwrites if at least one binding key is pressed (so gamepad / script
// writes survive frames where no binding key fires).
// ---------------------------------------------------------------------------

pub fn apply_bindings_to_drive_input(
    keys:     Res<ButtonInput<KeyCode>>,
    bindings: Res<KeyBindings>,
    mut drive: ResMut<DriveInput>,
) {
    let fwd   = keys.pressed(bindings.forward);
    let back  = keys.pressed(bindings.back);
    let left  = keys.pressed(bindings.left);
    let right = keys.pressed(bindings.right);
    let brk   = keys.pressed(bindings.brake);
    // handbrake and boost are read but mapped to brake/boost fields.
    let hbrk  = keys.pressed(bindings.handbrake);

    // Only overwrite DriveInput when at least one remapped binding is active.
    if fwd || back || left || right || brk || hbrk {
        drive.drive = (if fwd { 1.0 } else { 0.0 }) - (if back { 1.0 } else { 0.0 });
        drive.steer = (if right { 1.0 } else { 0.0 }) - (if left { 1.0 } else { 0.0 });
        drive.brake = brk || hbrk;
    }
}

// ---------------------------------------------------------------------------
// System: update_panel_view — show/hide panel and refresh row text/colours.
// ---------------------------------------------------------------------------

fn update_panel_view(
    state:    Res<InputRemapState>,
    bindings: Res<KeyBindings>,
    mut roots:       Query<&mut Node, With<RemapPanelRoot>>,
    mut row_nodes:   Query<(&BindingRow, &mut BackgroundColor)>,
    mut row_texts:   Query<(&BindingRowText, &mut Text, &mut TextColor)>,
) {
    // Show or hide the root overlay.
    for mut node in &mut roots {
        node.display = if state.open { Display::Flex } else { Display::None };
    }

    if !state.open {
        return;
    }

    // Update row backgrounds.
    for (row, mut bg) in &mut row_nodes {
        bg.0 = if row.0 == state.cursor_idx {
            ROW_BG_SELECTED
        } else {
            ROW_BG_NORMAL
        };
    }

    // Update row text content and colour.
    for (row_text, mut text, mut color) in &mut row_texts {
        let idx = row_text.0;
        let is_selected  = idx == state.cursor_idx;
        let is_awaiting  = state.awaiting_bind == Some(idx);

        if is_awaiting {
            text.0   = format!("{:<12} [ ...press a key... ]", action_label(idx));
            color.0  = COLOR_AWAITING;
        } else {
            let key_str = keycode_label(get_binding(bindings.as_ref(), idx));
            text.0   = format!("{:<12} [{}]", action_label(idx), key_str);
            color.0  = if is_selected { COLOR_SELECTED } else { COLOR_NORMAL };
        }
    }
}

// ---------------------------------------------------------------------------
// System: save_keybindings_on_change — debounced disk write.
// ---------------------------------------------------------------------------

fn save_keybindings_on_change(
    bindings: Res<KeyBindings>,
    mut deb:  ResMut<BindingSaveDebounce>,
    time:     Res<Time>,
) {
    // Also arm when the resource itself is mutated externally.
    if bindings.is_changed() && !deb.pending {
        deb.pending   = true;
        deb.elapsed_s = 0.0;
        return;
    }

    if !deb.pending {
        return;
    }

    deb.elapsed_s += time.delta_secs();
    if deb.elapsed_s < 0.5 {
        return;
    }

    deb.pending   = false;
    deb.elapsed_s = 0.0;
    write_keybindings_to_disk(bindings.as_ref());
}
