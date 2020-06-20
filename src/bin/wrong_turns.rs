use preference_splitting::graph::dijkstra::Dijkstra;
use preference_splitting::graph::{parse_minimal_graph_file, path::Path, Graph};

use preference_splitting::graphml::GraphData;
use preference_splitting::helpers::Preference;
use preference_splitting::trajectories::{check_trajectory, read_trajectories};
use preference_splitting::{statistics::read_representative_results, MyError, MyResult, helpers::randomized_preference,EDGE_COST_DIMENSION};

use rand::thread_rng;

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
    /// Set number of random preferences for comparison (if > 0 only number of wrong turns will be saved)
    #[structopt(short, long, default_value = "0")]
    compare_with_rng: usize,
    /// Number of threads to use
    #[structopt(short, long, default_value = "8")]
    threads: usize,
}

fn main() -> MyResult<()> {
    let Opts {
        repr_results_file,
        out_file,
        compare_with_rng,
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

    let threads = paths.len().min(threads);

    let items_per_thread = paths.len() / threads;

    #[allow(clippy::explicit_counter_loop)]
    crossbeam::scope(|scope| {
        for chunk in paths.chunks_mut(items_per_thread) {
            (scope.spawn(|_| {
                let mut d = Dijkstra::new(&graph);
                let mut counter = 0;
                let mut rng = thread_rng();
                for (p, s) in chunk {
                    //travel time only
                    let mut tt_preference = [0.0; EDGE_COST_DIMENSION];
                    tt_preference[0] = 1.0;
                    s.nr_of_wrong_turns_by_tt = Some(run_experiment(&graph, &mut d, p, &tt_preference).len());
                    if compare_with_rng == 0{
                        s.wrong_turns = Some(run_experiment(&graph, &mut d, p, &s.preference));
                    }
                    else{
                        let mut wrong_turns_by_rng = Vec::new();
                        s.nr_of_wrong_turns = Some(run_experiment(&graph, &mut d, p, &s.preference).len());
                        for _ in 0..compare_with_rng {
                            let rand_pref = randomized_preference(&mut rng);
                            wrong_turns_by_rng.push(run_experiment(&graph, &mut d, p, &rand_pref).len());
                        }
                        s.nr_of_wrong_turns_by_rng = Some(wrong_turns_by_rng);
                    }
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
        out_file.unwrap_or_else(|| format!("wrong_turns_results_{}.json", start_time));

    println!("writing results to \"{}\"", outfile_name);

    let outfile = std::fs::File::create(outfile_name)?;
    let mut outfile = std::io::BufWriter::new(outfile);

    results.start_time = start_time;

    outfile.write_all(serde_json::to_string_pretty(&results)?.as_bytes())?;
    Ok(())
}

fn run_experiment(g: &Graph, d: &mut Dijkstra, p: &Path, alpha: &Preference) -> Vec<usize> {
    let mut wrong_turns = vec![];

    let mut cur_node = *p.nodes.first().unwrap();
    let mut cur_index: usize = 0;
    let last_node = *p.nodes.last().unwrap();

    while cur_node != last_node {
        let path = g
            .find_shortest_path(d, 0, &[cur_node, last_node], *alpha)
            .expect("There must be a path");

        let identical_edges = p.edges.0[cur_index..]
            .iter()
            .zip(path.edges.0)
            .take_while(|(t, o)| *t == o)
            .count();

        cur_index += identical_edges + 1;
        if cur_index == p.nodes.len() {
            break;
        }
        cur_node = p.nodes[cur_index];
        wrong_turns.push(cur_index - 1);
    }

    wrong_turns
}
