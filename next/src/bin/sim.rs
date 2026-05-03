// CLI driver for the headless simulation harness.
// Usage: sim <scenario> [duration_seconds] [--verbose] [--json]
//
// Scenarios: idle, forward, reverse, left, right, brake-test, script:<path>

use sandk_offroad_next::headless::{run_scenario, Scenario, ScriptStep};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: sim <scenario> [duration_s] [--verbose] [--json]");
        eprintln!("Scenarios: idle, forward, reverse, left, right, brake-test, script:<path>");
        std::process::exit(1);
    }

    let scenario_arg = &args[1];
    let mut duration_s = 5.0_f32;
    let mut verbose = false;
    let mut json_out = false;

    for arg in &args[2..] {
        if arg == "--verbose" {
            verbose = true;
        } else if arg == "--json" {
            json_out = true;
        } else if let Ok(d) = arg.parse::<f32>() {
            duration_s = d;
        }
    }

    let scenario = parse_scenario(scenario_arg);
    let mut summary = run_scenario(scenario, duration_s);

    if !verbose {
        summary.samples.clear();
    }

    if json_out {
        println!("{}", serde_json::to_string(&summary).expect("json serialization failed"));
    } else {
        print_human(&summary, verbose);
    }
}

fn parse_scenario(s: &str) -> Scenario {
    match s {
        "idle"       => Scenario::Idle,
        "forward"    => Scenario::Forward,
        "reverse"    => Scenario::Reverse,
        "left"       => Scenario::Left,
        "right"      => Scenario::Right,
        "brake-test" => Scenario::BrakeTest { accel_s: 2.0 },
        other if other.starts_with("script:") => {
            let path = &other["script:".len()..];
            let data = std::fs::read_to_string(path)
                .unwrap_or_else(|e| { eprintln!("Cannot read script file {}: {}", path, e); std::process::exit(1); });
            let steps: Vec<serde_json::Value> = serde_json::from_str(&data)
                .unwrap_or_else(|e| { eprintln!("Invalid JSON in {}: {}", path, e); std::process::exit(1); });
            let parsed = steps.iter().map(|v| ScriptStep {
                at_seconds: v["at_seconds"].as_f64().unwrap_or(0.0) as f32,
                drive: v["drive"].as_f64().unwrap_or(0.0) as f32,
                steer: v["steer"].as_f64().unwrap_or(0.0) as f32,
                brake: v["brake"].as_bool().unwrap_or(false),
            }).collect();
            Scenario::Script(parsed)
        }
        unknown => {
            eprintln!("Unknown scenario: {}", unknown);
            std::process::exit(1);
        }
    }
}

fn print_human(s: &sandk_offroad_next::headless::TelemetrySummary, verbose: bool) {
    println!("=== Simulation Summary ===");
    println!("scenario         : {}", s.scenario_name);
    println!("duration_s       : {:.2}", s.duration_s);
    println!("ticks            : {}", s.ticks);
    println!("start_pos        : [{:.3}, {:.3}, {:.3}]", s.start_pos[0], s.start_pos[1], s.start_pos[2]);
    println!("end_pos          : [{:.3}, {:.3}, {:.3}]", s.end_pos[0], s.end_pos[1], s.end_pos[2]);
    println!("displacement     : [{:.3}, {:.3}, {:.3}]", s.displacement[0], s.displacement[1], s.displacement[2]);
    println!("distance_m       : {:.3}", s.distance_traveled_m);
    println!("max_speed_mps    : {:.3}", s.max_speed_mps);
    println!("mean_speed_mps   : {:.3}", s.mean_speed_mps);
    println!("max_tilt_deg     : {:.2}", s.max_tilt_deg);
    println!("did_flip         : {}", s.did_flip);
    println!("chassis_height   : {:.3}", s.final_chassis_height);
    println!("above_terrain    : {:.3}", s.final_chassis_above_terrain);

    if verbose && !s.samples.is_empty() {
        println!("--- samples ---");
        for sample in &s.samples {
            println!(
                "  t={:.1}s pos=[{:.2},{:.2},{:.2}] speed={:.2} tilt={:.1}",
                sample.t_seconds,
                sample.pos[0], sample.pos[1], sample.pos[2],
                sample.speed_mps,
                sample.tilt_deg
            );
        }
    }
}
