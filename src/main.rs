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
use statistics::SplittingStatistics;

use chrono::prelude::*;
use indicatif::{ProgressBar, ProgressStyle};

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
            .template("[{elapsed}] {bar:40.cyan/blue} {pos}/{len}")
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

    let outfile = std::fs::File::create(outfile_name)?;
    let mut outfile = std::io::BufWriter::new(outfile);

    outfile.write_all(serde_json::to_string_pretty(&statistics)?.as_bytes())?;
    Ok(())

    // graph.find_preference(&mut path);
    // // server::start_server(graph);
}
