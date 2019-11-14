use crate::graph::Graph;
use std::io::Read;
use std::path::Path;

use roxmltree::Document;

pub fn read_graphml<P: AsRef<Path>>(file_path: P) -> Result<Graph, String> {
    let mut contents = String::new();

    let file = std::fs::File::open(file_path).unwrap();
    let mut file = std::io::BufReader::new(file);

    file.read_to_string(&mut contents).unwrap();

    let doc = Document::parse(&contents).unwrap();

    for d in doc.root().descendants().filter(|n| n.has_tag_name("key")) {
        println!("{}", d.tag_name().name());
    }

    let nodes = vec![];
    let edges = vec![];

    Ok(Graph::new(nodes, edges))
}
