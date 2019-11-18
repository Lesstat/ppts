use std::error::Error;

use std::io::Read;
use std::path::Path;

use std::collections::HashMap;
use std::convert::TryInto;

use crate::graph::{Edge, Graph, Node};

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

pub type EdgeLookup = HashMap<String, usize>;

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

pub fn read_graphml<P: AsRef<Path>>(file_path: P) -> Result<(Graph, EdgeLookup), Box<dyn Error>> {
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
                    "id" => id = parse_node_id(text),
                    _ => (),
                }
            }
            Node::new(id, ch_level)
        })
        .collect();

    println!("parsed {} nodes", nodes.len());

    let edge_lookup: EdgeLookup = doc
        .root()
        .descendants()
        .filter(|n| n.has_tag_name("edge"))
        .enumerate()
        .map(|(id, edge)| {
            for d in edge.descendants().filter(|d| d.has_tag_name("data")) {
                let key = d.attribute("key").unwrap();
                if "name" == keys[key].name {
                    return (d.text().unwrap().to_owned(), id);
                }
            }
            panic!("could not find name for edge")
        })
        .collect();

    println!("lookup table size: {}", edge_lookup.len());

    let edges: Vec<Edge> = doc
        .root()
        .descendants()
        .filter(|n| n.has_tag_name("edge"))
        .enumerate()
        .map(|(id, e)| parse_edge_from_xml(id, &e, &keys, &edge_lookup))
        .collect();

    println!("parsed {} edges", edges.len());

    Ok((Graph::new(nodes, edges), edge_lookup))
}

fn parse_edge_from_xml<'a, 'input>(
    id: usize,
    node: &roxmltree::Node<'a, 'input>,
    keys: &HashMap<&str, GraphmlAttribute>,
    edge_lookup: &EdgeLookup,
) -> Edge {
    let mut costs = [0.0; super::EDGE_COST_DIMENSION];

    let source_text = node
        .attribute("source")
        .expect("edge element has no source attribute.");
    let target_text = node
        .attribute("target")
        .expect("edge element has no target attribute.");

    let source = parse_node_id(source_text);
    let target = parse_node_id(target_text);

    let mut edge_a: i64 = -1;
    let mut edge_b: i64 = -1;

    let mut cost_idx = 0;
    for d in node.descendants().filter(|n| n.has_tag_name("data")) {
        let attr = &keys[d
            .attribute("key")
            .expect("data element has no key attribute")];

        let text = if let Some(t) = d.text() {
            t
        } else {
            continue;
        };

        match attr.name {
            "edgeA" => {
                edge_a = if text != "-1" {
                    edge_lookup[text].try_into().unwrap()
                } else {
                    -1
                }
            }
            "edgeB" => {
                edge_b = if text != "-1" {
                    edge_lookup[text].try_into().unwrap()
                } else {
                    -1
                }
            }
            _ => (),
        }

        if let AttributeType::Double = attr.attribute_type {
            costs[cost_idx] = text
                .parse()
                .unwrap_or_else(|t| panic!("could not parse text {} of {}", t, attr.name));
            cost_idx += 1;
        }
    }

    let skips = if edge_a >= 0 && edge_b >= 0 {
        Some((edge_a as usize, edge_b as usize))
    } else {
        None
    };

    Edge::new(id, source, target, costs, skips)
}

fn parse_node_id(node_id: &str) -> usize {
    let tail: String = node_id.chars().skip(1).collect();

    tail.parse().expect("could not parse node id")
}
