use std::ops::Range;

pub struct Bitmap {
    contents: Vec<bool>,
}

impl Bitmap {
    pub fn new(length: usize) -> Self {
        Bitmap {
            contents: vec![false; length],
        }
    }

    /// Sets the bit on position `pos` to true (1).
    pub fn set_bit(&mut self, pos: usize) {
        self.contents[pos] = true;
    }

    /// Gets the bit from position `pos`
    pub fn get_bit(&self, pos: usize) -> bool {
        self.contents[pos]
    }

    pub fn search(&self, r: Range<usize>) -> bool {
        self.contents[r].contains(&true)
    }
}
