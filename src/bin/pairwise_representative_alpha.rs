use preference_splitting::graph::dijkstra::Dijkstra;
use preference_splitting::graph::{
    parse_minimal_graph_file, trajectory_analysis::evaluations::overlap
};

use preference_splitting::graphml::{AttributeType, GraphData};
use preference_splitting::trajectories::{read_trajectories};
use preference_splitting::{
    helpers::{costs_by_alpha},
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
    /// Vehicle id
    #[structopt(short, long, default_value = "1")]
    vehicle_id: i64,
    /// Number of threads to use
    #[structopt(short, long, default_value = "8")]
    threads: usize,
}

fn main() -> MyResult<()> {
    let Opts {
        graph_file,
        trajectory_file,
        out_file,
        vehicle_id,
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

    let mut trajectories_of_vehicle = Vec::new();
    let mut paths_of_vehicle = Vec::new();
    for t in &trajectories{
        if t.vehicle_id == vehicle_id {
            trajectories_of_vehicle.push(t);
            paths_of_vehicle.push(t.to_path(&graph, &edge_lookup));
        }
    }
    let mut statistics : Vec<RepresentativeAlphaResult> = Vec::new();
    let mut path_pairs = Vec::new();
    for i in 0..trajectories_of_vehicle.len()-1{
        for j in (i+1)..trajectories_of_vehicle.len(){
            let mut statistic = RepresentativeAlphaResult::new(&trajectories_of_vehicle[i]);
            if statistic.trip_id.len() > 1 {
                statistic.trip_id[1] = trajectories_of_vehicle[j].trip_id[0];
            }
            else{
                statistic.trip_id.push(trajectories_of_vehicle[j].trip_id[0]);
            }
            let mut pair = Vec::new();
            pair.push(paths_of_vehicle[i].to_owned());
            pair.push(paths_of_vehicle[j].to_owned());
            path_pairs.push(pair);
            statistics.push(statistic);
        }
    }

    let progress = ProgressBar::new(path_pairs.len().try_into().unwrap());
    progress.set_style(
        ProgressStyle::default_spinner()
            .template(
                "[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} [{eta_precise} remaining]",
            )
            .progress_chars("#>-"),
    );

    let start_time = Utc::now().format("%Y-%m-%d_%H:%M:%S").to_string();
    println!("finiding representatitve alphas");

    let mut paths_with_statistics = path_pairs
        .into_iter()
        .zip(statistics.iter_mut())
        .collect::<Vec<_>>();

    

    let items_per_thread = paths_with_statistics.len() / threads;

    #[allow(clippy::explicit_counter_loop)]
    crossbeam::scope(|scope| {
        for chunk in paths_with_statistics.chunks_mut(items_per_thread) {
            (scope.spawn(|_| {
                let mut d = Dijkstra::new(&graph);
                let mut lp = LpProcess::new().unwrap();
                let mut estimator = PreferenceEstimator::new(&graph, &mut lp);
                let mut counter = 0;
                for (paths, s) in chunk {
                    let start = Instant::now();
                    let preference = estimator
                        .calc_representative_preference_for_multiple_paths(&mut d, paths)
                        .expect("an error occured");
                    let time = start.elapsed();
                    s.run_time = time
                        .as_millis()
                        .try_into()
                        .expect("Couldn't convert run time into usize");
                    s.preference = preference;
                    let mut alpha_costs = Vec::new();
                    let mut aggregated_cost_diffs = Vec::new();
                    let mut overlaps = Vec::new();
                    let mut trajectory_costs = Vec::new();
                    for p in paths{
                        let alpha_path = graph.find_shortest_path(
                               &mut d,
                               0,
                               &[*p.nodes.first().unwrap(), *p.nodes.last().unwrap()],
                               s.preference,
                            )
                            .expect("there must be a path");
                        
                        alpha_costs.push(alpha_path.total_dimension_costs);
                        aggregated_cost_diffs.push(costs_by_alpha(&p.total_dimension_costs, &s.preference)
                            - costs_by_alpha(&alpha_path.total_dimension_costs, &s.preference));
                        overlaps.push(overlap(p, &alpha_path));
                        trajectory_costs.push(p.total_dimension_costs);
                    }
                    s.alpha_costs = Some(alpha_costs);
                    s.aggregated_cost_diffs = Some(aggregated_cost_diffs);
                    s.overlaps= Some(overlaps);
                    s.trajectory_costs = Some(trajectory_costs);
                    counter = counter + 1;
                    if counter % 100 == 0 {
                        progress.inc(100);
                    }
                }
            }));
        }
    })
    .unwrap();
    progress.finish();

    let outfile_name =
        out_file.unwrap_or_else(|| format!("representative_alpha_pairwise_results_{}.json", start_time));

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
