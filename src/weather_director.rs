// Weather Director — state machine that drives the atmosphere over time.
//
// Sprint 84 — Dynamic Weather
//
// Cycles weather conditions over real time:
//   Clear -> Cloudy -> Overcast -> Rain -> Storm -> Clearing -> Clear
//
// The director OWNS the WeatherState resource and WRITES existing resources:
//   - StormState.active + StormState.flash_alpha  (rain_splash, wet_ground, distant_thunder react)
//   - FogDensity                                  (fog_volumetric puffs react)
//   - WindState.speed_mps                         (nudged up in storm, down in clear)
//
// WeatherCloudsPlugin does not expose a density resource so cloud visuals stay
// independent (clouds continue drifting by wind at fixed count).
//
// Manual cycle hotkey: Shift+N  (verified free in all src/*.rs files)
//
// Tier gating:
//   Low    — state machine runs, resource writes work; lightning flash suppressed.
//   Medium — lightning flash enabled.
//   High   — same as Medium (lightning overlay already owned by storm.rs).

use bevy::prelude::*;

use crate::fog_volumetric::FogDensity;
use crate::graphics_quality::GraphicsQuality;
use crate::storm::StormState;
use crate::wind::WindState;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct WeatherDirectorPlugin;

impl Plugin for WeatherDirectorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WeatherState::default())
           .add_systems(Update, (
               advance_weather_phase,
               apply_weather_to_systems,
               manual_cycle_hotkey,
               lightning_flash_system,
           ));
    }
}

// ---------------------------------------------------------------------------
// Weather condition phases
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherCondition {
    Clear,
    Cloudy,
    Overcast,
    Rain,
    Storm,
    Clearing,
}

impl WeatherCondition {
    /// Duration (seconds) each phase lasts before advancing.
    pub fn phase_duration(self) -> f32 {
        match self {
            WeatherCondition::Clear    => 180.0,  // 3 min
            WeatherCondition::Cloudy   => 120.0,  // 2 min
            WeatherCondition::Overcast =>  90.0,  // 1.5 min
            WeatherCondition::Rain     => 120.0,  // 2 min
            WeatherCondition::Storm    =>  90.0,  // 1.5 min
            WeatherCondition::Clearing =>  60.0,  // 1 min
        }
    }

    /// Next condition in the cycle.
    pub fn next(self) -> WeatherCondition {
        match self {
            WeatherCondition::Clear    => WeatherCondition::Cloudy,
            WeatherCondition::Cloudy   => WeatherCondition::Overcast,
            WeatherCondition::Overcast => WeatherCondition::Rain,
            WeatherCondition::Rain     => WeatherCondition::Storm,
            WeatherCondition::Storm    => WeatherCondition::Clearing,
            WeatherCondition::Clearing => WeatherCondition::Clear,
        }
    }
}

// ---------------------------------------------------------------------------
// WeatherState resource
// ---------------------------------------------------------------------------

/// Public resource that exposes the current weather condition and intensity
/// to any system that wants to react to the weather.
#[derive(Resource)]
pub struct WeatherState {
    /// Current phase of the weather cycle.
    pub condition: WeatherCondition,
    /// Intensity in [0.0, 1.0] — smoothly ramps at transitions.
    pub intensity: f32,
    /// How many real seconds have elapsed in the current phase.
    pub phase_elapsed: f32,
}

impl Default for WeatherState {
    fn default() -> Self {
        Self {
            condition:     WeatherCondition::Clear,
            intensity:     0.0,
            phase_elapsed: 0.0,
        }
    }
}

impl WeatherState {
    /// Target intensity for a given condition.
    pub fn target_intensity(condition: WeatherCondition) -> f32 {
        match condition {
            WeatherCondition::Clear    => 0.0,
            WeatherCondition::Cloudy   => 0.25,
            WeatherCondition::Overcast => 0.5,
            WeatherCondition::Rain     => 0.75,
            WeatherCondition::Storm    => 1.0,
            WeatherCondition::Clearing => 0.15,
        }
    }
}

// ---------------------------------------------------------------------------
// System: advance phase timer
// ---------------------------------------------------------------------------

fn advance_weather_phase(
    time:         Res<Time>,
    mut weather:  ResMut<WeatherState>,
) {
    let dt = time.delta_secs();
    weather.phase_elapsed += dt;

    let duration = weather.condition.phase_duration();
    if weather.phase_elapsed >= duration {
        let old = weather.condition;
        weather.condition     = old.next();
        weather.phase_elapsed = 0.0;
        info!(
            "[WeatherDirector] phase: {:?} -> {:?}",
            old, weather.condition
        );
    }

    // Lerp intensity toward the target for this condition.
    let target = WeatherState::target_intensity(weather.condition);
    let delta  = dt / 30.0; // 30-second ramp time
    if weather.intensity < target {
        weather.intensity = (weather.intensity + delta).min(target);
    } else if weather.intensity > target {
        weather.intensity = (weather.intensity - delta).max(target);
    }
}

// ---------------------------------------------------------------------------
// System: write existing-system resources
// ---------------------------------------------------------------------------

fn apply_weather_to_systems(
    weather:      Res<WeatherState>,
    mut storm:    ResMut<StormState>,
    mut fog:      ResMut<FogDensity>,
    mut wind:     ResMut<WindState>,
) {
    // --- StormState ---
    // Storm and Rain conditions activate the storm (rain splash, wet ground,
    // distant thunder all gate on StormState.active).
    let storm_active = matches!(
        weather.condition,
        WeatherCondition::Rain | WeatherCondition::Storm
    );
    storm.active = storm_active;

    // --- FogDensity ---
    // Map intensity to fog density.  Clear → 0.1 base, Storm → 0.7.
    let fog_target = match weather.condition {
        WeatherCondition::Clear    => 0.10,
        WeatherCondition::Cloudy   => 0.20,
        WeatherCondition::Overcast => 0.40,
        WeatherCondition::Rain     => 0.55,
        WeatherCondition::Storm    => 0.70,
        WeatherCondition::Clearing => 0.25,
    };
    // Gentle lerp so puffs don't pop — manage_fog_puffs handles count changes.
    fog.0 = fog.0 + (fog_target - fog.0) * 0.02;
    fog.0 = fog.0.clamp(0.0, 1.0);

    // --- WindState ---
    // Nudge wind speed: Storm → 8+ m/s, Clear → calm ~2 m/s.
    // The native wind system continuously overwrites direction and speed each
    // frame via update_wind_state; we just nudge the speed multiplier here by
    // adding a storm-surge on top.  We keep the direction as-is.
    let wind_boost = match weather.condition {
        WeatherCondition::Clear    =>  0.0,
        WeatherCondition::Cloudy   =>  0.5,
        WeatherCondition::Overcast =>  1.5,
        WeatherCondition::Rain     =>  3.0,
        WeatherCondition::Storm    =>  5.0,
        WeatherCondition::Clearing =>  1.0,
    };
    // wind.rs updates speed to 2.5 + 4.0 * sin.abs() (range 2.5-6.5 m/s).
    // We add boost on top; clamp to a sane max.
    wind.speed_mps = (wind.speed_mps + wind_boost).min(12.0);
}

// ---------------------------------------------------------------------------
// System: Shift+N manual cycle hotkey
// ---------------------------------------------------------------------------

fn manual_cycle_hotkey(
    keys:         Res<ButtonInput<KeyCode>>,
    mut weather:  ResMut<WeatherState>,
) {
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if shift && keys.just_pressed(KeyCode::KeyN) {
        let old = weather.condition;
        weather.condition     = old.next();
        weather.phase_elapsed = 0.0;
        info!(
            "[WeatherDirector] manual cycle: {:?} -> {:?}",
            old, weather.condition
        );
    }
}

// ---------------------------------------------------------------------------
// System: lightning flash during Storm (Medium+ only)
// ---------------------------------------------------------------------------

/// Drives StormState.flash_alpha with an occasional extra burst when the
/// weather director is in Storm condition.  storm.rs tick_lightning already
/// fires flashes while StormState.active is true; this system adds an extra
/// burst every few seconds for dramatic effect (separate timer).
fn lightning_flash_system(
    time:         Res<Time>,
    weather:      Res<WeatherState>,
    quality:      Res<GraphicsQuality>,
    mut storm:    ResMut<StormState>,
    mut timer:    Local<f32>,
    mut seeded:   Local<bool>,
    mut lseed:    Local<u32>,
) {
    // Low tier: skip extra flashes to keep it cheap.
    if matches!(*quality, GraphicsQuality::Low) {
        return;
    }

    // Only fire during Storm condition.
    if !matches!(weather.condition, WeatherCondition::Storm) {
        *timer = 5.0;
        return;
    }

    if !*seeded {
        *lseed  = 0xCAFE_D00D;
        *seeded = true;
        *timer  = next_lightning_interval(&mut lseed);
    }

    *timer -= time.delta_secs();
    if *timer <= 0.0 {
        // Trigger a flash — storm.rs update_flash_overlay will decay it.
        storm.flash_alpha = 0.85;
        *timer = next_lightning_interval(&mut lseed);
        info!("[WeatherDirector] extra lightning flash");
    }
}

// ---------------------------------------------------------------------------
// LCG helpers
// ---------------------------------------------------------------------------

#[inline]
fn lcg_next(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *seed as f32 / u32::MAX as f32
}

/// Random interval in [4, 9] seconds between extra lightning flashes.
#[inline]
fn next_lightning_interval(seed: &mut u32) -> f32 {
    4.0 + lcg_next(seed) * 5.0
}
