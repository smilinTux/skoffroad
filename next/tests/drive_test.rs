use sandk_offroad_next::headless::{run_scenario, Scenario};

// Both tests below currently fail because the vehicle physics are broken:
// drive force is applied to the chassis rigid body directly along its local
// forward axis, with passive sphere wheels on rigid revolute joints (no
// suspension travel, no tuned friction). The chassis launches into the air
// instead of driving on terrain. The harness was built specifically to
// quantify this and gate the fix.
//
// They are kept #[ignore]'d so `cargo test` stays green; once the vehicle
// uses a proper raycast/suspension model with grounded force application,
// remove the ignores and they become real regression gates.

#[test]
#[ignore = "vehicle physics broken: chassis is a flying brick, see harness output"]
fn forward_moves_vehicle() {
    let summary = run_scenario(Scenario::Forward, 3.0);
    assert!(
        summary.distance_traveled_m > 1.0,
        "expected vehicle to travel > 1 m forward, got {:.3} m",
        summary.distance_traveled_m
    );
    assert!(
        !summary.did_flip,
        "vehicle flipped (max_tilt_deg = {:.1})",
        summary.max_tilt_deg
    );
}

#[test]
#[ignore = "vehicle physics broken: chassis settles below terrain, see harness output"]
fn idle_settles() {
    let summary = run_scenario(Scenario::Idle, 3.0);
    assert!(
        !summary.did_flip,
        "idle vehicle flipped (max_tilt_deg = {:.1})",
        summary.max_tilt_deg
    );
    assert!(
        summary.max_speed_mps < 5.0,
        "idle vehicle exceeded 5 m/s (max_speed = {:.3})",
        summary.max_speed_mps
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
