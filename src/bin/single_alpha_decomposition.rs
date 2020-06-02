use preference_splitting::graph::dijkstra::Dijkstra;
use preference_splitting::graph::{
    parse_minimal_graph_file, trajectory_analysis::TrajectoryAnalysis,
};

use preference_splitting::graphml::{GraphData};
use preference_splitting::trajectories::{check_trajectory, read_trajectories};
use preference_splitting::{
    lp::{LpProcess, PreferenceEstimator},
    statistics::{RepresentativeAlphaResult, read_representative_results},
    MyError, MyResult,
};

use std::convert::TryInto;

use chrono::prelude::*;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use structopt::StructOpt;

use std::io::{Write};

#[derive(StructOpt)]
struct Opts {
    /// Json file containing results of representative alpha experiments
    repr_results_file: String,
    /// File to write output to
    out_file: Option<String>,
    /// modus: 0 for longest optimal subpath, 1 for greedy, 2 for representative alpha, 3 for all
    #[structopt(short, long, default_value = "0")]
    modus: usize,
    /// Number of threads to use
    #[structopt(short, long, default_value = "8")]
    threads: usize,
}

fn main() -> MyResult<()> {
    let Opts {
        repr_results_file,
        out_file,
        modus,
        threads,
    } = Opts::from_args();

    println!("reading results");
    let mut results = read_representative_results(repr_results_file)?;

    println!("reading graph file: {}", results.graph_file);
    let GraphData {
        graph, edge_lookup, ..
    } = parse_minimal_graph_file(&results.graph_file)?;

    println!("reading trajectories {}", results.trajectory_file);
    let mut trajectories = read_trajectories(&results.trajectory_file)?;

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
    println!("calculating single alpha decomposition");

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
                let mut lp = LpProcess::new().unwrap();
                let mut analyzer = TrajectoryAnalysis::new(&graph, &mut d, &mut lp);
                let mut counter = 0;
                for (p, s) in chunk {
                    if modus == 0 || modus == 3 {
                        let nops = analyzer.find_all_non_optimal_segments(p).unwrap();
                        let mut start = 0 as u32;
                        let mut stop = p.nodes.len() as u32;
                        let mut best_length = 0 as u32;
                        let mut best_start = 0 as u32;
                        let mut best_stop = 0 as u32;
                        for nop in nops.iter() {
                            stop = nop.start_index;
                            if start <= stop && stop - start > best_length {
                                best_length = stop - start;
                                best_start = start;
                                best_stop = stop;
                            }
                            start = nop.end_index + 1;
                        }
                        if start <= stop && stop - start > best_length {
                            best_start = start;
                            best_stop = stop;
                        }
                        let mut optimal_subpath_constraint = Vec::new();
                        if best_stop - best_start > 0 {
                            let optimal_subpath = p.get_subpath(&graph, best_start, best_stop);
                            optimal_subpath_constraint.push(optimal_subpath);
                        }
                        let decomposition = analyzer.get_single_preference_decomposition(&optimal_subpath_constraint, &p).unwrap();
                        if decomposition.preference[0] >= 0.0 {
                            let mut cuts_vec = Vec::new();
                            for i in 0..decomposition.cuts.len(){
                                cuts_vec.push(decomposition.cuts[i] as usize);
                            }
                            s.single_preference_decomposition_longest_optimal_subpath = Some(cuts_vec);
                        }else{
                            s.single_preference_decomposition_longest_optimal_subpath = None;
                        }
                        
                    }
                    if modus == 1 || modus == 3 {
                        let empty_constraints = Vec::new();
                        let decomposition = analyzer.get_single_preference_decomposition(&empty_constraints, &p).unwrap();
                        if decomposition.preference[0] >= 0.0 {
                            let mut cuts_vec = Vec::new();
                            for i in 0..decomposition.cuts.len(){
                                cuts_vec.push(decomposition.cuts[i] as usize);
                            }
                            s.single_preference_decomposition_greedy = Some(cuts_vec);
                        }else{
                            s.single_preference_decomposition_greedy = None;
                        }
                    }
                    if modus == 2 || modus == 3 {
                        let mut lp2 = LpProcess::new().unwrap();
                        let mut estimator = PreferenceEstimator::new(&graph, &mut lp2);
                        let mut d2 = Dijkstra::new(&graph);
                        let representative_pref = estimator.calc_representative_preference(
                            &mut d2,
                            &p,
                        ).unwrap();
                        let decomposition = analyzer.get_single_preference_decomposition_for_given_preference(representative_pref, &p).unwrap();
                        if decomposition.preference[0] >= 0.0 {
                            let mut cuts_vec = Vec::new();
                            for i in 0..decomposition.cuts.len(){
                                cuts_vec.push(decomposition.cuts[i] as usize);
                            }
                            s.single_preference_decomposition_representative_pref = Some(cuts_vec);
                        }else{
                            s.single_preference_decomposition_representative_pref = None;
                        }
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
    //DEBUG
    println!("finished");
    //DEBUG END

    let outfile_name =
        out_file.unwrap_or_else(|| format!("single_alpha_decomposition_results_{}.json", start_time));

    println!("writing results to {}", outfile_name);

    let outfile = std::fs::File::create(outfile_name)?;
    let mut outfile = std::io::BufWriter::new(outfile);

    results.start_time = start_time;

    outfile.write_all(serde_json::to_string_pretty(&results)?.as_bytes())?;
    Ok(())
}
