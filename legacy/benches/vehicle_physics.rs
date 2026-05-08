use criterion::{black_box, criterion_group, criterion_main, Criterion};
use bevy::prelude::*;
use sandk_offroad::game::{
    vehicle::{Wheel, WheelBundle, update_wheel_physics},
    physics::PhysicsPlugin,
};

fn wheel_physics_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("vehicle_physics");
    group.sample_size(100);
    group.measurement_time(std::time::Duration::from_secs(10));

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(PhysicsPlugin);

    // Create test wheel
    let wheel = Wheel::default();
    let wheel_bundle = WheelBundle::default();
    let mut world = app.world;
    let wheel_entity = world.spawn(wheel_bundle).id();

    group.bench_function("update_wheel_physics", |b| {
        b.iter(|| {
            update_wheel_physics(
                black_box(&mut world),
                black_box(wheel_entity),
                black_box(0.016), // ~60 FPS
            );
        });
    });

    group.finish();
}

criterion_group!(benches, wheel_physics_benchmark);
criterion_main!(benches); 