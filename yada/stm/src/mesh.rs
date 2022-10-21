//! In the original codebase, this was `mesh.c`

use crate::cavity::Cavity;
use crate::element::{Edge, Element, Triangle};
use crate::point::Point;
use decorum::R64;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::str::FromStr;
use std::thread;
use stm::{atomically, StmError, StmResult, TVar, Transaction};

#[derive(Clone)]
pub struct Mesh {
    /// Triangles in the mesh with links to neighboring elements.
    /// The vector will **always** have 3 elements.
    ///
    /// The reason for this being a `Vec<Element>` instead of `[Element; 3]` is
    /// simply ergonimics: It's infinitely simpler to just append/remove elements
    /// from a vector than an array. This means that in the two places where the
    /// array is actually modified, the relaxed constraint allows the simple
    /// removal of elements without having to use indirections like casting the
    /// array to an array of `Option<Element>` etc.
    pub elements: TVar<HashMap<Triangle, TVar<Vec<Element>>>>,
    // pub elements: HashMap<Triangle, [Element; 3]>,
    pub boundary_set: TVar<HashMap<Edge, Element>>,
    /// Minimum angle to be achieved for all elements.
    pub min_angle: f64,
}

impl Mesh {
    pub fn load_from_file(filename_prefix: &str, min_angle: f64) -> std::io::Result<Self> {
        // load *.node file
        let node_file = File::open(format!("{}.node", filename_prefix))?;
        let mut node_reader = BufReader::new(node_file);

        let mut buf = String::new();
        node_reader.read_line(&mut buf)?;
        let node_header: Vec<usize> = buf
            .split_whitespace()
            .map(usize::from_str)
            .map(Result::unwrap)
            .collect();
        let entry_count = node_header[0]; // this is both numEntry and numCoordinate (because why??)
        let num_dimensions = node_header[1];
        assert_eq!(num_dimensions, 2);

        let mut coordinates = Vec::with_capacity(entry_count);
        for line in node_reader.lines() {
            let line = line?;
            // skip comments
            if line.starts_with('#') {
                continue;
            }

            let coords: Vec<f64> = line
                .split_whitespace()
                .skip(1)
                .take(2)
                .map(f64::from_str)
                .map(Result::unwrap)
                .collect();

            coordinates.push(Point {
                x: R64::from_inner(coords[0]),
                y: R64::from_inner(coords[1]),
            });
        }

        assert_eq!(coordinates.len(), entry_count);

        // load the *.poly file containing lines (?)
        let poly_file = File::open(format!("{}.poly", filename_prefix))?;
        let mut poly_reader = BufReader::new(poly_file);

        buf.clear();
        poly_reader.read_line(&mut buf)?;
        let poly_header: Vec<usize> = buf
            .split_whitespace()
            .map(usize::from_str)
            .map(Result::unwrap)
            .collect();
        assert_eq!(poly_header[0], 0);
        assert_eq!(poly_header[1], 2);
        // line 2 gives us the total element #
        poly_reader.read_line(&mut buf)?;
        let poly_header: Vec<usize> = buf
            .split_whitespace()
            .map(usize::from_str)
            .map(Result::unwrap)
            .collect();
        let num_segments = poly_header[0];

        let mut edge_set = HashSet::with_capacity(num_segments);
        for line in poly_reader.lines() {
            // may be unnecessary
            //if num_segments == 0 {
            //break;
            //}
            let line = line?;

            // skip comments
            if line.starts_with('#') {
                continue;
            }

            let coords: Vec<usize> = line
                .split_whitespace()
                .skip(1)
                .take(2)
                .map(usize::from_str)
                .map(Result::unwrap)
                .collect();

            if coords.is_empty() {
                continue;
            }
            assert!(coords[0] <= entry_count);
            assert!(coords[1] <= entry_count);

            // they count items from 1 for some reason
            let c_0 = coordinates[coords[0] - 1];
            let c_1 = coordinates[coords[1] - 1];
            edge_set.insert(Edge::new(c_0, c_1));
        }

        // load the *.ele file
        let ele_file = File::open(format!("{}.ele", filename_prefix))?;
        let mut ele_reader = BufReader::new(ele_file);

        buf.clear();
        ele_reader.read_line(&mut buf)?;
        let ele_header: Vec<usize> = buf
            .split_whitespace()
            .map(usize::from_str)
            .map(Result::unwrap)
            .collect();
        let num_elements = ele_header[0];

        let mut elem_vec = Vec::with_capacity(num_elements);
        for line in ele_reader.lines() {
            let line = line?;

            if line.starts_with('#') {
                continue;
            }

            let coords: Vec<usize> = line
                .split_whitespace()
                .skip(1)
                .take(3)
                .map(usize::from_str)
                .map(Result::unwrap)
                .collect();
            assert!(coords[0] <= entry_count);
            assert!(coords[1] <= entry_count);
            assert!(coords[2] <= entry_count);

            // they count items from 1 for some reason
            let c_0 = coordinates[coords[0] - 1];
            let c_1 = coordinates[coords[1] - 1];
            let c_2 = coordinates[coords[2] - 1];
            elem_vec.push(Triangle::new(c_0, c_1, c_2));
        }

        let mut edges: HashMap<Edge, Element> = HashMap::with_capacity(num_segments);
        let mut elems: HashMap<Triangle, Vec<Element>> = HashMap::with_capacity(num_elements);

        // establish neighboring relations
        let mut triangle_map: HashMap<Edge, Triangle> = HashMap::new();
        // NOTE: We assume here that the input files are properly structured, i.e., that no edge is neighbor to another edge

        for elem in elem_vec {
            for i in 0..3 {
                let e = elem.get_edge(i);

                if edge_set.contains(&e) {
                    // edge is shared with an outer edge -> establish link
                    // insert the item in the linked list
                    let item = elems.entry(elem).or_default();
                    item.push(e.into());

                    // insert backlink from the edge
                    // NOTE(feliix42): could be `assert!()`ed as `is_none()`
                    edges.insert(e, elem.into());
                } else if let Some(other) = triangle_map.remove(&e) {
                    let item = elems.entry(elem).or_default();
                    item.push(other.into());

                    let o_mut = elems.entry(other).or_default();
                    o_mut.push(elem.into());
                } else {
                    triangle_map.insert(e, elem);
                }
            }
        }
        // verify correctness
        #[cfg(verify)]
        {
            assert_eq!(edges.len(), edge_set.len());
            let _ = elems
                .iter()
                .map(|(_, v)| assert_eq!(v.len(), 3))
                .collect::<()>();
        }

        Ok(Mesh {
            elements: TVar::new(elems.into_iter().map(|(k, v)| (k, TVar::new(v))).collect()),
            boundary_set: TVar::new(edges),
            min_angle,
        })
    }

    /// NOTE: Reads atomically
    pub fn find_bad(&self) -> VecDeque<Triangle> {
        let mut r = VecDeque::new();

        let elems = self
            .elements
            .read_ref_atomic()
            .downcast::<HashMap<Triangle, TVar<Vec<Element>>>>()
            .unwrap();

        for elem in elems.keys() {
            if elem.is_bad(self.min_angle) {
                r.push_back(elem.to_owned());
            }
        }

        r
    }

    /// Tests whether `node` is contained in the graphs triangle set.
    pub fn contains_triangle(&self, node: &Triangle) -> bool {
        self.elements
            .read_ref_atomic()
            .downcast::<HashMap<Triangle, TVar<Vec<Element>>>>()
            .unwrap()
            .contains_key(node)
    }

    // NOTE(feliix42): So this one is fun: It can happen that a cavity is started
    // from an edge (only happens when the cavity is initialized and overwrites
    // itself). But apparently this is never translated to this stage of execution.
    //
    // It seems like that may not be a problem after all since the last few lines in the function are handling this.
    /// Update the mesh with the data of a corrected cavity. (Original code implements this in `Cavity.h`)
    ///
    /// Returns a list of new bad elements
    pub fn update(
        &self,
        cav: Cavity,
        original_bad: Triangle,
        trans: &mut Transaction,
    ) -> StmResult<VecDeque<Triangle>> {
        // NOTE(feliix42): we're essentially doing a double read here, after already doing a read
        // while building the cavity. Depending on the semantic model of the transactional memory
        // implementation it may be fine (to the model) to expose a different state here than in
        // the previous read without warning. It's maybe a safe idea to fail further down when
        // pruning old connections if one of the connections is not there anymore.
        let mut elements = self.elements.read(trans)?;
        let mut boundary_set = self.boundary_set.read(trans)?;

        // here we'd probably have to check if all elements of the `previous_nodes`
        // are still in the mesh before updating when doing this in parallel.

        // remove old elements
        for old in cav.previous_nodes {
            match old {
                Element::T(ref t) => {
                    // NOTE(feliix42): Picking up from the previous note, we'll (for now)
                    // explicitly fail here if the element is not in the set anymore.
                    if elements.remove(t).is_none() {
                        return Err(StmError::Failure);
                    }
                }
                Element::E(ref e) => {
                    if boundary_set.remove(e).is_none() {
                        return Err(StmError::Failure);
                    }
                }
            }
        }

        // prune old connections!
        for (old, _, outer) in cav.connections {
            match outer {
                Element::T(ref t) => {
                    if let Some(neighborhood_var) = elements.get(t) {
                        let mut neighborhood = neighborhood_var.read(trans)?;
                        let len_before = neighborhood.len();
                        let _ = neighborhood
                            .drain_filter(|&mut x| x == old)
                            .collect::<Vec<_>>();
                        let len_after = neighborhood.len();
                        if len_before != len_after + 1 {
                            return Err(StmError::Failure);
                        }

                        neighborhood_var.write(trans, neighborhood)?;
                    } else {
                        return Err(StmError::Failure);
                    }
                }
                Element::E(ref e) => {
                    // we can just remove the edge because the new neighboring relation will add it again.
                    boundary_set.remove(e);
                }
            }
        }

        // add new data
        let mut new_bad = VecDeque::new();
        for new_node in cav.new_nodes {
            match new_node {
                Element::T(t) => {
                    elements.insert(t, TVar::new(Vec::with_capacity(3)));
                    if t.is_bad(self.min_angle) {
                        new_bad.push_back(t);
                    }
                }
                Element::E(_) => (),
            }
        }

        // this `new_edge` is somewhat unnecessary I reckon, but that was part of the original code.
        // I'm now also realizing that this could probably be put into the
        // `compute` function of `cavity`, but thinking ahead it's probably
        // better kept here to make matters simpler when going for the parallel
        // versions.
        for (src, _, dst) in cav.new_connections {
            match src {
                Element::T(ref t) => {
                    if let Some(neighborhood) = elements.get(t) {
                        neighborhood.modify(trans, |mut v| {
                            v.push(dst);
                            v
                        })?;
                    } else {
                        return Err(StmError::Failure);
                    }
                }
                Element::E(e) => {
                    boundary_set.insert(e, dst);
                }
            }

            match dst {
                Element::T(ref t) => {
                    if let Some(neighborhood) = elements.get(t) {
                        neighborhood.modify(trans, |mut v| {
                            v.push(src);
                            v
                        })?;
                    } else {
                        return Err(StmError::Failure);
                    }
                }
                Element::E(e) => {
                    boundary_set.insert(e, src);
                }
            }
        }

        // if the original "bad element" is still in the mesh that's still a todo
        if elements.contains_key(&original_bad) {
            new_bad.push_back(original_bad);
        }

        // verify correctness
        #[cfg(verify)]
        {
            let _ = self
                .elements
                .iter()
                .map(|(_, v)| assert_eq!(v.len(), 3))
                .collect::<()>();
        }

        self.elements.write(trans, elements)?;
        self.boundary_set.write(trans, boundary_set)?;

        Ok(new_bad)
    }
}

pub fn refine(mesh_var: Mesh, bad: VecDeque<Triangle>, threadcount: usize) -> usize {
    let bad_list = splitup(bad, threadcount);

    let mut handles = Vec::with_capacity(threadcount);
    for mut bad in bad_list {
        let mesh = mesh_var.clone();
        handles.push(thread::spawn(move || {
            let mut i = 0;
            while !bad.is_empty() {
                i += 1;

                let item = bad.pop_front().unwrap();

                // TODO(feliix42): Count retries
                let result = atomically(|trans| {
                    // this happens atomically
                    if !mesh.contains_triangle(&item) {
                        return Ok(VecDeque::with_capacity(0));
                    }

                    if let Some(mut cav) = Cavity::new(&mesh, item.into()) {
                        cav.build(&mesh, trans)?;
                        cav.compute();
                        let result = mesh.update(cav, item, trans)?;

                        // NOTE(feliix42): We could update the `bad` list here and read in parallel
                        // from the queue
                        return Ok(result);
                    }
                    Ok(VecDeque::with_capacity(0))
                });

                for i in result {
                    bad.push_back(i);
                }
            }

            i
        }));
    }

    handles
        .into_iter()
        .map(thread::JoinHandle::join)
        .map(Result::unwrap)
        .sum()
}

fn splitup<T>(vec: VecDeque<T>, split_size: usize) -> Vec<VecDeque<T>>
where
    T: Clone,
{
    // TODO(feliix42): boost perf on this
    let vec = vec.into_iter().collect::<Vec<_>>();

    let element_count = vec.len();
    let mut rest = element_count % split_size;
    let window_len: usize = element_count / split_size;
    let per_vec = if rest != 0 {
        window_len + 1
    } else {
        window_len
    };

    let mut res = vec![Vec::with_capacity(per_vec); split_size];

    let mut start = 0;
    for i in 0..split_size {
        // calculate the length of the window (for even distribution of the `rest` elements)
        let len = if rest > 0 {
            rest -= 1;
            window_len + 1
        } else {
            window_len
        };

        let dst = start + len;

        res[i].extend_from_slice(&vec[start..dst]);

        start = dst;
    }

    res.into_iter()
        .map(|v| v.into_iter().collect::<VecDeque<_>>())
        .collect()
}
