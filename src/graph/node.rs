#[derive(Debug)]
pub struct Node {
    pub id: u32,
    pub ch_level: u32,
}

impl Node {
    pub fn new(id: u32, ch_level: u32) -> Node {
        Node { id, ch_level }
    }
}
