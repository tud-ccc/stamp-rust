#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

use clap::{App, Arg};
use genome::gene::Gene;
use genome::segments::Segments;
use genome::stm_sequencer as sequencer;
use genome::Nucleotide;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha12Rng;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::str::FromStr;
use std::sync::Arc;
use time::PreciseTime;
use cpu_time::ProcessTime;

fn main() {
    let matches = App::new("STM genome benchmark")
        .version("1.0")
        .author("Felix Suchert <dev@felixsuchert.de>")
        .about("A Rust port of the genome benchmark from the STAMP collection, implemented in STM.")
        .arg(
            Arg::with_name("genelength")
                .long("gene-length")
                .short("g")
                .help("Length of the gene")
                .takes_value(true)
                .default_value("16384"),
        )
        .arg(
            Arg::with_name("minnumber")
                .long("min-number")
                .short("n")
                .help("The minimal number of segments")
                .takes_value(true)
                .default_value("4194304"),
        )
        .arg(
            Arg::with_name("seglength")
                .long("segment-length")
                .short("s")
                .help("Length of a gene segment")
                .takes_value(true)
                .default_value("64"),
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
        .arg(
            Arg::with_name("threads")
                .long("threads")
                .short("t")
                .help("Sets the number of threads to use for computation")
                .takes_value(true)
                .default_value("4"),
        )
        .get_matches();

    // parse benchmark parameters
    let gene_length = usize::from_str(matches.value_of("genelength").unwrap())
        .expect("provided invalid value for `gene-length`");
    let min_number = usize::from_str(matches.value_of("minnumber").unwrap())
        .expect("provided invalid value for `min-number`");
    let segment_length = usize::from_str(matches.value_of("seglength").unwrap())
        .expect("provided invalid value for `segment-length`");

    // parse runtime parameters
    let runs =
        usize::from_str(matches.value_of("runs").unwrap()).expect("Could not parse number of runs");
    let json_dump = matches.is_present("json");
    let out_dir = matches.value_of("outdir").unwrap();
    let threads = usize::from_str(matches.value_of("threads").unwrap())
        .expect("provided invalid value for `threads`");

    // generate the gene and its segments
    let mut rng = ChaCha12Rng::seed_from_u64(0);

    let mut gene = Gene::create(gene_length, &mut rng);
    let segments = Segments::create(segment_length, min_number, &mut gene, &mut rng);
    if !json_dump {
        println!(
            "[INFO] Generated {} gene segments.",
            segments.contents.len()
        );
        // println!(
        //     "[DEBUG] Gene (still) has {} Nucleotides.",
        //     gene.contents.len()
        // );
    }

    let mut results = Vec::with_capacity(runs);
    let mut cpu_results = Vec::with_capacity(runs);

    for r in 0..runs {
        // prepare the data for the run
        let input_data = segments.clone();

        // start the clock
        let start = PreciseTime::now();
        let cpu_start = ProcessTime::now();

        // run the algorithm
        let result = run_benchmark(input_data, threads);

        // stop the clock
        let cpu_end = ProcessTime::now();
        let end = PreciseTime::now();
        let runtime_ms = start.to(end).num_milliseconds();
        let cpu_runtime_ms = cpu_end.duration_since(cpu_start).as_millis();

        if !json_dump {
            println!("[INFO] Genome sequencing run {} completed.", r + 1);
        }

        if result.len() != gene.contents.len() {
            eprintln!("[ERROR] Output verification failed. An error occured during genome sequencing. Sequenced genome length deviated from the original genome size ({}/{})", result.len(), gene.contents.len());
        } else {
            results.push(runtime_ms);
            cpu_results.push(cpu_runtime_ms);
        }
    }

    // generate output
    if json_dump {
        create_dir_all(out_dir).unwrap();
        let filename = format!(
            "{}/stm-g{}-n{}-s{}-t{}-r{}_log.json",
            out_dir, gene_length, min_number, segment_length, threads, runs
        );
        let mut f = File::create(&filename).unwrap();
        f.write_fmt(format_args!(
            "{{
    \"algorithm\": \"stm\",
    \"gene_length\": {gene_len},
    \"min_segment_count\": {min_segment},
    \"segment_length\": {seg_len},
    \"threadcount\": {threads},
    \"runs\": {runs},
    \"cpu_time\": {cpu:?},
    \"results\": {res:?}
}}",
            gene_len = gene_length,
            min_segment = min_number,
            seg_len = segment_length,
            threads = threads,
            runs = runs,
            cpu = cpu_results,
            res = results
        ))
        .unwrap();
    } else {
        println!("[INFO] All runs completed successfully.");
        println!("\nStatistics:");
        println!("    Length of the generated gene: {}", gene_length);
        println!("    Minimal number of segments:   {}", min_number);
        println!("    Length of a gene segment:     {}", segment_length);
        println!("    Threads:                      {}", threads);
        println!("    Runs:                         {}", runs);
        println!("\nCPU-time used (ms): {:?}", cpu_results);
        println!("Runtime in ms: {:?}", results);
    }
}

fn run_benchmark(segments: Segments, threadcount: usize) -> Vec<Nucleotide> {
    // Phase 1
    let segment_length = segments.length;

    let deduplicated = Arc::new(sequencer::deduplicate(segments, threadcount));

    // Phase 2
    let step: usize = deduplicated.len() / threadcount;
    let mut ranges = Vec::new();
    for t_no in 0..threadcount {
        let lower = step * t_no;
        let upper = if t_no+1 < threadcount {
            step * (t_no + 1)
        } else {
            deduplicated.len()
        };
        ranges.push(lower..upper);
    }

    sequencer::run_sequencer(deduplicated.clone(), segment_length, ranges);

    // Phase 3
    sequencer::reconstruct(deduplicated.clone())
}
