use std::cmp::Ordering;

use ordered_float::OrderedFloat;

#[derive(PartialEq, Copy, Clone)]
pub enum Direction {
    FORWARD,
    BACKWARD,
}

#[derive(PartialEq)]
pub struct State {
    pub node_id: usize,
    pub total_cost: f64,
    pub direction: Direction,
}

impl State {
    pub fn new(node_id: usize, direction: Direction) -> Self {
        State {
            node_id,
            total_cost: 0.0,
            direction,
        }
    }
}

impl std::cmp::Eq for State {}

impl std::cmp::Ord for State {
    // switch comparison, because we want a min-heap
    fn cmp(&self, other: &Self) -> Ordering {
        OrderedFloat(other.total_cost).cmp(&OrderedFloat(self.total_cost))
    }
}

impl std::cmp::PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
