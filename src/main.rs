use std::env;
use std::fmt::{Display, Formatter};

mod graph;
mod graphml;
mod helpers;
mod lp;
mod trajectories;

use graph::path::Path;

const EDGE_COST_DIMENSION: usize = 5;

#[derive(Debug)]
pub enum MyError {
    InvalidTrajectories,
}

impl Display for MyError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        match self {
            MyError::InvalidTrajectories => write!(f, "Invalid Trajectories"),
        }
    }
}

impl std::error::Error for MyError {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        panic!("Please provide exactly two parameter, which is the path to the graph file and the path to the trajectory file");
    }

    let (graph, edge_lookup) = graphml::read_graphml(&args[1])?;

    let mut trajectories = trajectories::read_trajectorries(&args[2])?;

    trajectories
        .iter_mut()
        .for_each(|t| t.filter_out_self_loops(&graph, &edge_lookup));

    if trajectories
        .iter()
        .all(|t| trajectories::check_trajectory(t, &graph, &edge_lookup))
    {
        println!("all {} trajectories seem valid :-)", trajectories.len());
    } else {
        println!("There are invalid trajectories :-(");
        return Err(Box::new(MyError::InvalidTrajectories));
    }

    let _results: Vec<Path> = trajectories
        .iter()
        .map(|t| t.to_path(&graph, &edge_lookup))
        .enumerate()
        .map(|(i, p)| {
            println!(
                "processing trajectory {} => {} of {} trajectories",
                p.id,
                i + 1,
                trajectories.len()
            );
            p
        })
        .map(|mut p| {
            graph.find_preference(&mut p);
            p
        })
        .collect();

    // graph.find_preference(&mut path);
    // // server::start_server(graph);
    Ok(())
}
