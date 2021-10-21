use std::{collections::BTreeMap,
          fmt::Formatter,
          sync::{Arc, RwLock}};

use log;
use rand::prelude::*;
use serde::Serialize;

use crate::firefighter::strategy::{OSMFStrategy, Strategy};
use crate::graph::Graph;

/// `u64` type alias to denote a time unit in the firefighter problem
pub type TimeUnit = u64;

/// State of a node in the firefighter problem
#[derive(Debug, Serialize)]
pub enum NodeState {
    Burning,
    Defended,
}
/// Settings for a firefighter problem instance
#[derive(Debug)]
pub struct OSMFSettings {
    num_roots: usize,
    pub num_firefighters: usize,
}

impl OSMFSettings {
    /// Create new settings for a firefighter problem instance
    pub fn new(num_roots: usize, num_firefighters: usize) -> Self {
        Self {
            num_roots,
            num_firefighters,
        }
    }
}

/// Node data related to the firefighter problem
#[derive(Debug, Serialize)]
pub struct NodeData {
    pub node_id: usize,
    state: NodeState,
    time: TimeUnit,
}

impl NodeData {
    /// Create new node data with state `state` for node with id `node_id`
    fn new(node_id: usize, state: NodeState, time: TimeUnit) -> Self {
        Self {
            node_id,
            state,
            time,
        }
    }

    /// Is corresponding node burning?
    pub fn is_burning(&self) -> bool {
        matches!(self.state, NodeState::Burning)
    }

    /// Is corresponding node defended?
    pub fn is_defended(&self) -> bool {
        matches!(self.state, NodeState::Defended)
    }
}

/// Storage for node data
#[derive(Debug)]
pub struct NodeDataStorage {
    storage: BTreeMap<usize, NodeData>,
}

impl NodeDataStorage {
    /// Create a new node data storage
    fn new() -> Self {
        Self {
            storage: BTreeMap::new(),
        }
    }

    /// Is node data attached to node with id `node_id`?
    pub fn is_node_data_attached(&self, node_id: &usize) -> bool {
        self.storage.contains_key(node_id)
    }

    /// Attach new node data to the node with id `node_id`
    pub fn attach_node_data(&mut self, node_id: usize, state: NodeState, time: TimeUnit) {
        self.storage.insert(node_id, NodeData::new(node_id, state, time));
    }

    /// Try to attach new node data to the node with id `node_id`.
    /// Return an error if node data is already attached to the node.
    pub fn try_attach_node_data(&mut self, node_id: usize, state: NodeState, time: TimeUnit) -> Result<(), OSMFProblemError> {
        if !self.is_node_data_attached(&node_id) {
            self.attach_node_data(node_id, state, time);
            Ok(())
        } else {
            Err(OSMFProblemError::NodeDataAlreadyAttached)
        }
    }

    /// Get the node data of all burning vertices
    pub fn get_all_burning(&self) -> Vec<&NodeData> {
        self.storage.values()
            .filter(|&nd| nd.is_burning())
            .collect::<Vec<_>>()
    }
}

/// Container for data about the simulation of a firefighter problem instance
#[derive(Serialize)]
pub struct OSMFSimulationResponse<'a> {
    node_data: &'a BTreeMap<usize, NodeData>,
    nodes_burned: usize,
    nodes_defended: usize,
    nodes_total: usize,
    end_time: TimeUnit,
}

/// A firefighter problem instance
#[derive(Debug)]
pub struct OSMFProblem {
    graph: Arc<RwLock<Graph>>,
    settings: OSMFSettings,
    strategy: OSMFStrategy,
    node_data: NodeDataStorage,
    global_time: TimeUnit,
    change_tracker: BTreeMap<TimeUnit, Vec<usize>>,
    is_active: bool,
}

impl OSMFProblem {
    /// Create a new firefighter problem instance
    pub fn new(graph: Arc<RwLock<Graph>>, settings: OSMFSettings, strategy: OSMFStrategy) -> Self {
        let num_nodes = graph.read().unwrap().num_nodes;
        if settings.num_roots > num_nodes {
            panic!("Number of fire roots must not be greater than {}", num_nodes);
        }

        let mut problem = Self {
            graph,
            settings,
            strategy,
            node_data: NodeDataStorage::new(),
            global_time: 0,
            change_tracker: BTreeMap::new(),
            is_active: true,
        };

        problem.gen_fire_roots();

        if let OSMFStrategy::ShortestDistance(ref mut sho_dist_strategy) = problem.strategy {
            let roots = problem.change_tracker.get(&0).unwrap();
            sho_dist_strategy.compute_shortest_dists(roots);
        }

        problem
    }

    /// Generate `num_roots` fire roots
    fn gen_fire_roots(&mut self) {
        let mut rng = thread_rng();
        let mut roots = Vec::with_capacity(self.settings.num_roots);
        let num_nodes = self.graph.read().unwrap().num_nodes;
        while roots.len() < self.settings.num_roots {
            let root = rng.gen_range(0..num_nodes);
            if !self.node_data.is_node_data_attached(&root) {
                self.node_data.attach_node_data(root, NodeState::Burning, self.global_time);
                roots.push(root);

                log::debug!("Set vertex {} as fire root", root);
            }
        }
        self.track_changes(roots);
    }

    /// Track a list of changed nodes.
    /// The changes will be attached to the current global time.
    fn track_changes(&mut self, changed: Vec<usize>) {
        match self.change_tracker.get_mut(&self.global_time) {
            Some(changes) => {
                changes.reserve_exact(changed.len());
                for node_id in changed {
                    changes.push(node_id);
                }
            }
            None => {
                self.change_tracker.insert(self.global_time, changed);
            }
        }
    }

    /// Spread the fire to all nodes that are adjacent to burning nodes.
    /// Defended nodes will remain defended.
    fn spread_fire(&mut self) {
        if !self.is_active {
            return;
        }

        let mut to_burn = Vec::new();
        {
            let burning = self.node_data.get_all_burning();

            let graph = self.graph.read().unwrap();
            let offsets = &graph.offsets;
            let edges = &graph.edges;

            // For all undefended neighbours that are not already burning, check whether they have
            // to be added to `to_burn`
            self.is_active = false;
            for node_data in burning {
                let node_id = node_data.node_id;
                for i in offsets[node_id]..offsets[node_id + 1] {
                    let edge = &edges[i];
                    if !self.node_data.is_node_data_attached(&edge.tgt) {
                        // There is at least one node to be burned at some point in the future
                        if !self.is_active {
                            self.is_active = true;
                        }
                        // Burn the node if the global time exceeds the time at which the edge source
                        // started burning plus the edge weight
                        if self.global_time >= node_data.time + edge.dist as u64 {
                            to_burn.push(edge.tgt);
                        }
                    }
                }
            }
        }

        // Burn all nodes in `to_burn`
        for node_id in &to_burn {
            self.node_data.attach_node_data(*node_id, NodeState::Burning, self.global_time);

            log::debug!("Node {} caught fire", node_id);
        }
        self.track_changes(to_burn);
    }

    /// Execute the containment strategy to prevent as much nodes as
    /// possible from catching fire
    fn contain_fire(&mut self) {
        if !self.is_active {
            return;
        }

        let defended = match self.strategy {
            OSMFStrategy::Greedy(ref mut greedy_strategy) =>
                greedy_strategy.execute(&self.settings, &mut self.node_data, self.global_time),
            _ => Vec::default()
        };

        if defended.is_empty()
            && matches!(self.strategy, OSMFStrategy::Greedy( .. )) { // TODO remove when shortest distance strategy is implemented
            self.is_active = false;
        }
        self.track_changes(defended);
    }

    /// Execute one time step in the firefighter problem.
    /// That is, execute the containment strategy, spread the fire and
    /// check whether the game is finished.
    fn exec_step(&mut self) {
        self.global_time += 1;

        self.contain_fire();
        self.spread_fire();
    }

    /// Simulate the firefighter problem until the `is_active` flag is set to `false`
    pub fn simulate(&mut self) {
        while self.is_active {
            self.exec_step();
        }
    }

    /// Generate the simulation response for this firefighter problem instance
    pub fn simulation_response(&self) -> OSMFSimulationResponse {
        let mut nodes_burned = 0;
        let mut nodes_defended = 0;
        for nd in self.node_data.storage.values() {
            if nd.is_burning() {
                nodes_burned += 1;
            } else {
                nodes_defended += 1;
            }
        }

        OSMFSimulationResponse {
            node_data: &self.node_data.storage,
            nodes_burned,
            nodes_defended,
            nodes_total: self.graph.read().unwrap().num_nodes,
            end_time: self.global_time,
        }
    }
}

#[derive(Debug)]
pub enum OSMFProblemError {
    NodeDataAlreadyAttached,
}

impl std::fmt::Display for OSMFProblemError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NodeDataAlreadyAttached => write!(f, "Node data is already attached to this node")
        }
    }
}

impl std::error::Error for OSMFProblemError {}

#[cfg(test)]
mod test {
    use std::{collections::BTreeMap,
              sync::{Arc, RwLock}};

    use rand::prelude::*;

    use crate::firefighter::{problem::{OSMFProblem, OSMFSettings},
                             strategy::{OSMFStrategy, ShoDistStrategy, Strategy}};
    use crate::graph::Graph;

    #[test]
    fn test() {
        let graph = Arc::new(RwLock::new(
            Graph::from_files("data/bbgrund")));
        let num_roots = 10;
        let strategy = OSMFStrategy::ShortestDistance(ShoDistStrategy::new(graph.clone()));
        let mut problem = OSMFProblem::new(
            graph.clone(), OSMFSettings::new(num_roots, 2), strategy);

        assert_eq!(problem.node_data.storage.len(), num_roots);
        assert_eq!(problem.change_tracker.len(), (problem.global_time + 1) as usize);
        assert_eq!(problem.change_tracker[&problem.global_time].len(), num_roots);

        let graph_ = graph.read().unwrap();
        let num_nodes = graph_.num_nodes;

        let roots: Vec<_>;
        {
            roots = problem.node_data.storage.keys()
                .into_iter()
                .map(|k| *k)
                .collect();

            for root in &roots {
                assert!(*root < num_nodes);
            }
        }

        let mut rng = thread_rng();
        let some_node = rng.gen_range(0..num_nodes);
        let mut dists_from_roots = Vec::with_capacity(num_roots);
        let max_dist = usize::MAX;
        for root in &roots {
            dists_from_roots.push(graph_.get_shortest_dist(*root, some_node)
                .unwrap_or(max_dist));
        }
        let min_dist = dists_from_roots.iter().min().unwrap();

        if let OSMFStrategy::ShortestDistance(sho_dist_strategy) = &problem.strategy {
            assert_eq!(*sho_dist_strategy.sho_dists.get(&some_node).unwrap_or(&max_dist), *min_dist);
        }

        problem.exec_step();

        assert_eq!(problem.change_tracker.len(), (problem.global_time + 1) as usize);

        let mut targets = Vec::new();
        let mut distances = BTreeMap::new();
        for root in &roots {
            let out_deg = graph_.get_out_degree(*root);
            targets.reserve(out_deg);
            for i in graph_.offsets[*root]..graph_.offsets[*root + 1] {
                let edge = &graph_.edges[i];
                targets.push(edge.tgt);
                distances.insert(edge.tgt, edge.dist);
            }
        }

        for node_id in &problem.change_tracker[&problem.global_time] {
            assert!(targets.contains(node_id));
        }

        for root in &roots {
            let root_nd = problem.node_data.storage.get(root).unwrap();
            for tgt in &targets {
                match problem.node_data.storage.get(tgt) {
                    Some(nd) => assert!(nd.is_burning()),
                    None => assert!(problem.global_time < root_nd.time + distances[tgt] as u64)
                }
            }
        }
    }
}