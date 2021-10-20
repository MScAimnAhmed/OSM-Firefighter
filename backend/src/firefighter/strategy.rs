use std::{cmp::min,
          collections::HashMap,
          fmt::Debug,
          sync::{Arc, RwLock}};

use crate::firefighter::problem::{NodeDataStorage, OSMFSettings, TimeUnit, NodeState};
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
    fn execute(&mut self, settings: &OSMFSettings, node_data: &mut NodeDataStorage, global_time: TimeUnit) -> Vec<usize>;
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

    fn execute(&mut self, settings: &OSMFSettings, node_data: &mut NodeDataStorage, global_time: TimeUnit) -> Vec<usize> {
        let graph = self.graph.read().unwrap();

        let burning = node_data.get_all_burning();

        // Get all edges with targets that are not burned or defended yet
        let mut edges = Vec::new();
        for nd in burning {
            for i in graph.offsets[nd.node_id]..graph.offsets[nd.node_id+1] {
                let edge = &graph.edges[i];
                if !node_data.is_node_data_attached(&edge.tgt) {
                    edges.push(edge);
                }
            }
        }

        // Sort the edges by their weight and by the _out degree_ of their targets
        edges.sort_unstable_by(|&e1, &e2|
            e1.dist.cmp(&e2.dist).then_with(|| {
                let tgt1_deg = graph.get_out_degree(e1.tgt);
                let tgt2_deg = graph.get_out_degree(e2.tgt);
                tgt1_deg.cmp(&tgt2_deg)
            }));

        // Defend as many targets as firefighters are available
        let num_defended = min(edges.len(), settings.num_firefighters);
        let mut defended = Vec::with_capacity(num_defended);
        for edge in &edges[0..num_defended] {
            node_data.attach_node_data(edge.tgt, NodeState::Defended, global_time);
            defended.push(edge.tgt);

            log::debug!("Node {} is defended", edge.tgt);
        }
        defended
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

    fn execute(&mut self, settings: &OSMFSettings, node_data: &mut NodeDataStorage, global_time: TimeUnit) -> Vec<usize> {
        todo!()
    }
}