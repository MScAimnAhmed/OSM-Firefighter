use std::{cmp::{min, max},
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
        let num_to_defend = min(edges.len(), settings.num_firefighters);
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
    nodes_by_sho_dist: BTreeMap<usize, Vec<usize>>,
    nodes_to_defend: Vec<usize>,
    dist_to_defend: usize,
    remaining_ffs: usize,
    total_defended: usize,
}

impl MinDistGroupStrategy {
    /// Group all nodes by their minimum shortest distance to any fire root
    pub fn group_nodes_by_sho_dist(&mut self, roots: &Vec<usize>) {
        let graph = self.graph.read().unwrap();

        // For every node, compute the minimum shortest distance between the node and
        // any fire root
        let mut sho_dists: HashMap<usize, usize> = HashMap::with_capacity(graph.num_nodes);
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
        for (&node_id, &dist) in sho_dists.iter() {
            self.nodes_by_sho_dist.entry(dist)
                .and_modify(|nodes| nodes.push(node_id))
                .or_insert(vec![node_id]);
        }

        //log::debug!("Grouped nodes by minimum shortest distance to any fire root {:#?}",
            //self.nodes_by_sho_dist);
    }
}

impl Strategy for MinDistGroupStrategy {
    fn new(graph: Arc<RwLock<Graph>>) -> Self {
        Self {
            graph,
            nodes_by_sho_dist: BTreeMap::new(),
            nodes_to_defend: vec![],
            dist_to_defend: 0,
            remaining_ffs: 0,
            total_defended: 0,
        }
    }

    fn execute(&mut self, settings: &OSMFSettings, node_data: &mut NodeDataStorage, global_time: TimeUnit) {
        // Try to defend as many nodes, as firefighters are available
        let mut step_defended = 0;
        while step_defended < settings.num_firefighters {
            // (Re-)compute nodes to defend
            if self.nodes_to_defend.is_empty() {
                let graph = self.graph.read().unwrap();

                let next_dist = max(self.dist_to_defend + 1, global_time as usize);

                log::debug!("Next dist: {}", next_dist);

                // Filter by relevant distances, i.e. distances that the fire did not reach yet
                self.nodes_by_sho_dist.retain(|&dist, _| dist >= next_dist);

                let mut best_diff = isize::MIN;
                let mut best_dist = 0;
                for (&dist, nodes) in self.nodes_by_sho_dist.iter() {
                    let remaining_dist = dist + 1 - global_time as usize;
                    let num_ffs = settings.num_firefighters;
                    let strategy_every = settings.exec_strategy_every as usize;
                    let num_to_defend = dist / strategy_every * num_ffs + self.remaining_ffs - self.total_defended;

                    log::debug!("dist / strategy_every: {}", dist / strategy_every);
                    log::debug!("num to defend: {}", num_to_defend);
                    log::debug!("dist: {}", dist);
                    log::debug!("global time: {}", global_time);



                    if nodes.len() > 0 {
                        let diff = num_to_defend as isize - nodes.len() as isize;
                        if diff >= 0 {
                            log::debug!("Num to defend: {}, num nodes: {}", num_to_defend, nodes.len());

                            best_dist = dist;
                            best_diff = diff;
                            break;
                        } else if diff > best_diff {
                            best_diff = diff;
                            best_dist = dist;
                        }
                    }
                }
                if let Some(nodes) = self.nodes_by_sho_dist.get(&best_dist) {
                    self.nodes_to_defend = nodes.clone();
                    if best_diff < 0 {
                        self.nodes_to_defend.sort_unstable_by(|&n1, &n2| {
                            let deg1 = graph.get_out_degree(n1);
                            let deg2 = graph.get_out_degree(n2);
                            deg2.cmp(&deg1)
                        });
                        let d = best_diff.abs() as usize;
                        self.nodes_to_defend = self.nodes_to_defend[0..d].to_vec();
                    }
                    self.dist_to_defend = best_dist;
                    self.remaining_ffs = max(best_diff, 0) as usize;

                    log::debug!("Computed nodes to defend {:?}", &self.nodes_to_defend);
                } else {
                    // If nodes to defend are empty, even after (re-)computation,
                    // then use a greedy approach
                    log::debug!("Using greedy approach");

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
                    let num_to_defend = min(edges.len(), settings.num_firefighters - step_defended);
                    let to_defend: Vec<_> = edges[0..num_to_defend].iter()
                        .map(|&e| e.tgt)
                        .collect();
                    log::debug!("Defending nodes {:?}", &to_defend);
                    node_data.mark_defended(to_defend, global_time);

                    log::debug!("Total defended nodes in execution step: {}", step_defended + num_to_defend);
                    return;
                }
            }

            // If there are any undefended nodes, defend them and update the total number of
            // defended nodes
            let current_defended = min(self.nodes_to_defend.len(), settings.num_firefighters - step_defended);
            let mut to_defend = Vec::with_capacity(current_defended);
            for _ in 0..current_defended {
                to_defend.push(self.nodes_to_defend.remove(0));
            }
            log::debug!("Defending nodes {:?}", &to_defend);
            node_data.mark_defended(to_defend, global_time);

            step_defended += current_defended;
        }
        self.total_defended += step_defended;
        log::debug!("Step defended nodes in one execution step: {}", step_defended);
        log::debug!("Total defended nodes yet: {}", self.total_defended);
    }
}



#[cfg(test)]
mod test {
    use std::sync::{Arc, RwLock};

    use rand::prelude::*;

    use crate::firefighter::strategy::{OSMFStrategy, MinDistGroupStrategy, Strategy};
    use crate::graph::Graph;

    #[test]
    fn test() {
        let graph = Arc::new(RwLock::new(
            Graph::from_files("data/bbgrund")));
        let num_roots = 10;
        let mut strategy = OSMFStrategy::MinDistanceGroup(MinDistGroupStrategy::new(graph.clone()));

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

        if let OSMFStrategy::MinDistanceGroup(ref mut mdg_strategy) = strategy {
            mdg_strategy.group_nodes_by_sho_dist(&roots);
        }

        let some_node = rng.gen_range(0..num_nodes);
        let mut dists_from_roots = Vec::with_capacity(num_roots);
        for root in roots {
            dists_from_roots.push(graph_.unchecked_get_shortest_dist(root, some_node));
        }
        let min_dist = dists_from_roots.iter().min().unwrap();

        let default = Vec::new();
        if let OSMFStrategy::MinDistanceGroup(mdg_strategy) = &strategy {
            let group = mdg_strategy.nodes_by_sho_dist.get(min_dist)
                .unwrap_or(&default);
            assert!(group.contains(&some_node), "min_dist = {}, group = {:?}, some_node = {}",
                    min_dist, group, some_node);
        }
    }

}
