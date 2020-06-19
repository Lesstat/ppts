use preference_splitting::graph::dijkstra::Dijkstra;
use preference_splitting::graph::{
    parse_minimal_graph_file, trajectory_analysis::evaluations::overlap, path::Path
};

use preference_splitting::graphml::{AttributeType, GraphData};
use preference_splitting::trajectories::{read_trajectories};
use preference_splitting::{
    helpers::{costs_by_alpha, Preference},
    lp::{LpProcess, PreferenceEstimator},
    statistics::{ExperimentResults, RepresentativeAlphaResult},
    MyResult, EDGE_COST_DIMENSION,
};

use std::convert::TryInto;
use std::{io::Write, time::Instant};

use chrono::prelude::*;
use indicatif::{ProgressBar, ProgressStyle};
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
    let trajectories = read_trajectories(&trajectory_file)?;

    if trajectories.len() == 0 {
        return Ok(());
    }

    let mut max_vehicle_id = 0;
    let mut min_vehicle_id = 999999;
    for t in &trajectories {
        if t.vehicle_id > max_vehicle_id {
            max_vehicle_id = t.vehicle_id;
        }
        if t.vehicle_id < min_vehicle_id {
            min_vehicle_id = t.vehicle_id;
        }
    }
    let num_vehicles = max_vehicle_id - min_vehicle_id + 1;
    let mut paths_per_vehicle_id : Vec<Vec<Path>> = Vec::new();
    let mut preference_per_vehicle: Vec<Preference> = Vec::new();
    let mut all_paths : Vec<Path> = Vec::new();
    let mut statistics : Vec<RepresentativeAlphaResult> = Vec::new();
    for _ in 0..num_vehicles {
        paths_per_vehicle_id.push(Vec::new());
        preference_per_vehicle.push([0.0; 4]);
    }
    for t in &trajectories {
        let index = (t.vehicle_id - min_vehicle_id) as usize;
        let path = t.to_path(&graph, &edge_lookup);
        paths_per_vehicle_id[index].push(path.clone());
        all_paths.push(path);
        statistics.push(RepresentativeAlphaResult::new(&t));
    }

    let progress = ProgressBar::new(paths_per_vehicle_id.len().try_into().unwrap());
    progress.set_style(
        ProgressStyle::default_spinner()
            .template(
                "[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} [{eta_precise} remaining]",
            )
            .progress_chars("#>-"),
    );

    let start_time = Utc::now().format("%Y-%m-%d_%H:%M:%S").to_string();
    println!("finiding representatitve alphas");

    let mut paths_with_prefs = paths_per_vehicle_id
        .into_iter()
        .zip(preference_per_vehicle.iter_mut())
        .collect::<Vec<_>>();

    

    let items_per_thread = paths_with_prefs.len() / threads;

    #[allow(clippy::explicit_counter_loop)]
    crossbeam::scope(|scope| {
        for chunk in paths_with_prefs.chunks_mut(items_per_thread) {
            (scope.spawn(|_| {
                let mut d = Dijkstra::new(&graph);
                let mut lp = LpProcess::new().unwrap();
                let mut estimator = PreferenceEstimator::new(&graph, &mut lp);
                for (paths, pref) in chunk {
                    **pref = estimator
                        .calc_representative_preference_for_multiple_paths(&mut d, paths)
                        .expect("an error occured");
                    progress.inc(1);
                }
            }));
        }
    })
    .unwrap();
    progress.finish();

    for i in 0..statistics.len() {
        statistics[i].preference = preference_per_vehicle[(statistics[i].vehicle_id-min_vehicle_id) as usize];
    }

    let mut paths = all_paths
        .into_iter()
        .zip(statistics.iter_mut())
        .collect::<Vec<_>>();

    let items_per_thread_2 = paths.len() / threads;

    #[allow(clippy::explicit_counter_loop)]
    crossbeam::scope(|scope| {
        for chunk in paths.chunks_mut(items_per_thread_2) {
            (scope.spawn(|_| {
                let mut d = Dijkstra::new(&graph);
                for (p, s) in chunk {
                    let start = Instant::now();
                    let time = start.elapsed();
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
                            s.preference,
                        )
                        .expect("there must be a path");
                    s.alpha_cost = alpha_path.total_dimension_costs;
                    s.aggregated_cost_diff = costs_by_alpha(&p.total_dimension_costs, &s.preference)
                        - costs_by_alpha(&alpha_path.total_dimension_costs, &s.preference);
                    s.overlap = overlap(p, &alpha_path);
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
