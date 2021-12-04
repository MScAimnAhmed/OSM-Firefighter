use std::{cmp::min,
          collections::{BTreeMap, HashMap},
          fmt::Debug,
          sync::{Arc, RwLock}};

use rand::seq::SliceRandom;
use rand::prelude::*;

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
    Priority(PriorityStrategy),
    Random(RandomStrategy)
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
    fn execute(&mut self, settings: &OSMFSettings, node_data: &mut NodeDataStorage, global_time: TimeUnit, contained_root: bool, undefended_roots: &Vec<usize>);
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

    fn execute(&mut self, settings: &OSMFSettings, node_data: &mut NodeDataStorage, global_time: TimeUnit, contained_root: bool, undefended_roots: &Vec<usize>) {
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
        node_data.mark_defended(&to_defend, global_time);
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

    fn execute(&mut self, settings: &OSMFSettings, node_data: &mut NodeDataStorage, global_time: TimeUnit, contained_root: bool, undefended_roots: &Vec<usize>) {
        // One or more fire roots have been defended and hence shouldn't be considered
        // in the min_distance_groups anymore
        if contained_root {
            self.compute_nodes_to_defend(undefended_roots, settings);
        }

        let num_to_defend = min(settings.num_ffs, self.nodes_to_defend.len() - self.current_defended);
        let to_defend = &self.nodes_to_defend[self.current_defended..self.current_defended + num_to_defend];
        log::debug!("Defending nodes {:?}", to_defend);

        for node in to_defend {
            assert!(node_data.is_undefended(node));
        }

        node_data.mark_defended2(to_defend, global_time);

        self.current_defended += num_to_defend;
    }
}

/// Priority based fire containment strategy
#[derive(Debug, Default)]
pub struct PriorityStrategy {
    graph: Arc<RwLock<Graph>>,
    nodes_to_defend: Vec<usize>,
    current_defended: usize,
}

impl PriorityStrategy {
    /// Compute nodes to defend and order in which nodes should be defended
    pub fn compute_nodes_to_defend(&mut self, roots: &Vec<usize>, settings: &OSMFSettings, node_data: &NodeDataStorage) {
        let graph = self.graph.read().unwrap();
        let mut priority_map = HashMap::with_capacity(graph.num_nodes);

        for node in &graph.nodes {
            if node_data.is_undefended(&node.id) {
                //let prio = 2 * graph.get_in_degree(node.id) + (5 * graph.get_out_degree(node.id));
                let prio = graph.get_out_degree(node.id);
                priority_map.insert(node.id, prio);
                //log::debug!("node: {}, in_deg {}, out_deg {} -> prio: {}", node.id, graph.get_in_degree(node.id), graph.get_out_degree(node.id), prio);
            } else {
                let prio = 0;
                priority_map.insert(node.id, prio);
            }
        }
        log::debug!("priority map: {:?}", priority_map);

        let mut sorted_priorities: Vec<_> = priority_map.values().map(|prio|*prio).collect();
        sorted_priorities.sort_unstable_by(|p1, p2| {
            p1.cmp(p2)
        });

        log::debug!("sorted prios {:?}", sorted_priorities);
        // let mid = graph.num_nodes / 2;
        // log::debug!("num of nodes {}, mid {}", graph.num_nodes, mid);
        /*let mut median = {
            if mid % 2 == 0 {
                log::debug!("mid - 1 {}, and mid {} -> median {}", sorted_priorities[mid - 1], sorted_priorities[mid], (sorted_priorities[mid - 1] + sorted_priorities[mid]) / 2);
                (sorted_priorities[mid - 1] + sorted_priorities[mid]) / 2
            } else {
                log::debug!("median {}", sorted_priorities[mid]);
                sorted_priorities[mid]
            }
        };*/

        let median = sorted_priorities.iter().sum::<usize>() as f64 / sorted_priorities.len() as f64;
        log::debug!("median {}", median);

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

        // Sort Node groups by priority
        for (_, nodes) in nodes_by_sho_dist.iter_mut() {
            nodes.sort_unstable_by(|n1, n2| {
                let prio1 = priority_map.get(n1).unwrap();
                let prio2 = priority_map.get(n2).unwrap();
                prio2.cmp(prio1)
            });
        }

        let strategy_every = settings.strategy_every as usize;
        let num_ffs = settings.num_ffs;
        let mut total_defended = 0;

        // Filter nodes with higher priority based on median
        let mut high_prio_map: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for (&dist, nodes) in nodes_by_sho_dist.iter() {
            let high_prio_nodes: Vec<_> = nodes.iter()
                .filter(|&node| {
                    *priority_map.get(node).unwrap() as f64 >= median
                })
                .map(|node| *node)
                .collect();
            high_prio_map.insert(dist, high_prio_nodes);
        }

        // Filter nodes with higher priority based on median
        let mut low_prio_map: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for (&dist, nodes) in nodes_by_sho_dist.iter() {
            let low_prio_nodes: Vec<_> = nodes.iter()
                .filter(|&node| {
                    (*priority_map.get(node).unwrap() as f64) < median
                })
                .map(|node| *node)
                .collect();
            low_prio_map.insert(dist, low_prio_nodes);
        }

        // Nodes with a higher priority than the median should be defended
        let mut high_prio_defend = Vec::new();
        for (&dist, nodes) in high_prio_map.iter() {
            let can_defend = dist / strategy_every * num_ffs - total_defended;
            let num_of_nodes = min(can_defend, nodes.len());
            high_prio_defend.reserve(num_of_nodes);
            for &node in &nodes[0..num_of_nodes] {
                high_prio_defend.push(node);
            }
            total_defended += num_of_nodes;
        }

        // Nodes with a lower priority than the median should be defended
        let mut low_prio_defend = Vec::with_capacity(graph.num_nodes - high_prio_defend.len());
        for (&dist, nodes) in low_prio_map.iter() {
            let can_defend_total = dist / strategy_every * num_ffs;
            if can_defend_total > total_defended {
                let can_defend = can_defend_total - total_defended;
                let num_of_nodes = min(can_defend, nodes.len());
                for &node in &nodes[0..num_of_nodes] {
                    low_prio_defend.push(node);
                }
                total_defended += num_of_nodes;
            }
        }
        assert!(high_prio_defend.len() + low_prio_defend.len() <= graph.num_nodes);

        self.nodes_to_defend.reserve_exact(total_defended);

        for node in high_prio_defend {
            self.nodes_to_defend.push(node);
        }

        for node in low_prio_defend {
            self.nodes_to_defend.push(node);
        }
    }
}

impl Strategy for PriorityStrategy {
    fn new(graph: Arc<RwLock<Graph>>) -> Self {
        Self {
            graph,
            nodes_to_defend: vec![],
            current_defended: 0,
        }
    }

    fn execute(&mut self, settings: &OSMFSettings, node_data: &mut NodeDataStorage, global_time: TimeUnit, contained_root: bool, undefended_roots: &Vec<usize>) {
        // One or more fire roots have been defended and hence shouldn't be considered
        // in the min_distance_groups anymore
        if contained_root {
            self.compute_nodes_to_defend(undefended_roots, settings, node_data);
        }

        let num_to_defend = min(settings.num_ffs, self.nodes_to_defend.len() - self.current_defended);
        let to_defend = &self.nodes_to_defend[self.current_defended..self.current_defended + num_to_defend];
        log::debug!("Defending nodes {:?}", to_defend);

        for node in to_defend {
            assert!(node_data.is_undefended(node));
        }

        node_data.mark_defended2(to_defend, global_time);

        self.current_defended += num_to_defend;
    }
}

/// Random fire containment strategy
#[derive(Debug, Default)]
pub struct RandomStrategy {
    graph: Arc<RwLock<Graph>>,
}

impl Strategy for RandomStrategy {
    fn new(graph: Arc<RwLock<Graph>>) -> Self {
        Self {
            graph,
        }
    }

    fn execute(&mut self, settings: &OSMFSettings, node_data: &mut NodeDataStorage, global_time: TimeUnit, contained_root: bool, undefended_roots: &Vec<usize>) {
        let graph = self.graph.read().unwrap();

        let nodes_to_defend: Vec<_> = graph.nodes.iter()
            .filter(|&node| node_data.is_undefended(&node.id))
            .map(|node| node.id)
            .collect();

        let num_to_defend = min(settings.num_ffs, nodes_to_defend.len());
        let mut rng = thread_rng();
        let to_defend: Vec<_> = nodes_to_defend
            .choose_multiple(&mut rng, num_to_defend)
            .cloned()
            .collect();

        log::debug!("Defending nodes {:?}", &to_defend);
        node_data.mark_defended(&to_defend, global_time);
    }
}