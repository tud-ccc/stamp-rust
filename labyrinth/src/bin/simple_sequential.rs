#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

use clap::{App, Arg};
use labyrinth::grid;
use labyrinth::parser;
use labyrinth::pathfinder;
use labyrinth::types::{Maze, Point};
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::str::FromStr;
use time::PreciseTime;

fn main() {
    let matches = App::new("Sequential Labyrinth Benchmark")
        .version("1.0")
        .author("Felix Suchert <dev@felixsuchert.de>")
        .about("A Rust port of the labyrinth benchmark from the STAMP collection without any parallelism.")
        .arg(
            Arg::with_name("INPUT")
                .help("Input file to describe the grid and paths.")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("runs")
                .long("runs")
                .short("r")
                .takes_value(true)
                .help("The number of runs to conduct.")
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

    // JSON Dump?
    let json_dump = matches.is_present("json");
    let out_dir = matches.value_of("outdir").unwrap();

    // #runs
    let runs = usize::from_str(matches.value_of("runs").unwrap_or("1")).unwrap();

    let input_file = matches.value_of("INPUT").unwrap();
    let (dimensions, paths) = parser::parse_file(input_file);

    let mut results = Vec::with_capacity(runs);
    let mut mapped_paths = Vec::with_capacity(runs);
    let mut computations = Vec::with_capacity(runs);

    for _ in 0..runs {
        let maze = Maze::new(dimensions.clone(), None);

        if !json_dump {
            println!("[INFO] Loaded maze data from file.");
        }

        let start = PreciseTime::now();
        let filled_maze = route_paths(maze, paths.clone());
        let end = PreciseTime::now();

        if !json_dump {
            println!("[INFO] Routing complete.");
        }

        let runtime_ms = start.to(end).num_milliseconds();

        if filled_maze.is_valid() {
            results.push(runtime_ms);
            mapped_paths.push(filled_maze.paths.len());
            computations.push(paths.len());
        } else {
            eprintln!("Incorrect path mappings found in maze: {:?}", filled_maze);
            return;
        }
    }

    if json_dump {
        create_dir_all(out_dir).unwrap();
        let filename = format!(
            "{}/seq-{}-p{}-r{}_log.json",
            out_dir,
            dimensions,
            paths.len(),
            runs
        );
        let mut f = File::create(&filename).unwrap();
        f.write_fmt(format_args!(
            "{{
    \"application\": \"labyrinth\",
    \"algorithm\": \"sequential\",
    \"configuration\": \"{conf}\",
    \"paths\": {paths},
    \"threads\": 1,
    \"computations\": {comps:?},
    \"runs\": {runs},
    \"mapped\": {mapped:?},
    \"results\": {res:?}
}}",
            conf = dimensions,
            paths = paths.len(),
            runs = runs,
            comps = computations,
            mapped = mapped_paths,
            res = results
        ))
        .unwrap();
    } else {
        println!("[INFO] Successfully validated the maze.");
        println!("\nStatistics:");
        println!("    Maze configuration: {}", dimensions);
        println!("    Paths overall:      {}", paths.len());
        println!("    Runs:               {}", runs);
        println!("    Mapped:             {:?}", mapped_paths);
        println!("\nRouting Times: {:?} ms", results);
    }
}

fn route_paths(mut maze: Maze, mut to_map: Vec<(Point, Point)>) -> Maze {
    // search for a path for all point pairs (sort out any pairs w/o path)
    for pair in to_map.drain(..) {
        if let Some(path) = pathfinder::find_path(pair.clone(), &maze.grid) {
            grid::update_maze(&mut maze, path);
        } else {
            maze.unmappable_paths.push(pair);
        }
    }

    maze
}
