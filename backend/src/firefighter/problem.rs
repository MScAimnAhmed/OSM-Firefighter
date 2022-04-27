use std::{collections::BTreeMap,
          // fmt::Formatter,
          sync::{Arc, RwLock}};

use log;
use rand::prelude::*;
use serde::{Serialize, Deserialize};

use crate::firefighter::{strategy::{OSMFStrategy, Strategy},
                         TimeUnit,
                         view::{View, Coords}};
use crate::graph::{Graph, GridBounds};

/// Settings for a firefighter problem instance
#[derive(Debug, Deserialize, Clone)]
pub struct OSMFSettings {
    pub graph_name: String,
    pub strategy_name: String,
    pub num_roots: usize,
    pub num_ffs: usize,
    pub strategy_every: TimeUnit,
}

/// Node data related to the firefighter problem
#[derive(Debug, Serialize)]
pub struct NodeData {
    pub node_id: usize,
    time: TimeUnit,
}

/// Storage for node data
#[derive(Debug, Serialize)]
pub struct NodeDataStorage {
    burning: BTreeMap<usize, NodeData>,
    defended: BTreeMap<usize, NodeData>,
}

impl NodeDataStorage {
    /// Create a new node data storage
    fn new() -> Self {
        Self {
            burning: BTreeMap::new(),
            defended: BTreeMap::new(),
        }
    }

    /// Is node with id `node_id` a fire root?
    pub fn is_root(&self, node_id: &usize) -> bool {
        match self.burning.get(node_id) {
            Some(nd) => nd.time == 0,
            None => false
        }
    }

    /// Is node with id `node_id` burning?
    pub fn is_burning(&self, node_id: &usize) -> bool {
        self.burning.contains_key(node_id)
    }

    /// Is node with id `node_id` burning by time `time`?
    pub fn is_burning_by(&self, node_id: &usize, time: &TimeUnit) -> bool {
        match self.burning.get(node_id) {
            Some(nd) => nd.time <= *time,
            None => false
        }
    }

    /// Count all nodes burning by time `time`
    pub fn count_burning_by(&self, time: &TimeUnit) -> usize {
        self.burning.values()
            .filter(|nd| nd.time <= *time)
            .count()
    }

    /// Is node with id `node_id` defended?
    fn is_defended(&self, node_id: &usize) -> bool {
        self.defended.contains_key(node_id)
    }

    /// Is node with id `node_id` defended by time `time`?
    pub fn is_defended_by(&self, node_id: &usize, time: &TimeUnit) -> bool {
        match self.defended.get(node_id) {
            Some(nd) => nd.time <= *time,
            None => false
        }
    }

    /// Count all nodes defended by time `time`
    pub fn count_defended_by(&self, time: &TimeUnit) -> usize {
        self.defended.values()
            .filter(|nd| nd.time <= *time)
            .count()
    }

    /// Is node with id `node_id` undefended?
    pub fn is_undefended(&self, node_id: &usize) -> bool {
        !(self.is_burning(node_id) || self.is_defended(node_id))
    }

    /// Mark all nodes in `nodes` as burning at time `time`
    pub fn mark_burning(&mut self, nodes: &Vec<usize>, time: TimeUnit) {
        if !nodes.is_empty() {
            log::debug!("Burning nodes {:?} in round {}", nodes, time);
        }
        for node_id in nodes {
            self.burning.insert(*node_id, NodeData {
                node_id: *node_id,
                time,
            });
        }
    }

    /// Mark all nodes in `nodes` as defended at time `time`
    pub fn mark_defended(&mut self, nodes: &Vec<usize>, time: TimeUnit) {
        if !nodes.is_empty() {
            log::debug!("Defending nodes {:?} in round {}", nodes, time);
        }
        for node_id in nodes {
            self.defended.insert(*node_id, NodeData {
                node_id: *node_id,
                time,
            });
        }
    }

    /// Mark all nodes in `nodes` as defended at time `time`
    pub fn mark_defended2(&mut self, nodes: &[usize], time: TimeUnit) {
        if !nodes.is_empty() {
            log::debug!("Defending nodes {:?} in round {}", nodes, time);
        }
        for node_id in nodes {
            self.defended.insert(*node_id, NodeData {
                node_id: *node_id,
                time,
            });
        }
    }

    /// Get the node data of all burning vertices
    pub fn get_burning(&self) -> Vec<&NodeData> {
        self.burning.values().collect()
    }

    /// Get the id's of all burning vertices at time `time`
    pub fn get_burning_at(&self, time: &TimeUnit) -> Vec<usize> {
        self.burning.values()
            .filter(|&nd| nd.time == *time)
            .map(|nd| nd.node_id)
            .collect::<Vec<_>>()
    }

    /// Get the id's of all defended vertices at time `time`
    pub fn get_defended_at(&self, time: &TimeUnit) -> Vec<usize> {
        self.defended.values()
            .filter(|&nd| nd.time == *time)
            .map(|nd| nd.node_id)
            .collect::<Vec<_>>()
    }
}

/// Container for data about the simulation of a firefighter problem instance
#[derive(Serialize)]
pub struct OSMFSimulationResponse<'a> {
    pub nodes_burned: usize,
    pub nodes_defended: usize,
    nodes_total: usize,
    pub end_time: TimeUnit,
    view_bounds: &'a GridBounds,
    view_center: Coords,
}

/// Container for data about a specific step of a firefighter simulation
#[derive(Serialize)]
pub struct OSMFSimulationStepMetadata {
    nodes_burned_by: usize,
    nodes_defended_by: usize,
    nodes_burned_at: Vec<usize>,
    nodes_defended_at: Vec<usize>,
}

/// A firefighter problem instance
#[derive(Debug)]
pub struct OSMFProblem {
    graph: Arc<RwLock<Graph>>,
    settings: OSMFSettings,
    strategy: OSMFStrategy,
    node_data: NodeDataStorage,
    global_time: TimeUnit,
    is_active: bool,
    view: View,
}

impl OSMFProblem {
    /// Create a new firefighter problem instance
    pub fn new(graph: Arc<RwLock<Graph>>, settings: OSMFSettings, strategy: OSMFStrategy) -> Self {
        let num_nodes = graph.read().unwrap().num_nodes;
        if settings.num_roots > num_nodes {
            let err_msg = format!("Number of fire roots must not be greater than {}", num_nodes);
            log::warn!("{}", &err_msg);
            panic!("{}", err_msg);
        }

        let mut problem = Self {
            graph: graph.clone(),
            settings,
            strategy,
            node_data: NodeDataStorage::new(),
            global_time: 0,
            is_active: true,
            view: View::new(graph, 1920, 1080),
        };

        let roots = problem.gen_fire_roots();
        problem.initialize_strategy(&roots);

        log::info!("Initialized problem configuration. settings={:?}.", &problem.settings);

        problem
    }

    /// Initialize the strategy used to contain the fire
    fn initialize_strategy(&mut self, roots: &Vec<usize>) {
        if let OSMFStrategy::MultiMinDistanceSets(ref mut min_dist_sets_strategy_strategy) = self.strategy {
            min_dist_sets_strategy_strategy.initialize_undefended_roots(roots);
            min_dist_sets_strategy_strategy.compute_nodes_to_defend(roots, &self.settings, &self.node_data);
        } else if let OSMFStrategy::SingleMinDistanceSet(ref mut min_dist_sets_strategy) = self.strategy {
            min_dist_sets_strategy.compute_nodes_to_defend(roots, &self.settings);
        } else if let OSMFStrategy::Priority(ref mut priority_strategy) = self.strategy {
            priority_strategy.initialize_undefended_roots(roots);
            priority_strategy.compute_nodes_to_defend(roots, &self.settings, &self.node_data);
        }

        log::info!("Initialized fire containment strategy");
    }

    /// Generate `num_roots` fire roots
    fn gen_fire_roots(&mut self) -> Vec<usize> {
        let graph = self.graph.read().unwrap();

        let mut rng = thread_rng();
        let roots = graph.nodes.iter()
            .map(|node| node.id)
            .choose_multiple(&mut rng, self.settings.num_roots);

        self.node_data.mark_burning(&roots, self.global_time);

        log::info!("Generated fire roots");

        roots
    }

    /// Spread the fire to all nodes that are adjacent to burning nodes.
    /// Defended nodes will remain defended.
    fn spread_fire(&mut self) {
        let mut to_burn = Vec::new();
        {
            let burning = self.node_data.get_burning();

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
                    if self.node_data.is_undefended(&edge.tgt) {
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
        self.node_data.mark_burning(&to_burn, self.global_time);
    }

    /// Execute the containment strategy to prevent as much nodes as
    /// possible from catching fire
    fn contain_fire(&mut self) {
        if self.global_time % self.settings.strategy_every == 0 {
            match self.strategy {
                OSMFStrategy::Greedy(ref mut greedy_strategy) =>
                    greedy_strategy.execute(&self.settings, &mut self.node_data, self.global_time),
                OSMFStrategy::MultiMinDistanceSets(ref mut min_dist_sets_strategy) =>
                    min_dist_sets_strategy.execute(&self.settings, &mut self.node_data, self.global_time),
                OSMFStrategy::SingleMinDistanceSet(ref mut min_dist_sets_strategy) =>
                    min_dist_sets_strategy.execute(&self.settings, &mut self.node_data, self.global_time),
                OSMFStrategy::Priority(ref mut priority_strategy) =>
                    priority_strategy.execute(&self.settings, &mut self.node_data, self.global_time),
                OSMFStrategy::Score(ref mut score_strategy) =>
                    score_strategy.execute(&self.settings, &mut self.node_data, self.global_time),
                OSMFStrategy::Random(ref mut random_strategy) =>
                    random_strategy.execute(&self.settings, &mut self.node_data, self.global_time)
            }
        }
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
        log::info!("Starting problem simulation");

        while self.is_active {
            self.exec_step();
        }
    }

    /// Generate the simulation response for this firefighter problem instance
    pub fn simulation_response(&self) -> OSMFSimulationResponse {
        log::info!("Generating simulation response");

        OSMFSimulationResponse {
            nodes_burned: self.node_data.burning.len(),
            nodes_defended: self.node_data.defended.len(),
            nodes_total: self.graph.read().unwrap().num_nodes,
            end_time: self.global_time,
            view_bounds: &self.view.grid_bounds,
            view_center: self.view.initial_center,
        }
    }

    /// Generate the view response for this firefighter problem instance
    pub fn view_response(&mut self, center: Coords, zoom: f64, time: &TimeUnit) -> Vec<u8> {
        log::info!("Generating view response. center={:?}, zoom={}, time={}.", center, zoom, time);

        self.view.compute(center, zoom, time, &self.node_data);
        self.view.png_bytes()
    }

    /// Generate the alternative view response for this firefighter problem instance
    pub fn view_response_alt(&mut self, zoom: f64, time: &TimeUnit) -> Vec<u8> {
        log::info!("Generating view response. zoom={}, time={}.", zoom, time);

        self.view.compute_alt(zoom, time, &self.node_data);
        self.view.png_bytes()
    }

    pub fn sim_step_metadata_response(&self, time: &TimeUnit) -> OSMFSimulationStepMetadata {
        log::info!("Generating simulation step metadata response. time={}.", time);

        OSMFSimulationStepMetadata {
            nodes_burned_by: self.node_data.count_burning_by(time),
            nodes_defended_by: self.node_data.count_defended_by(time),
            nodes_burned_at: self.node_data.get_burning_at(time),
            nodes_defended_at: self.node_data.get_defended_at(time),
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, RwLock};

    use once_cell::sync::Lazy;

    use crate::firefighter::{problem::{OSMFProblem, OSMFSettings},
                             strategy::{OSMFStrategy,
                                        GreedyStrategy,
                                        MultiMinDistSetsStrategy,
                                        RandomStrategy,
                                        PriorityStrategy,
                                        Strategy}};
    use crate::graph::Graph;

    static GRAPH: Lazy<Arc<RwLock<Graph>>> = Lazy::new(||
        Arc::new(RwLock::new(Graph::from_file("data/bbgrund_undirected.fmi"))));

    fn initialize(strategy: OSMFStrategy) -> OSMFProblem {
        OSMFProblem::new(
            GRAPH.clone(),
            OSMFSettings {
                graph_name: "bbgrund".to_string(),
                strategy_name:"greedy".to_string(),
                num_roots: 10,
                num_ffs: 2,
                strategy_every: 10,
            },
            strategy)
    }

    #[test]
    fn test_roots() {
        let problem = initialize(OSMFStrategy::Random(
            RandomStrategy::new(GRAPH.clone())));

        let num_burning = problem.node_data.burning.len();
        let num_roots = problem.settings.num_roots;
        assert_eq!(num_burning, num_roots, "num burning: {}, num roots: {}", num_burning, num_roots);
    }

    #[test]
    fn test_active() {
        let mut problem = initialize(OSMFStrategy::Random(
            RandomStrategy::new(GRAPH.clone())));
        problem.simulate();

        assert!(!problem.is_active);
    }

    #[test]
    fn test_burned() {
        let mut problem = initialize(OSMFStrategy::Random(
            RandomStrategy::new(GRAPH.clone())));
        problem.simulate();

        let burned_times: Vec<_> = problem.node_data.burning.values()
            .map(|nd| nd.time)
            .collect();
        for time in burned_times {
            assert!(time <= problem.global_time, "burned time: {}, global time: {}",
                    time, problem.global_time);
        }
    }

    #[test]
    fn test_greedy() {
        let mut problem = initialize(OSMFStrategy::Greedy(
            GreedyStrategy::new(GRAPH.clone())));
        problem.simulate();

        let ffs = problem.settings.num_ffs;
        let gt = problem.global_time as usize;
        let se = problem.settings.strategy_every as usize;
        let num_defended = problem.node_data.defended.len();
        let should_defended = ffs * (gt / se);
        assert!(num_defended <= should_defended, "num defended: {}, should defended: {}",
                num_defended, should_defended);

        let num_ambiguous = problem.node_data.burning.keys()
            .filter(|&node_id| problem.node_data.defended.contains_key(node_id))
            .count();
        assert_eq!(num_ambiguous, 0, "num ambiguous: {}", num_ambiguous);
    }

    #[test]
    fn test_min_dist_group() {
        let mut problem = initialize(OSMFStrategy::MultiMinDistanceSets(
            MultiMinDistSetsStrategy::new(GRAPH.clone())));
        problem.simulate();

        let ffs = problem.settings.num_ffs;
        let gt = problem.global_time as usize;
        let se = problem.settings.strategy_every as usize;
        let num_defended = problem.node_data.defended.len();
        let should_defended = ffs * (gt / se);
        assert!(num_defended <= should_defended, "num defended: {}, should defended: {}",
                num_defended, should_defended);

        let num_ambiguous = problem.node_data.burning.keys()
            .filter(|&node_id| problem.node_data.defended.contains_key(node_id))
            .count();
        assert_eq!(num_ambiguous, 0, "num ambiguous: {}", num_ambiguous);
    }

    #[test]
    fn test_prio() {
        let mut problem = initialize(OSMFStrategy::Priority(
            PriorityStrategy::new(GRAPH.clone())));
        problem.simulate();

        let ffs = problem.settings.num_ffs;
        let gt = problem.global_time as usize;
        let se = problem.settings.strategy_every as usize;
        let num_defended = problem.node_data.defended.len();
        let should_defended = ffs * (gt / se);
        assert!(num_defended <= should_defended, "num defended: {}, should defended: {}",
                num_defended, should_defended);

        let num_ambiguous = problem.node_data.burning.keys()
            .filter(|&node_id| problem.node_data.defended.contains_key(node_id))
            .count();
        assert_eq!(num_ambiguous, 0, "num ambiguous: {}", num_ambiguous);
    }

    #[test]
    fn test_rand() {
        let mut problem = initialize(OSMFStrategy::Random(
            RandomStrategy::new(GRAPH.clone())));
        problem.simulate();

        let ffs = problem.settings.num_ffs;
        let gt = problem.global_time as usize;
        let se = problem.settings.strategy_every as usize;
        let num_defended = problem.node_data.defended.len();
        let should_defended = ffs * (gt / se);
        assert!(num_defended <= should_defended, "num defended: {}, should defended: {}",
                num_defended, should_defended);

        let num_ambiguous = problem.node_data.burning.keys()
            .filter(|&node_id| problem.node_data.defended.contains_key(node_id))
            .count();
        assert_eq!(num_ambiguous, 0, "num ambiguous: {}", num_ambiguous);
    }
}