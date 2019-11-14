use crate::EDGE_COST_DIMENSION;

pub type Preference = [f64; EDGE_COST_DIMENSION];
pub type Costs = [f64; EDGE_COST_DIMENSION];

pub fn costs_by_alpha(costs: Costs, alpha: Preference) -> f64 {
    costs
        .iter()
        .zip(alpha.iter())
        .fold(0.0, |acc, (cost, factor)| acc + cost * factor)
}

pub fn add_edge_costs(a: Costs, b: Costs) -> Costs {
    let mut result = [0.0; EDGE_COST_DIMENSION];
    a.iter()
        .zip(b.iter())
        .enumerate()
        .for_each(|(index, (first, second))| result[index] = first + second);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_edge_costs() {
        let a = [1.5, 2.0, 0.7, 1.3];
        let b = [1.3, 0.1, 0.3, 0.3];
        let result = add_edge_costs(a, b);
        assert_eq!([2.8, 2.1, 1.0, 1.6], result);
    }
}
