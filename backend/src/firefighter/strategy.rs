use std::{cmp::min,
          collections::{BTreeMap, HashMap},
          fmt::Debug,
          sync::{Arc, RwLock}};

use strum::VariantNames;
use strum_macros::{EnumString, EnumVariantNames};

use crate::firefighter::{problem::{NodeDataStorage, OSMFSettings},
                         TimeUnit};
use crate::graph::Graph;

/// Strategy to contain the fire in the firefighter problem
#[derive(Debug, EnumString, EnumVariantNames)]
#[strum(serialize_all = "snake_case")]
pub enum OSMFStrategy {
    Greedy(GreedyStrategy),
    MinDistanceGroup(MinDistGroupStrategy),
    //Priority(PriorityStrategy),
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
    fn execute(&mut self, settings: &OSMFSettings, node_data: &mut NodeDataStorage, global_time: TimeUnit);
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

    fn execute(&mut self, settings: &OSMFSettings, node_data: &mut NodeDataStorage, global_time: TimeUnit) {
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
                tgt2_deg.cmp(&tgt1_deg)
            }));

        // Defend as many targets as firefighters are available
        let num_to_defend = min(edges.len(), settings.num_ffs);
        let to_defend: Vec<_> = edges[0..num_to_defend].iter()
            .map(|&e| e.tgt)
            .collect();
        log::debug!("Defending nodes {:?}", &to_defend);
        node_data.mark_defended(to_defend, global_time);
    }
}

/// Shortest distance based fire containment strategy
#[derive(Debug, Default)]
pub struct MinDistGroupStrategy {
    graph: Arc<RwLock<Graph>>,
    nodes_to_defend: Vec<usize>,
    current_defended: usize,
}

impl MinDistGroupStrategy {
    /// Compute nodes to defend and order in which nodes should be defended
    pub fn compute_nodes_to_defend(&mut self, roots: &Vec<usize>, settings: &OSMFSettings) {
        let graph = self.graph.read().unwrap();

        // For every node, compute the minimum shortest distance between the node and
        // any fire root
        let mut sho_dists = HashMap::with_capacity(graph.num_nodes);
        for &root in roots {
            for node in &graph.nodes {
                let new_dist = graph.unchecked_get_shortest_dist(root, node.id);
                if new_dist < usize::MAX {
                    sho_dists.entry(node.id)
                        .and_modify(|cur_dist| if new_dist < *cur_dist { *cur_dist = new_dist })
                        .or_insert(new_dist);
                }
            }

            log::debug!("Computed shortest distances to fire root {}", root);
        }

        // Group nodes by minimum shortest distance
        let mut nodes_by_sho_dist: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for (&node_id, &dist) in sho_dists.iter() {
            nodes_by_sho_dist.entry(dist)
                .and_modify(|nodes| nodes.push(node_id))
                .or_insert(vec![node_id]);
        }

        log::debug!("Grouped nodes by minimum shortest distance to any fire root");

        let strategy_every = settings.strategy_every as usize;
        let num_ffs = settings.num_ffs;
        let mut total_defended = 0;

        // Node groups that can be defended completely
        let defend_completely: Vec<_> = nodes_by_sho_dist.iter()
            .filter(|(&dist, nodes)| {
                let must_defend = nodes.len();
                let can_defend = dist / strategy_every * num_ffs - total_defended;
                if can_defend >= must_defend {
                    total_defended += must_defend;
                    true
                } else {
                    false
                }
            })
            .map(|(_, nodes)| nodes)
            .collect();

        // Node groups that can be defended partially
        let mut defend_partially = Vec::with_capacity(
            nodes_by_sho_dist.len() - defend_completely.len());
        for (&dist, nodes) in nodes_by_sho_dist.iter() {
            let must_defend = nodes.len();
            let could_defend_total = dist / strategy_every * num_ffs;
            if could_defend_total > total_defended {
                let can_defend = could_defend_total - total_defended;
                if can_defend < must_defend  {
                    total_defended += can_defend;
                    // Sort by out degree
                    let mut nodes = nodes.clone();
                    nodes.sort_unstable_by(|&n1, &n2| {
                        let deg1 = graph.get_out_degree(n1);
                        let deg2 = graph.get_out_degree(n2);
                        deg2.cmp(&deg1)
                    });
                    // Take first 'can_defend' number of nodes
                    defend_partially.push(nodes[0..can_defend].to_vec());
                }
            }
        }

        self.nodes_to_defend.reserve_exact(total_defended);

        for nodes in defend_completely {
            for &node in nodes {
                self.nodes_to_defend.push(node);
            }
        }

        for nodes in defend_partially {
            for node in nodes {
                self.nodes_to_defend.push(node);
            }
        }
    }
}

impl Strategy for MinDistGroupStrategy {
    fn new(graph: Arc<RwLock<Graph>>) -> Self {
        Self {
            graph,
            nodes_to_defend: vec![],
            current_defended: 0,
        }
    }

    fn execute(&mut self, settings: &OSMFSettings, node_data: &mut NodeDataStorage, global_time: TimeUnit) {
        let num_to_defend = min(settings.num_ffs, self.nodes_to_defend.len() - self.current_defended);
        let to_defend = &self.nodes_to_defend[self.current_defended..self.current_defended + num_to_defend];
        log::debug!("Defending nodes {:?}", to_defend);
        node_data.mark_defended2(to_defend, global_time);

        self.current_defended += num_to_defend;
    }
}



// #[cfg(test)]
// mod test {
//     use std::sync::{Arc, RwLock};
//
//     use rand::prelude::*;
//
//     use crate::firefighter::strategy::{OSMFStrategy, MinDistGroupStrategy, Strategy};
//     use crate::graph::Graph;
//
//     #[test]
//     fn test() {
//         let graph = Arc::new(RwLock::new(
//             Graph::from_files("data/bbgrund")));
//         let num_roots = 10;
//         let mut strategy = OSMFStrategy::MinDistanceGroup(MinDistGroupStrategy::new(graph.clone()));
//
//         let graph_ = graph.read().unwrap();
//         let num_nodes = graph_.num_nodes;
//
//         let mut rng = thread_rng();
//         let mut roots = Vec::with_capacity(num_roots);
//         while roots.len() < num_roots {
//             let root = rng.gen_range(0..num_nodes);
//             if !roots.contains(&root) {
//                 roots.push(root);
//             }
//         }
//
//         if let OSMFStrategy::MinDistanceGroup(ref mut mdg_strategy) = strategy {
//             mdg_strategy.compute_nodes_to_defend(&roots);
//         }
//
//         let some_node = rng.gen_range(0..num_nodes);
//         let mut dists_from_roots = Vec::with_capacity(num_roots);
//         for root in roots {
//             dists_from_roots.push(graph_.unchecked_get_shortest_dist(root, some_node));
//         }
//         let min_dist = dists_from_roots.iter().min().unwrap();
//
//         let default = Vec::new();
//         if let OSMFStrategy::MinDistanceGroup(mdg_strategy) = &strategy {
//             let group = mdg_strategy.nodes_by_sho_dist.get(min_dist)
//                 .unwrap_or(&default);
//             assert!(group.contains(&some_node), "min_dist = {}, group = {:?}, some_node = {}",
//                     min_dist, group, some_node);
//         }
//     }
//
// }
