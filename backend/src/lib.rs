pub mod graph;
pub mod firefighter;
pub(crate) mod binary_minheap;

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::sync::Arc;

use crate::graph::Graph;

/// Load all available graphs from `graphs_path`.
/// Returns an `OSMFResult` containing  a `HashMap` with entries that allow to access shared
/// references to the graphs by their respective names if the operation succeeds, or an `Err`
/// otherwise.
pub fn load_graphs(graphs_path: &str) -> Result<HashMap<String, Arc<Graph>>, Box<dyn Error>> {
    match fs::read_dir(graphs_path) {
        Ok(paths) => {
            // Collect names and paths of files containing graphs
            let graph_data: Vec<_> = paths
                .filter_map(|path| path.ok())
                .filter(|path| path.path().to_str()
                    .expect("Invalid unicode path")
                    .ends_with(".fmi"))
                .map(|graph_path| {
                    let graph_name = graph_path.file_name().to_str().unwrap()
                        .split(".fmi").next().unwrap().to_string();
                    let graph_path = graph_path.path().to_str().unwrap().to_string();
                    (graph_name, graph_path)
                })
                .collect();

            // Parse and load graphs into a map
            let mut graphs = HashMap::with_capacity(graph_data.len());
            for (graph_name, graph_path) in graph_data {
                match Graph::parse_from_file(&graph_path) {
                    Ok(graph) => {
                        log::info!("Parsed graph: {}", &graph_name);
                        graphs.insert(graph_name, Arc::new(graph))
                    }
                    Err(err) => {
                        log::warn!("Failed to parse graph: {}", &graph_name);
                        return Err(err.into());
                    }
                };
            }

            Ok(graphs)
        }
        Err(err) => Err(err.into())
    }
}