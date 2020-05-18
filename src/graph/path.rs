use crate::graph::Graph;
use crate::helpers::{add_edge_costs, Costs, MyVec, Preference};
use crate::EDGE_COST_DIMENSION;

#[derive(Clone, Debug)]
pub struct PathSplit {
    pub cuts: MyVec<u32>,
    pub alphas: MyVec<Preference>,
    pub dimension_costs: MyVec<Costs>,
    pub costs_by_alpha: MyVec<f64>,
}

impl PathSplit {
    pub fn get_total_cost(&self) -> f64 {
        self.costs_by_alpha
            .iter()
            .fold(0.0, |acc, cost| acc + *cost)
    }
}

#[derive(Debug, Clone)]
pub struct Path {
    pub id: Vec<(Option<u32>, u32)>,
    pub nodes: MyVec<u32>,
    pub edges: MyVec<u32>,
    pub user_split: PathSplit,
    pub algo_split: Option<PathSplit>,
    pub total_dimension_costs: Costs,
}

impl Path {
    pub fn get_subpath_costs(&self, graph: &Graph, start: u32, end: u32) -> Costs {
        let edges = &self.edges[start..end];
        edges.iter().fold([0.0; EDGE_COST_DIMENSION], |acc, edge| {
            add_edge_costs(&acc, &graph.edges[*edge].edge_costs)
        })
    }
}
