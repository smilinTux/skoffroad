use std::time::Instant;
use bevy::prelude::*;

pub struct WeatherProfiler {
    spawn_timer: Option<Instant>,
    update_timer: Option<Instant>,
    render_timer: Option<Instant>,
}

impl WeatherProfiler {
    pub fn new() -> Self {
        Self {
            spawn_timer: None,
            update_timer: None,
            render_timer: None,
        }
    }

    pub fn start_spawn(&mut self) {
        self.spawn_timer = Some(Instant::now());
    }

    pub fn end_spawn(&mut self) -> f32 {
        self.spawn_timer.take()
            .map(|start| start.elapsed().as_secs_f32() * 1000.0)
            .unwrap_or(0.0)
    }

    pub fn start_update(&mut self) {
        self.update_timer = Some(Instant::now());
    }

    pub fn end_update(&mut self) -> f32 {
        self.update_timer.take()
            .map(|start| start.elapsed().as_secs_f32() * 1000.0)
            .unwrap_or(0.0)
    }

    pub fn start_render(&mut self) {
        self.render_timer = Some(Instant::now());
    }

    pub fn end_render(&mut self) -> f32 {
        self.render_timer.take()
            .map(|start| start.elapsed().as_secs_f32() * 1000.0)
            .unwrap_or(0.0)
    }
}

#[derive(Resource)]
pub struct WeatherTransitionTracker {
    current_transition: Option<(String, String, f32)>, // from, to, progress
    transition_duration: f32,
    elapsed_time: f32,
}

impl Default for WeatherTransitionTracker {
    fn default() -> Self {
        Self {
            current_transition: None,
            transition_duration: 2.0, // Default transition duration in seconds
            elapsed_time: 0.0,
        }
    }
}

impl WeatherTransitionTracker {
    pub fn start_transition(&mut self, from: String, to: String, duration: f32) {
        self.current_transition = Some((from, to, 0.0));
        self.transition_duration = duration;
        self.elapsed_time = 0.0;
    }

    pub fn update(&mut self, delta: f32) -> Option<f32> {
        if let Some((_, _, progress)) = &mut self.current_transition {
            self.elapsed_time += delta;
            *progress = (self.elapsed_time / self.transition_duration).min(1.0);
            
            if *progress >= 1.0 {
                self.current_transition = None;
                None
            } else {
                Some(*progress)
            }
        } else {
            None
        }
    }

    pub fn get_current_transition(&self) -> Option<(&str, &str, f32)> {
        self.current_transition.as_ref()
            .map(|(from, to, progress)| (from.as_str(), to.as_str(), *progress))
    }
}

pub fn update_weather_transitions(
    time: Res<Time>,
    mut transition_tracker: ResMut<WeatherTransitionTracker>,
) {
    if let Some(progress) = transition_tracker.update(time.delta_seconds()) {
        if let Some((from, to, _)) = transition_tracker.get_current_transition() {
            // No need to push to debug_name.transition_history
        }
    }
} 