use preference_splitting::graphml::{read_graphml, AttributeType};
use preference_splitting::EDGE_COST_DIMENSION;

use std::error::Error;
use std::io::{BufWriter, Write};

use structopt::StructOpt;

#[derive(StructOpt)]
struct Opts {
    graphml_file: String,
    fmi_file: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opts = Opts::from_args();

    let graph_data = read_graphml(&opts.graphml_file)?;

    let file = std::fs::File::create(opts.fmi_file)?;
    let mut writer = BufWriter::new(file);

    writer.write_fmt(format_args!("# original file: {}\n", opts.graphml_file))?;
    writer.write_fmt(format_args!("# Node Attributes: ID CH-Level\n"))?;
    writer.write_fmt(format_args!(
        "# Edge Attributes: ID source-id target-id {}-metrics replaced-edge1 replaced-edge2\n",
        EDGE_COST_DIMENSION
    ))?;
    writer.write_fmt(format_args!("# Cost metrics:"))?;
    let mut metrics = [""; EDGE_COST_DIMENSION];

    graph_data
        .keys
        .values()
        .filter_map(|key| {
            if let AttributeType::Double(index) = key.attribute_type {
                Some((index, &key.name))
            } else {
                None
            }
        })
        .for_each(|(i, name)| metrics[i] = name);
    for metric in metrics.iter() {
        writer.write_fmt(format_args!(" {}", metric))?;
    }

    writer.write_fmt(format_args!("\n\n"))?;
    writer.write_fmt(format_args!("{}\n", EDGE_COST_DIMENSION))?;
    writer.write_fmt(format_args!("{}\n", graph_data.graph.nodes.len()))?;
    writer.write_fmt(format_args!("{}\n", graph_data.graph.edges.len()))?;

    for n in graph_data.graph.nodes.iter() {
        let ch_level = if n.ch_level > 0 {
            n.ch_level as i32
        } else {
            -1
        };
        writer.write_fmt(format_args!("{} {}\n", n.id, ch_level))?;
    }

    for e in graph_data.graph.edges.iter() {
        writer.write_fmt(format_args!("{} {} {}", e.id, e.source_id, e.target_id))?;
        for c in e.edge_costs.iter() {
            writer.write_fmt(format_args!(" {}", c))?;
        }
        if let Some((edge_a, edge_b)) = e.replaced_edges {
            writer.write_fmt(format_args!(" {} {}\n", edge_a, edge_b))?;
        } else {
            writer.write_fmt(format_args!(" -1 -1\n"))?;
        }
    }

    Ok(())
}
