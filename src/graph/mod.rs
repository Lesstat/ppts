pub use edge::Edge;
use edge::HalfEdge;

use dijkstra::Dijkstra;
pub use node::Node;
use path::{Path, PathSplit};

use crate::graphml::{EdgeLookup, GraphData, GraphmlAttribute};
use crate::helpers::{MyVec, Preference};
use std::collections::HashMap;

pub mod dijkstra;
mod edge;
mod node;
pub mod path;

pub mod trajectory_analysis;

#[derive(Debug)]
pub struct Graph {
    pub nodes: MyVec<Node>,
    pub edges: MyVec<Edge>,
    offsets_in: MyVec<u32>,
    offsets_out: MyVec<u32>,
    half_edges_in: MyVec<HalfEdge>,
    half_edges_out: MyVec<HalfEdge>,
}

impl Graph {
    pub fn new(nodes: Vec<Node>, edges: Vec<Edge>) -> Graph {
        println!("Constructing graph...");
        let mut nodes = MyVec(nodes);
        let mut edges = MyVec(edges);
        let offsets_inner = vec![0; nodes.len() + 1];
        let mut offsets_out = MyVec(offsets_inner.clone());
        let mut offsets_in = MyVec(offsets_inner);
        let mut half_edges_out = MyVec(Vec::new());
        let mut half_edges_in = MyVec(Vec::new());

        nodes.sort_by(|a, b| b.ch_level.cmp(&a.ch_level));
        let mut id_map = HashMap::new();
        for (i, n) in nodes.0.iter_mut().enumerate() {
            id_map.insert(n.id, i as u32);
            n.id = i as u32;
        }

        edges.iter_mut().for_each(|e| {
            e.source_id = id_map[&e.source_id];
            e.target_id = id_map[&e.target_id]
        });

        // half_edges and offsets out
        edges.sort_by(|a, b| {
            a.source_id.cmp(&b.source_id).then_with(|| {
                nodes[b.target_id]
                    .ch_level
                    .cmp(&nodes[a.target_id].ch_level)
            })
        });

        edges
            .iter()
            // .filter(|edge| nodes[edge.target_id].ch_level >= nodes[edge.source_id].ch_level)
            .for_each(|edge| {
                offsets_out[edge.source_id + 1] += 1;
                half_edges_out.push(HalfEdge::new(edge.id, edge.target_id, edge.edge_costs));
            });

        // half_edges and offsets in
        edges.sort_by(|a, b| {
            a.target_id.cmp(&b.target_id).then_with(|| {
                nodes[b.source_id]
                    .ch_level
                    .cmp(&nodes[a.source_id].ch_level)
            })
        });
        edges
            .iter()
            // .filter(|edge| nodes[edge.source_id].ch_level >= nodes[edge.target_id].ch_level)
            .for_each(|edge| {
                offsets_in[edge.target_id + 1] += 1;
                half_edges_in.push(HalfEdge::new(edge.id, edge.source_id, edge.edge_costs));
            });

        // finish offset arrays
        for index in 1..offsets_out.len() {
            offsets_out[index] += offsets_out[index - 1];
            offsets_in[index] += offsets_in[index - 1];
        }

        // sort edges by id
        edges.sort_by(|a, b| a.id.cmp(&b.id));
        Graph {
            nodes,
            edges,
            offsets_in,
            offsets_out,
            half_edges_in,
            half_edges_out,
        }
    }

    pub fn find_shortest_path(
        &self,
        dijkstra: &mut Dijkstra,
        id: u32,
        include: &[u32],
        alpha: Preference,
    ) -> Option<Path> {
        if let Some(result) = dijkstra::find_path(dijkstra, &include, alpha) {
            let unpacked_edges: Vec<Vec<u32>> = result
                .edges
                .iter()
                .map(|subpath_edges| {
                    subpath_edges
                        .iter()
                        .flat_map(|edge| self.unpack_edge(*edge))
                        .collect()
                })
                .collect();
            let cuts = MyVec(
                unpacked_edges
                    .iter()
                    .map(|edges| edges.len() as u32)
                    .collect(),
            );

            let edges: Vec<u32> = unpacked_edges.into_iter().flatten().collect();
            let mut nodes: Vec<u32> = edges
                .iter()
                .map(|edge| self.edges[*edge].source_id)
                .collect();
            nodes.push(*include.last().unwrap());

            return Some(Path {
                id,
                nodes: MyVec(nodes),
                edges: MyVec(edges),
                user_split: PathSplit {
                    cuts,
                    alphas: MyVec(vec![alpha]),
                    dimension_costs: result.dimension_costs,
                    costs_by_alpha: result.costs_by_alpha,
                },
                algo_split: None,
                total_dimension_costs: result.total_dimension_costs,
            });
        }
        None
    }

    fn get_ch_edges_out(&self, node_id: u32) -> &[HalfEdge] {
        &self.half_edges_out[self.offsets_out[node_id]..self.offsets_out[node_id + 1]]
    }

    fn get_ch_edges_in(&self, node_id: u32) -> &[HalfEdge] {
        &self.half_edges_in[self.offsets_in[node_id]..self.offsets_in[node_id + 1]]
    }

    fn unpack_edge(&self, edge: u32) -> Vec<u32> {
        if let Some((edge1, edge2)) = self.edges[edge].replaced_edges {
            let mut first = self.unpack_edge(edge1);
            first.extend(self.unpack_edge(edge2).iter());
            return first;
        }
        vec![edge]
    }
}

pub fn parse_graph_file(file_path: &str) -> Result<Graph, Box<dyn std::error::Error>> {
    use crate::EDGE_COST_DIMENSION;
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    println!("Parsing graph...");
    let mut nodes: Vec<Node> = Vec::new();
    let mut edges: Vec<Edge> = Vec::new();
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    for _i in 0..4 {
        // comments and blanks
        lines.next();
    }
    let cost_dim: usize = lines.next().expect("No edge cost dim given")?.parse()?;
    assert_eq!(EDGE_COST_DIMENSION, cost_dim);
    let num_of_nodes = lines
        .next()
        .expect("Number of nodes not present in file")?
        .parse()?;
    let num_of_edges = lines
        .next()
        .expect("Number of edges not present in file")?
        .parse()?;

    let mut parsed_nodes: usize = 0;
    let mut parsed_edges: u32 = 0;
    while let Some(Ok(line)) = lines.next() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens[0] == "#" || tokens[0] == "\n" {
            continue;
        }
        if parsed_nodes < num_of_nodes {
            nodes.push(Node::new(
                tokens[0].parse()?,
                // tokens[2].parse()?,
                // tokens[3].parse()?,
                // tokens[4].parse()?,
                tokens[5].parse()?,
            ));
            parsed_nodes += 1;
        } else if parsed_edges < num_of_edges {
            let replaced_edges = if tokens[tokens.len() - 2] == "-1" {
                None
            } else {
                Some((
                    tokens[tokens.len() - 2].parse()?,
                    tokens[tokens.len() - 1].parse()?,
                ))
            };
            edges.push(Edge::new(
                parsed_edges,
                tokens[0].parse()?,
                tokens[1].parse()?,
                edge::parse_costs(&tokens[2..tokens.len() - 2]),
                replaced_edges,
            ));
            parsed_edges += 1;
        } else {
            panic!("Something doesn't add up with the amount of nodes and edges in graph file");
        }
    }
    Ok(Graph::new(nodes, edges))
}

pub fn parse_minimal_graph_file(file_path: &str) -> Result<GraphData, Box<dyn std::error::Error>> {
    use crate::EDGE_COST_DIMENSION;
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    println!("Parsing graph...");
    let mut nodes: Vec<Node> = Vec::new();
    let mut edges: Vec<Edge> = Vec::new();
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let mut edge_lookup: EdgeLookup = Default::default();

    loop {
        if let Some(Ok(line)) = lines.next() {
            if !line.starts_with('#') {
                break;
            }
        }
    }

    let cost_dim: usize = lines.next().expect("No edge cost dim given")?.parse()?;
    if EDGE_COST_DIMENSION != cost_dim {
        panic!("Graph has wrong dimension");
    }

    let metric_name_line = lines.next().expect("No metric names given")?;
    let metric_names: Vec<_> = metric_name_line.split(' ').collect();
    if metric_names.len() != EDGE_COST_DIMENSION {
        panic!("Wrong number of metric names in graph file");
    }
    let keys = metric_names
        .into_iter()
        .enumerate()
        .map(|(i, n)| (n.to_string(), GraphmlAttribute::new("edge", n, "double", i)))
        .collect();

    let num_of_nodes = lines
        .next()
        .expect("Number of nodes not present in file")?
        .parse()?;
    let num_of_edges = lines
        .next()
        .expect("Number of edges not present in file")?
        .parse()?;

    let mut parsed_nodes: usize = 0;
    let mut parsed_edges: u32 = 0;
    while let Some(Ok(line)) = lines.next() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens[0] == "#" || tokens[0] == "\n" {
            continue;
        }
        if parsed_nodes < num_of_nodes {
            nodes.push(Node::new(
                tokens[0].parse()?,
                tokens[1].parse().unwrap_or(0),
            ));
            parsed_nodes += 1;
        } else if parsed_edges < num_of_edges {
            let replaced_edges = if tokens[tokens.len() - 2] == "-1" {
                None
            } else {
                Some((
                    tokens[tokens.len() - 2].parse()?,
                    tokens[tokens.len() - 1].parse()?,
                ))
            };
            edges.push(Edge::new(
                parsed_edges,
                tokens[1].parse()?,
                tokens[2].parse()?,
                edge::parse_costs(&tokens[3..tokens.len() - 2]),
                replaced_edges,
            ));
            edge_lookup.insert(tokens[0].to_string(), parsed_edges);
            parsed_edges += 1;
        } else {
            panic!("Something doesn't add up with the amount of nodes and edges in graph file");
        }
    }
    let graph = Graph::new(nodes, edges);

    Ok(GraphData {
        graph,
        edge_lookup,
        keys,
    })
}
