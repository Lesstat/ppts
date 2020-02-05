use crate::graph::path::Path;
use crate::helpers::Costs;
use std::collections::HashSet;

/// Overlap takes two paths and calculates the ratio to which they
/// overlap.
///
/// The formula is #shared_edges/max(#edges_in_path1, #edges_in_path2).
/// A result of 1.0 means the paths are identical and a result of 0.0
/// means they are completely distinct.
pub fn overlap(path1: &Path, path2: &Path) -> f64 {
    let max_edge_count = path1.edges.len().max(path2.edges.len()) as f64;

    let path1_edges: HashSet<_> = path1.edges.iter().collect();
    let same_edge_count = path2
        .edges
        .iter()
        .filter(|e| path1_edges.contains(e))
        .count() as f64;

    same_edge_count / max_edge_count
}
