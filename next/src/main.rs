use bevy::prelude::*;
use avian3d::prelude::*;
use sandk_offroad_next::{CameraPlugin, DustPlugin, TerrainPlugin, VehiclePlugin};

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
        .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.98)))
        .add_plugins((
            TerrainPlugin,
            VehiclePlugin,
            CameraPlugin,
            DustPlugin,
        ))
        .add_systems(Startup, setup_lighting);

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

fn setup_lighting(mut commands: Commands, mut ambient: ResMut<GlobalAmbientLight>) {
    // Directional sun light.
    commands.spawn((
        DirectionalLight {
            illuminance: 50_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.4, 0.0)),
    ));

    // Global ambient (Resource in Bevy 0.18; AmbientLight is now a per-camera Component).
    ambient.color      = Color::WHITE;
    ambient.brightness = 400.0;
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
