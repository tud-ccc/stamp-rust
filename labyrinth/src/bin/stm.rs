#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

use clap::{App, Arg};
use cpu_time::ProcessTime;
use labyrinth::parser;
use labyrinth::pathfinder;
use labyrinth::stm_grid;
use labyrinth::types::{Maze, Path, Point, StmGrid};
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::str::FromStr;
use std::thread;
use stm::atomically;
use time::PreciseTime;

fn main() {
    let matches = App::new("STM Labyrinth Benchmark")
        .version("1.0")
        .author("Felix Suchert <dev@felixsuchert.de>")
        .about("A Rust port of the labyrinth benchmark from the STAMP collection using software transactional memory.")
        .arg(
            Arg::with_name("INPUT")
                .help("Input file to describe the grid and paths.")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("threadcount")
                .long("threads")
                .short("t")
                .takes_value(true)
                .help("The number of threads to run on.")
                .required(true)
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

    // thread number
    let thread_number = usize::from_str(matches.value_of("threadcount").unwrap())
        .expect("The entered thread count was not a valid uint");

    // JSON Dump?
    let json_dump = matches.is_present("json");
    let out_dir = matches.value_of("outdir").unwrap();

    // #runs
    let runs = usize::from_str(matches.value_of("runs").unwrap_or("1")).unwrap();

    // input location & parsing
    let input_file = matches.value_of("INPUT").unwrap();
    let (dimensions, paths) = parser::parse_file(input_file);

    let mut results = Vec::with_capacity(runs);
    let mut cpu_results = Vec::with_capacity(runs);
    let mut mapped_paths = Vec::with_capacity(runs);
    let mut retry_counts = Vec::with_capacity(runs);
    let mut computations = Vec::with_capacity(runs);

    for _ in 0..runs {
        let maze = Maze::new(dimensions.clone(), None);

        if !json_dump {
            println!("[INFO] Loaded maze data from file.");
        }

        let start = PreciseTime::now();
        let cpu_start = ProcessTime::now();
        let (filled_maze, retries) = route_paths(maze, paths.clone(), thread_number);
        let cpu_end = ProcessTime::now();
        let end = PreciseTime::now();

        if !json_dump {
            println!("[INFO] Routing complete.");
        }

        let runtime_ms = start.to(end).num_milliseconds();
        let cpu_runtime_ms = cpu_end.duration_since(cpu_start).as_millis();

        if filled_maze.is_valid() {
            results.push(runtime_ms);
            cpu_results.push(cpu_runtime_ms);
            mapped_paths.push(filled_maze.paths.len());
            retry_counts.push(retries);
            computations.push(retries + paths.len());
        } else {
            eprintln!("Incorrect path mappings found in maze: {:?}", filled_maze);
            return;
        }
    }

    if json_dump {
        create_dir_all(out_dir).unwrap();
        let algo = if cfg!(feature = "naive") {
            "stm-naive"
        } else {
            "stm"
        };
        let filename = format!(
            "{}/{}-{}-p{}-t{}-r{}_log.json",
            out_dir,
            algo,
            dimensions,
            paths.len(),
            thread_number,
            runs
        );
        let mut f = File::create(&filename).unwrap();
        f.write_fmt(format_args!(
            "{{
    \"algorithm\": \"{algo}\",
    \"configuration\": \"{conf}\",
    \"paths\": {paths},
    \"threadcount\": {threads},
    \"runs\": {runs},
    \"mapped\": {mapped:?},
    \"collisions\": {collisions:?},
    \"computations\": {comps:?},
    \"cpu_time\": {cpu:?},
    \"results\": {res:?}
}}",
            algo = algo,
            conf = dimensions,
            paths = paths.len(),
            threads = thread_number,
            runs = runs,
            mapped = mapped_paths,
            collisions = retry_counts,
            comps = computations,
            cpu = cpu_results,
            res = results
        ))
        .unwrap();
    } else {
        println!("[INFO] Successfully validated the maze.");
        println!("\nStatistics:");
        println!("    Maze configuration: {}", dimensions);
        println!("    Thread number:      {}", thread_number);
        println!("    Paths overall:      {}", paths.len());
        println!("    Runs:               {}", runs);
        println!("    Mapped:             {:?}", mapped_paths);
        println!("    Collisions:         {:?}", retry_counts);
        println!("    Computations:       {:?}", computations);
        println!("\nCPU time: {:?} ms", cpu_results);
        println!("Routing Time: {:?} ms", results);
    }
}

fn route_paths(
    mut maze: Maze,
    mut to_map: Vec<(Point, Point)>,
    thread_number: usize,
) -> (Maze, usize) {
    let mut global_retries = 0;
    // partition the vec
    let l = to_map.len() / thread_number;
    let mut rest = to_map.len() % thread_number;

    let mut paths_to_map = vec![Vec::with_capacity(l); thread_number];

    for t_num in 0..thread_number {
        if rest > 0 {
            paths_to_map[t_num] = to_map.split_off(to_map.len() - l - 1);
            rest -= 1;
        } else {
            if to_map.len() <= l {
                paths_to_map[t_num] = to_map.split_off(0);
            } else {
                paths_to_map[t_num] = to_map.split_off(to_map.len() - l);
            }
        }
    }

    let mut handles = Vec::new();

    for points in paths_to_map.drain(..) {
        let g = maze.grid.clone();
        handles.push(thread::spawn(move || route(&g, points)));
    }

    for handle in handles {
        let (mut mapped, mut not_mapped, retries) = handle.join().unwrap();
        maze.paths.append(&mut mapped);
        maze.unmappable_paths.append(&mut not_mapped);
        global_retries += retries;
    }

    (maze, global_retries)
}

/// Attempts to route the paths from `to_map` on he grid using STM.
fn route(
    grid: &StmGrid,
    mut to_map: Vec<(Point, Point)>,
) -> (Vec<Path>, Vec<(Point, Point)>, usize) {
    let mut mapped = Vec::new();
    let mut unmappable_paths = Vec::new();
    let mut overall_retries = 0;

    // search for a path for all point pairs (sort out any pairs w/o path)
    for pair in to_map.drain(..) {
        #[cfg(feature = "naive")]
        let ta_result = atomically(|trans| {
            if let Some(path) = pathfinder::find_path(pair.clone(), &grid, trans)? {
                stm_grid::update_grid(&grid, &path, trans)?;
                Ok(Some(path))
            } else {
                Ok(None)
            }
        });

        #[cfg(not(feature = "naive"))]
        let ta_result = atomically(|trans| {
            let copy_grid = stm_grid::create_working_copy(&grid);
            if let Some(path) = pathfinder::find_path(pair.clone(), &copy_grid) {
                stm_grid::update_grid(&grid, &path, trans)?;
                Ok(Some(path))
            } else {
                Ok(None)
            }
        });

        match ta_result {
            (Some(path), retries) => {
                mapped.push(path);
                overall_retries += retries;
            }
            (None, retries) => {
                unmappable_paths.push(pair);
                overall_retries += retries;
            }
        }
    }

    (mapped, unmappable_paths, overall_retries)
}
