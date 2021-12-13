use std::{cmp::min,
          collections::{BTreeMap, HashMap, VecDeque, HashSet},
          fmt::Debug,
          sync::{Arc, RwLock, RwLockReadGuard}};

use rand::seq::SliceRandom;
use rand::prelude::*;

use strum::VariantNames;
use strum_macros::{EnumString, EnumVariantNames};

use crate::firefighter::{problem::{NodeDataStorage, OSMFSettings},
                         TimeUnit};
use crate::graph::Graph;

/// Strategy to contain the fire in the firefighter problem
#[derive(Debug, EnumString, EnumVariantNames)]
#[strum(serialize_all = "PascalCase")]
pub enum OSMFStrategy {
    Greedy(GreedyStrategy),
    MultiMinDistanceSets(MultiMinDistSetsStrategy),
    SingleMinDistanceSet(SingleMinDistSetStrategy),
    Priority(PriorityStrategy),
    Random(RandomStrategy),
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
        node_data.mark_defended(&to_defend, global_time);
    }
}

/// Type alias for clarification
type Visited = HashSet<usize>;
/// Type alias for clarification
type RiskyNodes = HashSet<usize>;

fn compute_undefended_roots(undefended_roots: &mut HashMap<usize, (Visited, RiskyNodes)>,
                            graph: &Arc<RwLock<Graph>>, node_data: &NodeDataStorage) -> Option<Vec<usize>> {
    let graph = graph.read().unwrap();

    for (_, (visited, risky_nodes)) in undefended_roots.iter_mut() {
        // Filter all burning risky nodes
        let mut burning: VecDeque<_> = risky_nodes.iter()
            .filter(|&node| node_data.is_burning(node))
            .map(|node| *node)
            .collect();

        visited.reserve(burning.len());

        // Retain all undefended nodes
        risky_nodes.retain(|node| node_data.is_undefended(node));

        // Update risky nodes by tracking all paths from burning to undefended nodes
        while !burning.is_empty() {
            let node = burning.pop_front().unwrap();
            visited.insert(node);
            let out_deg = graph.get_out_degree(node);
            risky_nodes.reserve(out_deg);
            burning.reserve(out_deg);
            for i in graph.offsets[node]..graph.offsets[node+1] {
                let edge = &graph.edges[i];
                if node_data.is_undefended(&edge.tgt) {
                    risky_nodes.insert(edge.tgt);
                } else if node_data.is_burning(&edge.tgt) && !visited.contains(&edge.tgt) {
                    burning.push_back(edge.tgt);
                }
            }
        }
    }

    let old_num_roots = undefended_roots.len();
    undefended_roots.retain(|_, (_, risky_nodes)| !risky_nodes.is_empty());
    let new_num_roots = undefended_roots.len();

    if new_num_roots < old_num_roots {
        let undefended_roots: Vec<_> = undefended_roots.keys()
            .map(|&root| root)
            .collect();
        Some(undefended_roots)
    } else {
        None
    }
}

/// For every node, compute the minimum shortest distance between the node and any fire root.
/// Then, group the nodes by minimum shortest distance.
fn group_nodes_by_distance(undefended_roots: &Vec<usize>, graph: &RwLockReadGuard<Graph>,
                           node_data: &NodeDataStorage) -> BTreeMap<usize, Vec<usize>> {
    let mut sho_dists = HashMap::with_capacity(graph.num_nodes);
    for &root in undefended_roots {
        for node in &graph.nodes {
            if node_data.is_undefended(&node.id) {
                let new_dist = graph.unchecked_get_shortest_dist(root, node.id);
                if new_dist < usize::MAX {
                    sho_dists.entry(node.id)
                        .and_modify(|cur_dist| if new_dist < *cur_dist { *cur_dist = new_dist })
                        .or_insert(new_dist);
                }
            }
        }
    }

    let mut nodes_by_sho_dist: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (&node_id, &dist) in sho_dists.iter() {
        nodes_by_sho_dist.entry(dist)
            .and_modify(|nodes| nodes.push(node_id))
            .or_insert(vec![node_id]);
    }

    log::debug!("Computed distance sets:\n{:?}", &nodes_by_sho_dist);

    nodes_by_sho_dist
}

/// Shortest distance based fire containment strategy
/// that selects multiple sets to defend
#[derive(Debug, Default)]
pub struct MultiMinDistSetsStrategy {
    graph: Arc<RwLock<Graph>>,
    nodes_to_defend: VecDeque<usize>,
    current_defended: usize,
    undefended_roots: HashMap<usize, (Visited, RiskyNodes)>,
}

impl MultiMinDistSetsStrategy {
    /// Initialize the undefended roots datastructure
    pub fn initialize_undefended_roots(&mut self, roots: &Vec<usize>) {
        self.undefended_roots.reserve(roots.len());
        for &root in roots {
            self.undefended_roots.insert(root, (HashSet::new(), HashSet::from([root])));
        }
    }
    
    /// (Re-)compute undefended roots by tracking paths through burning vertices from
    /// all roots to any undefended node.
    /// Returns the remaining undefended roots, if the number of undefended roots
    /// has changed.
    fn compute_undefended_roots(&mut self, node_data: &NodeDataStorage) -> Option<Vec<usize>> {
        compute_undefended_roots(&mut self.undefended_roots, &self.graph, node_data)
    }
    
    /// Compute nodes to defend and order in which nodes should be defended
    pub fn compute_nodes_to_defend(&mut self, undefended_roots: &Vec<usize>, settings: &OSMFSettings,
                                   node_data: &NodeDataStorage) {
        let graph = self.graph.read().unwrap();

        let nodes_by_sho_dist = group_nodes_by_distance(undefended_roots,
                                                        &graph, node_data);

        let strategy_every = settings.strategy_every as usize;
        let num_ffs = settings.num_ffs;
        let mut total_defended = self.current_defended;

        // Node groups that can be defended completely
        let defend_completely: Vec<_> = nodes_by_sho_dist.iter()
            .filter(|(&dist, nodes)| {
                let can_defend_total = dist / strategy_every * num_ffs;
                if can_defend_total > total_defended {
                    let must_defend = nodes.len();
                    let can_defend = can_defend_total - total_defended;
                    if can_defend >= must_defend {
                        total_defended += must_defend;
                        true
                    } else {
                        false
                    }
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

        self.nodes_to_defend.clear();
        self.nodes_to_defend.reserve_exact(total_defended - self.current_defended);

        for nodes in defend_completely {
            for &node in nodes {
                self.nodes_to_defend.push_front(node);
            }
        }

        for nodes in defend_partially {
            for node in nodes {
                self.nodes_to_defend.push_front(node);
            }
        }

        self.nodes_to_defend.make_contiguous();
    }
}

impl Strategy for MultiMinDistSetsStrategy {
    fn new(graph: Arc<RwLock<Graph>>) -> Self {
        Self {
            graph,
            nodes_to_defend: VecDeque::new(),
            current_defended: 0,
            undefended_roots: HashMap::new(),
        }
    }

    fn execute(&mut self, settings: &OSMFSettings, node_data: &mut NodeDataStorage, global_time: TimeUnit) {
        let num_to_defend = min(settings.num_ffs, self.nodes_to_defend.len());
        let len = self.nodes_to_defend.len();
        let to_defend = &self.nodes_to_defend.as_slices().0[(len-num_to_defend)..len];

        for node in to_defend {
            assert!(node_data.is_undefended(node));
        }

        node_data.mark_defended2(to_defend, global_time);

        self.nodes_to_defend.truncate(len-num_to_defend);
        self.current_defended += num_to_defend;

        // One or more fire roots have been defended and hence shouldn't be considered
        // in the min_distance_groups anymore
        if let Some(roots) = self.compute_undefended_roots(node_data) {
            self.compute_nodes_to_defend(&roots, settings, node_data);
        }
    }
}

/// Shortest distance based fire containment strategy
/// that selects
#[derive(Debug, Default)]
pub struct SingleMinDistSetStrategy {
    graph: Arc<RwLock<Graph>>,
    nodes_to_defend: Vec<usize>,
    current_defended: usize,
}

impl SingleMinDistSetStrategy {
    /// Compute nodes to defend and order in which nodes should be defended
    pub fn compute_nodes_to_defend(&mut self, roots: &Vec<usize>, settings: &OSMFSettings) {
        let graph = self.graph.read().unwrap();

        // For each root, run an one-to-all Dijkstra to all nodes in the underlying graph.
        // Then, filter the distances to the nodes for the minimum distance from any fire root.
        let mut global_dists = HashMap::with_capacity(graph.num_nodes);
        for &root in roots {
            let dists = graph.run_dijkstra(root);
            for (node, &dist) in dists.iter().enumerate() {
                if dist < usize::MAX {
                    global_dists.entry(node)
                        .and_modify(|cur_dist| if dist < *cur_dist { *cur_dist = dist })
                        .or_insert(dist);
                }
            }
        }

        // For each node, get its predecessor with the lowest _global distance_ and
        // store that predecessor as its respective _global predecessor_
        let mut global_preds = vec![usize::MAX; graph.num_nodes];
        for edge in &graph.edges {
            let cur_pred = global_preds[edge.tgt];
            if cur_pred < usize::MAX {
                let cur_dist = global_dists.get(&cur_pred).unwrap();
                let dist = global_dists.get(&edge.src).unwrap();
                if dist < cur_dist {
                    global_preds[edge.tgt] = edge.src;
                }
            } else if global_dists.contains_key(&edge.src) {
                global_preds[edge.tgt] = edge.src;
            }
        }

        log::debug!("Global distances:\n{:?}", &global_dists);

        // Transform the global distance map into a data structure that maps each distance
        // to the nodes that have to be defended in order to protect all nodes with a higher
        // distance.
        let mut distance_nodes_map: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for (&node_id, &dist) in global_dists.iter() {
            let pred_id = global_preds[node_id];
            if let Some(pred_dist) = global_dists.get(&pred_id) {
                for d in (*pred_dist+1)..=dist {
                    distance_nodes_map.entry(d)
                        .and_modify(|nodes| nodes.push(node_id))
                        .or_insert(vec![node_id]);
                }
            }
        }

        log::debug!("Distance nodes map:\n{:?}", &distance_nodes_map);

        let strategy_every = settings.strategy_every as usize;
        let num_ffs = settings.num_ffs as usize;

        let mut it = distance_nodes_map.iter();
        loop {
            match it.next() {
                Some((&dist, nodes)) => {
                    if nodes.len() <= dist / strategy_every * num_ffs  {
                        self.nodes_to_defend = nodes.clone();
                        log::debug!("Selected {} nodes to defend: {:?} with distance {}",
                            nodes.len(), &nodes, dist);
                        break;
                    }
                }
                None => { break; }
            }
        }
    }
}

impl Strategy for SingleMinDistSetStrategy {
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
        node_data.mark_defended2(to_defend, global_time);

        self.current_defended += num_to_defend;
    }
}

/// Priority based fire containment strategy
#[derive(Debug, Default)]
pub struct PriorityStrategy {
    graph: Arc<RwLock<Graph>>,
    nodes_to_defend: VecDeque<usize>,
    current_defended: usize,
    undefended_roots: HashMap<usize, (Visited, RiskyNodes)>,
}

impl PriorityStrategy {
    /// Initialize the undefended roots datastructure
    pub fn initialize_undefended_roots(&mut self, roots: &Vec<usize>) {
        self.undefended_roots.reserve(roots.len());
        for &root in roots {
            self.undefended_roots.insert(root, (HashSet::new(), HashSet::from([root])));
        }
    }

    /// (Re-)compute undefended roots by tracking paths through burning vertices from
    /// all roots to any undefended node.
    /// Returns the remaining undefended roots, if the number of undefended roots
    /// has changed.
    fn compute_undefended_roots(&mut self, node_data: &NodeDataStorage) -> Option<Vec<usize>> {
        compute_undefended_roots(&mut self.undefended_roots, &self.graph, node_data)
    }
    
    /// Compute nodes to defend and order in which nodes should be defended
    pub fn compute_nodes_to_defend(&mut self, undefended_roots: &Vec<usize>, settings: &OSMFSettings,
                                   node_data: &NodeDataStorage) {
        let graph = self.graph.read().unwrap();

        let mut priority_map = HashMap::with_capacity(graph.num_nodes);
        for node in &graph.nodes {
            if node_data.is_undefended(&node.id) && graph.get_out_degree(node.id) > 0 {
                let mut prio = 0.0;
                for i in graph.offsets[node.id]..graph.offsets[node.id+1] {
                    let edge = &graph.edges[i];
                    prio += 1.0 / edge.dist as f64;
                }
                priority_map.insert(node.id, prio);
            }
        }

        log::debug!("Computed priority map:\n{:?}", &priority_map);

        /*
        let mut sorted_priorities: Vec<_> = priority_map.values().map(|prio|*prio).collect();
        sorted_priorities.sort_unstable_by(|p1, p2| {
            p1.cmp(&p2)
        });

        log::debug!("sorted prios {:?}", sorted_priorities);

         */

        let mean = priority_map.values().sum::<f64>() as f64 / priority_map.len() as f64;
        log::debug!("Computed mean: {}", mean);

        let mut nodes_by_sho_dist = group_nodes_by_distance(undefended_roots,
                                                        &graph, node_data);

        // Sort Node groups by priority
        for (_, nodes) in nodes_by_sho_dist.iter_mut() {
            nodes.sort_unstable_by(|n1, n2| {
                let prio1 = priority_map.get(n1).unwrap_or(&0.0);
                let prio2 = priority_map.get(n2).unwrap_or(&0.0);
                prio2.partial_cmp(&prio1).unwrap()
            });
        }

        log::debug!("Distance sets after sorting by priority:\n{:?}", &nodes_by_sho_dist);

        let strategy_every = settings.strategy_every as usize;
        let num_ffs = settings.num_ffs;
        let mut total_defended = self.current_defended;

        // Filter nodes with higher priority based on mean
        let mut high_prio_map: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for (&dist, nodes) in nodes_by_sho_dist.iter() {
            let high_prio_nodes: Vec<_> = nodes.iter()
                .filter(|&node| {
                    *priority_map.get(node).unwrap_or(&0.0) as f64 >= mean
                })
                .map(|node| *node)
                .collect();
            high_prio_map.insert(dist, high_prio_nodes);
        }

        // Filter nodes with higher priority based on mean
        let mut low_prio_map: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for (&dist, nodes) in nodes_by_sho_dist.iter() {
            let low_prio_nodes: Vec<_> = nodes.iter()
                .filter(|&node| {
                    (*priority_map.get(node).unwrap_or(&0.0) as f64) < mean
                })
                .map(|node| *node)
                .collect();
            low_prio_map.insert(dist, low_prio_nodes);
        }

        // Nodes with a higher priority than the mean should be defended
        let mut high_prio_defend = Vec::new();
        for (&dist, nodes) in high_prio_map.iter() {
            let can_defend_total = dist / strategy_every * num_ffs;
            if can_defend_total > total_defended {
                let can_defend = can_defend_total - total_defended;
                let num_of_nodes = min(can_defend, nodes.len());
                high_prio_defend.reserve(num_of_nodes);
                for &node in &nodes[0..num_of_nodes] {
                    high_prio_defend.push(node);
                }
                total_defended += num_of_nodes;
            }
        }

        // Nodes with a lower priority than the mean should be defended
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

        self.nodes_to_defend.clear();
        self.nodes_to_defend.reserve_exact(total_defended - self.current_defended);

        for node in high_prio_defend {
            self.nodes_to_defend.push_front(node);
        }

        for node in low_prio_defend {
            self.nodes_to_defend.push_front(node);
        }

        self.nodes_to_defend.make_contiguous();
    }
}

impl Strategy for PriorityStrategy {
    fn new(graph: Arc<RwLock<Graph>>) -> Self {
        Self {
            graph,
            nodes_to_defend: VecDeque::new(),
            current_defended: 0,
            undefended_roots: HashMap::new(),
        }
    }

    fn execute(&mut self, settings: &OSMFSettings, node_data: &mut NodeDataStorage, global_time: TimeUnit) {
        let num_to_defend = min(settings.num_ffs, self.nodes_to_defend.len());
        let len = self.nodes_to_defend.len();
        let to_defend = &self.nodes_to_defend.as_slices().0[(len-num_to_defend)..len];

        for node in to_defend {
            assert!(node_data.is_undefended(node));
        }

        node_data.mark_defended2(to_defend, global_time);

        self.nodes_to_defend.truncate(len-num_to_defend);
        self.current_defended += num_to_defend;

        // One or more fire roots have been defended and hence shouldn't be considered
        // in the min_distance_groups anymore
        if let Some(roots) = self.compute_undefended_roots(node_data) {
            self.compute_nodes_to_defend(&roots, settings, node_data);
        }
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

    fn execute(&mut self, settings: &OSMFSettings, node_data: &mut NodeDataStorage, global_time: TimeUnit) {
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

        node_data.mark_defended(&to_defend, global_time);
    }
}