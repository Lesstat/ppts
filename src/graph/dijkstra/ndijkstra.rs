use crate::{
    graph::Graph,
    helpers::{costs_by_alpha, MyVec, Preference},
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
        if self.last_from == from {
            if self.dist[to] < f64::MAX {
                return Some(self.dist[to]);
            }
        } else {
            // If not we initialize it normally
            self.last_from = from;
            self.reset_state();

            self.heap.push(HeapElement {
                dist: 0.0,
                node: from,
                prev_node: from,
            });
        }

        while let Some(HeapElement {
            dist: u_dist,
            node: u,
            prev_node,
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
            self.prev[u] = Some(prev_node);
            self.touched.push(u);

            for edge in self.g.get_ch_edges_out(u) {
                let alt = u_dist + costs_by_alpha(&edge.edge_costs, alpha);
                if alt < self.dist[edge.target_id] {
                    self.heap.push(HeapElement {
                        dist: alt,
                        node: edge.target_id,
                        prev_node: u,
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
}

#[derive(Debug, PartialEq)]
struct HeapElement {
    dist: f64,
    node: u32,
    prev_node: u32,
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
