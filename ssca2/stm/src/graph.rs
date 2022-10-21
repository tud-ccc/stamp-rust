//! This was originally `computeGraph.c`

use crate::generator::GraphSDG;
use std::sync::{Arc, Barrier};
use std::thread;
use std::thread::JoinHandle;
use stm::{atomically, TVar};

/// The data structure denoting the graph generrated by kernel 1.
#[derive(Debug)]
struct InternalGraph {
    pub num_vertices: TVar<usize>,
    pub num_edges: usize,

    pub num_directed_edges: TVar<usize>,
    pub num_undirected_edges: usize,

    pub num_int_edges: usize,
    pub num_str_edges: usize,

    pub out_degree: Vec<i64>,
    pub out_vertex_idx: Vec<usize>,
    pub out_vertex_list: Vec<usize>,
    pub paral_edge_idx: Vec<usize>,

    pub in_degree: Vec<i64>,
    pub in_vertex_idx: Vec<usize>,
    pub in_vertex_list: Vec<usize>,

    pub int_weight: Vec<i64>,
    pub str_weight: Vec<u8>,
}

impl Default for InternalGraph {
    fn default() -> Self {
        Self {
            num_vertices: TVar::new(usize::default()),
            num_edges: Default::default(),
            num_directed_edges: TVar::new(Default::default()),
            num_undirected_edges: Default::default(),
            num_int_edges: Default::default(),
            num_str_edges: Default::default(),
            out_degree: Default::default(),
            out_vertex_idx: Default::default(),
            out_vertex_list: Default::default(),
            paral_edge_idx: Default::default(),
            in_degree: Default::default(),
            in_vertex_idx: Default::default(),
            in_vertex_list: Default::default(),
            int_weight: Default::default(),
            str_weight: Default::default(),
        }
    }
}

impl InternalGraph {
    pub fn prefix_sums_out(&mut self) {
        // ignoring all multi-threading stuff for a moment
        for j in 1..self.num_vertices.read_atomic() {
            self.out_vertex_idx[j] = self.out_degree[j - 1] as usize + self.out_vertex_idx[j - 1];
        }
    }

    pub fn prefix_sums_in(&mut self) {
        // ignoring all multi-threading stuff for a moment
        for j in 1..self.num_vertices.read_atomic() {
            self.in_vertex_idx[j] = self.in_degree[j - 1] as usize + self.in_vertex_idx[j - 1];
        }
    }
}

pub struct Graph {
    pub num_vertices: usize,
    pub num_edges: usize,

    pub num_directed_edges: usize,
    pub num_undirected_edges: usize,

    pub num_int_edges: usize,
    pub num_str_edges: usize,

    pub out_degree: Vec<i64>,
    pub out_vertex_idx: Vec<usize>,
    pub out_vertex_list: Vec<usize>,
    pub paral_edge_idx: Vec<usize>,

    pub in_degree: Vec<i64>,
    pub in_vertex_idx: Vec<usize>,
    pub in_vertex_list: Vec<usize>,

    pub int_weight: Vec<i64>,
    pub str_weight: Vec<u8>,
}

impl Graph {
    /// Kernel 1, computes the graph data structure
    pub fn compute(mut sdg_graph: GraphSDG, thread_count: usize) -> Self {
        let mut graph = InternalGraph::default();

        // pulled up from further down: some initializations
        graph.num_edges = sdg_graph.num_edges_placed;
        std::mem::swap(&mut graph.int_weight, &mut sdg_graph.int_weight);
        //graph.int_weight = sdg_graph.int_weight;
        std::mem::swap(&mut graph.str_weight, &mut sdg_graph.str_weight);
        //graph.str_weight = sdg_graph.str_weight;

        for i in 0..graph.num_edges {
            let idx = (graph.num_edges - i - 1) as usize;
            if graph.int_weight[idx] < 0 {
                graph.num_str_edges = (graph.int_weight[idx].abs() + 1) as usize;
                break;
            }
        }

        // First, determine the number of vertices by scanning the tuple `start_vertex` list

        // threaded part
        let mut handles = Vec::with_capacity(thread_count);
        let barrier = Arc::new(Barrier::new(thread_count));
        let sdg_arc = Arc::new(sdg_graph);
        let graph_arc = Arc::new(graph);

        for thread_id in 0..thread_count {
            let num_edges_placed = graph_arc.num_edges;
            let b = barrier.clone();
            let sdg_graph = sdg_arc.clone();
            let g = graph_arc.clone();

            handles.push(thread::spawn(move || {
                let (i_start, i_stop) =
                    create_partition(0, num_edges_placed as usize, thread_id, thread_count);

                let mut max_num_vertices = 0;
                for i in i_start..i_stop {
                    if sdg_graph.start_vertex[i] > max_num_vertices {
                        max_num_vertices = sdg_graph.start_vertex[i];
                    }
                }

                // compute the max number of vertices
                atomically(|trans| {
                    g.num_vertices.modify(trans, |global_max_vertice_num| {
                        std::cmp::max(global_max_vertice_num, max_num_vertices) + 1
                    })
                });

                // NOTE(feliix42): Thread barrier
                b.wait();

                let num_vertices = g.num_vertices.read_atomic();

                let (i_start, i_stop) = create_partition(0, num_vertices, thread_id, thread_count);

                let mut out_vertex_list_size = 0;
                let mut i0 = usize::MAX;

                let mut local_out_degree = Vec::with_capacity(i_stop - i_start);

                for i in i_start..i_stop {
                    let mut out_degree_val = 0;

                    let mut k = i;
                    if out_vertex_list_size == 0 && k != 0 {
                        while i0 == usize::MAX {
                            for j in 0..g.num_edges {
                                if k == sdg_graph.start_vertex[j] {
                                    i0 = j;
                                    break;
                                }
                            }
                            k -= 1;
                        }
                    }

                    if out_vertex_list_size == 0 && k == 0 {
                        i0 = 0;
                    }

                    for j in i0..num_edges_placed {
                        if i == num_vertices - 1 {
                            break;
                        }

                        if i != sdg_graph.start_vertex[j] {
                            if j > 0 && i == sdg_graph.start_vertex[j - 1] && j - i0 >= 1 {
                                out_vertex_list_size += 1;
                                out_degree_val += 1;

                                for t in (i0 + 1)..j {
                                    if sdg_graph.end_vertex[t] != sdg_graph.end_vertex[t - 1] {
                                        out_vertex_list_size += 1;
                                        out_degree_val += 1;
                                    }
                                }
                            }
                            i0 = j;
                            break;
                        }
                    }

                    if i == num_vertices - 1 && num_edges_placed - i0 >= 0 {
                        out_vertex_list_size += 1;
                        out_degree_val += 1;
                        for t in (i0 + 1)..num_edges_placed {
                            if sdg_graph.end_vertex[t] != sdg_graph.end_vertex[t - 1] {
                                out_vertex_list_size += 1;
                                out_degree_val += 1;
                            }
                        }
                    }

                    local_out_degree.push(out_degree_val);
                } // for i

                // moved up slightly from after the sum prefixing
                atomically(|trans| {
                    g.num_directed_edges
                        .modify(trans, |num| num + out_vertex_list_size)
                });

                local_out_degree
            }));
        }

        let comb_out_degree = handles
            .into_iter()
            .map(JoinHandle::join)
            .map(Result::unwrap)
            .flatten()
            .collect();
        let mut graph =
            Arc::try_unwrap(graph_arc).expect("unpacking arc after threaded run failed?!");
        graph.out_degree = comb_out_degree;
        graph.out_vertex_idx = vec![0; graph.out_degree.len()];

        // NOTE(feliix42): This would normally be done in parallel, but since it doesn't involve
        // STM whatsoever, I skip this for now
        graph.prefix_sums_out();

        let out_vertex_list_size = graph.num_directed_edges.read_atomic();
        graph.out_vertex_list = vec![0; out_vertex_list_size];
        graph.paral_edge_idx = vec![0; out_vertex_list_size];
        graph.out_vertex_list[0] = sdg_arc.end_vertex[0];

        // NOTE(feliix42): This would normally be done in parallel but I think it's a bit too
        // complex to do so w/o locking/STM and there's no STM in this whatsoever

        // evaluate out_vertex_list
        let mut i0 = usize::MAX;
        let num_vertices = graph.num_vertices.read_atomic();
        let num_edges_placed = graph.num_edges;

        for i in 0..num_vertices {
            let mut k = i;
            while i0 == usize::MAX && k != 0 {
                for j in 0..num_edges_placed {
                    if k == sdg_arc.start_vertex[j] {
                        i0 = j;
                        break;
                    }
                }
                k -= 1;
            }

            if i0 == usize::MAX && k == 0 {
                // fallback when init fails
                i0 = 0;
            }

            for j in i0..num_edges_placed {
                if i == num_vertices - 1 {
                    break;
                }

                if i != sdg_arc.start_vertex[j] {
                    if j > 0 && i == sdg_arc.start_vertex[j - 1] && j - i0 >= 1 {
                        let ii = graph.out_vertex_idx[i];
                        let mut r = 0;
                        graph.paral_edge_idx[ii] = i0;
                        graph.out_vertex_list[ii] = sdg_arc.end_vertex[i0];
                        r += 1;

                        for t in (i0 + 1)..j {
                            if sdg_arc.end_vertex[t] != sdg_arc.end_vertex[t - 1] {
                                graph.paral_edge_idx[ii + r] = t;
                                graph.out_vertex_list[ii + r] = sdg_arc.end_vertex[t];
                                r += 1;
                            }
                        }
                    }
                    i0 = j;
                    break;
                }
            } // for j

            if i == num_vertices - 1 {
                let mut r = 0;
                if num_edges_placed - i0 >= 0 {
                    let ii = graph.out_vertex_idx[i];
                    graph.paral_edge_idx[ii + r] = i0;
                    graph.out_vertex_list[ii + r] = sdg_arc.end_vertex[i0];
                    r += 1;

                    for t in (i0 + 1)..num_edges_placed {
                        if sdg_arc.end_vertex[t] != sdg_arc.end_vertex[t - 1] {
                            graph.paral_edge_idx[ii + r] = t;
                            graph.out_vertex_list[ii + r] = sdg_arc.end_vertex[t];
                            r += 1;
                        }
                    }
                }
            }
        } // for i

        // TODO:
        // - allocate the tvars
        // - do dem threading
        // - collectors edition
        // - bob's your uncle

        // if thread_id == 0
        // normally, deallocate sdg_graph.start_vertex and sdg_graph.end_vertex here
        //graph.in_degree = vec![0; graph.num_vertices];
        let in_degree: Vec<TVar<i64>> = (0..num_vertices).map(|_| TVar::new(0)).collect();
        let in_deg_arc = Arc::new(in_degree);

        // a temporary array to store the implied edges
        //let mut implied_edge_list = vec![0; i_stop * sdg_graph.max_cluster_size];
        let implied_edge_list: Vec<TVar<usize>> = (0..(num_vertices * sdg_arc.max_cluster_size))
            .map(|_| TVar::new(0))
            .collect();
        let impl_edges_arc = Arc::new(implied_edge_list);

        // theoretically, call `create_partition` here. (unnecessary here)

        // an auxiliary array to store implied edges, in case we overshoot `max_cluster_size`
        let aux_array: Vec<TVar<Vec<usize>>> = (0..num_vertices)
            .map(|_| TVar::new(Vec::with_capacity(0)))
            .collect();
        let aux_arc = Arc::new(aux_array);

        // threaded!
        let mut handles = Vec::with_capacity(thread_count);
        let graph_arc = Arc::new(graph);

        for thread_id in 0..thread_count {
            let g = graph_arc.clone();
            let sdg_graph = sdg_arc.clone();
            let in_degree = in_deg_arc.clone();
            let implied_edge_list = impl_edges_arc.clone();
            let aux_array = aux_arc.clone();

            handles.push(thread::spawn(move || {
                let (i_start, i_stop) = create_partition(0, num_vertices, thread_id, thread_count);

                for i in i_start..i_stop {
                    // inspect adjacency list of vertex i
                    for j in g.out_vertex_idx[i]
                        ..((g.out_vertex_idx[i] as i64 + g.out_degree[i]) as usize)
                    {
                        let v = g.out_vertex_list[j];
                        let mut k = g.out_vertex_idx[v];

                        while k < (g.out_vertex_idx[v] as i64 + g.out_degree[v]) as usize {
                            if g.out_vertex_list[k] == i {
                                break;
                            }
                            k += 1;
                        }

                        if k == (g.out_vertex_idx[v] as i64 + g.out_degree[v]) as usize {
                            atomically(|trans| {
                                // Transaction!
                                // add i to the implied edge list of v
                                let local_in_deg = in_degree[v].read(trans)?;
                                in_degree[v].write(trans, local_in_deg + 1)?;

                                if local_in_deg < sdg_graph.max_cluster_size.try_into().unwrap() {
                                    implied_edge_list[((v * sdg_graph.max_cluster_size) as i64
                                        + local_in_deg)
                                        as usize]
                                        .write(trans, i)
                                } else {
                                    // use auxiliary array to store the implied edge
                                    // create an array if it's not present already
                                    if local_in_deg % sdg_graph.max_cluster_size as i64 == 0 {
                                        aux_array[v]
                                            .write(trans, vec![0; sdg_graph.max_cluster_size])?;
                                    }
                                    aux_array[v].modify(trans, |mut l| {
                                        l[(g.in_degree[v]
                                        % sdg_graph.max_cluster_size as i64)
                                        as usize] = i;
                                        l
                                    })
                                }
                            });
                        }
                    }
                } // for i
            }));
        }

        let _: Vec<()> = handles
            .into_iter()
            .map(JoinHandle::join)
            .map(Result::unwrap)
            .collect();

        let mut graph = Arc::try_unwrap(graph_arc).expect("couldn't unwrap after parallel section");
        graph.in_degree = in_deg_arc.iter().map(|x| x.read_atomic()).collect();

        graph.in_vertex_idx = vec![0; num_vertices];
        graph.prefix_sums_in();

        graph.num_undirected_edges = (graph.in_vertex_idx[num_vertices - 1] as i64
            + graph.in_degree[num_vertices - 1]) as usize;
        println!("num undirected edges: {}", graph.num_undirected_edges);
        graph.in_vertex_list = vec![0; graph.num_undirected_edges + 1];

        // NOTE(feliix42): This should normally run in parallel?!
        for i in 0..num_vertices {
            for j in graph.in_vertex_idx[i]
                ..((graph.in_vertex_idx[i] as i64 + graph.in_degree[i]) as usize)
            {
                if j - graph.in_vertex_idx[i] < sdg_arc.max_cluster_size {
                    graph.in_vertex_list[j] = impl_edges_arc
                        [i * sdg_arc.max_cluster_size + j - graph.in_vertex_idx[i]].read_atomic();
                } else {
                    graph.in_vertex_list[j] =
                        aux_arc[i].read_atomic()[(j - graph.in_vertex_idx[i]) % sdg_arc.max_cluster_size];
                }
            }
        }

        Graph::from(graph)

    }
}

impl From<InternalGraph> for Graph {
    fn from(other: InternalGraph) -> Self {
        Self {
            num_vertices: other.num_vertices.read_atomic(),
            num_edges: other.num_edges,
            num_directed_edges: other.num_directed_edges.read_atomic(),
            num_undirected_edges: other.num_undirected_edges,
            num_int_edges: other.num_int_edges,
            num_str_edges: other.num_str_edges,
            out_degree: other.out_degree,
            out_vertex_idx: other.out_vertex_idx,
            out_vertex_list: other.out_vertex_list,
            paral_edge_idx: other.paral_edge_idx,
            in_degree: other.in_degree,
            in_vertex_idx: other.in_vertex_idx,
            in_vertex_list: other.in_vertex_list,
            int_weight: other.int_weight,
            str_weight: other.str_weight,
        }
    }
}

/// Originally implemented in `createPartition.c`, this function yields 2 pointers to the start and
/// end.
pub fn create_partition(min: usize, max: usize, id: usize, n: usize) -> (usize, usize) {
    let range = max - min;
    let chunk = std::cmp::max(1, (range + n / 2) / n); // rounded
    let start = min + chunk * id;
    let stop = if id == (n - 1) {
        max
    } else {
        std::cmp::min(max, start + chunk)
    };

    (start, stop)
}
