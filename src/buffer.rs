pub struct Buffer {
    pub lines: Vec<Vec<char>>,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            lines: vec![vec![]],
        }
    }
}
