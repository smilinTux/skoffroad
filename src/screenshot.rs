// Screenshot capture — press F12 to save a PNG of the current frame.
//
// Save location:
//   Linux  : $XDG_DATA_HOME/skoffroad/screenshots/sandk_<unix_ts>.png
//          — fallback: $HOME/.local/share/skoffroad/screenshots/sandk_<unix_ts>.png
//   Windows: $APPDATA/skoffroad/screenshots/sandk_<unix_ts>.png
//   macOS  : $HOME/Library/Application Support/skoffroad/screenshots/sandk_<unix_ts>.png
//   Else   : ./screenshots/sandk_<unix_ts>.png
//
// Uses Bevy 0.18's Screenshot component + save_to_disk observer.
// A 2-second top-center HUD popup confirms the save path.

use std::{path::PathBuf, time::{SystemTime, UNIX_EPOCH}};

use bevy::{prelude::*, render::view::screenshot::{save_to_disk, Screenshot}};

// ---- Plugin -----------------------------------------------------------------

pub struct ScreenshotPlugin;

impl Plugin for ScreenshotPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScreenshotFlash>()
           .add_systems(Startup, spawn_screenshot_hud)
           .add_systems(Update, (capture_screenshot, update_screenshot_hud));
    }
}

// ---- Resources & components -------------------------------------------------

#[derive(Resource, Default)]
pub struct ScreenshotFlash {
    pub timer_s: f32,
    pub last_path: String,
}

#[derive(Component)]
struct ScreenshotHud;

#[derive(Component)]
struct ScreenshotHudText;

// ---- Path resolution --------------------------------------------------------

fn resolve_screenshot_path() -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let filename = format!("sandk_{}.png", ts);

    let base: Option<PathBuf> = {
        // 1. $XDG_DATA_HOME (Linux, explicit)
        std::env::var("XDG_DATA_HOME").ok().map(PathBuf::from)
        // 2. $HOME/.local/share (Linux, implicit)
        .or_else(|| {
            if cfg!(target_os = "linux") {
                std::env::var("HOME").ok()
                    .map(|h| PathBuf::from(h).join(".local").join("share"))
            } else {
                None
            }
        })
        // 3. $APPDATA (Windows)
        .or_else(|| {
            if cfg!(target_os = "windows") {
                std::env::var("APPDATA").ok().map(PathBuf::from)
            } else {
                None
            }
        })
        // 4. $HOME/Library/Application Support (macOS)
        .or_else(|| {
            if cfg!(target_os = "macos") {
                std::env::var("HOME").ok()
                    .map(|h| PathBuf::from(h)
                        .join("Library")
                        .join("Application Support"))
            } else {
                None
            }
        })
    };

    match base {
        Some(dir) => dir.join("skoffroad").join("screenshots").join(&filename),
        None => {
            warn!("screenshot: could not determine data directory; using ./screenshots/{}", filename);
            PathBuf::from("screenshots").join(filename)
        }
    }
}

// ---- Startup: spawn HUD popup -----------------------------------------------

fn spawn_screenshot_hud(mut commands: Commands) {
    commands.spawn((
        ScreenshotHud,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Px(12.0),
            padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.07, 0.80)),
        Visibility::Hidden,
    )).with_children(|parent| {
        parent.spawn((
            ScreenshotHudText,
            Text::new("SCREENSHOT SAVED"),
            TextFont { font_size: 15.0, ..default() },
            TextColor(Color::srgb(0.30, 0.95, 0.40)),
        ));
    });
}

// ---- Systems ----------------------------------------------------------------

/// Listen for F12; spawn a Screenshot entity that saves to disk and shows the HUD popup.
fn capture_screenshot(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut flash: ResMut<ScreenshotFlash>,
    mut hud_text_q: Query<&mut Text, With<ScreenshotHudText>>,
) {
    if !keys.just_pressed(KeyCode::F12) {
        return;
    }

    let path = resolve_screenshot_path();

    // Ensure the screenshots directory exists.
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            warn!("screenshot: could not create directory {:?}: {}", parent, e);
        }
    }

    let path_str = path.to_string_lossy().to_string();
    info!("screenshot: saving to {}", path_str);

    // Spawn the Screenshot component; the observer fires when the GPU readback completes.
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path.clone()));

    // Arm the HUD flash.
    flash.timer_s = 2.0;
    flash.last_path = path_str.clone();

    for mut text in &mut hud_text_q {
        text.0 = format!("SCREENSHOT SAVED\n{}", path_str);
    }
}

/// Tick the flash timer and show/hide the HUD node.
fn update_screenshot_hud(
    time: Res<Time>,
    mut flash: ResMut<ScreenshotFlash>,
    mut hud_q: Query<&mut Visibility, With<ScreenshotHud>>,
) {
    flash.timer_s = (flash.timer_s - time.delta_secs()).max(0.0);
    let target = if flash.timer_s > 0.0 { Visibility::Inherited } else { Visibility::Hidden };
    for mut vis in &mut hud_q {
        *vis = target;
    }
}
