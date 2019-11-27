#[derive(Debug)]
pub struct Node {
    pub id: usize,
    pub ch_level: usize,
}

impl Node {
    pub fn new(id: usize, ch_level: usize) -> Node {
        Node { id, ch_level }
    }
}
