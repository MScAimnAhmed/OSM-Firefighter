use std::{collections::HashMap,
          fmt::Formatter,
          sync::{RwLock, Arc}};

use log;
use rand::prelude::*;

use crate::graph::Graph;

/// `u64` type alias to denote a time unit in the firefighter problem
type TimeUnit = u64;

/// State of a node in the firefighter problem
#[derive(Debug)]
pub enum NodeState {
    Burning,
    Defended,
}

/// Node data related to the firefighter problem
#[derive(Debug)]
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

/// A firefighter problem instance
#[derive(Debug)]
pub struct OSMFProblem {
    graph: Arc<RwLock<Graph>>,
    node_data: HashMap<usize, NodeData>,
    global_time: TimeUnit,
    change_tracker: HashMap<TimeUnit, Vec<usize>>,
    pub is_active: bool,
}

impl OSMFProblem {
    /// Create a new firefighter problem instance
    pub fn new(graph: Arc<RwLock<Graph>>, num_roots: usize) -> Self {
        let mut problem = Self {
            graph,
            node_data: HashMap::new(),
            global_time: 0,
            change_tracker: HashMap::new(),
            is_active: true,
        };

        // Generate `num_roots` fire roots
        let mut rng = thread_rng();
        let num_nodes = problem.graph.read().unwrap().num_nodes;
        for _ in 0..num_roots {
            let root = rng.gen_range(0..num_nodes);
            if !problem.is_node_data_attached(&root) {
                problem.attach_node_data(root, NodeState::Burning);
                log::trace!("Set vertex {} as fire root", root);
            }
        }

        log::debug!("Created new firefighter problem {:#?}", problem);

        problem
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
    pub fn spread_fire(&mut self) {
        let mut to_burn = Vec::new();
        {
            // Get all burning nodes
            let burning: Vec<_> = self.node_data.values()
                .filter(|&nd| nd.is_burning())
                .collect();

            let graph = self.graph.read().unwrap();
            let offsets = &graph.offsets;
            let edges = &graph.edges;

            // Add all undefended nodes that are not already burning to `to_burn`
            for node_data in burning {
                let node_id = node_data.node_id;
                for i in offsets[node_id]..offsets[node_id + 1] {
                    let edge = &edges[i];
                    if !self.is_node_data_attached(&edge.tgt) {
                        to_burn.push(edge.tgt);
                    }
                }
            }
        }

        if !to_burn.is_empty() {
            // Burn all nodes in `to_burn`
            for node_id in &to_burn {
                self.attach_node_data(*node_id, NodeState::Burning);

                log::trace!("Node {} caught fire", node_id);
            }
        } else {
            self.is_active = false;
        }
        self.change_tracker.insert(self.global_time, to_burn);
    }

    /// Execute the containment strategy to prevent as much nodes as
    /// possible from catching fire
    fn contain_fire(&mut self) {
        todo!()
    }

    /// Execute one time step in the firefighter problem.
    /// That is, execute the containment strategy, spread the fire and
    /// check whether the game is finished.
    pub fn exec_step(&mut self) {
        //self.contain_fire();

        self.global_time += 1;
        self.spread_fire();
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
    use std::sync::{Arc, RwLock};

    use crate::firefighter::OSMFProblem;
    use crate::graph::Graph;

    #[test]
    fn test() {
        let graph = Arc::new(RwLock::new(
            Graph::from_file("resources/toy.fmi")));
        let mut problem = OSMFProblem::new(graph.clone(), 1);

        assert_eq!(problem.node_data.len(), 1);

        let root;
        {
            let node_data: Vec<_> = problem.node_data.values().collect();
            root = node_data.first().unwrap().node_id;

            assert!(root < graph.read().unwrap().num_nodes);
        }

        problem.exec_step();

        let graph_ = graph.read().unwrap();
        let mut targets = Vec::with_capacity(graph_.offsets[root+1] - graph_.offsets[root]);
        for i in graph_.offsets[root]..graph_.offsets[root+1] {
            let edge = &graph_.edges[i];
            targets.push(edge.tgt);
        }
        for node_id in problem.change_tracker.get(&problem.global_time).unwrap() {
            assert!(targets.contains(node_id));
        }
        let not_burning_targets: Vec<_> = targets.iter()
            .filter(|&tgt| !problem.node_data.get(tgt).unwrap().is_burning())
            .collect();

        assert!(not_burning_targets.is_empty());
    }
}