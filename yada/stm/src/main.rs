#![feature(drain_filter)]
mod cavity;
mod element;
mod mesh;
mod point;

use crate::element::{Element, Triangle};
use crate::mesh::Mesh;
use clap::{App, Arg};
use cpu_time::ProcessTime;
use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::str::FromStr;
use std::time::Instant;
use stm::TVar;

fn main() {
    let matches = App::new("Transactional yada benchmark")
        .version("1.0")
        .author("Felix Suchert <dev@felixsuchert.de>")
        .about(
            "A Rust port of the yada benchmark from the Galois collection, implemented using STM.",
        )
        .arg(
            Arg::with_name("INPUT")
                .help("Input file name stem.")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("minangle")
                .long("angle")
                .short("a")
                .takes_value(true)
                .help("Minimum angle")
                .default_value("20")
        )
        .arg(
            Arg::with_name("runs")
                .long("runs")
                .short("r")
                .takes_value(true)
                .help("The number of runs to conduct.")
                .default_value("1"),
        )
        .arg(
            Arg::with_name("threadcount")
                .long("threads")
                .short("t")
                .takes_value(true)
                .help("Number of threads to use for execution.")
                .default_value("4"),
        )
        .arg(
            Arg::with_name("json")
                .long("json")
                .short("j")
                .help("Dump results as JSON file."),
        )
        .arg(
            Arg::with_name("outdir")
                .long("outdir")
                .short("o")
                .help("Sets the output directory for JSON dumps")
                .takes_value(true)
                .default_value("results"),
        )
        .get_matches();

    // parse parameters
    let input_file = matches.value_of("INPUT").unwrap();
    let minimum_angle = f64::from_str(matches.value_of("minangle").unwrap())
        .expect("Could not parse minimum angle as f64");

    // parse runtime parameters
    let runs =
        usize::from_str(matches.value_of("runs").unwrap()).expect("Could not parse number of runs");
    let json_dump = matches.is_present("json");
    let out_dir = matches.value_of("outdir").unwrap();
    let threadcount = usize::from_str(matches.value_of("threadcount").unwrap())
        .expect("Could not parse number of runs");

    // read and parse input data
    let input_data = Mesh::load_from_file(input_file, minimum_angle)
        .expect("Loading of input data failed. Ensure that all necessary files are present.");

    let mesh_elements = input_data
        .elements
        .read_ref_atomic()
        .downcast::<HashMap<Triangle, TVar<Vec<Element>>>>()
        .unwrap()
        .len();

    if !json_dump {
        println!(
            "[info] Loaded {} mesh elements.",
            mesh_elements //input_data.elements.len() + input_data.boundary_set.len()
        );
    }

    // run the benchmark itself
    let mut results = Vec::with_capacity(runs);
    let mut cpu_time = Vec::with_capacity(runs);
    let mut computations = Vec::with_capacity(runs);

    if !json_dump {
        print!("[info] Running benchmark");
    }

    for _ in 0..runs {
        // clone the necessary data
        let mesh =
            Mesh::load_from_file(input_file, minimum_angle).expect("Failed to parse input file");

        // start the clock
        let cpu_start = ProcessTime::now();
        let start = Instant::now();

        // run the algorithm
        let comp = run_refining(mesh.clone(), threadcount);

        // stop the clock
        let cpu_end = ProcessTime::now();
        let end = Instant::now();
        let runtime_ms = end.duration_since(start).as_millis();
        let cpu_runtime_ms = cpu_end.duration_since(cpu_start).as_millis();

        // verification
        assert!(mesh.find_bad().is_empty());

        if !json_dump {
            print!(".");
            std::io::stdout().flush().unwrap();
        }

        results.push(runtime_ms);
        cpu_time.push(cpu_runtime_ms);
        computations.push(comp);
    }

    // write output
    if json_dump {
        create_dir_all(out_dir).unwrap();
        let filename = format!(
            "{}/stm-{}ele-t{}-r{}_log.json",
            out_dir, mesh_elements, threadcount, runs
        );
        let mut f = File::create(&filename).unwrap();
        f.write_fmt(format_args!(
            "{{
    \"application\": \"yada\",
    \"algorithm\": \"sequential\",
    \"elements\": {ele},
    \"threadcount\": {threads},
    \"runs\": {runs},
    \"computations\": {comps:?},
    \"cpu_time\": {cpu:?},
    \"results\": {res:?}
}}",
            ele = mesh_elements,
            threads = threadcount,
            runs = runs,
            comps = computations,
            cpu = cpu_time,
            res = results
        ))
        .unwrap();
    } else {
        println!(" done!");

        println!("[info] All runs completed.");
        println!("\nStatistics:");
        println!("    Number of Mesh elements: {}", mesh_elements);
        println!("    Input file used:         {}", input_file);
        println!("    Runs:                    {}", runs);
        println!("    Threads:                 {}", threadcount);
        println!("\nCPU-time used (ms): {:?}", cpu_time);
        println!("Computations: {:?}", computations);
        println!("Runtime (ms): {:?}", results);
    }
}

fn run_refining(mesh: Mesh, threadcount: usize) -> usize {
    let bad_queue = mesh.find_bad();

    mesh::refine(mesh, bad_queue, threadcount)
}
