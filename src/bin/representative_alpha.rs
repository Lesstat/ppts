use preference_splitting::graph::dijkstra::Dijkstra;
use preference_splitting::graph::parse_minimal_graph_file;
use preference_splitting::graph::trajectory_analysis::get_linear_combination;
use preference_splitting::helpers::{Costs, Preference};
use preference_splitting::trajectories::{check_trajectory, read_trajectories, Trajectory};
use preference_splitting::{MyError, MyResult, EDGE_COST_DIMENSION};

use std::convert::TryInto;
use std::time::Instant;

use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opts {
    /// Graph file in minimal fmi syntax
    graph_file: String,
    /// Json file containing trajectories
    trajectory_file: String,
}

#[derive(Serialize, Deserialize)]
struct Results {
    graph_file: String,
    trajectory_file: String,
    metrics: Vec<String>,
    results: Vec<AlphaStatistic>,
}

#[derive(Serialize, Deserialize)]
struct AlphaStatistic {
    trip_id: i64,
    vehicle_id: i64,
    trajectory_length: usize,
    alpha: Preference,
    trajectory_cost: Costs,
    alpha_cost: Costs,
    run_time: usize,
}

fn main() -> MyResult<()> {
    let Opts {
        graph_file,
        trajectory_file,
    } = Opts::from_args();

    println!("reading graph file");
    let graph_data = parse_minimal_graph_file(&graph_file)?;

    println!("reading trajectories");
    let trajectories = read_trajectories(&trajectory_file)?;

    println!("checking trajectory consistency");
    if trajectories
        .par_iter()
        .all(|t| check_trajectory(&t, &graph_data.graph, &graph_data.edge_lookup))
    {
        println!("all {} trajectories seem valid :-)", trajectories.len());
    } else {
        println!("There are invalid trajectories :-(");
        return Err(Box::new(MyError::InvalidTrajectories));
    }

    let mut statistics: Vec<_> = trajectories.iter().map(AlphaStatistic::new).collect();

    let progress = ProgressBar::new(trajectories.len().try_into().unwrap());
    progress.set_style(
        ProgressStyle::default_spinner()
            .template(
                "[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} [{eta_precise} remaining]",
            )
            .progress_chars("#>-"),
    );

    trajectories
        .par_iter()
        .zip(statistics.par_iter_mut())
        .map(|(t, s)| {
            let p = t.to_path(&graph_data.graph, &graph_data.edge_lookup);
            s.trajectory_cost = p.total_dimension_costs;
            (p, s)
        })
        .for_each(|(p, s)| {
            let start = Instant::now();
            let mut d = Dijkstra::new(&graph_data.graph);
            let source_id = p.nodes.first().expect("Unexpected empty trajectory");
            let target_id = p.nodes.last().expect("Unexpected empty trajectory");

            let mut cost_vec = Vec::new();
            for i in 0..EDGE_COST_DIMENSION {
                let mut alpha = [0.0; EDGE_COST_DIMENSION];
                alpha[i] = 1.0;

                let path = graph_data.graph.find_shortest_path(
                    &mut d,
                    p.id,
                    &[*source_id, *target_id],
                    alpha,
                );
                cost_vec.push(
                    path.expect("Did not find path for trajectory")
                        .total_dimension_costs,
                );
            }
            let alpha = get_linear_combination(&cost_vec, &p.total_dimension_costs);
            let time = start.elapsed();

            let alpha_path = graph_data
                .graph
                .find_shortest_path(&mut d, p.id, &[*source_id, *target_id], alpha)
                .unwrap();

            s.run_time = time
                .as_millis()
                .try_into()
                .expect("Could not convert runtime into usize");

            s.alpha_cost = alpha_path.total_dimension_costs;
            s.alpha = alpha;
        });

    Ok(())
}

impl AlphaStatistic {
    fn new(t: &Trajectory) -> Self {
        AlphaStatistic {
            trip_id: t.trip_id,
            vehicle_id: t.vehicle_id,
            trajectory_length: t.path.len() + 1,
            trajectory_cost: [0.0; EDGE_COST_DIMENSION],
            alpha: [0.0; EDGE_COST_DIMENSION],
            alpha_cost: [0.0; EDGE_COST_DIMENSION],
            run_time: 0,
        }
    }
}
