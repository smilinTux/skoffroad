Okay, let's rewrite this plan for a Rust-based implementation, focusing on performance, leveraging Rust's strengths, and providing a detailed, step-by-step guide suitable for an LLM coder.

**Critical First Step: Choosing the Right Foundation**

The original plan used Three.js (rendering) and Cannon.es (physics). In Rust, we have excellent alternatives. The most prominent choices for a game like this are:

1.  **Bevy Engine + Rapier Physics:**
    *   **Pros:** Modern, data-oriented (ECS - Entity Component System), rapidly developing, strong community focus, highly ergonomic, excellent performance potential, Rapier is a mature and fast physics engine developed in Rust. Favors composition over inheritance. Great for performance and potentially "beautiful" code structure due to ECS.
    *   **Cons:** Still evolving (APIs can change), ecosystem plugins might be less mature than older engines in other languages.
2.  **Fyrox Engine (formerly rg3d) + Rapier Physics:**
    *   **Pros:** More mature than Bevy in some aspects, includes a scene editor (can significantly speed up level design/prototyping), feature-rich (UI, sound, scripting built-in), also uses Rapier for physics.
    *   **Cons:** Can have a slightly steeper initial learning curve for some, architecture is more traditional scene-graph oriented (though it uses ECS internally).

**Recommendation:**

For the goal of the "most beautiful and performant solution" *from a code structure and raw potential perspective*, **Bevy + Rapier** is arguably the stronger contender due to its pure ECS focus, which often leads to highly decoupled, cache-friendly, and parallelizable code. Its ergonomics are frequently praised. Fyrox is a very capable alternative, especially if a visual editor is a high priority *now*.

**❓ Action Required: Please Choose Your Engine**

*   **Option A:** Proceed with **Bevy + Rapier** (Recommended for performance focus and modern Rust practices).
*   **Option B:** Proceed with **Fyrox + Rapier** (Consider if a built-in editor is more important).

*(I will proceed assuming you choose **Option A: Bevy + Rapier** as it aligns slightly better with the "performant and beautiful" code goal. If you prefer Fyrox, the overall steps will be similar, but specific API calls and architectural patterns will differ.)*

---

**Rewritten Plan: Offroad Jeep Simulation (Rust - Bevy Engine)**

**Goal:** Develop a high-performance, visually appealing, physics-based offroad simulation game using Rust, the Bevy game engine, and the Rapier physics engine. The game features a Jeep Wrangler TJ navigating challenging Appalachian-inspired terrain.

**Core Technology Stack:**

*   **Language:** Rust (stable toolchain)
*   **Game Engine:** Bevy (`bevy`)
*   **Physics Engine:** Rapier (`bevy_rapier3d`)
*   **Potential Crates:** `noise-rs` (terrain generation), `serde` (save/load), `bevy_egui` (UI), `kira` or `bevy_kira_audio` (advanced audio), `bevy_asset_loader` (asset management), `bevy_hanabi` (particles), `wgpu` (underlying graphics API via Bevy).

**High-Level Architecture (Bevy - ECS):**

*   **Entities:** Represent game objects (Jeep, terrain chunks, trees, rocks, UI elements, etc.). These are just unique IDs.
*   **Components:** Represent data associated with entities (e.g., `Transform`, `Velocity`, `Mesh`, `Material`, `PlayerControlled`, `Wheel`, `Suspension`, `Health`, `TerrainChunkData`). Define the properties and state of entities.
*   **Systems:** Represent logic that operates on entities with specific sets of components (e.g., `fn apply_engine_force`, `fn update_suspension`, `fn render_meshes`, `fn process_player_input`, `fn update_physics_world`). This is where game logic lives.
*   **Resources:** Global data or services accessible by systems (e.g., `PhysicsWorld`, `Time`, `AssetServer`, `Input<KeyCode>`, `GameSettings`).
*   **Plugins:** Groups of related components, systems, and resources for modularity (e.g., `VehiclePlugin`, `TerrainPlugin`, `AudioPlugin`, `UIPlugin`).

**Detailed Implementation Steps (Step-by-Step for LLM):**

**Phase 1: Project Setup & Basic Window**

1.  **Setup Rust:** Ensure Rust stable toolchain is installed (`rustup`).
2.  **Create Project:** `cargo new offroad_jeep_game && cd offroad_jeep_game`
3.  **Add Dependencies:** Edit `Cargo.toml`:
    ```toml
    [dependencies]
    bevy = { version = "0.13", features = ["dynamic_linking"] } # Check for latest bevy version
    bevy_rapier3d = { version = "0.25", features = ["simd-stable"] } # Check for latest bevy_rapier3d version
    # Add other dependencies as needed later (noise, serde, etc.)

    # Enable optimizations for release builds
    [profile.release]
    lto = "thin" # Link Time Optimization
    codegen-units = 1 # Maximize optimizations
    opt-level = 3 # Maximum optimization level
    ```
    *Explanation for LLM:* We add `bevy` for the core engine and `bevy_rapier3d` for physics integration. Release profile settings boost performance. `dynamic_linking` speeds up *debug* compile times.
4.  **Create Basic Bevy App:** In `src/main.rs`:
    ```rust
    use bevy::prelude::*;

    fn main() {
        App::new()
            .add_plugins(DefaultPlugins) // Basic Bevy setup (input, window, rendering, etc.)
            .add_systems(Startup, setup)
            .add_systems(Update, bevy::window::close_on_esc) // Close window with ESC
            .run();
    }

    fn setup(mut commands: Commands) {
        // Setup initial scene (camera, light) - will be expanded later
        commands.spawn(Camera3dBundle {
            transform: Transform::from_xyz(-10.0, 10.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        });
        commands.spawn(DirectionalLightBundle {
            directional_light: DirectionalLight {
                shadows_enabled: true,
                ..default()
            },
            transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, -0.5, 0.0)),
            ..default()
        });
        println!("Offroad Jeep Game Setup Complete!");
    }
    ```
    *Explanation for LLM:* This creates a Bevy application, adds default plugins, adds a `setup` system to run once at startup, and a system to close the window on Escape. The `setup` system spawns a basic 3D camera and a directional light.

**Phase 2: Physics Integration & Basic Scene**

1.  **Add Rapier Plugin:** In `src/main.rs`, modify the `App::new()` chain:
    ```rust
    use bevy::prelude::*;
    use bevy_rapier3d::prelude::*;

    fn main() {
        App::new()
            .add_plugins(DefaultPlugins)
            .add_plugins(RapierPhysicsPlugin::<NoUserData>::default()) // Add Rapier physics
            // Optional: Add for debugging colliders
            // .add_plugins(RapierDebugRenderPlugin::default())
            .add_systems(Startup, setup_physics) // Renamed setup function
            .add_systems(Update, bevy::window::close_on_esc)
            .run();
    }
    ```
2.  **Setup Physics Scene:** Modify the `setup` function (now `setup_physics`):
    ```rust
    fn setup_physics(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        // Camera & Light (as before)
        commands.spawn(Camera3dBundle {
            transform: Transform::from_xyz(-10.0, 10.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        });
        commands.spawn(DirectionalLightBundle { /* ... as before ... */ });

        // --- Physics Ground Plane ---
        commands.spawn((
            Collider::cuboid(100.0, 0.1, 100.0), // Rapier collider shape
            TransformBundle::from(Transform::from_xyz(0.0, -0.1, 0.0)), // Position
            PbrBundle { // Bevy mesh for rendering
                mesh: meshes.add(Cuboid::new(200.0, 0.2, 200.0)),
                material: materials.add(Color::rgb(0.3, 0.5, 0.3)),
                ..default()
            },
            RigidBody::Fixed, // Make it static
        ));

        // --- Physics Dynamic Cube ---
        commands.spawn((
            RigidBody::Dynamic,
            Collider::cuboid(0.5, 0.5, 0.5),
            Restitution::coefficient(0.7), // Bounciness
            TransformBundle::from(Transform::from_xyz(0.0, 4.0, 0.0)),
            PbrBundle {
                mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
                material: materials.add(Color::rgb(0.8, 0.7, 0.6)),
                ..default()
            },
        ));
         println!("Physics Scene Setup Complete!");
    }
    ```
    *Explanation for LLM:* We add the `RapierPhysicsPlugin`. The `setup_physics` function now creates a static ground plane and a dynamic cube. Note how entities have both Bevy rendering components (`PbrBundle`, `Mesh`, `Material`, `Transform`) and Rapier physics components (`RigidBody`, `Collider`, `Restitution`). `TransformBundle` conveniently groups `Transform` and `GlobalTransform`.

**Phase 3: Basic Vehicle - Model & Physics Shell**

1.  **Asset Loading:**
    *   Create an `assets` folder in the project root. Place your Jeep Wrangler TJ model file (e.g., `jeep.glb` or `jeep.gltf`) inside.
    *   Add `bevy_asset_loader` (optional but recommended for cleaner loading states): Add to `Cargo.toml`.
    *   Implement asset loading logic (either using `AssetServer` directly in `setup` or via `bevy_asset_loader`).
2.  **Vehicle Entity:** In a new file `src/vehicle.rs` (and declare `mod vehicle;` in `main.rs`):
    ```rust
    use bevy::prelude::*;
    use bevy_rapier3d::prelude::*;

    #[derive(Component)]
    pub struct Vehicle; // Marker component for the main vehicle entity

    #[derive(Component)]
    pub struct Wheel; // Marker component for wheel entities

    // Plugin to organize vehicle setup
    pub struct VehiclePlugin;

    impl Plugin for VehiclePlugin {
        fn build(&self, app: &mut App) {
            app.add_systems(Startup, spawn_vehicle);
            // Add vehicle control systems later in Update schedule
        }
    }

    fn spawn_vehicle(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        // Add Rapier context if needed for joints later
    ) {
        // --- Spawn Vehicle Chassis ---
        let vehicle_entity = commands.spawn((
            Vehicle,
            RigidBody::Dynamic,
            // Approximate chassis collider (adjust dimensions)
            Collider::cuboid(1.0, 0.5, 2.0),
            // More realistic mass/inertia needed later
            ColliderMassProperties::Density(1.0), // Placeholder density
            TransformBundle::from(Transform::from_xyz(0.0, 2.0, 0.0)),
            // Load the GLTF model as a child of the physics body
            SceneBundle {
                 scene: asset_server.load("jeep.glb#Scene0"), // Ensure path is correct
                 transform: Transform::from_xyz(0.0, -0.5, 0.0), // Offset mesh relative to collider center
                ..default()
            },
            // Add components for controls, suspension later
            ExternalForce::default(), // To apply engine force
            Velocity::default(), // To read/write velocity directly if needed
        )).id(); // Get the Entity ID

        println!("Vehicle Spawned (Entity: {:?})", vehicle_entity);

        // --- Spawn Wheels (Simplified Example - No Suspension Yet) ---
        // For a real vehicle, wheels need to be separate entities connected by joints
        // and might have their own colliders (spheres or cylinders).
        // This is a complex part - Rapier has MultibodyJoint examples, or custom constraints are needed.
        // Placeholder: Just visually attach for now if model includes wheels.
        // Actual physics wheels will be added in a later step.

        // TODO: Spawn separate wheel entities with colliders
        // TODO: Add Joints (e.g., RevoluteJoint or custom suspension) between chassis and wheels
    }
    ```
3.  **Add Vehicle Plugin:** In `main.rs`, add `.add_plugins(vehicle::VehiclePlugin)` to the app builder.
    *Explanation for LLM:* We create a `VehiclePlugin` for organization. The `spawn_vehicle` system loads the Jeep model using `AssetServer` and spawns a parent entity (`vehicle_entity`) with the main physics components (`RigidBody`, `Collider`). The visual model (`SceneBundle`) is added as a child. We mark the entity with `Vehicle`. Real wheels with physics and joints are complex and deferred to a later step. `ExternalForce` allows applying forces for movement.

**Phase 4: Basic Terrain Generation**

1.  **Add Noise Library:** Add `noise-rs` to `Cargo.toml`: `noise = "0.8"`
2.  **Terrain Plugin:** Create `src/terrain.rs` (and `mod terrain;` in `main.rs`).
    ```rust
    use bevy::prelude::*;
    use bevy_rapier3d::prelude::*;
    use noise::{NoiseFn, Perlin, Seedable}; // Example using Perlin noise

    #[derive(Component)]
    pub struct TerrainChunk;

    // Plugin for terrain systems
    pub struct TerrainPlugin;

    impl Plugin for TerrainPlugin {
        fn build(&self, app: &mut App) {
            app.insert_resource(TerrainConfig { // Store terrain settings globally
                seed: 123,
                chunk_size: 64, // Size in vertices
                amplitude: 5.0,
                frequency: 0.05,
            })
            .add_systems(Startup, setup_terrain);
        }
    }

    #[derive(Resource)]
    struct TerrainConfig {
        seed: u32,
        chunk_size: usize,
        amplitude: f32,
        frequency: f64,
    }

    fn setup_terrain(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        config: Res<TerrainConfig>,
    ) {
        let perlin = Perlin::new(config.seed);
        let chunk_size = config.chunk_size;
        let vertex_scale = 1.0; // How far apart vertices are in world space

        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut normals = Vec::new(); // Required for lighting
        let mut uvs = Vec::new(); // Required for texturing

        // Generate vertices and height data
        for j in 0..=chunk_size {
            for i in 0..=chunk_size {
                let x = i as f32 * vertex_scale;
                let z = j as f32 * vertex_scale;

                // Calculate noise coordinates (adjust frequency/sampling as needed)
                let noise_x = i as f64 * config.frequency;
                let noise_z = j as f64 * config.frequency;
                let y = perlin.get([noise_x, noise_z]) as f32 * config.amplitude;

                vertices.push([x, y, z]);
                // Simple normal calculation (pointing up) - improve later
                normals.push([0.0, 1.0, 0.0]);
                // Basic UV mapping
                uvs.push([i as f32 / chunk_size as f32, j as f32 / chunk_size as f32]);
            }
        }

        // Generate indices for triangles
        for j in 0..chunk_size {
            for i in 0..chunk_size {
                let row1 = j * (chunk_size + 1);
                let row2 = (j + 1) * (chunk_size + 1);

                // Triangle 1
                indices.push((row1 + i) as u32);
                indices.push((row1 + i + 1) as u32);
                indices.push((row2 + i + 1) as u32);

                // Triangle 2
                indices.push((row1 + i) as u32);
                indices.push((row2 + i + 1) as u32);
                indices.push((row2 + i) as u32);
            }
        }

        // Create Bevy Mesh
        let mut mesh = Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::TriangleList,
            Default::default(), // Use default AssetLifecycleMode
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices.clone()); // Clone vertices for physics
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.set_indices(Some(bevy::render::mesh::Indices::U32(indices.clone()))); // Clone indices for physics

        // --- Spawn Terrain Entity ---
        commands.spawn((
            TerrainChunk,
            PbrBundle {
                mesh: meshes.add(mesh),
                material: materials.add(StandardMaterial { // Basic green material
                    base_color: Color::rgb(0.3, 0.6, 0.3),
                    perceptual_roughness: 0.8, // Make it less shiny
                    ..default()
                }),
                transform: Transform::from_xyz(
                    -(chunk_size as f32 * vertex_scale / 2.0), // Center the mesh
                    0.0,
                    -(chunk_size as f32 * vertex_scale / 2.0),
                ),
                ..default()
            },
            // --- Add Rapier Collider for the Terrain ---
            Collider::trimesh(vertices, indices), // Use vertex/index data for physics shape
            RigidBody::Fixed, // Terrain doesn't move
        ));

        println!("Terrain Generation Complete.");
    }

    // TODO: Improve normal calculation
    // TODO: Add terrain texturing/materials based on height/slope
    // TODO: Implement chunking and Level of Detail (LOD) for large terrains
    ```
3.  **Add Terrain Plugin:** In `main.rs`, add `.add_plugins(terrain::TerrainPlugin)` and remove the old ground plane from `setup_physics`. Adjust the vehicle's starting height if necessary.
    *Explanation for LLM:* We create `TerrainPlugin` and a `TerrainConfig` resource. The `setup_terrain` system uses the `noise-rs` crate to generate height data using Perlin noise. It then constructs vertex and index buffers for a triangle mesh. This mesh data is used to create both a Bevy `Mesh` for rendering (`PbrBundle`) and a Rapier `Collider::trimesh` for physics. This creates a single, static terrain chunk. Chunking/LOD is noted as a TODO for larger worlds.

**Phase 5: Vehicle Controls & Physics**

1.  **Input Handling System:** In `vehicle.rs`:
    ```rust
    use bevy::input::keyboard::KeyboardInput;
    use bevy::input::ButtonState;
    use bevy::prelude::*;
    use bevy_rapier3d::prelude::*;

    // Define vehicle parameters (move to a Resource or Component later)
    const MAX_STEERING_ANGLE: f32 = 0.6; // Radians
    const ENGINE_FORCE: f32 = 1500.0;
    const BRAKE_FORCE: f32 = 3000.0; // Higher than engine force for quick stops
    const WHEEL_FRICTION: f32 = 1.5; // Sideways friction factor

    #[derive(Resource, Default)]
    struct VehicleInputState {
        throttle: f32,
        steering: f32,
        braking: bool,
    }

    // Add to VehiclePlugin in build function:
    // .init_resource::<VehicleInputState>()
    // .add_systems(Update, (
    //     read_vehicle_input,
    //     apply_vehicle_controls.after(read_vehicle_input), // Ensure input is read first
    // ))

    fn read_vehicle_input(
        keyboard_input: Res<ButtonInput<KeyCode>>,
        mut input_state: ResMut<VehicleInputState>,
    ) {
        input_state.throttle = 0.0;
        if keyboard_input.pressed(KeyCode::KeyW) || keyboard_input.pressed(KeyCode::ArrowUp) {
            input_state.throttle = 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyS) || keyboard_input.pressed(KeyCode::ArrowDown) {
             input_state.throttle = -0.5; // Reverse slower than forward
        }

        input_state.steering = 0.0;
        if keyboard_input.pressed(KeyCode::KeyA) || keyboard_input.pressed(KeyCode::ArrowLeft) {
            input_state.steering = 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyD) || keyboard_input.pressed(KeyCode::ArrowRight) {
            input_state.steering = -1.0;
        }

        input_state.braking = keyboard_input.pressed(KeyCode::Space);
    }


    // NOTE: This is a VERY simplified arcade physics model.
    // Real vehicle physics requires simulating individual wheels, suspension,
    // friction curves (Pacejka), differentials, etc. It's complex!
    // Consider using Rapier's vehicle controller if available/suitable,
    // or implementing a custom constraint-based system.
    fn apply_vehicle_controls(
        mut query: Query<(
            &mut ExternalForce,
            &mut Velocity,
            &Transform,
            With<Vehicle>, // Filter for entities with the Vehicle component
        )>,
        input_state: Res<VehicleInputState>,
        time: Res<Time>,
    ) {
        for (mut external_force, mut velocity, transform, _) in query.iter_mut() {
            let forward_vector = transform.forward();
            let right_vector = transform.right(); // Needed for friction/steering effect

            // --- Apply Steering Rotation (Arcade Style) ---
            // This directly rotates the chassis. A real sim rotates front wheels via joints.
            let steering_rotation = Quat::from_axis_angle(
                Vec3::Y, // Steering axis
                input_state.steering * MAX_STEERING_ANGLE * time.delta_seconds() * velocity.linvel.length() * 0.1 // Rudimentary speed-sensitive steering
            );
             // We need Query<&mut Transform> for this. Let's skip direct rotation for now
             // and focus on forces. Direct transform manipulation fights the physics engine.
             // Real steering force comes from angled wheel friction.


            // --- Calculate Forces ---
            let mut engine_force_vector = Vec3::ZERO;
            let mut brake_force_vector = Vec3::ZERO;
            let mut friction_force_vector = Vec3::ZERO;

            if !input_state.braking {
                // Apply engine force (forward/backward)
                engine_force_vector = forward_vector * input_state.throttle * ENGINE_FORCE;
            } else {
                // Apply braking force (opposite to current velocity)
                 // Stronger braking if moving faster
                 let brake_magnitude = (BRAKE_FORCE * velocity.linvel.length() * 0.1).min(BRAKE_FORCE);
                 if velocity.linvel.length_squared() > 0.1 { // Only apply if moving
                     brake_force_vector = -velocity.linvel.normalize() * brake_magnitude;
                 } else {
                     // Clamp velocity to zero if slow enough while braking
                     velocity.linvel = Vec3::ZERO;
                 }
            }

            // --- Apply Sideways Friction (Arcade Style) ---
            // Reduce velocity component perpendicular to the car's forward direction
            let sideways_velocity = velocity.linvel.dot(right_vector) * right_vector;
            friction_force_vector = -sideways_velocity * WHEEL_FRICTION; // Force opposing sideways slide

            // --- Apply Forces to Physics Body ---
            // ExternalForce applies a continuous force over the timestep
            external_force.force = engine_force_vector + friction_force_vector + brake_force_vector;

             // Apply steering torque (rudimentary) - better to do via wheel forces
             let steering_torque_magnitude = input_state.steering * velocity.linvel.dot(forward_vector) * 50.0; // Torque proportional to speed and steering input
             external_force.torque = Vec3::Y * steering_torque_magnitude;


        }
    }
    ```
2.  **Integrate into Plugin:** Add the resource initialization and systems to `VehiclePlugin`'s `build` method as commented in the code above. Ensure the systems are added to the `Update` schedule.
    *Explanation for LLM:* We create a `VehicleInputState` resource to store player input. The `read_vehicle_input` system reads keyboard presses and updates this state. `apply_vehicle_controls` queries for the `Vehicle` entity, reads the input state, and applies forces using `ExternalForce`. This is a *highly simplified, arcade-style* model. It applies force directly forward/backward and a friction force to resist sideways sliding. Real vehicle physics is much more involved, requiring simulation of individual wheels connected by joints/constraints, suspension forces, and tire friction models (like Pacejka). Applying direct torque for steering is also a simplification.

**Phase 6: Camera System**

1.  **Camera Plugin:** Create `src/camera.rs` (and `mod camera;` in `main.rs`).
2.  **Camera Controller:**
    ```rust
    use bevy::prelude::*;
    use std::f32::consts::PI;

    pub struct CameraPlugin;

    #[derive(Component)]
    pub struct PlayerCamera {
        pub distance: f32,
        pub focus: Vec3,
        pub yaw: f32,   // Rotation around Y
        pub pitch: f32, // Rotation around X (local)
    }

    impl Default for PlayerCamera {
        fn default() -> Self {
            PlayerCamera {
                distance: 15.0,
                focus: Vec3::ZERO,
                yaw: 0.0,
                pitch: PI / 4.0, // Start looking down slightly
            }
        }
    }

    impl Plugin for CameraPlugin {
        fn build(&self, app: &mut App) {
            app.add_systems(Startup, setup_camera)
               .add_systems(Update, follow_vehicle_camera); // Use LateUpdate if jitter occurs
        }
    }

    // Find the vehicle's transform (replace with a marker component query if needed)
    #[derive(Component)]
    pub struct VehicleToFollow; // Add this component to the vehicle entity in vehicle.rs

    fn setup_camera(mut commands: Commands) {
        commands.spawn((
            Camera3dBundle::default(), // Transform will be updated by the system
            PlayerCamera::default(),
        ));
         // Remove the static camera from main.rs or setup_physics
    }


    fn follow_vehicle_camera(
        mut camera_query: Query<(&mut Transform, &mut PlayerCamera), With<Camera>>,
        vehicle_query: Query<&Transform, (With<crate::vehicle::Vehicle>, Without<Camera>)>, // Ensure vehicle transform is read-only here
        // Add input for camera rotation/zoom later (MouseMotion, MouseWheel)
        time: Res<Time>,
    ) {
        if let Ok(vehicle_transform) = vehicle_query.get_single() {
             if let Ok((mut camera_transform, mut player_camera)) = camera_query.get_single_mut() {

                // --- Smoothly update focus point ---
                let target_focus = vehicle_transform.translation + Vec3::Y * 1.0; // Focus slightly above vehicle center
                player_camera.focus = player_camera.focus.lerp(target_focus, time.delta_seconds() * 5.0); // Adjust lerp factor for smoothness

                // TODO: Add input handling here to modify player_camera.yaw and player_camera.pitch
                // player_camera.pitch = player_camera.pitch.clamp(0.1, PI - 0.1); // Prevent gimbal lock/flipping

                // --- Calculate camera position based on yaw, pitch, distance ---
                let rotation = Quat::from_rotation_y(player_camera.yaw) * Quat::from_rotation_x(-player_camera.pitch);
                let position = player_camera.focus + rotation * (Vec3::Z * player_camera.distance); // Z is forward in Bevy's default coord system

                // --- Update camera transform ---
                camera_transform.translation = position;
                camera_transform.look_at(player_camera.focus, Vec3::Y);
            }
        }
    }
    ```
3.  **Integrate Plugin:** Add `.add_plugins(camera::CameraPlugin)` in `main.rs`. Remove the old static camera spawning. Add the `VehicleToFollow` component to the vehicle entity in `vehicle.rs`.
    *Explanation for LLM:* Creates a `CameraPlugin`. The `PlayerCamera` component holds state like distance, focus point, and angles. The `setup_camera` system spawns the camera entity. The `follow_vehicle_camera` system finds the vehicle's transform, smoothly updates the camera's focus point (where it looks), calculates the desired position based on yaw/pitch/distance, and updates the camera's `Transform`. Input handling for rotating/zooming the camera is marked as TODO. The `VehicleToFollow` marker component helps uniquely identify the target.

**Phase 7: UI System (Basic HUD)**

1.  **Add UI Crate:** Add `bevy_egui` to `Cargo.toml`: `bevy_egui = "0.27"` (check latest).
2.  **UI Plugin:** Create `src/ui.rs` (and `mod ui;` in `main.rs`).
    ```rust
    use bevy::prelude::*;
    use bevy_egui::{egui, EguiContexts, EguiPlugin};
    use crate::vehicle::{Vehicle, VehicleInputState}; // Access vehicle state if needed
    use bevy_rapier3d::prelude::Velocity; // To display speed

    pub struct GameUiPlugin;

    impl Plugin for GameUiPlugin {
        fn build(&self, app: &mut App) {
            app.add_plugins(EguiPlugin) // Add egui integration
               .add_systems(Update, hud_system);
        }
    }

    fn hud_system(
        mut contexts: EguiContexts,
        vehicle_query: Query<&Velocity, With<Vehicle>>,
        // Optional: Access input state or other resources
        // input_state: Res<VehicleInputState>,
    ) {
        let ctx = contexts.ctx_mut();

        egui::Window::new("HUD")
            .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(10.0, -10.0)) // Position window
            .resizable(false)
            .title_bar(false) // No window title bar
            .frame(egui::Frame::none().fill(egui::Color32::from_rgba_premultiplied(0, 0, 0, 120))) // Semi-transparent background
            .show(ctx, |ui| {
                ui.label(format!("Offroad Jeep Sim (Bevy/Rust)"));

                if let Ok(velocity) = vehicle_query.get_single() {
                    let speed_kmh = velocity.linvel.length() * 3.6; // m/s to km/h
                     ui.separator();
                    ui.label(format!("Speed: {:.1} km/h", speed_kmh));
                } else {
                    ui.label("Speed: N/A");
                }

                // Add more UI elements: throttle/brake state, gear, damage, etc.
                // ui.label(format!("Throttle: {:.2}", input_state.throttle));
                // ui.label(format!("Braking: {}", input_state.braking));
            });
    }
    ```
3.  **Integrate Plugin:** Add `.add_plugins(ui::GameUiPlugin)` in `main.rs`.
    *Explanation for LLM:* We add the `bevy_egui` plugin. The `GameUiPlugin` sets up egui. The `hud_system` uses `EguiContexts` to get an egui context (`ctx`). It then uses egui's immediate mode API (`egui::Window`, `ui.label`, etc.) to draw UI elements. In this case, it creates a simple HUD window in the bottom-left showing the vehicle's speed (calculated from its `Velocity` component).

**Subsequent Phases (Outline - High-Level Steps):**

*   **Phase 8: Audio System:**
    *   Choose audio crate (`bevy_kira_audio` or `bevy_audio`).
    *   Create `AudioPlugin`.
    *   Load sound assets (`.ogg`, `.wav`).
    *   Create components/events for triggering sounds (e.g., `PlaySoundEvent`).
    *   Systems to play engine sounds (pitch based on throttle/RPM), collision sounds, ambient sounds.
    *   Implement 3D spatial audio linked to entity positions.
*   **Phase 9: Particle System:**
    *   Use `bevy_hanabi` or a custom solution.
    *   Create `ParticlePlugin`.
    *   Define particle effects (e.g., exhaust smoke, dust trails, mud splashes).
    *   Spawn particle emitters attached to the vehicle (wheels, exhaust pipe).
    *   Trigger effects based on events (e.g., collision, wheel spin on dirt).
*   **Phase 10: Weather System:**
    *   Create `WeatherPlugin` and `WeatherState` resource (e.g., `Sunny`, `Raining`, `Snowing`).
    *   Systems to:
        *   Change lighting/skybox based on state.
        *   Spawn rain/snow particles.
        *   Apply physics effects (e.g., reduced friction when raining).
        *   Play weather-related sounds.
*   **Phase 11: Damage & Deformation System:**
    *   **Simple:** `Health` component, reduce on high-impact collision events from Rapier (`CollisionEvents`). Apply visual changes (e.g., swap material) or gameplay effects (reduced engine power).
    *   **Advanced:** Requires mesh deformation. Complex. Could involve:
        *   Storing original vertex data.
        *   Calculating deformation based on collision impulse/location.
        *   Updating the `Mesh` vertex attributes directly (performance intensive) or using vertex shaders. Might require specific plugins or custom rendering logic.
*   **Phase 12: Advanced Terrain Features:**
    *   **Chunking/LOD:** Break terrain into chunks. Load/unload chunks based on camera distance. Generate lower-resolution meshes/colliders for distant chunks. This is essential for large worlds. Requires managing chunk entities and potentially async generation.
    *   **Texturing:** Use shaders (WGSL in Bevy) to blend textures based on height, slope. Tri-planar mapping can hide UV seams.
    *   **Deformation:** Modify terrain collider and mesh vertices based on vehicle interaction (e.g., tire tracks). Very complex, might involve specialized data structures or compute shaders.
*   **Phase 13: Advanced Vehicle Physics:**
    *   **Replace Arcade Model:** Implement proper wheel simulation.
        *   Spawn separate entities for each wheel with `Collider` (sphere or cylinder).
        *   Use Rapier `MultibodyJoint` (if suitable) or implement custom constraints/springs for suspension between chassis and wheels.
        *   Calculate tire forces (longitudinal and lateral) based on slip ratio/angle, load, and friction model (e.g., simplified Pacejka).
        *   Apply forces from wheels to the chassis.
        *   Simulate drivetrain (engine torque -> gearbox -> differential -> wheels).
*   **Phase 14: Vegetation System:**
    *   Use GPU instancing for performance (`RenderCommand` / custom shader or plugins like `bevy_foliage`).
    *   Scatter vegetation entities based on terrain properties (slope, height, noise).
    *   Add basic physics interaction (e.g., simple colliders for large trees, maybe visual-only bending for grass).
*   **Phase 15: Level, Achievement, Save/Load:**
    *   **Level System:** Define level data (terrain parameters, vehicle start pos, objectives). Load data at startup.
    *   **Save/Load:** Use `serde` to serialize/deserialize game state (vehicle position, player progress, settings) into a file (e.g., RON, JSON, binary). Integrate with Bevy's state management.
    *   **Achievements:** Define achievement conditions (e.g., reach location, perform stunt). Create systems to check conditions and store completion status (e.g., in save file).
*   **Phase 16: Planned Features:**
    *   **Multiplayer:** Very complex. Requires choosing a networking approach (e.g., P2P vs client-server), a networking library (`bevy_replicon`, `bevy_spicy_networking`, `laminar`, custom), handling state synchronization, latency compensation, authoritative server logic (especially for physics). Start simple (e.g., synchronizing transforms).
    *   **Additional Vehicles:** Define different vehicle stats, models, colliders. Refactor vehicle code to handle multiple types.
    *   **More Weather:** Add fog, different rain intensities, wind affecting particles/vegetation.

**Performance & Beauty Focus Points:**

*   **ECS:** Leverage Bevy's ECS for data locality and parallelism. Keep components small and focused.
*   **Profiling:** Use tools like `tracing` and `perf` (on Linux) or platform-specific profilers to identify bottlenecks.
*   **Systems:** Write focused systems. Use Bevy's scheduling (`.before`, `.after`, `.in_set`) to control execution order.
*   **Rendering:** Optimize meshes, use texture atlases, implement LODs, use instancing heavily (vegetation, rocks). Write efficient WGSL shaders.
*   **Physics:** Use appropriate collider shapes (primitives are faster than meshes). Optimize physics queries. Tune Rapier settings (`RapierConfiguration` resource).
*   **Code Structure:** Use Bevy Plugins extensively for modularity. Keep `main.rs` minimal. Follow Rust best practices (Clippy, rustfmt).
*   **Async:** Use Bevy's async tasks (`AsyncComputeTaskPool`) for heavy computations like chunk generation off the main thread.

This detailed plan provides a step-by-step guide tailored for Rust, Bevy, and Rapier, addressing the requested features and focusing on a performant and well-structured implementation suitable for an LLM to follow. Remember that game development is iterative; many steps, especially advanced physics and terrain, will require significant refinement and potentially exploring specialized crates or custom solutions.
