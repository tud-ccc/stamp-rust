use clap::{App, Arg};
use cpu_time::ProcessTime;
use kmeans::{self, Centroid, Value};
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::str::FromStr;
use time::PreciseTime;

fn main() {
    let matches = App::new("Sequential kmeans benchmark")
        .version("1.0")
        .author("Felix Suchert <dev@felixsuchert.de>")
        .about("A Rust port of the kmeans benchmark from the STAMP collection, implemented in a sequential manner.")
        .arg(
            Arg::with_name("INPUT")
                .help("Input file to describe the grid and paths.")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("clusters-count")
            .long("clusters")
            .short("n")
            .help("Number of clusters")
            .takes_value(true)
            .default_value("40")
        )
        .arg(
            Arg::with_name("threshold")
            .long("threshold")
            .short("t")
            .help("Threshold below which a convergence is assumed. (Defined as maximum percentage of values that may change cluster in a single iteration)")
            .takes_value(true).default_value("0.05")
        )
        .arg(
            Arg::with_name("no_zscore")
            .short("z")
            .help("Don't perform zscore transformations on the data")
            .takes_value(false)
        )
        .arg(
            Arg::with_name("runs")
                .long("runs")
                .short("r")
                .takes_value(true)
                .help("The number of runs to conduct.")
                .default_value("1")
        )
        .arg(
            Arg::with_name("json")
                .long("json")
                .short("j")
                .help("Dump results as JSON file.")
        )
        .arg(
            Arg::with_name("outdir")
                .long("outdir")
                .short("o")
                .help("Sets the output directory for JSON dumps")
                .takes_value(true)
                .default_value("results")
        )
        .get_matches();

    // parse benchmark parameters
    let input_path = matches.value_of("INPUT").unwrap();
    let cluster_count = usize::from_str(matches.value_of("clusters-count").unwrap())
        .expect("Provided invalid value for `clusters`. Must be an uint.");
    let threshold = f32::from_str(matches.value_of("threshold").unwrap()).expect(
        "Provided invalid value for `threshold`. Must be a non-negative floatin gpoint number",
    );
    let dont_use_zscore = matches.is_present("no_zscore");

    // parse runtime parameters
    let runs =
        usize::from_str(matches.value_of("runs").unwrap()).expect("Could not parse number of runs");
    let json_dump = matches.is_present("json");
    let out_dir = matches.value_of("outdir").unwrap();

    // read and prepare the input data
    let mut clusters =
        Value::load_from_text_file(input_path).expect("Failed to load input from file");
    // apply zscore transformation
    if !dont_use_zscore {
        kmeans::apply_zscore_transform(&mut clusters);
    }

    let centroids = Centroid::randomly_generate(&clusters, cluster_count);

    // run benchmark itself
    let mut results = Vec::with_capacity(runs);
    let mut cpu_results = Vec::with_capacity(runs);
    let mut convergence_after = Vec::with_capacity(runs);
    let mut computations = Vec::with_capacity(runs);

    for r in 0..runs {
        // prepare the data for the run
        let input_data = clusters.clone();
        let initial_centers = centroids.clone();

        // start the clock
        let start = PreciseTime::now();
        let cpu_start = ProcessTime::now();

        // run the algorithm
        let (iterations, comp) = run_kmeans(input_data, initial_centers, threshold);

        // stop the clock
        let cpu_end = ProcessTime::now();
        let end = PreciseTime::now();
        let runtime_ms = start.to(end).num_milliseconds();
        let cpu_runtime_ms = cpu_end.duration_since(cpu_start).as_millis();

        if !json_dump {
            println!("[INFO] kmeans run {} completed.", r + 1);
        }

        results.push(runtime_ms);
        cpu_results.push(cpu_runtime_ms);
        convergence_after.push(iterations);
        computations.push(comp);
    }

    // generate output
    if json_dump {
        create_dir_all(out_dir).unwrap();
        let filename = format!(
            "{}/seq-n{}-t{}-r{}_log.json",
            out_dir, cluster_count, threshold, runs
        );
        let mut f = File::create(&filename).unwrap();
        f.write_fmt(format_args!(
            "{{
    \"algorithm\": \"sequential\",
    \"cluster-count\": {cluster_count},
    \"threshold\": {threshold},
    \"input\": \"{input_path}\",
    \"values-count\": {value_count},
    \"runs\": {runs},
    \"converged_after\": {conv:?},
    \"computations\": {comp:?},
    \"cpu_time\": {cpu:?},
    \"results\": {res:?}
}}",
            cluster_count = cluster_count,
            threshold = threshold,
            input_path = input_path,
            value_count = clusters.len(),
            runs = runs,
            conv = convergence_after,
            comp = computations,
            cpu = cpu_results,
            res = results
        ))
        .unwrap();
    } else {
        println!("[INFO] All runs completed successfully.");
        println!("\nStatistics:");
        println!("    Number of clusters:          {}", cluster_count);
        println!("    Threshold for conversion:    {}", threshold);
        println!("    Input file used:             {}", input_path);
        println!("    Number of values from input: {}", clusters.len());
        println!("    Runs:                        {}", runs);
        println!("\nConvergence after: {:?}", convergence_after);
        println!("CPU-time used (ms): {:?}", cpu_results);
        println!("Runtime in ms: {:?}", results);
    }
}

fn run_kmeans(
    mut values: Vec<Value>,
    mut centroids: Vec<Centroid>,
    threshold: f32,
) -> (usize, usize) {
    let mut runs = 0;
    let mut delta = std::f32::MAX;

    let mut computations = 0;

    // exit conditions: either we are below our self-set threshold or 500 iterations have passed
    while runs < 500 && delta > threshold {
        runs += 1;
        delta = 0f32;

        computations += values.len();

        // Step 1: Assign all clusters to a centroid
        for val in values.iter_mut() {
            let new_cluster = val.find_nearest_centroid(&centroids);
            if new_cluster != val.associated_cluster {
                delta += 1.0;
                val.associated_cluster = new_cluster;
            }
        }
        delta /= values.len() as f32;

        // Step 2: Calculate new centroids
        centroids = Centroid::from_assignments(&values, centroids.len());
    }

    (runs, computations)
}
