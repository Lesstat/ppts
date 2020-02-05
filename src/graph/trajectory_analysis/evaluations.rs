use super::{get_length, get_scalar_product};
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

/// Calculates the angle between two cost vectors.
///
/// Identical vectors have an angle of 1.0 while orthogonal ones have
/// an angle of 0.0.
pub fn cost_angle(cost1: &Costs, cost2: &Costs) -> f64 {
    let length1 = get_length(cost1);
    let length2 = get_length(cost2);
    let product = get_scalar_product(cost1, cost2);
    product / (length1 * length2)
}

/// Calculates length ratio between costs.
///
/// 1.0 means same length, smaller values mean more difference.
pub fn cost_length_ratio(cost1: &Costs, cost2: &Costs) -> f64 {
    let length1 = get_length(cost1);
    let length2 = get_length(cost2);

    let (longer, shorter) = if length1 < length2 {
        (length2, length1)
    } else {
        (length1, length2)
    };

    shorter / longer
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EDGE_COST_DIMENSION;

    #[test]
    fn test_angle_between_identical_costs_is_1() {
        let costs = [1.0; EDGE_COST_DIMENSION];
        assert_eq!(1.0, cost_angle(&costs, &costs));
    }

    #[test]
    fn test_angle_between_orthongonal_costs_is_0() {
        let mut costs1 = [0.0; EDGE_COST_DIMENSION];
        let mut costs2 = [0.0; EDGE_COST_DIMENSION];

        costs1[0] = 1.0;
        costs2[1] = 1.0;
        assert_eq!(0.0, cost_angle(&costs1, &costs2));
    }
}
