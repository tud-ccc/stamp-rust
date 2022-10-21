use crate::segments::Segments;
use crate::Nucleotide;
use itertools::Itertools;
use std::collections::VecDeque;
use std::ops::Range;
use std::sync::{Arc, Barrier};
use std::thread;
use stm::{atomically, TVar};
use stm_datastructures::THashSet;

#[derive(Clone, Debug)]
pub struct SequencerItem {
    pub segment: Arc<Vec<Nucleotide>>,
    pub links: TVar<LinkInfo>,
}

#[derive(Clone, Copy, Debug)]
pub struct LinkInfo {
    pub prev: Option<usize>,
    pub next: Option<usize>,
    pub overlap_with_prev: usize,
}

impl Default for LinkInfo {
    fn default() -> Self {
        Self {
            prev: None,
            next: None,
            overlap_with_prev: 0,
        }
    }
}

impl From<Vec<Nucleotide>> for SequencerItem {
    fn from(nucleotide_sequence: Vec<Nucleotide>) -> Self {
        SequencerItem {
            segment: Arc::new(nucleotide_sequence),
            links: TVar::new(LinkInfo::default()),
        }
    }
}

/// Splits the input vector into evenly sized vectors for `split_size` workers.
fn split_evenly(
    mut to_split: Vec<Vec<Nucleotide>>,
    split_size: usize,
) -> Vec<Vec<Vec<Nucleotide>>> {
    let l = to_split.len() / split_size;
    let mut rest = to_split.len() % split_size;

    let mut splitted = Vec::new();

    for t_num in 0..split_size {
        splitted.push(Vec::with_capacity(l));
        if rest > 0 {
            splitted[t_num] = to_split.split_off(to_split.len() - l - 1);
            rest -= 1;
        } else {
            if to_split.len() <= l {
                splitted[t_num] = to_split.split_off(0);
            } else {
                splitted[t_num] = to_split.split_off(to_split.len() - l);
            }
        }
    }

    splitted
}

pub fn deduplicate(segments: Segments, threadcount: usize) -> VecDeque<SequencerItem> {
    // Step 1: deduplicate all segments by placing them in a hashmap
    let tmp: Arc<THashSet<Vec<Nucleotide>>> = Arc::new(THashSet::new(segments.orig_gene_length));
    let to_dedup = split_evenly(segments.contents, threadcount);

    let mut handles = Vec::new();
    for items in to_dedup {
        let local_tmp = tmp.clone();
        handles.push(thread::spawn(move || {
            for item in items {
                // the following `clone` operation on `item` is absolutely necessary, since
                // the `Fn` closure may be called numerous times in case of a retry,
                // requiring the cloning of the variable.
                atomically(|trans| local_tmp.insert(trans, item.clone()));
            }
        }));
    }

    handles.into_iter().for_each(|h| h.join().unwrap());

    // now unpack the Arc and get the values
    let hash_set = match Arc::try_unwrap(tmp) {
        Ok(content) => content,
        Err(_) => panic!("Unexpectedly failed to unpack arc"),
    };
    atomically(|trans| hash_set.as_vec(trans))
        .into_iter()
        .map(SequencerItem::from)
        .collect()
}

pub fn run_sequencer(
    unique_segments: Arc<VecDeque<SequencerItem>>,
    segment_length: usize,
    iteration_ranges: Vec<Range<usize>>,
) {
    let mut handles = Vec::with_capacity(iteration_ranges.len());
    let barrier = Arc::new(Barrier::new(iteration_ranges.len()));

    // spawn threads to work in parallel
    for rng in iteration_ranges.clone() {
        let iteration_range = rng.clone();
        let segments = unique_segments.clone();
        let c = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            // Step 2: go through the prefixes and suffixes of the genomes in descending size and stitch the genome back together
            for match_length in (1..segment_length).rev() {
                /*
                 * - loop through possible subsegment lengths
                 * - loop through all segments once (x)
                 * - loop through all segments again (y) and match against the starts and ends of x and y, stitch together on match
                 */

                for idx in iteration_range.clone() {
                    atomically(|trans| {
                        let cur_seg = &segments[idx];
                        let mut cur_links = cur_seg.links.read(trans)?;

                        // only continue if the current segment is not linked already
                        if cur_links.prev.is_none() {
                            let slice = &cur_seg.segment[0..match_length];

                            // go over all items in Vec and test whether we can append our
                            // `cur_seg` to the item. If so, stop
                            'inner: for it in 0..segments.len() {
                                // skip the element itself -- this might be unnecessary but we want to avoid
                                // breaking the matching algorithm by linking an element to itself
                                if idx == it {
                                    continue;
                                }
                                // skip the current item when it already has an appended segment
                                if segments[it]
                                    .links
                                    .read_ref_atomic()
                                    .downcast::<LinkInfo>()
                                    .unwrap()
                                    .next
                                    .is_some()
                                {
                                    continue;
                                }

                                let cur_slice = &segments[it].segment
                                    [(segment_length - match_length)..segment_length];
                                if slice == cur_slice {
                                    // link both items together
                                    //segments[it].next.write(trans, Some(idx))?;
                                    segments[it].links.modify(trans, |mut l| {
                                        l.next = Some(idx);
                                        l
                                    })?;
                                    cur_links.prev = Some(it); //.write(trans, Some(it))?;
                                    cur_links.overlap_with_prev = match_length; //.write(trans, match_length)?;
                                    cur_seg.links.write(trans, cur_links)?;
                                    break 'inner;
                                }
                            }
                        }
                        Ok(())
                    });
                }
                c.wait();
            }
        }));
    }

    // wait for threads to finish
    handles.into_iter().for_each(|h| h.join().unwrap());
}

pub fn reconstruct(unique_segments: Arc<VecDeque<SequencerItem>>) -> Vec<Nucleotide> {
    if cfg!(feature = "verify") {
        // TMP test
        println!("[TEST] checking segment links");
        atomically(|trans| {
            let mut forward_links = 0;
            let mut backward_links = 0;
            for item in unique_segments.iter() {
                if item.links.read(trans)?.next.is_none() {
                    forward_links += 1;
                }
                if item.links.read(trans)?.prev.is_none() {
                    backward_links += 1;
                }
            }
            assert_eq!(forward_links, 1);
            assert_eq!(backward_links, 1);
            Ok(())
        });
    }

    // Step 3 link together sequence
    atomically(|trans| {
        // find first element
        let mut cur = unique_segments
            .iter()
            .find(|seg| seg.links.read_atomic().prev.is_none())
            .unwrap();

        let mut reconstructed_sequence = Vec::new();

        loop {
            let link_info = cur.links.read(trans)?;

            reconstructed_sequence.extend_from_slice(&cur.segment[link_info.overlap_with_prev..]);

            if let Some(next_idx) = link_info.next {
                // move to the next value -> have to assign to another let binding first to drop `val` due to ownership issues
                cur = &unique_segments[next_idx];
            } else {
                break;
            }
        }

        Ok(reconstructed_sequence)
    })
}
