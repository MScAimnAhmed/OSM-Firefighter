use std::env;

use osmff_lib::firefighter::problem::{OSMFProblem, OSMFSettings};
use osmff_lib::firefighter::strategy::OSMFStrategy;

#[derive(Debug)]
struct BenchResults {
    avg_burned: f64,
    avg_def: f64,
    avg_end_time: f64,
}

fn main() {
    // Initialize logger
    env::set_var("RUST_LOG", "info");
    env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();

    let graphs = osmff_lib::load_graphs("data/")
        .expect("Failed to load graphs. Check whether 'data/' directory exists.");

    let args: Vec<_> = env::args().collect();

    if !args.contains(&"--graph".to_string()) {
        let err = "Missing required argument: --graph";
        log::error!("{}", err);
        panic!("{}", err);
    } else if !args.contains(&"--strategy".to_string()) {
        let err = "Missing required argument: --strategy";
        log::error!("{}", err);
        panic!("{}", err);
    }

    let mut settings = OSMFSettings {
        graph_name: "".to_string(),
        strategy_name: "".to_string(),
        num_roots: 1,
        num_ffs: 1,
        strategy_every: 1,
    };

    let mut loop_count: usize = 1;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--graph" => {
                settings.graph_name = args[i+1].clone();
            }
            "--strategy" => {
                settings.strategy_name = args[i+1].clone();
            }
            "-r" => {
                settings.num_roots = args[i+1].parse()
                    .expect("Invalid argument: num_roots");
            }
            "-f" => {
                settings.num_ffs = args[i+1].parse()
                    .expect("Invalid argument: num_ffs");
            }
            "-e" => {
                settings.strategy_every = args[i+1].parse()
                    .expect("Invalid argument: strategy_every");
            }
            "--loop" => {
                loop_count = args[i+1].parse()
                    .expect("Invalid argument: loop_count");
            }
            _ => {
                panic!("Unknown argument: {}", &args[i]);
            }
        }
        i += 2;
    }

    log::info!("Benchmarking with the following problem settings: {:?}", &settings);
    log::info!("Loop count: {}", loop_count);
    log::info!("Starting benchmarks");

    let graph = graphs.get(&settings.graph_name)
        .expect("No such graph parsed");
    let mut sum_burned = 0;
    let mut sum_defended = 0;
    let mut sum_end_time = 0;
    for _ in 0..loop_count {
        let strategy = OSMFStrategy::from_name_and_graph(&settings.strategy_name, graph.clone())
            .expect("Invalid strategy specified");
        let mut problem = OSMFProblem::new(graph.clone(), settings.clone(), strategy)
            .expect("Invalid simulation settings");

        problem.simulate();

        let results = problem.simulation_response();
        sum_burned += results.nodes_burned;
        sum_defended += results.nodes_defended;
        sum_end_time += results.end_time;
    }

    let bench_results = BenchResults {
        avg_burned: sum_burned as f64 / loop_count as f64,
        avg_def: sum_defended as f64 / loop_count as f64,
        avg_end_time: sum_end_time as f64 / loop_count as f64,
    };

    log::info!("Benchmark results:\n{:#?}", bench_results);
}