use crate::segments::Segments;
use crate::Nucleotide;
use itertools::Itertools;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

#[derive(Debug, Eq, PartialEq)]
struct SequencerItem {
    pub segment: Vec<Nucleotide>,
    pub prev: Option<Rc<RefCell<SequencerItem>>>,
    pub next: Option<Rc<RefCell<SequencerItem>>>,
    pub overlap_with_prev: usize,
}

impl From<Vec<Nucleotide>> for SequencerItem {
    fn from(nucleotide_sequence: Vec<Nucleotide>) -> Self {
        SequencerItem {
            segment: nucleotide_sequence,
            prev: None,
            next: None,
            overlap_with_prev: 0,
        }
    }
}

pub fn run_sequencer(segments: Segments) -> Vec<Nucleotide> {
    // Step 1: deduplicate all segments
    let mut unique_segments: VecDeque<Rc<RefCell<SequencerItem>>> = segments
        .contents
        .into_iter()
        .unique() // itertools magic for deduplication: fast because of the enums representation in memory as u8
        .map(SequencerItem::from)
        .map(RefCell::new)
        .map(Rc::new)
        .collect();
    let segment_count = unique_segments.len();

    // Step 2: go through the prefixes and suffixes of the genomes in descending size and stitch the genome back together
    for match_length in (1..segments.length).rev() {
        /*
         * What I want to do:
         * - loop through possible subsegment lengths
         * - loop through all segments once (x)
         * - loop through all segments again (y) and match against the starts and ends of x and y, stitch together on match
         *
         * Question: *How* do I remove matches from the set on the go?
         * -> Should be parallelizable for STM and Ohua.
         */

        for _ in 0..segment_count {
            let cs = unique_segments
                .pop_front()
                .expect("No segments were generated");
            let mut cur_seg = cs.borrow_mut();

            // only continue if the current segment is not linked already
            if cur_seg.prev.is_none() {
                let slice = &cur_seg.segment[0..match_length];

                // go over all items in Vec and test whether we can append our `cur_seg` to the item. If so, stop
                'inner: for it in unique_segments.iter() {
                    let mut item = it.borrow_mut();
                    // skip the current item when it already has an appended segment
                    if item.next.is_some() {
                        continue;
                    }

                    let cur_slice =
                        &item.segment[(segments.length - match_length)..segments.length];
                    if slice == cur_slice {
                        // link both items together
                        item.next = Some(cs.clone());
                        cur_seg.prev = Some(it.clone());
                        cur_seg.overlap_with_prev = match_length;
                        break 'inner;
                    }
                }
            }

            std::mem::drop(cur_seg);
            unique_segments.push_back(cs);
        }
    }

    if cfg!(feature = "verify") {
        // TMP test
        println!("[TEST] checking segment links");
        let mut forward_links = 0;
        let mut backward_links = 0;
        for it in &unique_segments {
            let item = it.borrow();
            if item.next.is_none() {
                forward_links += 1;
            }
            if item.prev.is_none() {
                backward_links += 1;
            }
        }
        assert_eq!(forward_links, 1);
        assert_eq!(backward_links, 1);
    }

    // Step 3 link together sequence
    // find first element
    let mut cur = unique_segments
        .iter()
        .find(|seg| seg.borrow().prev.is_none())
        .unwrap()
        .clone();
    let mut reconstructed_sequence = Vec::new();

    loop {
        let val = cur.borrow();
        reconstructed_sequence.extend_from_slice(&val.segment[val.overlap_with_prev..]);

        if val.next.is_some() {
            // move to the next value -> have to assign to another let binding first to drop `val` due to ownership issues
            let next = val.next.as_ref().unwrap().clone();
            std::mem::drop(val);
            cur = next;
        } else {
            break;
        }
    }

    reconstructed_sequence
}
