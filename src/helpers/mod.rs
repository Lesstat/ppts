use crate::EDGE_COST_DIMENSION;

use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut, Index, IndexMut, Range, RangeInclusive};

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MyVec<T>(pub Vec<T>);

impl<T> MyVec<T> {
    pub fn new() -> MyVec<T> {
        MyVec(Vec::new())
    }
}

impl<T> Index<u32> for MyVec<T> {
    type Output = T;

    fn index(&self, idx: u32) -> &Self::Output {
        &self.0[idx as usize]
    }
}

impl<T> IndexMut<u32> for MyVec<T> {
    fn index_mut(&mut self, idx: u32) -> &mut Self::Output {
        &mut self.0[idx as usize]
    }
}

impl<T> Index<Range<u32>> for MyVec<T> {
    type Output = [T];

    fn index(&self, r: Range<u32>) -> &Self::Output {
        &self.0[r.start as usize..r.end as usize]
    }
}

impl<T> Index<RangeInclusive<u32>> for MyVec<T> {
    type Output = [T];

    fn index(&self, r: RangeInclusive<u32>) -> &Self::Output {
        let start = *r.start() as usize;
        let end = *r.end() as usize;
        &self.0[start..=end]
    }
}

impl<T> Index<usize> for MyVec<T> {
    type Output = T;

    fn index(&self, idx: usize) -> &Self::Output {
        &self.0[idx]
    }
}

impl<T> IndexMut<usize> for MyVec<T> {
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        &mut self.0[idx]
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
