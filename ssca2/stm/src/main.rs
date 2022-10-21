use crate::generator::GraphSDG;
use crate::graph::Graph;
use crate::parameters::Parameters;
use clap::Parser;
use std::fs::{create_dir_all, File};
use std::io::Write;
use cpu_time::ProcessTime;

mod generator;
mod graph;
mod parameters;

fn main() {
    let mut params = Parameters::parse();
    params.process();

    let mut results = Vec::with_capacity(params.runs);
    let mut cpu_results = Vec::with_capacity(params.runs);

    for r in 0..params.runs {
        let p = params.clone();

        let start_gen = std::time::Instant::now();
        let start_gen_cpu = ProcessTime::now();
        let sdg_data = GraphSDG::threaded_generate(p);
        let end_gen_cpu = ProcessTime::now();
        let end_gen = std::time::Instant::now();

        let start_comp = std::time::Instant::now();
        let start_comp_cpu = ProcessTime::now();
        #[allow(unused_variables)]
        let graph = Graph::compute(sdg_data, params.threads);
        let end_comp_cpu = ProcessTime::now();
        let end_comp = std::time::Instant::now();

        let gen_duration = end_gen.duration_since(start_gen);
        let comp_duration = end_comp.duration_since(start_comp);

        let total = gen_duration + comp_duration;
        let total_cpu = end_comp_cpu.duration_since(start_comp_cpu) + end_gen_cpu.duration_since(start_gen_cpu);

        results.push(total.as_millis());
        cpu_results.push(total_cpu.as_millis());

        if !params.json {
            println!("[run {:2.}] Time taken:", r);
            println!("         Data Generation: {} ms", gen_duration.as_millis());
            println!(
                "         Kernel 1 Computation: {} ms",
                comp_duration.as_millis()
            );
            println!("         Total: {} ms", total.as_millis());
        }
    }


    if params.json {
        create_dir_all(&params.outdir).unwrap();
        let filename = format!("{}/stm-s{}-i{:.1}-u{:.1}-l{}-p{}-t{}-r{}_log.json",
            params.outdir,
            params.scale,
            params.probability_interclique_edges,
            params.probability_unidirectional,
            params.subgraph_edge_length,
            params.max_parallel_edges,
            params.threads,
            params.runs,
        );

        let mut f = File::create(&filename).unwrap();
        f.write_fmt(format_args!(
            "{{
    \"application\": \"ssca2\",
    \"algorithm\": \"sequential\",
    \"threadcount\": {threadcount},
    \"scaling-heuristic\": {scale},
    \"probability-interclique\": {prob_ie},
    \"probability-unidirectional\": \"{prob_u}\",
    \"max-path-length\": {sub_edge_len},
    \"max-par-edges\": {mpe},
    \"runs\": {runs},
    \"cpu_time\": {cpu:?},
    \"results\": {res:?}
}}",
            threadcount = params.threads,
            scale = params.scale,
            prob_ie = params.probability_interclique_edges,
            prob_u = params.probability_unidirectional,
            sub_edge_len = params.subgraph_edge_length,
            mpe = params.max_parallel_edges,
            runs = params.runs,
            cpu = cpu_results,
            res = results
        ))
        .unwrap();
    } else {
        println!("[INFO] All runs completed successfully.");
        println!("\nStatistics:");
        println!("    [...]");
        println!("    Threads: {}", params.threads);
        println!("    Runs: {}", params.runs);
        println!("\nCPU-time used (ms): {:?}", cpu_results);
        println!("Runtime in ms: {:?}", results);
    }

}
