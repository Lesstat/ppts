use std::collections::binary_heap::BinaryHeap;

use state::Direction::{BACKWARD, FORWARD};
use state::State;

use crate::graph::Graph;
use crate::helpers::{add_edge_costs, costs_by_alpha, Costs, Preference};
use crate::EDGE_COST_DIMENSION;

mod state;

pub struct HalfPath {
    pub edges: Vec<Vec<usize>>,
    pub dimension_costs: Vec<Costs>,
    pub total_dimension_costs: Costs,
    pub costs_by_alpha: Vec<f64>,
}

pub struct DijkstraResult {
    pub edges: Vec<usize>,
    pub costs: Costs,
    pub total_cost: f64,
}

pub struct Dijkstra<'a> {
    graph: &'a Graph,
    candidates: BinaryHeap<State>,
    touched_nodes: Vec<usize>,
    found_best_b: bool,
    found_best_f: bool,

    // Best dist to/from node
    pub cost_f: Vec<(Costs, f64)>,
    pub cost_b: Vec<(Costs, f64)>,

    // Best edge to/from node
    pub previous_f: Vec<Option<usize>>,
    pub previous_b: Vec<Option<usize>>,

    // (node_id, cost array, total_cost)
    best_node: (Option<usize>, Costs, f64),
}

impl<'a> Dijkstra<'a> {
    pub fn new(graph: &Graph) -> Dijkstra {
        let num_of_nodes = graph.nodes.len();
        Dijkstra {
            graph,
            candidates: BinaryHeap::new(),
            touched_nodes: Vec::new(),
            found_best_b: false,
            found_best_f: false,
            cost_f: vec![([0.0; EDGE_COST_DIMENSION], std::f64::MAX); num_of_nodes],
            cost_b: vec![([0.0; EDGE_COST_DIMENSION], std::f64::MAX); num_of_nodes],
            previous_f: vec![None; num_of_nodes],
            previous_b: vec![None; num_of_nodes],
            best_node: (None, [0.0; EDGE_COST_DIMENSION], std::f64::MAX),
        }
    }

    fn prepare(&mut self, source: usize, target: usize) {
        // Candidates
        self.candidates = BinaryHeap::new();
        self.candidates.push(State::new(source, FORWARD));
        self.candidates.push(State::new(target, BACKWARD));

        // Touched nodes
        for node_id in &self.touched_nodes {
            self.cost_f[*node_id] = ([0.0; EDGE_COST_DIMENSION], std::f64::MAX);
            self.cost_b[*node_id] = ([0.0; EDGE_COST_DIMENSION], std::f64::MAX);
            self.previous_f[*node_id] = None;
            self.previous_b[*node_id] = None;
        }
        self.touched_nodes.clear();

        self.found_best_b = false;
        self.found_best_f = false;

        // Node states
        self.cost_f[source].1 = 0.0;
        self.cost_b[target].1 = 0.0;
        self.touched_nodes.push(source);
        self.touched_nodes.push(target);

        // Best node
        self.best_node = (None, [0.0; EDGE_COST_DIMENSION], std::f64::MAX);
    }

    fn run(&mut self, source: usize, target: usize, alpha: Preference) -> Option<DijkstraResult> {
        self.prepare(source, target);

        // let now = Instant::now();
        // let mut n_popped: usize = 0;
        while let Some(candidate) = self.candidates.pop() {
            // n_popped += 1;
            if self.found_best_f && self.found_best_b {
                break;
            }
            self.process_state(candidate, alpha);
        }

        match self.best_node {
            (None, _, _) => None,
            (Some(node_id), costs, total_cost) => {
                /*
                    println!(
                    "Found path with cost {:?} in {:?}ms with {:?} nodes popped",
                    total_cost,
                    now.elapsed().as_millis(),
                    n_popped
                );
                     */
                // println!(
                //     "Found path with dim_costs {:?} and cost {:?}",
                //     costs, total_cost
                // );
                let edges = self.make_edge_path(node_id);
                Some(DijkstraResult {
                    edges,
                    costs,
                    total_cost,
                })
            }
        }
    }

    fn process_state(&mut self, candidate: State, alpha: Preference) {
        let State {
            node_id,
            costs,
            total_cost,
            direction,
        } = candidate;

        let my_costs;
        let other_costs;
        let found_best;
        let previous;
        if direction == FORWARD {
            my_costs = &mut self.cost_f;
            other_costs = &self.cost_b;
            found_best = &mut self.found_best_f;
            previous = &mut self.previous_f;
        } else {
            my_costs = &mut self.cost_b;
            other_costs = &self.cost_f;
            found_best = &mut self.found_best_b;
            previous = &mut self.previous_b;
        };

        if total_cost > my_costs[node_id].1 {
            return;
        };
        if total_cost > self.best_node.2 {
            *found_best = true;
            return;
        }
        if other_costs[node_id].1 != std::f64::MAX {
            let merged_cost = total_cost + other_costs[node_id].1;
            if merged_cost < self.best_node.2 {
                let merged_cost_vector = add_edge_costs(costs, other_costs[node_id].0);
                self.best_node = (Some(node_id), merged_cost_vector, merged_cost);
            }
        }

        let edges = if direction == FORWARD {
            self.graph.get_ch_edges_out(node_id)
        } else {
            self.graph.get_ch_edges_in(node_id)
        };
        for half_edge in edges {
            let next_node = half_edge.target_id;
            let next_costs = add_edge_costs(costs, half_edge.edge_costs);
            let next_total_cost = total_cost + costs_by_alpha(half_edge.edge_costs, alpha);

            if next_total_cost < my_costs[next_node].1 {
                my_costs[next_node] = (next_costs, next_total_cost);
                previous[next_node] = Some(half_edge.edge_id);
                self.touched_nodes.push(next_node);
                self.candidates.push(State {
                    node_id: next_node,
                    costs: next_costs,
                    total_cost: next_total_cost,
                    direction,
                });
            }
        }
    }

    fn make_edge_path(&self, connector: usize) -> Vec<usize> {
        let mut edges = Vec::new();
        let mut previous_edge = self.previous_f[connector];
        let mut successive_edge = self.previous_b[connector];

        // backwards
        while let Some(edge_id) = previous_edge {
            edges.push(edge_id);
            previous_edge = self.previous_f[self.graph.edges[edge_id].source_id];
        }
        edges.reverse();

        // forwards
        while let Some(edge_id) = successive_edge {
            edges.push(edge_id);
            successive_edge = self.previous_b[self.graph.edges[edge_id].target_id];
        }
        edges
    }
}

pub fn find_path(
    dijkstra: &mut Dijkstra,
    include: &[usize],
    alpha: Preference,
) -> Option<HalfPath> {
    // println!("=== Running Dijkstra search ===");
    let mut edges = Vec::new();
    let mut dimension_costs = Vec::new();
    let mut total_dimension_costs = [0.0; EDGE_COST_DIMENSION];
    let mut costs_by_alpha = Vec::new();

    for win in include.windows(2) {
        if let Some(result) = dijkstra.run(win[0], win[1], alpha) {
            edges.push(result.edges);
            result
                .costs
                .iter()
                .enumerate()
                .for_each(|(index, val)| total_dimension_costs[index] += *val);
            dimension_costs.push(result.costs);
            costs_by_alpha.push(result.total_cost);
        } else {
            println!("=== Dijkstra could not find a route ===");
            return None;
        }
    }
    // println!("=== Found path with costs: {:?} ===", costs_by_alpha);
    Some(HalfPath {
        edges,
        dimension_costs,
        total_dimension_costs,
        costs_by_alpha,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{parse_graph_file, Graph};

    fn get_graph() -> Graph {
        parse_graph_file("./src/test_graphs/testGraph").unwrap()
    }

    fn get_conc_graph() -> Graph {
        parse_graph_file("./src/test_graphs/concTestGraph").unwrap()
    }

    #[test]
    fn normal_case() {
        let graph = get_graph();
        let conc_graph = get_conc_graph();
        let mut dijkstra = Dijkstra::new(&graph);
        let mut dijkstra_conc = Dijkstra::new(&conc_graph);
        let alpha = [0.0, 1.0, 0.0, 0.0];

        let mut shortest_path;
        let mut shortest_path_conc;
        let mut path;
        let mut path_conc;

        // first query
        assert!(dijkstra.run(0, 4, alpha).is_none());
        assert!(dijkstra_conc.run(0, 4, alpha).is_none());

        // second query
        assert!(dijkstra.run(4, 11, alpha).is_none());
        assert!(dijkstra_conc.run(4, 11, alpha).is_none());

        // third query
        shortest_path = dijkstra.run(2, 5, alpha);
        shortest_path_conc = dijkstra_conc.run(2, 5, alpha);
        assert!(shortest_path.is_some());
        assert!(shortest_path_conc.is_some());

        path = shortest_path.unwrap();
        path_conc = shortest_path_conc.unwrap();
        assert_eq!(path.edges, vec![4, 7]);
        assert_eq!(path.total_cost, 2.0);
        assert_eq!(path_conc.edges, vec![4, 7]);
        assert_eq!(path_conc.total_cost, 2.0);

        // fourth query
        shortest_path = dijkstra.run(2, 10, alpha);
        shortest_path_conc = dijkstra_conc.run(2, 10, alpha);
        assert!(shortest_path.is_some());
        assert!(shortest_path_conc.is_some());

        path = shortest_path.unwrap();
        path_conc = shortest_path_conc.unwrap();
        assert_eq!(path.edges, vec![4, 7, 9, 12]);
        assert_eq!(path.total_cost, 4.0);
        assert_eq!(path_conc.edges, vec![4, 21]);
        assert_eq!(path_conc.total_cost, 4.0);

        // fifth query
        shortest_path = dijkstra.run(4, 10, alpha);
        shortest_path_conc = dijkstra_conc.run(4, 10, alpha);
        assert!(shortest_path.is_some());
        assert!(shortest_path_conc.is_some());

        path = shortest_path.unwrap();
        path_conc = shortest_path_conc.unwrap();
        assert_eq!(path.edges, vec![7, 9, 12]);
        assert_eq!(path.total_cost, 3.0);
        assert_eq!(path_conc.edges, vec![21]);
        assert_eq!(path_conc.total_cost, 3.0);
    }
}
