use std::fmt::{Display, Formatter};

pub mod graph;
pub mod graphml;
pub mod helpers;
pub mod lp;
pub mod statistics;
pub mod trajectories;

pub const EDGE_COST_DIMENSION: usize = 4;

#[derive(Debug)]
pub enum MyError {
    InvalidTrajectories,
}

impl Display for MyError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        match self {
            MyError::InvalidTrajectories => write!(f, "Invalid Trajectories"),
        }
    }
}

impl std::error::Error for MyError {}
