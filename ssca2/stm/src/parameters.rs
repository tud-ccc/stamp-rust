use clap::Parser;
use std::str::FromStr;

#[derive(Parser, Clone, Debug)]
#[clap(author = "Felix Suchert", version = "1.0", about = "A Rust port of the ssca2 benchmark from the rust-stm collection.", long_about = None)]
pub struct Parameters {
    #[clap(long = "runs", short = 'r', default_value_t = 1)]
    pub runs: usize,
    #[clap(long = "json")]
    pub json: bool,
    #[clap(long, default_value = "results")]
    pub outdir: String,
    #[clap(long = "threads", short = 't', default_value_t = 1)]
    pub threads: usize,
    /// Binary scaling heuristic
    #[clap(long = "scale", short = 's', default_value_t = 20)]
    pub scale: usize,
    /// Maximal number of parallel edges between vertices
    #[clap(long = "max_par_edges", short = 'p', default_value_t = 3)]
    pub max_parallel_edges: usize,
    /// Fraction of Integer (vs. string) weights
    #[clap(long = "weights", short = 'w', default_value_t = 0.6)]
    pub percent_int_weights: f32,
    
    #[clap(long = "prob_unidirectional", short = 'u', default_value_t = 0.1)]
    pub probability_unidirectional: f32,
    /// Initial probability for a link between cliques
    #[clap(long = "prob_interclique", short = 'i', default_value_t = 0.5)]
    pub probability_interclique_edges: f32,
    /// Kernel 3: maximum path lengh, measured by # of edges in the subgraph generated from the end
    /// vertex of SI and SC lists.
    #[clap(long = "max_path_length", short = 'l', default_value_t = 3)]
    pub subgraph_edge_length: usize,

    // some implementation specific variables
    #[clap(long = "kind", short = 'k', default_value = "vector")]
    pub k3_ds: DataStructureType,

    // derived information
    #[clap(skip)]
    pub total_vertices: usize,
    #[clap(skip)]
    pub max_clique_size: usize,
    /// Maximal int value in edge weight
    #[clap(skip)]
    pub max_int_weight: usize,
    #[clap(skip)]
    pub max_str_len: usize,

    /// Kernel 2: Character String sought: specify here, else it is picked from a randomly selected
    /// entry in `gen_scal_data`.
    #[clap(skip)]
    pub sought_string: String,

    /// Kernel 4: Clustering search box size.
    #[clap(skip)]
    pub max_cluster_size: usize,
}

#[derive(Parser, Copy, Clone, Debug)]
pub enum DataStructureType {
    Array,
    LinkedList,
    DynamicArray
}

impl FromStr for DataStructureType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "0" |
            "array" => Ok(Self::Array),
            "1" |
            "list" => Ok(Self::LinkedList),
            "2" |
            "vector" => Ok(Self::DynamicArray),
            _ => Err("Unsupported data structure kind".into())
        }
    }
}

impl Parameters {
    /// Post-process the input parameters after parsing, setting the missing parameters.
    pub fn process(&mut self) {
        self.total_vertices = 1 << self.scale;
        self.max_clique_size = 1 << (self.scale / 3);
        self.max_int_weight = 1 << self.scale;
        self.max_str_len = self.scale;

        self.max_cluster_size = self.max_clique_size;

    }
}
