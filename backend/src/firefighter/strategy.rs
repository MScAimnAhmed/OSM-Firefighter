use std::{cmp::min,
          collections::{BTreeMap, HashMap},
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
    nodes_by_sho_dist: BTreeMap<usize, Vec<usize>>,
    residual_firefighters: usize,
    nodes_to_defend: Vec<usize>,
    dist_to_defend: usize,
    total_defended_nodes: usize
}

impl ShoDistStrategy {
    /// Group all nodes by their minimum shortest distance to any fire root
    pub fn group_nodes_by_sho_dist(&mut self, roots: &Vec<usize>) {
        let graph = self.graph.read().unwrap();

        // For every node, compute the minimum shortest distance between the node and
        // any fire root
        let mut sho_dists: HashMap<usize, usize> = HashMap::with_capacity(graph.num_nodes);
        for &root in roots {
            for node in &graph.nodes {
                let new_dist = graph.unchecked_get_shortest_dist(root, node.id);
                sho_dists.entry(node.id)
                    .and_modify(|cur_dist| if new_dist < *cur_dist { *cur_dist = new_dist })
                    .or_insert(new_dist);
            }

            log::debug!("Computed shortest distances to fire root {}", root);
        }

        // Group nodes by minimum shortest distance
        for (&node_id, &dist) in sho_dists.iter() {
            self.nodes_by_sho_dist.entry(dist)
                .and_modify(|nodes| nodes.push(node_id))
                .or_insert(vec![node_id]);
        }

        log::debug!("Grouped nodes by minimum shortest distance to any fire root {:#?}",
            self.nodes_by_sho_dist);
    }
}

impl Strategy for ShoDistStrategy {
    fn new(graph: Arc<RwLock<Graph>>) -> Self {
        Self {
            graph,
            nodes_by_sho_dist: BTreeMap::new(),
            residual_firefighters: 0,
            nodes_to_defend: vec![],
            dist_to_defend: 0,
            total_defended_nodes: 0
        }
    }

    fn execute(&mut self, settings: &OSMFSettings, node_data: &mut NodeDataStorage, global_time: TimeUnit) -> usize {
        if self.nodes_to_defend.is_empty() {
            let next_dist = self.dist_to_defend + 1;
            for dist in next_dist..=*self.nodes_by_sho_dist.keys().max().unwrap_or(&next_dist) {
                if let Some(nodes) = self.nodes_by_sho_dist.get(&dist) {
                    let n = settings.num_firefighters as isize;
                    let e = settings.exec_strategy_every as isize;
                    let num = ((((dist as isize - 1) / e) + 1) * n) + self.residual_firefighters as isize - self.total_defended_nodes as isize;

                    if nodes.len() as isize <= num && nodes.len() > 0 {
                        self.nodes_to_defend = nodes.clone();
                        self.residual_firefighters = num as usize - nodes.len();
                        self.dist_to_defend = dist;
                        self.total_defended_nodes += nodes.len();
                        break;
                    }
                }
            }
        }

        // Defend as many targets as firefighters are available
        let num_to_defend = min(self.nodes_to_defend.len(), settings.num_firefighters);
        let mut to_defend = Vec::with_capacity(num_to_defend);
        for _ in 0..num_to_defend {
            let node = self.nodes_to_defend.pop().unwrap();
            to_defend.push(node);
        }
        log::debug!("Defending nodes {:?}", &to_defend);
        node_data.mark_defended(to_defend, global_time);

        num_to_defend
    }
}



#[cfg(test)]
mod test {
    use std::sync::{Arc, RwLock};

    use rand::prelude::*;

    use crate::firefighter::strategy::{OSMFStrategy, ShoDistStrategy, Strategy};
    use crate::graph::Graph;

    #[test]
    fn test() {
        let graph = Arc::new(RwLock::new(
            Graph::from_files("data/bbgrund")));
        let num_roots = 10;
        let mut strategy = OSMFStrategy::ShortestDistance(ShoDistStrategy::new(graph.clone()));

        let graph_ = graph.read().unwrap();
        let num_nodes = graph_.num_nodes;

        let mut rng = thread_rng();
        let mut roots = Vec::with_capacity(num_roots);
        while roots.len() < num_roots {
            let root = rng.gen_range(0..num_nodes);
            if !roots.contains(&root) {
                roots.push(root);
            }
        }

        if let OSMFStrategy::ShortestDistance(ref mut sd_strategy) = strategy {
            sd_strategy.group_nodes_by_sho_dist(&roots);
        }

        let some_node = rng.gen_range(0..num_nodes);
        let mut dists_from_roots = Vec::with_capacity(num_roots);
        for root in roots {
            dists_from_roots.push(graph_.unchecked_get_shortest_dist(root, some_node));
        }
        let min_dist = dists_from_roots.iter().min().unwrap();

        let default = Vec::new();
        if let OSMFStrategy::ShortestDistance(sd_strategy) = &strategy {
            let group = sd_strategy.nodes_by_sho_dist.get(min_dist)
                .unwrap_or(&default);
            assert!(group.contains(&some_node), "min_dist = {}, group = {:?}, some_node = {}",
                    min_dist, group, some_node);
        }
    }

}
