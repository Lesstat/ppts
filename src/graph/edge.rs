use crate::helpers::Costs;
use crate::EDGE_COST_DIMENSION;

#[derive(Debug)]
pub struct Edge {
    pub id: usize,
    pub source_id: usize,
    pub target_id: usize,
    pub edge_costs: Costs,
    pub replaced_edges: Option<(usize, usize)>,
}

pub fn parse_costs(tokens: &[&str]) -> Costs {
    let mut edge_costs: Costs = [0.0; EDGE_COST_DIMENSION];
    for (index, token) in tokens.iter().enumerate() {
        edge_costs[index] = token.parse().unwrap();
    }
    edge_costs
}

impl Edge {
    pub fn new(
        id: usize,
        source_id: usize,
        target_id: usize,
        edge_costs: Costs,
        replaced_edges: Option<(usize, usize)>,
    ) -> Edge {
        Edge {
            id,
            source_id,
            target_id,
            edge_costs,
            replaced_edges,
        }
    }
}

#[derive(Debug)]
pub struct HalfEdge {
    pub edge_id: usize,
    pub target_id: usize,
    pub edge_costs: Costs,
}

impl HalfEdge {
    pub fn new(edge_id: usize, target_id: usize, edge_costs: Costs) -> HalfEdge {
        HalfEdge {
            edge_id,
            target_id,
            edge_costs,
        }
    }
}
