use bevy::ecs::schedule::{LogLevel, ScheduleBuildSettings};
use bevy::prelude::*;
use avian3d::prelude::*;
use sandk_offroad_next::{
    AchievementToastPlugin, AirtimePlugin, ArrowPlugin, AudioPlugin, BannersPlugin,
    BoostPlugin, BreadcrumbsPlugin, CameraPlugin, CollectiblesPlugin, CompassPlugin,
    ConfettiPlugin, CoursePlugin, DamagePlugin, DronePlugin, DustPlugin, EngineProPlugin,
    EventLogPlugin, ExhaustPlugin, FuelPlugin, GaugePlugin, HeadlightsPlugin, HelpPlugin,
    HornPlugin, HudPlugin, LiveryPlugin, MarkersPlugin, MenuPlugin, MinimapPlugin,
    MixerPlugin, MudPlugin, MusicPlugin, ObstaclesPlugin, PerfPlugin, PhotoModePlugin,
    PinsPlugin, RampsPlugin, RecoveryPlugin, RepairPlugin, ReplayPlugin, RoutePlugin,
    SavePlugin, ScatterPlugin, ScreenshotPlugin, SettingsPlugin, ShakePlugin,
    SkidmarksPlugin, SkyPlugin, SpeedLinesPlugin, SpeedTrapPlugin, StarsPlugin,
    StatsScreenPlugin, SurfacesPlugin, TerrainPlugin, TrailPlugin, TrampolinesPlugin,
    TrialPlugin, TutorialPlugin, VariantsPlugin, VehiclePlugin, WaterPlugin,
    WheelieCounterPlugin, WindPlugin, WorldAudioPlugin, XpPlugin,
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
        ))
        .add_plugins((
            XpPlugin,
            SpeedLinesPlugin,
            StarsPlugin,
            ShakePlugin,
            ConfettiPlugin,
        ))
        .add_plugins((
            VariantsPlugin,
            BannersPlugin,
            MarkersPlugin,
            ObstaclesPlugin,
            RoutePlugin,
        ))
        .add_plugins((
            CollectiblesPlugin,
            BoostPlugin,
            ScreenshotPlugin,
            DronePlugin,
            ExhaustPlugin,
        ))
        .add_plugins((
            CoursePlugin,
            ArrowPlugin,
            TrailPlugin,
            PinsPlugin,
            TutorialPlugin,
        ))
        .add_plugins((
            MusicPlugin,
            EngineProPlugin,
            SurfacesPlugin,
            WorldAudioPlugin,
            MixerPlugin,
        ));

    // Multiple plugins (vehicle suspension, water buoyancy, mud drag,
    // trampoline bounce, wind) all add commutative external forces to the
    // chassis in PhysicsSchedule. They access the same Avian rigid-body
    // components, so Bevy's default strict ambiguity detection panics with
    // 10 conflict pairs. Their final force sum is order-independent (each
    // calls forces.apply_force which accumulates), so downgrade ambiguity
    // detection to a warning for that schedule only.
    app.edit_schedule(PhysicsSchedule, |schedule| {
        schedule.set_build_settings(ScheduleBuildSettings {
            ambiguity_detection: LogLevel::Warn,
            ..default()
        });
    });

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
