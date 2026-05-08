use bevy::ecs::schedule::{LogLevel, ScheduleBuildSettings};
use bevy::prelude::*;
use avian3d::prelude::*;
use skoffroad::{
    AccessibilityPlugin, AchievementToastPlugin, AiDriverPlugin, AiPathPlugin,
    AirtimePlugin, ArrowPlugin, AsciiLogoPlugin, AssetAttributionPlugin,
    AssetBrowserPlugin, AssetManifestPlugin, AssistsPlugin, AudioPlugin,
    BannersPlugin, BenchmarkPlugin, BillboardsPlugin, BiomeCanyonPlugin,
    BiomeDesertPlugin, BirdsFlockPlugin, BloomPpPlugin, BoatsPlugin, BoostPlugin,
    BreadcrumbsPlugin, BuildingsPlugin,
    CameraModesPlugin, CameraPlugin, CampfiresPlugin, CareerPlugin, ChallengesPlugin,
    ChangelogPlugin, ChassisUnderglowPlugin, ChromePolishPlugin, ClimbAssistPlugin,
    CollectiblesPlugin,
    ComboPlugin, CompassHudPlugin, CompassPlugin,
    ConfettiPlugin, ConfigPlugin, CoursePlugin, CrashAudioPlugin, CreditsPlugin,
    DailyPlugin, DamagePlugin, DamageVisualPlugin, DecalsPlugin, DemoModePlugin,
    DemolitionPlugin, DiffLockPlugin, DistantThunderPlugin, DriftMeterPlugin, DriveModePlugin,
    DronePlugin, GroundRutsPlugin,
    DustPlugin, EngineBayPlugin, EngineProPlugin, EngineTorquePlugin, EventLogPlugin,
    ExhaustPlugin, ExhaustSmokePlugin,
    ExplorePlugin, FastTravelMenuPlugin, FencePostsPlugin, FireworksPlugin, FishPlugin,
    FogHornPlugin, FontAssetsPlugin, FuelPlugin, GaragePlugin, GasStationsPlugin,
    GaugePlugin, GlbLoaderPlugin, GodraysPlugin, GrassTuftsPlugin, GraphicsQualityPlugin, HeadlightsPlugin,
    HeatHazePlugin, HeightmapLoaderPlugin,
    HelpPlugin, HillclimbPlugin, HillclimbTrackPlugin, HornPlugin, HudPlugin,
    ImpactFlashPlugin, InputRemapPlugin, Interior3dPlugin,
    IntroVideoPlugin, JumpMeterPlugin, LandmarksPlugin, LicensePlatePlugin, LiveryPlugin,
    LoadingScreenPlugin, LowFuelWarningPlugin, LowRangePlugin, MapSelectPlugin, MapsPlugin,
    MarkersPlugin,
    MedalsPlugin, MenuPlugin, MinimapPlugin, MinimapZoomPlugin, MixerPlugin,
    MountainRangePlugin, MudPlugin, MudPuddlesPlugin, MusicPlugin, NightGlowPlugin,
    NitroGaugePlugin, NotificationsPlugin,
    ObstaclesPlugin, PaintShopPlugin, PerfPlugin, PhotoModePlugin, PinsPlugin,
    ProgressionPlugin, PursuitPlugin, RacePlugin, RadarPickupsPlugin, RampArrowsPlugin,
    RampsPlugin, RecoveryPlugin, RockGardenPlugin, RoofRackPlugin,
    RepairPlugin, ReplayPlugin, RivalHudPlugin, RivalPlugin, RoutePlugin, SavePlugin,
    ScatterPlugin, ScreenshotPlugin, SeasonPlugin, SessionSummaryPlugin,
    SettingsPlugin, ShakePlugin, SkidmarksPlugin, SkyPlugin, SnowPlugin,
    SpawnPointsPlugin, SpeedLinesPlugin, SpeedTrapPlugin, SplashParticlesPlugin,
    StarsPlugin, StatsScreenPlugin, StormPlugin, StuntScorePlugin, SurfacesPlugin,
    SuspensionArmsPlugin, TerrainPlugin, ThemePlugin, TimeTrialPlugin, TirePressurePlugin,
    TireSmokePlugin, TireSquashPlugin, TrafficPlugin, TrailPlugin, TrailersPlugin,
    TrampolinesPlugin,
    TransitionPlugin, TreeVariantsPlugin, TrialPlugin,
    TruckBedCargoPlugin,
    TutorialPlugin, UfoPlugin, UnlocksPlugin, VariantsPlugin, VehicleDetailPlugin,
    VehicleDirtPlugin, VehiclePlugin, WaterPlugin, WeatherCloudsPlugin, WheelDetailPlugin,
    WheelRimsPlugin, WheelWellPlugin,
    WheelieCounterPlugin, WildlifePlugin,
    WindPlugin, WinchPlugin, WorldAudioPlugin, XpPlugin,
    MudDepthPlugin, TerrainLodPlugin, TerrainNormalMapPlugin, TerrainSplatmapPlugin,
    WaterReflectivePlugin,
    EngineAudioLayeredPlugin, FuelConsumptionRealPlugin, TransferCasePlugin,
    TransmissionPlugin, WinchCablePhysicsPlugin,
    HdrSkyboxPlugin, PhotoHudPlugin, PhotorealRocksPlugin, TerrainDecalsPlugin,
    TerrainGrassBladesPlugin,
};

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "skoffroad".into(),
                // WindowResolution requires u32 or UVec2 in Bevy 0.18.
                resolution: (1280u32, 720u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PhysicsPlugins::default())
        // GraphicsQualityPlugin must register first so other plugins' Startup
        // systems can read the GraphicsQuality resource.
        .add_plugins(GraphicsQualityPlugin)
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
        ))
        .add_plugins((
            AiPathPlugin,
            AiDriverPlugin,
            RivalPlugin,
            RacePlugin,
            RivalHudPlugin,
        ))
        .add_plugins((
            ProgressionPlugin,
            UnlocksPlugin,
            CareerPlugin,
            DailyPlugin,
            MedalsPlugin,
        ))
        .add_plugins((
            MapsPlugin,
            BiomeDesertPlugin,
            BiomeCanyonPlugin,
            MapSelectPlugin,
            TransitionPlugin,
        ))
        .add_plugins((
            ConfigPlugin,
            FontAssetsPlugin,
            ThemePlugin,
            LoadingScreenPlugin,
            CreditsPlugin,
        ))
        .add_plugins((
            InputRemapPlugin,
            AccessibilityPlugin,
            BenchmarkPlugin,
            DemoModePlugin,
            ChangelogPlugin,
        ))
        .add_plugins((
            StormPlugin,
            VehicleDirtPlugin,
            DecalsPlugin,
            BloomPpPlugin,
            GodraysPlugin,
        ))
        .add_plugins((
            TimeTrialPlugin,
            PursuitPlugin,
            DemolitionPlugin,
            ExplorePlugin,
            ChallengesPlugin,
        ))
        .add_plugins((
            AsciiLogoPlugin,
            IntroVideoPlugin,
            TrafficPlugin,
            BuildingsPlugin,
            BirdsFlockPlugin,
        ))
        .add_plugins((
            LandmarksPlugin,
            ExhaustSmokePlugin,
            MinimapZoomPlugin,
            SeasonPlugin,
            WeatherCloudsPlugin,
        ))
        .add_plugins((
            AssistsPlugin,
            ClimbAssistPlugin,
            GaragePlugin,
            FireworksPlugin,
            SpawnPointsPlugin,
        ))
        .add_plugins((
            WildlifePlugin,
            MountainRangePlugin,
            JumpMeterPlugin,
            ComboPlugin,
            CampfiresPlugin,
        ))
        .add_plugins((
            UfoPlugin,
            SnowPlugin,
            CompassHudPlugin,
            DamageVisualPlugin,
            FastTravelMenuPlugin,
        ))
        .add_plugins((
            GasStationsPlugin,
            BillboardsPlugin,
            TireSmokePlugin,
            ImpactFlashPlugin,
            LowFuelWarningPlugin,
        ))
        .add_plugins((
            BoatsPlugin,
            FishPlugin,
            CrashAudioPlugin,
            FogHornPlugin,
            DistantThunderPlugin,
        ))
        .add_plugins((
            PaintShopPlugin,
            LicensePlatePlugin,
            SessionSummaryPlugin,
            NotificationsPlugin,
            RadarPickupsPlugin,
        ))
        .add_plugins((
            DriftMeterPlugin,
            NitroGaugePlugin,
            TrailersPlugin,
            StuntScorePlugin,
            RampArrowsPlugin,
        ))
        .add_plugins((
            CameraModesPlugin,
            SuspensionArmsPlugin,
            WheelDetailPlugin,
            VehicleDetailPlugin,
            ChassisUnderglowPlugin,
        ))
        .add_plugins((
            EngineBayPlugin,
            Interior3dPlugin,
            EngineTorquePlugin,
            DriveModePlugin,
            DiffLockPlugin,
        ))
        .add_plugins((
            TreeVariantsPlugin,
            GrassTuftsPlugin,
            FencePostsPlugin,
            WheelWellPlugin,
            RoofRackPlugin,
        ))
        .add_plugins((
            RockGardenPlugin,
            TireSquashPlugin,
            MudPuddlesPlugin,
            SplashParticlesPlugin,
            GroundRutsPlugin,
        ))
        .add_plugins((
            HillclimbTrackPlugin,
            HillclimbPlugin,
            LowRangePlugin,
            TirePressurePlugin,
            WinchPlugin,
        ))
        .add_plugins((
            TruckBedCargoPlugin,
            WheelRimsPlugin,
            HeatHazePlugin,
            NightGlowPlugin,
            ChromePolishPlugin,
        ))
        .add_plugins((
            GlbLoaderPlugin,
            HeightmapLoaderPlugin,
            AssetManifestPlugin,
            AssetBrowserPlugin,
            AssetAttributionPlugin,
        ))
        .add_plugins((
            TerrainSplatmapPlugin,
            TerrainNormalMapPlugin,
            TerrainLodPlugin,
            MudDepthPlugin,
            WaterReflectivePlugin,
        ))
        .add_plugins((
            TransmissionPlugin,
            TransferCasePlugin,
            WinchCablePhysicsPlugin,
            EngineAudioLayeredPlugin,
            FuelConsumptionRealPlugin,
        ))
        .add_plugins((
            HdrSkyboxPlugin,
            PhotorealRocksPlugin,
            TerrainGrassBladesPlugin,
            TerrainDecalsPlugin,
            PhotoHudPlugin,
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
