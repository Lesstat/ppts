use std::env;

mod graph;
mod graphml;
mod helpers;
mod lp;

const EDGE_COST_DIMENSION: usize = 4;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        panic!("Please provide exactly one parameter, which is the path to the graph file");
    }
    let graph = graphml::read_graphml(&args[1]).unwrap();

    let user_split = graph::path::PathSplit {
        cuts: vec![1],
        alphas: vec![[0.0, 0.0, 0.0, 1.0]],
        dimension_costs: vec![[1.0, 2.0, 3.0, 4.0]],
        costs_by_alpha: vec![0.0],
    };

    let mut path = graph::path::Path {
        id: 0,

        nodes: vec![1, 2],
        edges: vec![5],
        user_split,
        algo_split: None,
        total_dimension_costs: [1.0, 2.0, 3.0, 4.0],
    };

    graph.find_preference(&mut path);
    // server::start_server(graph);
}
