use std::convert::TryInto;
use std::io::Write;
use std::time::Instant;

use preference_splitting::graph::path::Path;
use preference_splitting::graph::trajectory_analysis::TrajectoryAnalysis;
use preference_splitting::graph::{parse_minimal_graph_file, Graph};
use preference_splitting::graphml::{read_graphml, AttributeType, GraphData};
use preference_splitting::helpers::MyVec;
use preference_splitting::statistics::{
    NonOptSubPathsResult, SplittingResults, SplittingStatistics,
};
use preference_splitting::trajectories::{check_trajectory, read_trajectories};
use preference_splitting::{MyError, EDGE_COST_DIMENSION};

use chrono::prelude::*;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opts {
    #[structopt(short = "g", long = "graphml")]
    graphml: bool,
    graph_file: String,
    trajectory_file: String,
}

fn run_experiment(graph: &Graph, p: &mut Path, s: &mut SplittingStatistics) {
fn run_experiment<'a, 'b>(
    graph: &'a Graph,
    d: &'b mut Dijkstra<'a>,
    p: &mut Path,
    s: &mut SplittingStatistics,
) {
    let start = Instant::now();
    let mut ta = TrajectoryAnalysis::new(&graph, d);
    ta.find_preference(p);
    let time = start.elapsed();
    s.splitting_run_time = time
        .as_millis()
        .try_into()
        .expect("Couldn't convert run time into usize");

    if let Some(ref algo_split) = p.algo_split {
        s.preferences = algo_split.alphas.clone();
        s.cuts = algo_split.cuts.clone();

        let start = Instant::now();

        let ta = TrajectoryAnalysis::new(&graph);
        let subpaths = ta.find_non_optimal_segments(p);
        let time = start.elapsed();
        let non_opt_subpaths = NonOptSubPathsResult {
            non_opt_subpaths: MyVec::<_>::from(
                subpaths
                    .iter()
                    .map(|s| (s.start_index, s.end_index))
                    .collect::<Vec<_>>(),
            ),
            runtime: time
                .as_millis()
                .try_into()
                .expect("Couldn't convert run time into usize"),
        };

        s.non_opt_subpaths = Some(non_opt_subpaths);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Opts {
        graphml,
        graph_file,
        trajectory_file,
    } = Opts::from_args();

    let GraphData {
        graph,
        edge_lookup,
        keys,
    } = if graphml {
        read_graphml(&graph_file)?
    } else {
        parse_minimal_graph_file(&graph_file)?
    };

    let mut trajectories = read_trajectories(&trajectory_file)?;

    let mut statistics: Vec<_> = trajectories.iter().map(SplittingStatistics::new).collect();

    trajectories
        .iter_mut()
        .zip(statistics.iter_mut())
        .for_each(|(t, s)| {
            s.removed_self_loop_indices = t.filter_out_self_loops(&graph, &edge_lookup);
        });

    if trajectories
        .iter()
        .all(|t| check_trajectory(t, &graph, &edge_lookup))
    {
        println!("all {} trajectories seem valid :-)", trajectories.len());
    } else {
        println!("There are invalid trajectories :-(");
        return Err(Box::new(MyError::InvalidTrajectories));
    }

    let progress = ProgressBar::new(trajectories.len().try_into().unwrap());
    progress.set_style(
        ProgressStyle::default_spinner()
            .template(
                "[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} [{eta_precise} remaining]",
            )
            .progress_chars("#>-"),
    );

    progress.set_draw_delta((trajectories.len().min(100)).try_into().unwrap());

    trajectories
        .par_iter()
        .zip(statistics.par_iter_mut())
        .map(|(t, s)| (t.to_path(&graph, &edge_lookup), s))
        .for_each(|(mut p, mut s)| {
            run_experiment(&graph, &mut p, &mut s);
            progress.inc(1);
        });

    progress.finish();

    let outfile_name = format!(
        "splitting_results_{}.json",
        Utc::now().format("%Y-%m-%d_%H:%M:%S").to_string()
    );

    println!("writing results to \"{}\"", outfile_name);

    let mut metrics = vec!["".to_owned(); EDGE_COST_DIMENSION];

    for key in keys.values() {
        if let AttributeType::Double(idx) = key.attribute_type {
            metrics[idx] = key.name.clone();
        }
    }

    let outfile = std::fs::File::create(outfile_name)?;
    let mut outfile = std::io::BufWriter::new(outfile);

    let results = SplittingResults {
        graph_file,
        trajectory_file,
        metrics,
        results: statistics,
    };

    outfile.write_all(serde_json::to_string_pretty(&results)?.as_bytes())?;
    Ok(())
}
