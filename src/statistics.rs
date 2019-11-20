use crate::helpers::Preference;
use crate::trajectories::Trajectory;

use serde::Serialize;

#[derive(Serialize, Default)]
pub struct SplittingStatistics {
    trip_id: i64,
    vehicle_id: i64,
    trajectory_length: usize,
    pub removed_self_loop_indices: Vec<usize>,
    pub preferences: Vec<Preference>,
    pub cuts: Vec<usize>,
    pub run_time: usize,
}

impl SplittingStatistics {
    pub fn new(t: &Trajectory) -> SplittingStatistics {
        let mut stat = SplittingStatistics::default();

        stat.trip_id = t.trip_id;
        stat.vehicle_id = t.vehicle_id;
        stat.trajectory_length = t.path.len() + 1; // no. of nodes

        stat
    }
}
