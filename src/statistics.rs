use crate::helpers::{MyVec, Preference};
use crate::trajectories::Trajectory;

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::from_reader;

#[derive(Serialize, Deserialize, Default)]
pub struct SplittingStatistics {
    pub trip_id: Vec<(Option<u32>, u32)>,
    vehicle_id: i64,
    trajectory_length: usize,
    pub removed_self_loop_indices: MyVec<u32>,
    pub preferences: MyVec<Preference>,
    pub cuts: MyVec<u32>,
    pub splitting_run_time: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub non_opt_subpaths: Option<NonOptSubPathsResult>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct NonOptSubPathsResult {
    pub non_opt_subpaths: MyVec<(u32, u32)>,
    pub runtime: usize,
}

#[derive(Serialize, Deserialize)]
pub struct SplittingResults {
    pub graph_file: String,
    pub trajectory_file: String,
    pub metrics: Vec<String>,
    pub results: Vec<SplittingStatistics>,
}

impl SplittingStatistics {
    pub fn new(t: &Trajectory) -> SplittingStatistics {
        let mut stat = SplittingStatistics::default();

        stat.trip_id = t.trip_id.clone();
        stat.vehicle_id = t.vehicle_id;
        stat.trajectory_length = t.path.len() + 1; // no. of nodes

        stat
    }
}

pub fn read_splitting_results<P: AsRef<Path>>(
    path: P,
) -> Result<SplittingResults, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let file = std::io::BufReader::new(file);
    Ok(from_reader(file)?)
}
