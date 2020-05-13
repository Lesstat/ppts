use preference_splitting::{
    graph::parse_minimal_graph_file, graphml::read_graphml,
    trajectories::create_randomwalk_trajectory, MyResult,
};

use rand::{
    distributions::{Distribution, Uniform},
    thread_rng,
};
use std::{collections::HashMap, fs::File, io::BufWriter};
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opts {
    /// Graph file to use
    graph_file: String,
    /// Number of walks to create
    #[structopt(default_value = "1000")]
    routes: u32,
    /// File should be read as graphml
    #[structopt(long = "graphml")]
    graphml_format: bool,
    /// File to save the created random walks into
    walks_file: String,
}
fn main() -> MyResult<()> {
    let Opts {
        graph_file,
        routes,
        graphml_format,
        walks_file,
    } = Opts::from_args();

    let graph_data = if graphml_format {
        read_graphml(&graph_file)?
    } else {
        parse_minimal_graph_file(&graph_file)?
    };

    let mut rng = thread_rng();

    let dist = Uniform::new(0, graph_data.graph.nodes.len() as u32);

    let pairs: Vec<_> = (0..routes)
        .map(|_| (dist.sample(&mut rng), dist.sample(&mut rng)))
        .collect();

    let mut d = preference_splitting::graph::dijkstra::Dijkstra::new(&graph_data.graph);

    let mut walks: Vec<_> = pairs
        .iter()
        .enumerate()
        .filter_map(|(i, pair)| {
            let mut tra =
                create_randomwalk_trajectory(pair.0, pair.1, &graph_data.graph, &mut d, &mut rng);

            if let Some(ref mut tra) = tra {
                tra.trip_id = -(i as i64);
            }

            println!("finished {} walks", i);
            tra
        })
        .collect();

    println!("created {} walks", walks.len());
    println!("Mapping internal to external edge ids");

    let reverse_lookup: HashMap<_, _> =
        graph_data.edge_lookup.iter().map(|(k, v)| (v, k)).collect();

    for w in &mut walks {
        w.path.iter_mut().for_each(|internal| {
            *internal = reverse_lookup[&(*internal as u32)]
                .parse()
                .unwrap_or_else(|_| panic!("could not map {} back to exteranl id", internal))
        })
    }

    let writer = BufWriter::new(File::create(walks_file)?);

    serde_json::to_writer(writer, &walks)?;

    Ok(())
}
