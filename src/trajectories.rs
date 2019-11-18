// use super::MyError;
use crate::graph::path::{Path, PathSplit};
use crate::graph::Graph;
use crate::graphml::EdgeLookup;

use serde::Deserialize;
use serde_json::from_reader;

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

pub fn read_trajectorries<P: AsRef<std::path::Path>>(
    file_path: P,
) -> Result<Vec<Trajectory>, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(file_path)?;
    let file = std::io::BufReader::new(file);

    Ok(from_reader(file)?)
}

impl Trajectory {
    pub fn to_path(
        &self,
        graph: &Graph,
        edge_lookup: &EdgeLookup,
    ) -> Result<Path, Box<dyn std::error::Error>> {
        let id = 1;
        let edges: Vec<usize> = self
            .path
            .iter()
            .map(|id| edge_lookup[&id.to_string()])
            .collect();

        let first_node = edges.iter().take(1).map(|e| &graph.edges[*e].source_id);
        let nodes: Vec<usize> = first_node
            .chain(edges.iter().map(|e| &graph.edges[*e].target_id))
            .copied()
            .collect();

        let algo_split = None;
        let total_dimension_costs = [0.0; super::EDGE_COST_DIMENSION];

        let user_split = PathSplit {
            cuts: Vec::new(),
            alphas: Vec::new(),
            dimension_costs: Vec::new(),
            costs_by_alpha: Vec::new(),
        };
        let node_count = nodes.len();
        let mut path = Path {
            id,
            nodes,
            edges,
            user_split,
            algo_split,
            total_dimension_costs,
        };

        path.total_dimension_costs = path.get_subpath_costs(graph, 0, node_count - 1);

        Ok(path)
    }
}
