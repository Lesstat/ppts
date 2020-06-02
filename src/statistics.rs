use crate::helpers::{Costs, MyVec, Preference};
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
pub struct RepresentativeAlphaResult {
    pub trip_id: Vec<(Option<u32>, u32)>,
    vehicle_id: i64,
    trajectory_length: usize,
    pub removed_self_loop_indices: MyVec<u32>,
    pub preference: Preference,
    pub trajectory_cost: Costs,
    pub alpha_cost: Costs,
    pub aggregated_cost_diff: f64,
    pub overlap: f64,
    pub run_time: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub better_overlap_by_rng: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub better_aggregated_cost_diff_by_rng: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrong_turns: Option<Vec<usize>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub single_preference_decomposition_longest_optimal_subpath: Option<Vec<usize>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub single_preference_decomposition_representative_pref: Option<Vec<usize>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub single_preference_decomposition_greedy: Option<Vec<usize>>,
}

impl RepresentativeAlphaResult {
    pub fn new(t: &Trajectory) -> Self {
        let mut stat = Self::default();

        stat.trip_id = t.trip_id.clone();
        stat.vehicle_id = t.vehicle_id;
        stat.trajectory_length = t.path.len() + 1; // no. of nodes

        stat
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct NonOptSubPathsResult {
    pub non_opt_subpaths: MyVec<(u32, u32)>,
    pub runtime: usize,
}

#[derive(Serialize, Deserialize)]
pub struct ExperimentResults<T> {
    pub graph_file: String,
    pub trajectory_file: String,
    pub metrics: Vec<String>,
    pub start_time: String,
    pub results: Vec<T>,
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
) -> Result<ExperimentResults<SplittingStatistics>, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let file = std::io::BufReader::new(file);
    Ok(from_reader(file)?)
}

pub fn read_representative_results<P: AsRef<Path>>(
    path: P,
) -> Result<ExperimentResults<RepresentativeAlphaResult>, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let file = std::io::BufReader::new(file);
    Ok(from_reader(file)?)
}
