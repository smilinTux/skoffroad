use bevy::prelude::*;
use avian3d::prelude::*;
use sandk_offroad_next::{
    AchievementToastPlugin, AirtimePlugin, AudioPlugin, BreadcrumbsPlugin, CameraPlugin,
    CompassPlugin, DamagePlugin, DustPlugin, EventLogPlugin, FuelPlugin, GaugePlugin,
    HeadlightsPlugin, HelpPlugin, HornPlugin, HudPlugin, LiveryPlugin, MenuPlugin,
    MinimapPlugin, MudPlugin, PerfPlugin, PhotoModePlugin, RampsPlugin, RecoveryPlugin,
    RepairPlugin, ReplayPlugin, SavePlugin, ScatterPlugin, SettingsPlugin, SkidmarksPlugin,
    SkyPlugin, SpeedTrapPlugin, StatsScreenPlugin, TerrainPlugin, TrampolinesPlugin,
    TrialPlugin, VehiclePlugin, WaterPlugin, WheelieCounterPlugin, WindPlugin,
};

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "SandK Offroad - Next".into(),
                // WindowResolution requires u32 or UVec2 in Bevy 0.18.
                resolution: (1280u32, 720u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PhysicsPlugins::default())
        // SkyPlugin owns the sky dome + sun + ambient + fog;
        // ClearColor and the old setup_lighting are no longer needed.
        .add_plugins((
            TerrainPlugin,
            VehiclePlugin,
            CameraPlugin,
            DustPlugin,
            SkyPlugin,
            HudPlugin,
            AudioPlugin,
            ScatterPlugin,
            MinimapPlugin,
            EventLogPlugin,
            SavePlugin,
        ))
        .add_plugins((
            WaterPlugin,
            SettingsPlugin,
            DamagePlugin,
            MenuPlugin,
            HelpPlugin,
            CompassPlugin,
            PerfPlugin,
            StatsScreenPlugin,
        ))
        .add_plugins((
            TrialPlugin,
            MudPlugin,
            RepairPlugin,
            LiveryPlugin,
            RecoveryPlugin,
        ))
        .add_plugins((
            PhotoModePlugin,
            ReplayPlugin,
            AchievementToastPlugin,
            WindPlugin,
            HornPlugin,
        ))
        .add_plugins((
            HeadlightsPlugin,
            BreadcrumbsPlugin,
            FuelPlugin,
            RampsPlugin,
            SkidmarksPlugin,
        ))
        .add_plugins((
            GaugePlugin,
            TrampolinesPlugin,
            SpeedTrapPlugin,
            WheelieCounterPlugin,
            AirtimePlugin,
        ));

    // F3 world inspector — only compiled when `--features dev` is passed.
    // Inspector defaults to hidden; press F3 to toggle.
    #[cfg(feature = "dev")]
    {
        use bevy_inspector_egui::bevy_egui::EguiPlugin;
        use bevy_inspector_egui::quick::WorldInspectorPlugin;

        app.add_plugins(EguiPlugin::default())
           .insert_resource(InspectorVisible(false))
           .add_plugins(
               WorldInspectorPlugin::new()
                   .run_if(|vis: Res<InspectorVisible>| vis.0),
           )
           .add_systems(Update, toggle_inspector);
    }

    app.run();
}

// ---- Dev-only inspector toggle ----------------------------------------------

#[cfg(feature = "dev")]
#[derive(Resource)]
struct InspectorVisible(bool);

#[cfg(feature = "dev")]
fn toggle_inspector(
    keys: Res<ButtonInput<KeyCode>>,
    mut vis: ResMut<InspectorVisible>,
) {
    if keys.just_pressed(KeyCode::F3) {
        vis.0 = !vis.0;
    }
}
