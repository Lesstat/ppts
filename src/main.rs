use std::convert::TryInto;
use std::env;
use std::fmt::{Display, Formatter};
use std::io::Write;
use std::time::Instant;

mod graph;
mod graphml;
mod helpers;
mod lp;
mod statistics;
mod trajectories;

use graph::path::Path;
use graphml::{AttributeType, GraphData};
use statistics::SplittingStatistics;

use chrono::prelude::*;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;

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

#[derive(Serialize)]
struct Results<'a> {
    graph_file: &'a str,
    trajectory_file: &'a str,
    metrics: [&'a str; EDGE_COST_DIMENSION],
    results: Vec<SplittingStatistics>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        panic!("Please provide exactly two parameter, which is the path to the graph file and the path to the trajectory file");
    }

    let graph_file = &args[1];

    let GraphData {
        graph,
        edge_lookup,
        keys,
    } = graphml::read_graphml(graph_file)?;

    let trajectory_file = &args[2];
    let mut trajectories = trajectories::read_trajectorries(trajectory_file)?;

    let mut statistics: Vec<_> = trajectories.iter().map(SplittingStatistics::new).collect();

    trajectories
        .iter_mut()
        .zip(statistics.iter_mut())
        .for_each(|(t, s)| {
            s.removed_self_loop_indices = t.filter_out_self_loops(&graph, &edge_lookup);
        });

    if trajectories
        .iter()
        .all(|t| trajectories::check_trajectory(t, &graph, &edge_lookup))
    {
        println!("all {} trajectories seem valid :-)", trajectories.len());
    } else {
        println!("There are invalid trajectories :-(");
        return Err(Box::new(MyError::InvalidTrajectories));
    }

    let progress = ProgressBar::new(trajectories.len().try_into().unwrap());
    progress.set_style(
        ProgressStyle::default_spinner()
            .template("[{elapsed}] {bar:40.cyan/blue} {pos}/{len} [{eta} remaining]")
            .progress_chars("#>-"),
    );

    progress.set_draw_delta((trajectories.len() / 100).try_into().unwrap());

    let _results: Vec<Path> = trajectories
        .iter()
        .zip(statistics.iter_mut())
        .map(|(t, s)| (t.to_path(&graph, &edge_lookup), s))
        .map(|(mut p, s)| {
            let start = Instant::now();
            graph.find_preference(&mut p);
            let time = start.elapsed();
            s.run_time = time
                .as_millis()
                .try_into()
                .expect("Couldn't convert run time into usize");

            if let Some(ref algo_split) = p.algo_split {
                s.preferences = algo_split.alphas.clone();
                s.cuts = algo_split.cuts.clone();
            }
            progress.inc(1);
            p
        })
        .collect();

    progress.finish();

    let outfile_name = format!(
        "splitting_results_{}.json",
        Utc::now().format("%Y-%m-%d_%H:%M:%S").to_string()
    );

    println!("writing results to \"{}\"", outfile_name);

    let mut metrics = [""; EDGE_COST_DIMENSION];

    for key in keys.values() {
        if let AttributeType::Double(idx) = key.attribute_type {
            metrics[idx] = key.name.as_str();
        }
    }

    let outfile = std::fs::File::create(outfile_name)?;
    let mut outfile = std::io::BufWriter::new(outfile);

    let results = Results {
        graph_file,
        trajectory_file,
        metrics,
        results: statistics,
    };

    outfile.write_all(serde_json::to_string_pretty(&results)?.as_bytes())?;
    Ok(())

    // graph.find_preference(&mut path);
    // // server::start_server(graph);
}
