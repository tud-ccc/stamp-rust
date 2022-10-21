use crate::parameters::Parameters;
use rand_chacha::ChaCha12Rng;
use rand_core::{RngCore, SeedableRng};
use std::fs::File;
use std::io::{BufWriter, Write};

/// Tuple for Scalable Data Generation
pub struct GraphSDG {
    pub start_vertex: Vec<usize>,
    pub end_vertex: Vec<usize>,

    pub int_weight: Vec<i64>,

    pub str_weight: Vec<u8>,
    pub num_edges_placed: usize,

    pub max_cluster_size: usize,
}

impl GraphSDG {
    pub fn generate(parameters: Parameters) -> Self {
        // STEP 0: Create the permutations required to randomize the vertices
        // --------------------------------------------------------------------
        let mut rng = ChaCha12Rng::seed_from_u64(0);

        // the variables associated with the graph tuple
        let mut perm_v: Vec<usize> = (0..parameters.total_vertices).into_iter().collect();
        for i in 0..parameters.total_vertices {
            let t1 = rng.next_u32() as usize;
            let t = i + t1 % (parameters.total_vertices - i);

            if t != i {
                perm_v.swap(i, t);
            }
        }

        // STEP 1: Create Cliques
        // --------------------------------------------------------------------

        // estimate number of cliques required & pad by 50 %
        let estimated_total_cliques = ((parameters.total_vertices as f64 * 1.5)
            / ((1 + parameters.max_clique_size) as f64 / 2_f64))
            .ceil() as usize;

        // generate random clique sizes
        let mut clique_sizes: Vec<usize> = (0..estimated_total_cliques)
            .map(|_| 1 + (rng.next_u32() as usize % parameters.max_clique_size))
            .collect();

        let mut last_vs_in_cliques = vec![0; estimated_total_cliques];
        let mut first_vs_in_cliques = vec![0; estimated_total_cliques];

        // sum up vertices in each clique to determine the `last_vs_in_cliques` array
        last_vs_in_cliques[0] = clique_sizes[0] - 1;

        let mut i = 1;
        while i < estimated_total_cliques {
            last_vs_in_cliques[i] = clique_sizes[i] + last_vs_in_cliques[i - 1];
            if last_vs_in_cliques[i] >= parameters.total_vertices - 1 {
                break;
            }
            i += 1;
        }

        let total_cliques = i + 1;
        // TODO(feliix42): On early abort: should we shrink the arrays?

        // fix the size of the last clique
        clique_sizes[i] = parameters.total_vertices - last_vs_in_cliques[i - 1] - 1;
        last_vs_in_cliques[i] = parameters.total_vertices - 1;

        // compute start vertices in cliques
        first_vs_in_cliques[0] = 0;
        for i in 1..total_cliques {
            first_vs_in_cliques[i] = last_vs_in_cliques[i - 1] + 1;
        }

        // Optionally write the generates cliques to file for comparison with kernel 4
        if cfg!(feature = "write_result_files") {
            let f = File::create("cliques.txt").expect("Could not open `cliques.txt` for writing.");
            let mut b = BufWriter::new(f);

            writeln!(b, "No. of cliques - {}", total_cliques).unwrap();
            for i in 0..total_cliques {
                write!(b, "Clq {} - ", i).unwrap();
                for j in first_vs_in_cliques[i]..=last_vs_in_cliques[i] {
                    write!(b, "{} ", perm_v[j]).unwrap();
                }
                writeln!(b, "").unwrap();
            }
        }

        // STEP 2: Create the edges within the cliques
        // --------------------------------------------------------------------

        // estimate the number of edges - using an empirical measure
        let estimated_total_edges = if parameters.scale >= 12 {
            (parameters.max_clique_size - 1) * parameters.total_vertices
        } else {
            (1.2 * ((((parameters.max_clique_size - 1) * parameters.total_vertices)
                * ((1 + parameters.max_parallel_edges / 2) + parameters.total_vertices * 2))
                as f64))
                .ceil() as usize
        };

        //let p = parameters.probability_unidirectional;

        // partial edge lists
        let mut start_vertex: Vec<usize> = Vec::with_capacity(estimated_total_edges);
        let mut end_vertex: Vec<usize> = Vec::with_capacity(estimated_total_edges);

        // tmp array to keep track of the no. of parallel edges in each direction
        let mut tmp_edge_counter: Vec<Vec<usize>> =
            vec![vec![0; parameters.max_clique_size]; parameters.max_clique_size];

        // create edges
        for i_clique in 0..total_cliques {
            // get current clique parameters
            let i_clique_size = clique_sizes[i_clique];
            let i_first_vs_in_clique = first_vs_in_cliques[i_clique];

            // first, create at least one edge between two vertices in a clique
            for i in 0..i_clique_size {
                for j in 0..i {
                    let r = (rng.next_u32() % 1000) as f32 / 1000_f32;

                    if r >= parameters.probability_unidirectional {
                        start_vertex.push(i + i_first_vs_in_clique);
                        end_vertex.push(j + i_first_vs_in_clique);
                        tmp_edge_counter[i][j] = 1;

                        start_vertex.push(j + i_first_vs_in_clique);
                        end_vertex.push(i + i_first_vs_in_clique);
                        tmp_edge_counter[j][i] = 1;
                    } else if r >= 0.5 {
                        start_vertex.push(i + i_first_vs_in_clique);
                        end_vertex.push(j + i_first_vs_in_clique);
                        tmp_edge_counter[i][j] = 1;
                        tmp_edge_counter[j][i] = 0;
                    } else {
                        start_vertex.push(j + i_first_vs_in_clique);
                        end_vertex.push(i + i_first_vs_in_clique);
                        tmp_edge_counter[j][i] = 1;
                        tmp_edge_counter[i][j] = 0;
                    }
                }
            }

            // add a random number of edges
            if i_clique_size != 1 {
                let random_no_edges =
                    rng.next_u32() as usize % (2 * i_clique_size * parameters.max_parallel_edges);

                for _ in 0..random_no_edges {
                    let i = rng.next_u32() as usize % i_clique_size;
                    let j = rng.next_u32() as usize % i_clique_size;

                    if i != j && tmp_edge_counter[i][j] < parameters.max_parallel_edges {
                        let r = (rng.next_u32() % 1000) as f32 / 1000_f32;

                        if r >= parameters.probability_unidirectional {
                            // copy the edge structure
                            start_vertex.push(i + i_first_vs_in_clique);
                            end_vertex.push(j + i_first_vs_in_clique);
                            tmp_edge_counter[i][j] += 1;
                        }
                    }
                }
            }
        }

        // housekeeping
        std::mem::drop(tmp_edge_counter);

        //let mut start_vertex: Vec<usize> = start_v;
        //let mut end_vertex: Vec<usize> = end_v;

        let num_edges_placed_in_cliques = start_vertex.len();

        // STEP 3: Connect the cliques
        // --------------------------------------------------------------------

        let mut p; // = parameters.probability_interclique_edges;
        
        let mut start_v = Vec::new();
        let mut end_v = Vec::new();

        // generating inter-clique edges as given in the specs
        for i in 0..parameters.total_vertices {
            let tmp_vertex1 = i;

            let mut h = total_cliques;
            let mut l = 0;
            let mut t = None;

            while h - l > 1 {
                let m = (h + l) / 2;
                if tmp_vertex1 >= first_vs_in_cliques[m] {
                    l = m;
                } else {
                    if m > 0 {
                        if tmp_vertex1 >= first_vs_in_cliques[m - 1] {
                            t = Some(m - 1);
                            break;
                        } else {
                            h = m;
                        }
                    }
                }
            }

            if t.is_none() {
                let mut m = l + 1;
                while m < h {
                    if tmp_vertex1 < first_vs_in_cliques[m] {
                        break;
                    }
                    m += 1;
                }

                t = Some(m - 1);
            }

            let t1 = first_vs_in_cliques[t.unwrap()];

            let mut d = 1;
            p = parameters.probability_interclique_edges;
            while d < parameters.total_vertices {
                let r = (rng.next_u32() % 1000) as f32 / 1000_f32;

                if r <= p {
                    let tmp_vertex2 = (i + d) % parameters.total_vertices;

                    h = total_cliques;
                    l = 0;
                    t = None;
                    while h - l > 1 {
                        let m = (h + l) / 2;
                        if tmp_vertex2 >= first_vs_in_cliques[m] {
                            l = m;
                        } else {
                            if m > 0 {
                                if tmp_vertex2 >= first_vs_in_cliques[m - 1] {
                                    t = Some(m - 1);
                                    break;
                                } else {
                                    h = m;
                                }
                            }
                        }
                    }

                    if t.is_none() {
                        let mut m = l + 1;
                        while m < h {
                            if tmp_vertex2 < first_vs_in_cliques[m] {
                                break;
                            }
                            m += 1;
                        }

                        t = Some(m - 1);
                    }

                    let t2 = first_vs_in_cliques[t.unwrap()];

                    if t1 != t2 {
                        let random_no_edges =
                            rng.next_u32() as usize % parameters.max_parallel_edges + 1;

                        for _ in 0..random_no_edges {
                            start_v.push(tmp_vertex1);
                            end_v.push(tmp_vertex2);
                        }
                    }
                } // r <= p

                let r0 = (rng.next_u32() % 1000) as f32 / 1000_f32;

                let diff: isize = i as isize - d as isize;
                if r0 <= p && diff >= 0 {
                    let tmp_vertex2 = ((i - d) % parameters.total_vertices) as usize;

                    h = total_cliques;
                    l = 0;
                    t = None;
                    while h - l > 1 {
                        let m = (h + l) / 2;
                        if tmp_vertex2 >= first_vs_in_cliques[m] {
                            l = m;
                        } else {
                            if m > 0 {
                                if tmp_vertex2 >= first_vs_in_cliques[m - 1] {
                                    t = Some(m - 1);
                                    break;
                                } else {
                                    h = m;
                                }
                            }
                        }
                    }

                    if t.is_none() {
                        let mut m = l + 1;
                        while m < h {
                            if tmp_vertex2 < first_vs_in_cliques[m] {
                                break;
                            }
                            m += 1;
                        }

                        t = Some(m - 1);
                    }

                    let t2 = first_vs_in_cliques[t.unwrap()];

                    if t1 != t2 {
                        let random_no_edges =
                            rng.next_u32() as usize % parameters.max_parallel_edges + 1;

                        for _ in 0..random_no_edges {
                            start_v.push(tmp_vertex1);
                            end_v.push(tmp_vertex2);
                        }
                    }
                } // r0 <= p && (i - d) > 0

                d *= 2;
                p /= 2_f32;
            } // for d, p
        } // for i

        let num_edges_placed_outside = start_v.len();

        start_vertex.extend(start_v.into_iter());
        end_vertex.extend(end_v.into_iter());

        let num_edges_placed = num_edges_placed_in_cliques + num_edges_placed_outside;

        println!("Finished generating edges");
        println!(
            "No. of intra-clique edges - {}",
            num_edges_placed_in_cliques
        );
        println!("No. of inter-clique edges - {}", num_edges_placed_outside);
        println!("Total no. of edges        - {}", num_edges_placed);

        //std::mem::drop(clique_sizes);
        //std::mem::drop(first_vs_in_cliques);
        std::mem::drop(last_vs_in_cliques);

        // STEP 4: Generate Edge weights
        // --------------------------------------------------------------------

        p = parameters.percent_int_weights;
        let mut num_str_wt_edges = 0;

        let mut int_weight: Vec<i64> = (0..num_edges_placed)
            .map(|_| {
                let r = (rng.next_u32() % 1000) as f32 / 1000_f32;
                if r <= p {
                    (1 + rng.next_u32() as usize % (parameters.max_int_weight - 1)) as i64
                } else {
                    num_str_wt_edges += 1;
                    -1
                }
            })
            .collect();

        let mut t = 0;
        for item in int_weight.iter_mut() {
            if item.is_negative() {
                *item = -t;
                t += 1;
            }
        }

        let mut str_weight: Vec<u8> = vec![0; num_str_wt_edges * parameters.max_str_len];
        for wgt in &int_weight {
            if wgt.is_negative() {
                for j in 0..parameters.max_str_len {
                    str_weight[wgt.abs() as usize * parameters.max_str_len + j] =
                        1 + (rng.next_u32() % 127) as u8;
                }
            }
        }

        let mut sought_string = parameters.sought_string;
        // Choose SOUGHT STRING at random it it's not assigned yet
        if sought_string.is_empty() {
            // NOTE(feliix42): This seemed broken in the original implementation. The sought string
            // was replaced, no matter if it was pre-set or not.
            // I fixed this here.
            sought_string.reserve(parameters.max_str_len);

            let t = rng.next_u32() as usize % num_str_wt_edges;
            sought_string.extend(
                (0..parameters.max_str_len)
                    .map(|j| char::from(str_weight[t * parameters.max_str_len + j]))
                    .collect::<Vec<char>>(),
            );
        }

        // STEP 5: Permute Vertices
        // --------------------------------------------------------------------

        for i in 0..num_edges_placed {
            start_vertex[i] = perm_v[start_vertex[i]];
            end_vertex[i] = perm_v[end_vertex[i]];
        }

        // STEP 6: Sort Vertices
        // --------------------------------------------------------------------

        // radix sort with start vertex as primary key
        // NOTE(feliix42): I'm not 100% certain this is implemented correctly here
        let mut zipped: Vec<(usize, usize)> = start_vertex
            .into_iter()
            .zip(end_vertex.into_iter())
            .collect();
        zipped.sort_unstable_by_key(|(x, _)| *x);
        let (start_vert, mut end_vert): (Vec<usize>, Vec<usize>) = zipped.into_iter().unzip();

        if parameters.scale < 12 {
            // sort with end_vertex as secondary key
            let mut i0 = 0;
            let mut i1 = 0;
            let mut i = 0;

            while i < num_edges_placed {
                i = i0;
                while i < num_edges_placed {
                    if start_vert[i] != start_vert[i1] {
                        i1 = i;
                        break;
                    }
                    i += 1;
                }

                for j in i0..i1 {
                    for k in (j + 1)..i1 {
                        if end_vert[k] < end_vert[j] {
                            end_vert.swap(j, k);
                        }
                    }
                }

                if start_vert[i0] != parameters.total_vertices - 1 {
                    i0 = i1;
                } else {
                    for j in i0..num_edges_placed {
                        for k in (j + 1)..num_edges_placed {
                            if end_vert[k] < end_vert[j] {
                                end_vert.swap(j, k);
                            }
                        }
                    }
                }
            } // while i < num_edges_placed
        } else {
            // update degree of each vertex
            let mut tmp_index = vec![0; parameters.total_vertices + 1];
            tmp_index[parameters.total_vertices] = num_edges_placed;

            let mut i0 = 0;
            for i in 0..parameters.total_vertices {
                tmp_index[i + 1] = tmp_index[i];
                for j in i0..num_edges_placed {
                    if start_vert[j] != start_vert[i0] {
                        if start_vert[i0] == i {
                            tmp_index[i + 1] = j;
                            i0 = j;
                            break;
                        }
                    }
                }
            }

            // original comment: "Insertion sort for now, replace with something better later on"
            for i in 0..parameters.total_vertices {
                for j in tmp_index[i]..tmp_index[i + 1] {
                    for k in (j + 1)..tmp_index[i + 1] {
                        if end_vert[k] < end_vert[j] {
                            end_vert.swap(j, k);
                        }
                    }
                }
            }
        } // scale >= 12

        Self {
            start_vertex: start_vert,
            end_vertex: end_vert,

            int_weight,
            str_weight,
            num_edges_placed,

            max_cluster_size: parameters.max_cluster_size,
        }
    }
}
