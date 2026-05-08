use bevy::prelude::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sandk_offroad::game::{
    plugins::weather::{WeatherState, WeatherManager},
    state::GameState,
};

fn bench_weather_system(c: &mut Criterion) {
    let mut group = c.benchmark_group("Weather System");
    
    // Setup test app with weather system
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<WeatherState>()
        .init_resource::<WeatherManager>();
    
    group.bench_function("weather_update", |b| {
        b.iter(|| {
            let mut weather_state = black_box(WeatherState::default());
            weather_state.update(0.016); // Simulate 16ms frame time
        });
    });

    group.bench_function("weather_transition", |b| {
        b.iter(|| {
            let mut weather_manager = black_box(WeatherManager::default());
            weather_manager.update(0.016); // Simulate 16ms frame time
        });
    });

    group.finish();
}

fn bench_game_state(c: &mut Criterion) {
    let mut group = c.benchmark_group("Game State");
    
    group.bench_function("game_state_update", |b| {
        b.iter(|| {
            let mut game_state = black_box(GameState::default());
            game_state.update(0.016); // Simulate 16ms frame time
        });
    });

    group.finish();
}

criterion_group!(benches, bench_weather_system, bench_game_state);
criterion_main!(benches); 