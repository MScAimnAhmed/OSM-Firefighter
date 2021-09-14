use std::{collections::HashMap,
          fmt::Formatter,
          sync::{RwLock, Arc}};

use log;
use rand::prelude::*;

use crate::graph::Graph;

/// A firefighter problem instance
#[derive(Debug)]
pub struct OSMFProblem {
    graph: Arc<RwLock<Graph>>,
    node_data: HashMap<usize, NodeData>,
}

/// Node data related to the firefighter problem
#[derive(Debug)]
pub struct NodeData {
    node_id: usize,
    state: NodeState,
    time: u64,
}

/// State of a node in the firefighter problem
#[derive(Debug)]
pub enum NodeState {
    Burning,
    Defended,
}

impl OSMFProblem {
    /// Create a new firefighter problem instance
    pub fn new(graph: Arc<RwLock<Graph>>) -> Self {
        let mut problem = Self {
            graph,
            node_data: HashMap::new(),
        };

        // Generate the root of the fire
        let root: usize = thread_rng().gen_range(0..problem.graph.read().unwrap().num_nodes);
        // Attach new node data with state burning to root
        problem.attach_node_data(root, NodeState::Burning, 0);

        log::trace!("Created new firefighter problem {:#?}", problem);
        log::debug!("Created new firefighter problem with root node data {:?}",
            problem.node_data.get(&root).unwrap());

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