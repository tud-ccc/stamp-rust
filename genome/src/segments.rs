use crate::gene::Gene;
use crate::Nucleotide;
use rand::Rng;
use rand_chacha::ChaCha12Rng;

#[derive(Clone)]
pub struct Segments {
    pub length: usize,
    pub orig_gene_length: usize,
    pub minimal_segmentcount: usize,
    pub contents: Vec<Vec<Nucleotide>>,
}

impl Segments {
    pub fn create(
        length: usize,
        minimal_count: usize,
        gene: &mut Gene,
        rng: &mut ChaCha12Rng,
    ) -> Self {
        let mut cont = Vec::with_capacity(minimal_count);

        let start_number = gene.length - length + 1;

        // Pick some random segments to start
        for _ in 0..minimal_count {
            let j = rng.gen_range(0, start_number);
            gene.bitmap.set_bit(j);
            let slice = &gene.contents[j..(j + length)];
            cont.push(slice.to_owned());
        }

        // make sure the start is covered, one way or the other
        if !gene.bitmap.get_bit(0) {
            cont.push(gene.contents[0..length].to_owned());
            gene.bitmap.set_bit(0);
        }

        // Add extra segments to fill holes and ensure there exist overlaps.
        let maximal_hole_size = length - 1;
        let mut idx = 0;
        while idx < start_number {
            // we check, if holes that are too "large" exist
            let upper_bound = std::cmp::min(idx + maximal_hole_size, start_number);
            while idx < upper_bound && !gene.bitmap.get_bit(idx) {
                idx += 1;
            }

            if idx == upper_bound {
                // hole is big enough, fill it
                idx -= 1;
                cont.push(gene.contents[idx..(idx + length)].to_owned());
                gene.bitmap.set_bit(idx);
            }
            idx += 1;
        }

        // done. Return the finished segments part
        Segments {
            length,
            minimal_segmentcount: minimal_count,
            orig_gene_length: gene.length,
            contents: cont,
        }
    }
}
