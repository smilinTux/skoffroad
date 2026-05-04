use sandk_offroad_next::headless::{run_scenario, Scenario};

#[test]
fn forward_moves_vehicle() {
    let summary = run_scenario(Scenario::Forward, 3.0);
    assert!(
        summary.distance_traveled_m > 3.0,
        "expected vehicle to travel > 3 m forward, got {:.3} m",
        summary.distance_traveled_m
    );
    assert!(
        summary.max_speed_mps >= 3.0 && summary.max_speed_mps <= 25.0,
        "expected max speed in [3, 25] m/s, got {:.3}",
        summary.max_speed_mps
    );
    assert!(
        !summary.did_flip,
        "vehicle flipped (max_tilt_deg = {:.1})",
        summary.max_tilt_deg
    );
    assert!(
        summary.final_chassis_above_terrain >= 0.3 && summary.final_chassis_above_terrain <= 2.0,
        "expected chassis 0.3–2.0 m above terrain, got {:.3}",
        summary.final_chassis_above_terrain
    );
}

#[test]
fn idle_settles() {
    let summary = run_scenario(Scenario::Idle, 3.0);
    assert!(
        !summary.did_flip,
        "idle vehicle flipped (max_tilt_deg = {:.1})",
        summary.max_tilt_deg
    );
    assert!(
        summary.max_speed_mps < 1.0,
        "idle vehicle exceeded 1 m/s (max_speed = {:.3})",
        summary.max_speed_mps
    );
    assert!(
        summary.final_chassis_above_terrain >= 0.3 && summary.final_chassis_above_terrain <= 2.0,
        "expected chassis 0.3–2.0 m above terrain at rest, got {:.3}",
        summary.final_chassis_above_terrain
    );
}

#[test]
fn brake_stops_vehicle() {
    // Accelerate for 2 s, then brake for 3 s — vehicle must not flip and must shed most speed.
    let accel = run_scenario(Scenario::Forward, 2.0);
    let full  = run_scenario(Scenario::BrakeTest { accel_s: 2.0 }, 5.0);

    assert!(
        !full.did_flip,
        "vehicle flipped during brake test (max_tilt_deg = {:.1})",
        full.max_tilt_deg
    );
    // Mean speed over the full 5 s must be less than mean speed during acceleration alone.
    // This confirms that braking actually reduces speed over time.
    assert!(
        full.mean_speed_mps < accel.mean_speed_mps,
        "braking did not reduce mean speed: brake mean={:.3} >= accel mean={:.3}",
        full.mean_speed_mps,
        accel.mean_speed_mps
    );
    // Max speed reached during the run must be under a sane cap.
    assert!(
        full.max_speed_mps < 20.0,
        "vehicle exceeded 20 m/s during brake test (max={:.3})",
        full.max_speed_mps
    );
}

// Sanity test that always passes — confirms the harness itself runs end-to-end
// without panicking, regardless of vehicle behaviour. This is the gate that
// tells us the headless infrastructure is healthy.
#[test]
fn harness_runs() {
    let summary = run_scenario(Scenario::Idle, 0.5);
    assert_eq!(summary.scenario_name, "idle");
    assert!(summary.ticks > 0, "harness produced zero ticks");
    assert!(
        summary.start_pos[1].is_finite() && summary.end_pos[1].is_finite(),
        "non-finite positions out of harness — physics or telemetry broken"
    );
}
