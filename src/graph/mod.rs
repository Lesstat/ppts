pub use edge::Edge;
use edge::HalfEdge;

pub use node::Node;
use path::Path;

use crate::graph::path::PathSplit;
use crate::helpers::Preference;
use crate::lp::PreferenceEstimator;
use crate::EDGE_COST_DIMENSION;

mod dijkstra;
mod edge;
mod node;
pub mod path;

#[derive(Debug)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    offsets_in: Vec<usize>,
    offsets_out: Vec<usize>,
    half_edges_in: Vec<HalfEdge>,
    half_edges_out: Vec<HalfEdge>,
}

impl Graph {
    pub fn new(mut nodes: Vec<Node>, mut edges: Vec<Edge>) -> Graph {
        println!("Constructing graph...");
        let mut offsets_out: Vec<usize> = vec![0; nodes.len() + 1];
        let mut offsets_in: Vec<usize> = vec![0; nodes.len() + 1];
        let mut half_edges_out: Vec<HalfEdge> = Vec::new();
        let mut half_edges_in: Vec<HalfEdge> = Vec::new();

        // sort nodes by id
        nodes.sort_by(|a, b| a.id.cmp(&b.id));

        // half_edges and offsets out
        edges.sort_by(|a, b| a.source_id.cmp(&b.source_id));
        edges
            .iter()
            .filter(|edge| nodes[edge.target_id].ch_level >= nodes[edge.source_id].ch_level)
            .for_each(|edge| {
                offsets_out[edge.source_id + 1] += 1;
                half_edges_out.push(HalfEdge::new(edge.id, edge.target_id, edge.edge_costs));
            });

        // half_edges and offsets in
        edges.sort_by(|a, b| a.target_id.cmp(&b.target_id));
        edges
            .iter()
            .filter(|edge| nodes[edge.source_id].ch_level >= nodes[edge.target_id].ch_level)
            .for_each(|edge| {
                offsets_in[edge.target_id + 1] += 1;
                half_edges_in.push(HalfEdge::new(edge.id, edge.source_id, edge.edge_costs));
            });

        // finish offset arrays
        for index in 1..offsets_out.len() {
            offsets_out[index] += offsets_out[index - 1];
            offsets_in[index] += offsets_in[index - 1];
        }

        // sort edges by id
        edges.sort_by(|a, b| a.id.cmp(&b.id));
        Graph {
            nodes,
            edges,
            offsets_in,
            offsets_out,
            half_edges_in,
            half_edges_out,
        }
    }

    pub fn find_shortest_path(
        &self,
        id: usize,
        include: Vec<usize>,
        alpha: Preference,
    ) -> Option<Path> {
        if let Some(result) = dijkstra::find_path(self, &include, alpha) {
            let unpacked_edges: Vec<Vec<usize>> = result
                .edges
                .iter()
                .map(|subpath_edges| {
                    subpath_edges
                        .iter()
                        .flat_map(|edge| self.unpack_edge(*edge))
                        .collect()
                })
                .collect();
            let cuts = unpacked_edges.iter().map(|edges| edges.len()).collect();

            let edges: Vec<usize> = unpacked_edges.into_iter().flatten().collect();
            let mut nodes: Vec<usize> = edges
                .iter()
                .map(|edge| self.edges[*edge].source_id)
                .collect();
            nodes.push(*include.last().unwrap());

            return Some(Path {
                id,
                nodes,
                edges,
                user_split: PathSplit {
                    cuts,
                    alphas: vec![alpha],
                    dimension_costs: result.dimension_costs,
                    costs_by_alpha: result.costs_by_alpha,
                },
                algo_split: None,
                total_dimension_costs: result.total_dimension_costs,
            });
        }
        None
    }

    pub fn find_preference(&self, path: &mut Path) {
        println!("=== Calculate Preference ===");
        let path_length = path.nodes.len();
        let mut cuts = Vec::new();
        let mut alphas = Vec::new();
        let mut start: usize = 0;
        while start <= path_length {
            let mut low = start;
            let mut high = path_length;
            let mut best_pref = None;
            let mut best_cut = 0;
            loop {
                let m = (low + high) / 2;
                println!("looking for pref between nodes {} and {} ", start, m);
                let mut estimator = PreferenceEstimator::new(self);
                let pref = estimator.calc_preference(&path, start, m);
                if pref.is_some() {
                    println!("found pref {:?}", pref);
                    low = m + 1;
                    best_pref = pref;
                    best_cut = m;
                } else {
                    high = m;
                }
                if low >= high {
                    alphas.push(best_pref.unwrap());
                    cuts.push(best_cut);
                    break;
                }
            }
            start = best_cut;
        }
        let dimension_costs = Vec::new();
        let costs_by_alpha = Vec::new();
        path.algo_split = Some(PathSplit {
            cuts,
            alphas,
            dimension_costs,
            costs_by_alpha,
        });
        println!("=== Found Preference ===");
    }

    fn get_ch_edges_out(&self, node_id: usize) -> &[HalfEdge] {
        &self.half_edges_out[self.offsets_out[node_id]..self.offsets_out[node_id + 1]]
    }

    fn get_ch_edges_in(&self, node_id: usize) -> &[HalfEdge] {
        &self.half_edges_in[self.offsets_in[node_id]..self.offsets_in[node_id + 1]]
    }

    fn unpack_edge(&self, edge: usize) -> Vec<usize> {
        if let Some((edge1, edge2)) = self.edges[edge].replaced_edges {
            let mut first = self.unpack_edge(edge1);
            first.extend(self.unpack_edge(edge2).iter());
            return first;
        }
        vec![edge]
    }
}
