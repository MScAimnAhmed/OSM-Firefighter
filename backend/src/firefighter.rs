use std::{collections::HashMap,
          fmt::Formatter,
          sync::{RwLock, Arc}};

use log;
use rand::prelude::*;
use serde::Serialize;

use crate::graph::Graph;

/// `u64` type alias to denote a time unit in the firefighter problem
type TimeUnit = u64;

/// State of a node in the firefighter problem
#[derive(Debug, Serialize)]
pub enum NodeState {
    Burning,
    Defended,
}

/// Node data related to the firefighter problem
#[derive(Debug, Serialize)]
pub struct NodeData {
    node_id: usize,
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
    fn is_burning(&self) -> bool {
        matches!(self.state, NodeState::Burning)
    }

    /// Is corresponding node defended?
    fn is_defended(&self) -> bool {
        matches!(self.state, NodeState::Defended)
    }
}

#[derive(Debug)]
pub enum OSMFStrategy {
    Greedy,
    ShortestDistance,
}

/// Settings for a firefighter problem instance
#[derive(Debug)]
pub struct OSMFSettings {
    num_roots: usize,
    num_firefighters: usize,
    strategy: OSMFStrategy,
}

impl OSMFSettings {
    /// Create new settings for a firefighter problem instance
    pub fn new(num_roots: usize, num_firefighters: usize, strategy: OSMFStrategy) -> Self {
        Self {
            num_roots,
            num_firefighters,
            strategy,
        }
    }
}

/// A firefighter problem instance
#[derive(Debug)]
pub struct OSMFProblem {
    graph: Arc<RwLock<Graph>>,
    settings: OSMFSettings,
    pub node_data: HashMap<usize, NodeData>,
    global_time: TimeUnit,
    change_tracker: HashMap<TimeUnit, Vec<usize>>,
    pub is_active: bool,
}

impl OSMFProblem {
    /// Create a new firefighter problem instance
    pub fn new(graph: Arc<RwLock<Graph>>, settings: OSMFSettings) -> Self {
        let num_nodes = graph.read().unwrap().num_nodes;
        if settings.num_roots > num_nodes {
            panic!("Number of fire roots must not be greater than {}", num_nodes);
        }

        let mut problem = Self {
            graph,
            settings,
            node_data: HashMap::new(),
            global_time: 0,
            change_tracker: HashMap::new(),
            is_active: true,
        };
        problem.gen_fire_roots();

        log::debug!("Created new firefighter problem {:#?}", problem);

        problem
    }

    /// Generate `num_roots` fire roots
    fn gen_fire_roots(&mut self) {
        let mut rng = thread_rng();
        let mut roots = Vec::with_capacity(self.settings.num_roots);
        let num_nodes = self.graph.read().unwrap().num_nodes;
        while roots.len() < self.settings.num_roots {
            let root = rng.gen_range(0..num_nodes);
            if !self.is_node_data_attached(&root) {
                self.attach_node_data(root, NodeState::Burning);
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

    /// Is node data attached to node with id `node_id`?
    fn is_node_data_attached(&self, node_id: &usize) -> bool {
        self.node_data.contains_key(node_id)
    }

    /// Attach new node data to the node with id `node_id`
    fn attach_node_data(&mut self, node_id: usize, state: NodeState) {
        self.node_data.insert(node_id, NodeData::new(node_id, state, self.global_time));
    }

    /// Try to attach new node data to the node with id `node_id`.
    /// Return an error if node data is already attached to the node.
    pub fn try_attach_node_data(&mut self, node_id: usize, state: NodeState) -> Result<(), OSMFProblemError> {
        if !self.is_node_data_attached(&node_id) {
            self.attach_node_data(node_id, state);
            Ok(())
        } else {
            Err(OSMFProblemError::NodeDataAlreadyAttached)
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
            // Get all burning nodes
            let burning: Vec<_> = self.node_data.values()
                .filter(|&nd| nd.is_burning())
                .collect();

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
                    if !self.is_node_data_attached(&edge.tgt) {
                        // There is at least one node to be burned at some point in the future
                        if !self.is_active {
                            self.is_active = true;
                        }
                        // Burn the node if the global time exceeds the time at which the edge source
                        // started burning plus the edge weight
                        if self.global_time >= node_data.time + edge.weight {
                            to_burn.push(edge.tgt);
                        }
                    }
                }
            }
        }

        // Burn all nodes in `to_burn`
        for node_id in &to_burn {
            self.attach_node_data(*node_id, NodeState::Burning);

            log::debug!("Node {} caught fire", node_id);
        }
        self.track_changes(to_burn);
    }

    /// Execute the containment strategy to prevent as much nodes as
    /// possible from catching fire
    fn contain_fire(&mut self) {
        todo!()
    }

    /// Execute one time step in the firefighter problem.
    /// That is, execute the containment strategy, spread the fire and
    /// check whether the game is finished.
    fn exec_step(&mut self) {
        self.global_time += 1;

        //self.contain_fire();
        self.spread_fire();
    }

    /// Simulate the firefighter problem until the `is_active` flag is set to `false`
    pub fn simulate(&mut self) {
        while self.is_active {
            self.exec_step();
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
    use std::{collections::HashMap,
              sync::{Arc, RwLock}};

    use crate::firefighter::{OSMFProblem, OSMFStrategy, OSMFSettings};
    use crate::graph::Graph;

    #[test]
    fn test() {
        let graph = Arc::new(RwLock::new(
            Graph::from_file("resources/toy.fmi")));
        let num_roots = 1;
        let mut problem = OSMFProblem::new(
            graph.clone(),
            OSMFSettings::new(1, 1, OSMFStrategy::Greedy));

        assert_eq!(problem.node_data.len(), num_roots);
        assert_eq!(problem.change_tracker.len(), (problem.global_time + 1) as usize);
        assert_eq!(problem.change_tracker.get(&problem.global_time).unwrap().len(), num_roots);

        let root;
        {
            let node_data: Vec<_> = problem.node_data.values().collect();
            root = node_data.first().unwrap().node_id;

            assert!(root < graph.read().unwrap().num_nodes);
        }

        problem.exec_step();

        assert_eq!(problem.change_tracker.len(), (problem.global_time + 1) as usize);

        let graph_ = graph.read().unwrap();
        let mut targets = Vec::with_capacity(graph_.get_out_degree(root));
        let mut weights =
            HashMap::with_capacity(graph_.get_out_degree(root));
        for i in graph_.offsets[root]..graph_.offsets[root + 1] {
            let edge = &graph_.edges[i];
            targets.push(edge.tgt);
            weights.insert(edge.tgt, edge.weight);
        }

        for node_id in problem.change_tracker.get(&problem.global_time).unwrap() {
            assert!(targets.contains(node_id));
        }

        let root_nd = problem.node_data.get(&root).unwrap();
        for tgt in targets {
            match problem.node_data.get(&tgt) {
                Some(nd) => assert!(nd.is_burning()
                    && problem.global_time >= root_nd.time + *weights.get(&tgt).unwrap()),
                None => assert!(problem.global_time < root_nd.time + *weights.get(&tgt).unwrap())
            }
        }
    }
}