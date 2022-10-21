use rand::distributions::{Distribution, Standard};
use rand::Rng;

mod bitmap;
pub mod gene;
pub mod segments;

#[cfg(not(feature = "transactional"))]
pub mod sequencer;
#[cfg(feature = "transactional")]
pub mod stm_sequencer;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum Nucleotide {
    Adenine,
    Cytosine,
    Guanine,
    Thyamine,
}

impl Distribution<Nucleotide> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Nucleotide {
        match rng.gen_range(0, 4) {
            0 => Nucleotide::Adenine,
            1 => Nucleotide::Cytosine,
            2 => Nucleotide::Guanine,
            3 => Nucleotide::Thyamine,
            _ => unreachable!("Illegal range value generated"),
        }
    }
}
