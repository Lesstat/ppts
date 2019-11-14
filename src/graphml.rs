use std::error::Error;
use std::io::Read;
use std::path::Path;

use std::collections::HashMap;

use crate::graph::{Graph, Node};

use roxmltree::Document;

enum GraphObject {
    Node,
    Edge,
}

enum AttributeType {
    Boolean,
    String,
    Double,
    Long,
}

struct GraphmlAttribute<'a> {
    obj_type: GraphObject,
    name: &'a str,
    attribute_type: AttributeType,
}

impl<'a> GraphmlAttribute<'a> {
    fn new(obj_type: &'a str, name: &'a str, attribute_type: &'a str) -> GraphmlAttribute<'a> {
        let obj_type = match obj_type {
            "node" => GraphObject::Node,
            "edge" => GraphObject::Edge,
            _ => panic!("unknown graph object type"),
        };

        let attribute_type = match attribute_type {
            "boolean" => AttributeType::Boolean,
            "string" => AttributeType::String,
            "long" => AttributeType::Long,
            "double" => AttributeType::Double,
            _ => panic!("unkown attribute type"),
        };

        GraphmlAttribute {
            obj_type,
            name,
            attribute_type,
        }
    }
}

pub fn read_graphml<P: AsRef<Path>>(file_path: P) -> Result<Graph, Box<dyn Error>> {
    let mut contents = String::new();

    let file = std::fs::File::open(file_path)?;
    let mut file = std::io::BufReader::new(file);

    file.read_to_string(&mut contents)?;

    let doc = Document::parse(&contents)?;

    let keys: HashMap<&str, GraphmlAttribute> = doc
        .root()
        .descendants()
        .filter(|n| n.has_tag_name("key"))
        .map(|d| {
            let obj_type = d
                .attribute("for")
                .expect("No 'for' attribute on key element");
            let name = d
                .attribute("attr.name")
                .expect("No 'attr.name' attribute on key element");
            let attr_type = d
                .attribute("attr.type")
                .expect("No 'attr.type' attribute on key element");
            let id = d.attribute("id").expect("No 'id' attribute on key element");
            (id, GraphmlAttribute::new(obj_type, name, attr_type))
        })
        .collect();

    let nodes: Vec<Node> = doc
        .root()
        .descendants()
        .filter(|n| n.has_tag_name("node"))
        .map(|n| {
            let mut id = 0;
            let mut ch_level = 0;
            for d in n.descendants().filter(|n| n.has_tag_name("data")) {
                let key = d
                    .attribute("key")
                    .expect("data element has no key attribute.");
                let text = d.text().expect("data element has no text.");

                let attribute = &keys[key];

                match attribute.name {
                    "level" => ch_level = text.parse().expect("Could not parse ch level"),
                    "name" => id = text.parse().expect("could not parse node name"),
                    _ => (),
                }
            }
            Node::new(id, ch_level)
        })
        .collect();

    println!("prased {} nodes", nodes.len());
    println!(
        "max level was {}",
        nodes.iter().map(|n| n.ch_level).max().unwrap()
    );
    let edges = vec![];

    Ok(Graph::new(nodes, edges))
}
