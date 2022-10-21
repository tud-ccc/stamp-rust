use crate::bitmap::Bitmap;
use crate::Nucleotide;
use rand::Rng;
use rand_chacha::ChaCha12Rng;

pub struct Gene {
    pub length: usize,
    pub contents: Vec<Nucleotide>,
    /// Bitmap for creating gene `segments`.
    pub bitmap: Bitmap,
}

impl Gene {
    /// Create a new gene with a random nucleotide sequence
    pub fn create(length: usize, rng: &mut ChaCha12Rng) -> Self {
        // generate the nucleotide sequence
        let mut cont = Vec::with_capacity(length);
        for _ in 0..length {
            cont.push(rng.gen());
        }

        Gene {
            length,
            contents: cont,
            bitmap: Bitmap::new(length),
        }
    }
}
