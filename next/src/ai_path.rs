// AI race path: shared waypoint resource + per-follower progress tracking.
//
// Builds a closed loop of waypoints from the course gates, plus densified
// intermediate points so AI cars steer along smooth curves rather than
// snapping between corners. PathFollower components track per-entity
// progress (current segment + lap + cumulative distance for leaderboard).
//
// Public API consumed by ai_driver, rival, race:
//   RacePath::waypoints     — Vec<Vec3>, ordered clockwise around the map.
//   RacePath::lookahead(start_idx, distance_m) -> Vec3
//   RacePath::closest_segment(pos) -> usize
//   PathFollower { current_idx, lap, total_distance }

use bevy::prelude::*;

use crate::terrain::terrain_height_at;

// ---------------------------------------------------------------------------
// Gate definitions (mirrored from course.rs — cannot import private consts)
// ---------------------------------------------------------------------------

/// XZ positions of each arch in the same order as course::GATES.
/// START (0) -> CKPT1 (1) -> CKPT2 (2) -> FINISH (3).
const GATES: [(f32, f32); 4] = [
    (5.0, -5.0),    // START  (idx 0)
    (40.0, 30.0),   // CKPT 1 (idx 1)
    (-40.0, 50.0),  // CKPT 2 (idx 2)
    (60.0, -40.0),  // FINISH (idx 3)
];

/// Number of intermediate points to insert between each consecutive gate pair.
const SUBDIVISIONS: usize = 5;

/// Height above terrain for each waypoint (matches course Y offset style).
const WAYPOINT_Y_OFFSET: f32 = 1.0;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct AiPathPlugin;

impl Plugin for AiPathPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RacePath>()
            .add_systems(Startup, build_race_path)
            .add_systems(Update, advance_path_followers);
    }
}

// ---------------------------------------------------------------------------
// RacePath resource
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct RacePath {
    pub waypoints: Vec<Vec3>,
}

impl RacePath {
    pub fn new() -> Self {
        let waypoints = build_waypoints();
        Self { waypoints }
    }

    /// Find the index of the segment closest to a world position.
    /// Returns the index of the START point of that segment (0..waypoints.len()).
    /// The path is a closed loop so segment N goes from waypoints[N] to
    /// waypoints[(N+1) % len].
    pub fn closest_segment(&self, pos: Vec3) -> usize {
        let n = self.waypoints.len();
        if n == 0 {
            return 0;
        }
        let mut best_idx = 0;
        let mut best_dist_sq = f32::MAX;
        for i in 0..n {
            let a = self.waypoints[i];
            let b = self.waypoints[(i + 1) % n];
            let d = point_to_segment_dist_sq(pos, a, b);
            if d < best_dist_sq {
                best_dist_sq = d;
                best_idx = i;
            }
        }
        best_idx
    }

    /// Look ahead `distance_m` meters along the path from `start_idx`,
    /// wrapping around if needed. Returns the world point.
    pub fn lookahead(&self, start_idx: usize, distance_m: f32) -> Vec3 {
        let n = self.waypoints.len();
        if n == 0 {
            return Vec3::ZERO;
        }
        let mut remaining = distance_m;
        let mut idx = start_idx % n;
        loop {
            let next = (idx + 1) % n;
            let seg_len = self.waypoints[next].distance(self.waypoints[idx]);
            if remaining <= seg_len || seg_len < 1e-6 {
                // Interpolate within this segment.
                let t = if seg_len > 1e-6 { remaining / seg_len } else { 0.0 };
                return self.waypoints[idx].lerp(self.waypoints[next], t.min(1.0));
            }
            remaining -= seg_len;
            idx = next;
            // Safety: if we've gone all the way around, return current point.
            if idx == start_idx % n && remaining > 0.0 {
                return self.waypoints[idx];
            }
        }
    }

    /// Total path length (sum of segment lengths). Useful for race progress.
    pub fn total_length(&self) -> f32 {
        let n = self.waypoints.len();
        if n < 2 {
            return 0.0;
        }
        let mut total = 0.0_f32;
        for i in 0..n {
            total += self.waypoints[i].distance(self.waypoints[(i + 1) % n]);
        }
        total
    }

    /// Progress in meters along the closed loop for a position projected to
    /// its closest segment. Result is in 0..total_length.
    pub fn progress_meters(&self, pos: Vec3) -> f32 {
        let n = self.waypoints.len();
        if n < 2 {
            return 0.0;
        }
        let seg = self.closest_segment(pos);
        // Sum lengths of all complete segments before `seg`.
        let mut base = 0.0_f32;
        for i in 0..seg {
            base += self.waypoints[i].distance(self.waypoints[(i + 1) % n]);
        }
        // Add partial progress within the current segment.
        let a = self.waypoints[seg];
        let b = self.waypoints[(seg + 1) % n];
        let seg_len = a.distance(b);
        if seg_len > 1e-6 {
            let ab = b - a;
            let ap = pos - a;
            let t = ap.dot(ab) / (seg_len * seg_len);
            base += t.clamp(0.0, 1.0) * seg_len;
        }
        base
    }
}

// ---------------------------------------------------------------------------
// PathFollower component
// ---------------------------------------------------------------------------

#[derive(Component, Default)]
pub struct PathFollower {
    pub current_idx: usize,
    pub lap: u32,
    /// Cumulative meters since race start: lap * loop_len + within-loop progress.
    pub total_distance: f32,
}

// ---------------------------------------------------------------------------
// Startup system: build the RacePath resource
// ---------------------------------------------------------------------------

fn build_race_path(mut path: ResMut<RacePath>) {
    *path = RacePath::new();
    info!(
        "ai_path: built race path with {} waypoints, total length {:.1} m",
        path.waypoints.len(),
        path.total_length()
    );
}

// ---------------------------------------------------------------------------
// Update system: advance PathFollower for each entity with a Transform
// ---------------------------------------------------------------------------

fn advance_path_followers(
    path: Res<RacePath>,
    mut followers: Query<(&Transform, &mut PathFollower)>,
) {
    let n = path.waypoints.len();
    if n < 2 {
        return;
    }
    let loop_len = path.total_length();

    for (transform, mut follower) in followers.iter_mut() {
        let pos = transform.translation;
        let cur_idx = follower.current_idx;
        let next_idx = (cur_idx + 1) % n;

        let dist_to_cur  = pos.distance(path.waypoints[cur_idx]);
        let dist_to_next = pos.distance(path.waypoints[next_idx]);

        // Advance when entity is closer to the next waypoint than the current.
        if dist_to_next < dist_to_cur {
            follower.current_idx = next_idx;
            // Detect a lap completion: wrapping from the last segment back to 0.
            if next_idx == 0 {
                follower.lap += 1;
            }
        }

        // Update cumulative distance.
        let within = path.progress_meters(pos);
        follower.total_distance = follower.lap as f32 * loop_len + within;
    }
}

// ---------------------------------------------------------------------------
// Waypoint construction helpers
// ---------------------------------------------------------------------------

/// Build the full waypoint list from the 4 course gates with Catmull-Rom
/// interpolation for smooth curves between gates. Returns ~24-28 points in a
/// closed loop.
fn build_waypoints() -> Vec<Vec3> {
    // Convert gate XZ positions to Vec3 at terrain height.
    let gates: Vec<Vec3> = GATES
        .iter()
        .map(|&(x, z)| Vec3::new(x, terrain_height_at(x, z) + WAYPOINT_Y_OFFSET, z))
        .collect();

    let n = gates.len();
    let mut waypoints = Vec::with_capacity(n * (SUBDIVISIONS + 1));

    for i in 0..n {
        // Catmull-Rom needs four control points: p0, p1, p2, p3 (the path
        // goes from p1 to p2, using p0 and p3 as tension guides).
        let p0 = gates[(i + n - 1) % n];
        let p1 = gates[i];
        let p2 = gates[(i + 1) % n];
        let p3 = gates[(i + 2) % n];

        // Emit p1 plus SUBDIVISIONS intermediate points (do not emit p2 here;
        // it will be the p1 of the next iteration).
        waypoints.push(p1);
        for s in 1..=SUBDIVISIONS {
            let t = s as f32 / (SUBDIVISIONS + 1) as f32;
            let pt = catmull_rom(p0, p1, p2, p3, t);
            // Clamp Y to terrain height + offset so the path doesn't clip ground.
            let terrain_y = terrain_height_at(pt.x, pt.z) + WAYPOINT_Y_OFFSET;
            waypoints.push(Vec3::new(pt.x, terrain_y.max(pt.y), pt.z));
        }
    }

    waypoints
}

/// Catmull-Rom spline evaluation at parameter t in [0, 1] between p1 and p2.
/// p0 and p3 are the surrounding control points.
fn catmull_rom(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;
    let t3 = t2 * t;
    // Standard uniform Catmull-Rom formula (alpha = 0.5 tangent scaling).
    let v = 0.5
        * (2.0 * p1
            + (-p0 + p2) * t
            + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
            + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3);
    v
}

/// Squared distance from point `p` to the line segment [a, b].
fn point_to_segment_dist_sq(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_squared();
    if len_sq < 1e-12 {
        return p.distance_squared(a);
    }
    let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    let closest = a + ab * t;
    p.distance_squared(closest)
}

// ---------------------------------------------------------------------------
// Unit tests (no Bevy App required — pure math)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_square_path() -> RacePath {
        // Simple 4-point square in the XZ plane at Y=0.
        RacePath {
            waypoints: vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(10.0, 0.0, 0.0),
                Vec3::new(10.0, 0.0, 10.0),
                Vec3::new(0.0, 0.0, 10.0),
            ],
        }
    }

    #[test]
    fn total_length_square() {
        let path = make_square_path();
        // Square with 10 m sides: perimeter = 40 m.
        let len = path.total_length();
        assert!(
            (len - 40.0).abs() < 0.01,
            "expected 40.0, got {len}"
        );
    }

    #[test]
    fn closest_segment_basic() {
        let path = make_square_path();
        // Point near the first segment (Y=0 -> (10,0,0)).
        let idx = path.closest_segment(Vec3::new(5.0, 0.0, 0.5));
        assert_eq!(idx, 0, "expected segment 0, got {idx}");
    }

    #[test]
    fn lookahead_wraps() {
        let path = make_square_path();
        // Start at segment 3 (last segment), look ahead 15 m — should wrap.
        let pt = path.lookahead(3, 15.0);
        // 15 m from segment 3 start (0,0,10): 10 m finishes seg 3 -> at (0,0,0),
        // then 5 m into seg 0 -> ends near (5,0,0).
        assert!(
            (pt.x - 5.0).abs() < 0.1 && pt.z.abs() < 0.1,
            "lookahead wrap expected ~(5,0,0), got {pt:?}"
        );
    }

    #[test]
    fn progress_meters_start() {
        let path = make_square_path();
        let prog = path.progress_meters(Vec3::new(0.0, 0.0, 0.0));
        assert!(prog < 1.0, "progress at start should be near 0, got {prog}");
    }

    #[test]
    fn waypoints_count_in_range() {
        // Full path build uses terrain — just verify count is in expected range.
        let path = RacePath::new();
        let n = path.waypoints.len();
        assert!(
            n >= 24 && n <= 32,
            "expected 24-32 waypoints, got {n}"
        );
    }
}
