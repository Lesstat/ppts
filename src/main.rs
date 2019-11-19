use std::env;
use std::fmt::{Display, Formatter};

mod graph;
mod graphml;
mod helpers;
mod lp;
mod trajectories;

use graph::path::Path;

const EDGE_COST_DIMENSION: usize = 4;

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

    let trajectories = trajectories::read_trajectorries(&args[2])?;

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
        .map(|mut p| {
            graph.find_preference(&mut p);
            if let Some(ref splits) = p.algo_split {
                println!("cut trajectory into {} parts", splits.cuts.len());
                println!("with preferences {:?}", splits.alphas);
            }
            p
        })
        .collect();

    // graph.find_preference(&mut path);
    // // server::start_server(graph);
    Ok(())
}
