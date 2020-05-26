use preference_splitting::graph::dijkstra::Dijkstra;
use preference_splitting::graph::{
    parse_minimal_graph_file, trajectory_analysis::evaluations::overlap,
};
use rand::thread_rng;

use preference_splitting::graphml::GraphData;

use preference_splitting::trajectories::{check_trajectory, read_trajectories};
use preference_splitting::{
    helpers::randomized_preference, statistics::read_representative_results, MyError, MyResult,
};

use std::convert::TryInto;
use std::io::Write;

use chrono::prelude::*;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opts {
    /// Json file containing results of representative alpha experiments
    repr_results_file: String,
    /// File to write output to
    out_file: Option<String>,
    /// Number of threads to use
    #[structopt(short, long, default_value = "8")]
    threads: usize,
}

fn main() -> MyResult<()> {
    let Opts {
        repr_results_file,
        out_file,
        threads,
    } = Opts::from_args();

    let mut results = read_representative_results(repr_results_file)?;

    println!("reading graph file: {}", results.graph_file);
    let GraphData {
        graph, edge_lookup, ..
    } = parse_minimal_graph_file(&results.graph_file)?;

    println!("reading trajectories {}", results.trajectory_file);
    let mut trajectories = read_trajectories(&results.trajectory_file)?;

    trajectories.iter_mut().for_each(|t| {
        t.filter_out_self_loops(&graph, &edge_lookup);
    });

    println!("checking trajectory consistency");
    if trajectories
        .par_iter()
        .all(|t| check_trajectory(&t, &graph, &edge_lookup))
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

    let start_time = Utc::now().format("%Y-%m-%d_%H:%M:%S").to_string();
    println!("checking for better overlap with random alphas");

    let mut paths = trajectories
        .into_iter()
        .map(|t| t.to_path(&graph, &edge_lookup))
        .zip(results.results.iter_mut())
        .collect::<Vec<_>>();

    let items_per_thread = paths.len() / threads;

    #[allow(clippy::explicit_counter_loop)]
    crossbeam::scope(|scope| {
        for chunk in paths.chunks_mut(items_per_thread) {
            (scope.spawn(|_| {
                let mut d = Dijkstra::new(&graph);
                let mut rng = thread_rng();
                let mut counter = 0;
                for (p, s) in chunk {
                    if s.overlap >= 1.0 {
                        s.better_overlap_by_rng = Some(0);
                        continue;
                    }
                    let ids = [*p.nodes.first().unwrap(), *p.nodes.last().unwrap()];
                    let mut better = 0;
                    for _ in 0..100 {
                        let rand_pref = randomized_preference(&mut rng);

                        let alpha_path = graph
                            .find_shortest_path(&mut d, 0, &ids, rand_pref)
                            .expect("there must be a path");

                        if overlap(p, &alpha_path) > s.overlap {
                            better += 1;
                        }
                    }
                    s.better_overlap_by_rng = Some(better);

                    if counter % 10 == 0 {
                        progress.inc(10);
                    }
                    counter += 1;
                }
            }));
        }
    })
    .unwrap();
    progress.finish();

    let outfile_name =
        out_file.unwrap_or_else(|| format!("overlap_test_results_{}.json", start_time));

    println!("writing results to {}", outfile_name);

    let outfile = std::fs::File::create(outfile_name)?;
    let mut outfile = std::io::BufWriter::new(outfile);

    results.start_time = start_time;

    outfile.write_all(serde_json::to_string_pretty(&results)?.as_bytes())?;
    Ok(())
}
