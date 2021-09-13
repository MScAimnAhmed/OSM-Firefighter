use std::{collections::HashMap,
          fmt::Formatter};

use rand::prelude::*;

use crate::graph::Graph;

/// A firefighter problem instance
pub struct OSMFProblem<'a> {
    graph: &'a Graph,
    node_data: HashMap<usize, NodeData>,
}

/// Node data related to the firefighter problem
pub struct NodeData {
    node_id: usize,
    state: NodeState,
    time: u64,
}

/// State of a node in the firefighter problem
pub enum NodeState {
    Burning,
    Defended,
}

impl<'a> OSMFProblem<'a> {
    /// Create a new firefighter problem instance
    pub fn new(graph: &'a Graph) -> Self {
        let mut problem = OSMFProblem {
            graph,
            node_data: HashMap::new(),
        };
        // Generate the root of the fire
        let root: usize = thread_rng().gen_range(0..graph.num_nodes);
        // Attach new node data with state burning to root
        problem.attach_node_data(root, NodeState::Burning, 0);
        problem
    }

    /// Attach new node data to the node with id `node_id`
    fn attach_node_data(&mut self, node_id: usize, state: NodeState, time: u64) {
        self.node_data.insert(node_id, NodeData {
            node_id,
            state,
            time,
        });
    }

    /// Try to attach new node data to the node with id `node_id`.
    /// Return an error if node data is already attached to the node.
    pub fn try_attach_node_data(&mut self, node_id: usize, state: NodeState, time: u64) -> Result<(), OSMFProblemError> {
        if !self.node_data.contains_key(&node_id) {
            self.attach_node_data(node_id, state, time);
            Ok(())
        } else {
            Err(OSMFProblemError::NodeDataAlreadyAttached)
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