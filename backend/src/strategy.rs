use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, RwLock};

use crate::firefighter::NodeDataStorage;
use crate::graph::Graph;

/// Strategy to contain the fire in the firefighter problem
#[derive(Debug)]
pub enum OSMFStrategy {
    Greedy(GreedyStrategy),
    ShortestDistance(ShoDistStrategy),
}

/// Strategy trait that each strategy needs to implement
pub trait Strategy {
    /// Create a new fire containment strategy instance
    fn new (graph: Arc<RwLock<Graph>>) -> Self;

    /// Execute the fire containment strategy
    fn execute(&mut self, node_data: &mut NodeDataStorage);
}

/// Greedy fire containment strategy
#[derive(Debug)]
pub struct GreedyStrategy {
    graph: Arc<RwLock<Graph>>,
}

impl Strategy for GreedyStrategy {
    fn new(graph: Arc<RwLock<Graph>>) -> Self {
        Self {
            graph,
        }
    }

    fn execute(&mut self, node_data: &mut NodeDataStorage) {
        todo!()
    }
}

/// Shortest distance based fire containment strategy
#[derive(Debug)]
pub struct ShoDistStrategy {
    graph: Arc<RwLock<Graph>>,
    pub sho_dists: HashMap<usize, usize>,
}

impl ShoDistStrategy {
    /// For every node, calculate the minimum shortest distance between the node and
    /// any fire root in `roots`
    pub fn compute_shortest_dists(&mut self, roots: &Vec<usize>) {
        let graph = self.graph.read().unwrap();
        for root in roots {
            for node in &graph.nodes {
                match graph.get_shortest_dist(*root, node.id) {
                    Ok(new_dist) => {
                        self.sho_dists.entry(node.id)
                            .and_modify(|cur_dist| if new_dist < *cur_dist { *cur_dist = new_dist })
                            .or_insert(new_dist);
                    }
                    Err(err) => {
                        log::warn!("{}", err.to_string());
                    }
                }
            }

            log::debug!("Computed shortest distances to fire root {}", root);
        }
    }
}

impl Strategy for ShoDistStrategy {
    fn new(graph: Arc<RwLock<Graph>>) -> Self {
        let num_nodes = graph.read().unwrap().num_nodes;
        Self {
            graph,
            sho_dists: HashMap::with_capacity(num_nodes),
        }
    }

    fn execute(&mut self, node_data: &mut NodeDataStorage) {
        todo!()
    }
}
