use std::{cmp::min,
          collections::BTreeMap,
          fmt::Debug,
          sync::{Arc, RwLock}};

use strum::VariantNames;
use strum_macros::{EnumString, EnumVariantNames};

use crate::firefighter::problem::{NodeDataStorage, OSMFSettings, TimeUnit};
use crate::graph::Graph;

/// Strategy to contain the fire in the firefighter problem
#[derive(Debug, EnumString, EnumVariantNames)]
#[strum(serialize_all = "snake_case")]
pub enum OSMFStrategy {
    Greedy(GreedyStrategy),
    ShortestDistance(ShoDistStrategy),
}

impl OSMFStrategy {
    /// Returns a list of available fire containment strategies
    pub fn available_strategies() -> Vec<String> {
        Self::VARIANTS.iter()
            .map(<&str>::to_string)
            .collect::<Vec<_>>()
    }
}

/// Strategy trait that each strategy needs to implement
pub trait Strategy {
    /// Create a new fire containment strategy instance
    fn new (graph: Arc<RwLock<Graph>>) -> Self;

    /// Execute the fire containment strategy
    fn execute(&mut self, settings: &OSMFSettings, node_data: &mut NodeDataStorage, global_time: TimeUnit) -> usize;
}

/// Greedy fire containment strategy
#[derive(Debug, Default)]
pub struct GreedyStrategy {
    graph: Arc<RwLock<Graph>>,
}

impl Strategy for GreedyStrategy {
    fn new(graph: Arc<RwLock<Graph>>) -> Self {
        Self {
            graph,
        }
    }

    fn execute(&mut self, settings: &OSMFSettings, node_data: &mut NodeDataStorage, global_time: TimeUnit) -> usize {
        let burning = node_data.get_burning();

        let graph = self.graph.read().unwrap();

        // Get all edges with targets that are not burned or defended yet
        let mut edges = Vec::new();
        for nd in burning {
            for i in graph.offsets[nd.node_id]..graph.offsets[nd.node_id+1] {
                let edge = &graph.edges[i];
                if node_data.is_undefended(&edge.tgt) {
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
        let num_to_defend = min(edges.len(), settings.num_firefighters);
        let to_defend: Vec<_> = edges[0..num_to_defend].iter()
            .map(|&e| e.tgt)
            .collect();
        log::debug!("Defending nodes {:?}", &to_defend);
        node_data.mark_defended(to_defend, global_time);

        num_to_defend
    }
}

/// Shortest distance based fire containment strategy
#[derive(Debug, Default)]
pub struct ShoDistStrategy {
    graph: Arc<RwLock<Graph>>,
    pub sho_dists: BTreeMap<usize, usize>,
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
        Self {
            graph,
            sho_dists: BTreeMap::new(),
        }
    }

    fn execute(&mut self, settings: &OSMFSettings, node_data: &mut NodeDataStorage, global_time: TimeUnit) -> usize {
        todo!()
    }
}
