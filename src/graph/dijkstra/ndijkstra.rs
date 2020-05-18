use crate::{
    graph::{
        path::{Path, PathSplit},
        Graph,
    },
    helpers::{add_edge_costs, costs_by_alpha, MyVec, Preference, EQUAL_WEIGHTS},
    EDGE_COST_DIMENSION,
};

use ordered_float::OrderedFloat;
use std::collections::BinaryHeap;

pub struct NDijkstra<'a> {
    g: &'a Graph,
    dist: MyVec<f64>,
    prev: MyVec<Option<u32>>,
    heap: BinaryHeap<HeapElement>,
    touched: Vec<u32>,
    last_from: u32,
    last_pref: Preference,
}

impl<'a> NDijkstra<'a> {
    pub fn new(g: &'a Graph) -> Self {
        let dist = vec![f64::MAX; g.nodes.len()].into();
        let prev = vec![None; g.nodes.len()].into();
        let heap = BinaryHeap::new();
        let touched = Vec::new();
        NDijkstra {
            g,
            dist,
            prev,
            heap,
            touched,
            last_from: u32::MAX,
            last_pref: EQUAL_WEIGHTS,
        }
    }

    pub fn reset_state(&mut self) {
        for &t in &self.touched {
            self.dist[t] = f64::MAX;
            self.prev[t] = None;
        }
        self.heap.clear();
        self.touched.clear();
    }

    pub fn run(&mut self, from: u32, to: u32, alpha: &Preference) -> Option<f64> {
        // If the query starts from the same node as before we can reuse it
        if self.last_from == from && self.last_pref == *alpha {
            if self.dist[to] < f64::MAX {
                return Some(self.dist[to]);
            }
        } else {
            // If not we initialize it normally
            self.last_from = from;
            self.last_pref = *alpha;
            self.reset_state();

            self.heap.push(HeapElement {
                dist: 0.0,
                node: from,
                prev_edge: from,
            });
        }

        while let Some(HeapElement {
            dist: u_dist,
            node: u,
            prev_edge,
        }) = self.heap.pop()
        {
            // If your heap does not support a decrease key operation, you can
            // include nodes multiple times and with the following condition
            // ensure, that each is only processed once. (This is also said to
            // perform better than decrease key, but I never benchmarked it)
            if u_dist >= self.dist[u] {
                continue;
            }

            self.dist[u] = u_dist;
            self.prev[u] = Some(prev_edge);
            self.touched.push(u);

            for edge in self.g.get_ch_edges_out(u) {
                let alt = u_dist + costs_by_alpha(&edge.edge_costs, alpha);
                if alt < self.dist[edge.target_id] {
                    self.heap.push(HeapElement {
                        dist: alt,
                        node: edge.target_id,
                        prev_edge: edge.edge_id,
                    });
                }
            }
            // We moved this down here to have the heap in a consistent state
            // (all outgoing neighbors of `to` are in the heap)
            if u == to {
                return Some(u_dist);
            }
        }

        None
    }

    pub fn path(&mut self, to: u32) -> Option<Path> {
        if self.prev[to] == None {
            let alpha = self.last_pref;
            self.run(self.last_from, to, &alpha);
        }
        // early return if `to` is unreachable
        self.prev[to]?;

        let mut edges = MyVec::new();
        let mut nodes = MyVec::new();
        let mut total_dimension_costs = [0.0; EDGE_COST_DIMENSION];
        let mut cur_node = to;

        while cur_node != self.last_from {
            let edge = self.prev[cur_node].expect("Previous Edge must exist");
            edges.push(edge);
            nodes.push(cur_node);
            cur_node = self.g.edges[edge].source_id;
            total_dimension_costs =
                add_edge_costs(&self.g.edges[edge].edge_costs, &total_dimension_costs);
        }

        nodes.push(cur_node);

        edges.reverse();
        nodes.reverse();
        let edges = edges
            .0
            .into_iter()
            .flat_map(|e| self.g.unpack_edge(e))
            .collect::<Vec<_>>()
            .into();

        Some(Path {
            id: 0,
            nodes,
            edges,
            user_split: PathSplit {
                cuts: MyVec(vec![0]),
                alphas: MyVec(vec![self.last_pref]),
                dimension_costs: MyVec(vec![total_dimension_costs]),
                costs_by_alpha: MyVec(vec![costs_by_alpha(
                    &total_dimension_costs,
                    &self.last_pref,
                )]),
            },
            algo_split: None,
            total_dimension_costs,
        })
    }
}

#[derive(Debug, PartialEq)]
struct HeapElement {
    dist: f64,
    node: u32,
    prev_edge: u32,
}

impl Eq for HeapElement {}

// The binary heap we are using is a max-heap. Therefore, we need to define a
// custom ordering which reverses the sorting.
impl Ord for HeapElement {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        OrderedFloat(other.dist).cmp(&OrderedFloat(self.dist))
    }
}

impl PartialOrd for HeapElement {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
