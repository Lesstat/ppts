use preference_splitting::graph::dijkstra::{find_path, Dijkstra};
use preference_splitting::graph::parse_minimal_graph_file;
use preference_splitting::graphml::read_graphml;
use preference_splitting::helpers::randomized_preference;

use std::error::Error;
use std::time::{Duration, Instant};

use rand::distributions::{Distribution, Uniform};
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opts {
    /// Graph file to use
    graph_file: String,
    /// Number of routes to measure
    #[structopt(default_value = "1000")]
    routes: u32,
    /// File should be read as graphml
    #[structopt(long = "graphml")]
    graphml_format: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opts = Opts::from_args();

    let graph_data = if opts.graphml_format {
        read_graphml(&opts.graph_file)?
    } else {
        parse_minimal_graph_file(&opts.graph_file)?
    };

    let graph = graph_data.graph;

    let node_dstribution = Uniform::new(0, graph.nodes.len() as u32);
    let mut rng = rand::thread_rng();

    let mut d = Dijkstra::new(&graph);

    let mut whole_time = Duration::new(0, 0);

    let mut found_routes = 0;
    let mut missed_routes = 0;

    for _ in 0..opts.routes {
        let source_id = node_dstribution.sample(&mut rng);
        let dest_id = node_dstribution.sample(&mut rng);

        let alpha = randomized_preference(&mut rng);

        let now = Instant::now();
        let path = find_path(&mut d, &[source_id, dest_id], alpha);
        let elapsed = now.elapsed();

        match path {
            Some(_) => found_routes += 1,
            None => missed_routes += 1,
        };
        whole_time += elapsed;
    }

    println!(
        "Did {} Dijkstra runs in {}s ",
        opts.routes,
        whole_time.as_secs()
    );
    println!(
        "found {} routes while no path for {} s-t-pairs could be found",
        found_routes, missed_routes
    );

    let average_duration = whole_time.as_secs_f64() / opts.routes as f64;
    println!("Average time per Dijkstra run is {}s", average_duration);

    Ok(())
}
