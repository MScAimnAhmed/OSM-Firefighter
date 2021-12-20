use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, RwLock};
use crate::graph::Graph;

pub mod error;
pub mod graph;
pub mod session;
pub mod firefighter;
pub mod query;
pub mod binary_minheap;

/// Load all available graphs from `graphs_path`
pub fn load_graphs(graphs_path: &str) -> HashMap<String, Arc<RwLock<Graph>>> {
    let paths: Vec<_> = match fs::read_dir(graphs_path) {
        Ok(paths) => paths
            .filter_map(|path| path.ok())
            .filter(|path| path.path().to_str()
                .expect("Invalid unicode path")
                .ends_with(".fmi"))
            .collect(),
        Err(err) => panic!("{}", err.to_string())
    };

    let mut graphs = HashMap::with_capacity(graphs_path.len());
    for path in paths {
        let file_name = path.file_name().to_str().unwrap().split(".").collect::<Vec<_>>()[0].to_string();
        let file_path = path.path().to_str().unwrap().to_string();
        graphs.entry(file_name.clone()).or_insert_with(|| {
            let graph = Arc::new(RwLock::new(Graph::from_file(&file_path)));

            log::info!("Loaded graph {}", file_name);

            graph
        });
    }

    graphs
}