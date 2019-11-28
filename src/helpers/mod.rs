use crate::EDGE_COST_DIMENSION;

use std::ops::{Deref, DerefMut, Index};

pub type Preference = [f64; EDGE_COST_DIMENSION];
pub type Costs = [f64; EDGE_COST_DIMENSION];

pub fn costs_by_alpha(costs: &Costs, alpha: &Preference) -> f64 {
    costs
        .iter()
        .zip(alpha.iter())
        .fold(0.0, |acc, (cost, factor)| acc + cost * factor)
}

pub fn add_edge_costs(a: &Costs, b: &Costs) -> Costs {
    let mut result = [0.0; EDGE_COST_DIMENSION];
    a.iter()
        .zip(b.iter())
        .enumerate()
        .for_each(|(index, (first, second))| result[index] = first + second);
    result
}

#[derive(Debug)]
pub struct MyVec<T>(pub Vec<T>);

impl<T> Index<u32> for MyVec<T> {
    type Output = T;

    fn index(&self, idx: u32) -> &Self::Output {
        &self.0[idx as usize]
    }
}

impl<T> Index<usize> for MyVec<T> {
    type Output = T;

    fn index(&self, idx: usize) -> &Self::Output {
        &self.0[idx]
    }
}

impl<T> Deref for MyVec<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for MyVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_edge_costs() {
        let a = [1.5, 2.0, 0.7, 1.3];
        let b = [1.3, 0.1, 0.3, 0.3];
        let result = add_edge_costs(&a, &b);
        assert_eq!([2.8, 2.1, 1.0, 1.6], result);
    }
}
