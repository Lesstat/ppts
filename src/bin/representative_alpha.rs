use preference_splitting::graph::dijkstra::Dijkstra;
use preference_splitting::graph::{
    parse_minimal_graph_file, trajectory_analysis::evaluations::overlap,
};

use preference_splitting::graphml::{AttributeType, GraphData};
use preference_splitting::trajectories::{check_trajectory, read_trajectories};
use preference_splitting::{
    helpers::costs_by_alpha,
    lp::{LpProcess, PreferenceEstimator},
    statistics::{ExperimentResults, RepresentativeAlphaResult},
    MyError, MyResult, EDGE_COST_DIMENSION,
};

use std::convert::TryInto;
use std::{io::Write, time::Instant};

use chrono::prelude::*;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opts {
    /// Graph file in minimal fmi syntax
    graph_file: String,
    /// Json file containing trajectories
    trajectory_file: String,
    /// File to write output to
    out_file: Option<String>,
    /// Number of threads to use
    #[structopt(short, long, default_value = "8")]
    threads: usize,
}

fn main() -> MyResult<()> {
    let Opts {
        graph_file,
        trajectory_file,
        out_file,
        threads,
    } = Opts::from_args();

    println!("reading graph file");
    let GraphData {
        graph,
        edge_lookup,
        keys,
    } = parse_minimal_graph_file(&graph_file)?;

    println!("reading trajectories");
    let mut trajectories = read_trajectories(&trajectory_file)?;

    let mut statistics: Vec<RepresentativeAlphaResult> = trajectories
        .iter()
        .map(RepresentativeAlphaResult::new)
        .collect();

    trajectories
        .iter_mut()
        .zip(statistics.iter_mut())
        .for_each(|(t, s)| {
            s.removed_self_loop_indices = t.filter_out_self_loops(&graph, &edge_lookup);
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
    println!("finiding representatitve alphas");

    let mut paths = trajectories
        .into_iter()
        .map(|t| t.to_path(&graph, &edge_lookup))
        .zip(statistics.iter_mut())
        .collect::<Vec<_>>();

    let items_per_thread = paths.len() / threads;

    #[allow(clippy::explicit_counter_loop)]
    crossbeam::scope(|scope| {
        for chunk in paths.chunks_mut(items_per_thread) {
            (scope.spawn(|_| {
                let mut d = Dijkstra::new(&graph);
                let mut lp = LpProcess::new().unwrap();
                let mut estimator = PreferenceEstimator::new(&graph, &mut lp);
                let mut counter = 0;
                for (p, s) in chunk {
                    let start = Instant::now();
                    let pref = estimator
                        .calc_representative_preference(&mut d, p)
                        .expect("an error occured");
                    let time = start.elapsed();

                    s.preference = pref;
                    s.run_time = time
                        .as_millis()
                        .try_into()
                        .expect("Couldn't convert run time into usize");

                    s.trajectory_cost = p.total_dimension_costs;

                    let alpha_path = graph
                        .find_shortest_path(
                            &mut d,
                            0,
                            &[*p.nodes.first().unwrap(), *p.nodes.last().unwrap()],
                            pref,
                        )
                        .expect("there must be a path");
                    s.alpha_cost = alpha_path.total_dimension_costs;
                    s.aggregated_cost_diff = costs_by_alpha(&p.total_dimension_costs, &pref)
                        - costs_by_alpha(&alpha_path.total_dimension_costs, &pref);
                    s.overlap = overlap(p, &alpha_path);

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
        out_file.unwrap_or_else(|| format!("representative_alpha_results_{}.json", start_time));

    println!("writing results to \"{}\"", outfile_name);

    let mut metrics = vec!["".to_owned(); EDGE_COST_DIMENSION];

    for key in keys.values() {
        if let AttributeType::Double(idx) = key.attribute_type {
            metrics[idx] = key.name.clone();
        }
    }

    let outfile = std::fs::File::create(outfile_name)?;
    let mut outfile = std::io::BufWriter::new(outfile);

    let results = ExperimentResults {
        graph_file,
        trajectory_file,
        metrics,
        start_time,
        results: statistics,
    };

    outfile.write_all(serde_json::to_string_pretty(&results)?.as_bytes())?;
    Ok(())
}
