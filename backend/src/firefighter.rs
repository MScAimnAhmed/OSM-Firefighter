

/// A firefighter problem instance
pub struct OSMFProblem {
    node_data: Vec<NodeData>,
}

/// Node data related to the firefighter problem
pub struct NodeData {
    node_id: usize,
    state: NodeState,
    time: u64,
}

/// State of a node in the firefighter problem
enum NodeState {
    Burning,
    Saved,
}

impl OSMFProblem {
    /// Create a new firefighter problem instance
    pub fn new() -> Self {
        OSMFProblem {
            node_data: Vec::new(),
        }
    }
}