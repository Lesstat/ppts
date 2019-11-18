use crate::graph::Graph;
use crate::graphml::EdgeLookup;
use serde::Deserialize;
use serde_json::from_reader;

use std::path::Path;
use std::string::ToString;

#[derive(Debug, Deserialize)]
pub struct Trajectory {
    trip_id: i64,
    vehicle_id: i64,
    path: Vec<i64>,
}

pub fn check_trajectory(tra: &Trajectory, graph: &Graph, edge_lookup: &EdgeLookup) -> bool {
    tra.path.windows(2).all(|window| {
        let e0_idx = edge_lookup
            .get(&window[0].to_string())
            .unwrap_or_else(|| panic!("Could not find edge {}", window[0]));
        let e1_idx = edge_lookup
            .get(&window[1].to_string())
            .unwrap_or_else(|| panic!("Could not find edge {}", window[1]));

        let edge0 = &graph.edges[*e0_idx];
        let edge1 = &graph.edges[*e1_idx];

        edge0.target_id == edge1.source_id
    })
}

pub fn read_trajectorries<P: AsRef<Path>>(
    file_path: P,
) -> Result<Vec<Trajectory>, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(file_path)?;
    let file = std::io::BufReader::new(file);

    Ok(from_reader(file)?)
}
