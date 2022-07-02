use std::collections::BTreeMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Instant;

use derive_more::{Display, Error};
use log;
use rand::prelude::*;
use serde::{Serialize, Deserialize};

use crate::firefighter::strategy::OSMFStrategy;
use crate::firefighter::TimeUnit;
use crate::firefighter::view::{View, Coords};
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

#[derive(Debug, Display, Error)]
pub enum OSMFSettingsError {
    #[display(fmt = "Number of fire roots must not be greater than {}: {}", num_nodes, num_roots)]
    InvalidNumRoots { num_nodes: usize, num_roots: usize },
}

/// Node data related to the firefighter problem
#[derive(Debug, Serialize)]
pub(super) struct NodeData {
    pub node_id: usize,
    time: TimeUnit,
}

/// Storage for node data
#[derive(Debug, Serialize)]
pub(super) struct NodeDataStorage {
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
    pub fn is_defended(&self, node_id: &usize) -> bool {
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
    fn mark_burning(&mut self, nodes: &Vec<usize>, time: TimeUnit) {
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
    pub fn mark_defended(&mut self, nodes: &[usize], time: TimeUnit) {
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
    fn get_burning_node_data(&self) -> Vec<&NodeData> {
        self.burning.values().collect()
    }

    /// Get the id's of all burning vertices
    pub fn get_burning(&self) -> Vec<usize> {
        self.burning.keys().map(usize::to_owned).collect()
    }

    /// Get the id's of all burning vertices at time `time`
    pub fn get_burning_at(&self, time: &TimeUnit) -> Vec<usize> {
        self.burning.values()
            .filter(|&nd| nd.time == *time)
            .map(|nd| nd.node_id)
            .collect::<Vec<_>>()
    }

    /// Get the id's of all fire roots, i.e., all burning vertices at time `0`
    pub fn get_roots(&self) -> Vec<usize> {
        self.get_burning_at(&0)
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
    pub simulation_time_millis: u128,
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
    graph: Arc<Graph>,
    settings: OSMFSettings,
    strategy: OSMFStrategy,
    node_data: NodeDataStorage,
    global_time: TimeUnit,
    simulation_time_millis: u128,
    is_active: bool,
    view: View,
}

impl OSMFProblem {
    /// Create a new firefighter problem instance
    pub fn new(graph: Arc<Graph>, settings: OSMFSettings, strategy: OSMFStrategy) -> Result<Self, OSMFSettingsError> {
        if settings.num_roots > graph.num_nodes {
            let err = OSMFSettingsError::InvalidNumRoots {
                num_nodes: graph.num_nodes,
                num_roots: settings.num_roots,
            };
            log::warn!("{}", err.to_string());
            return Err(err);
        }

        let problem = Self {
            graph: graph.clone(),
            settings,
            strategy,
            node_data: NodeDataStorage::new(),
            global_time: 0,
            simulation_time_millis: 0,
            is_active: true,
            view: View::new(graph, 1920, 1080),
        };
        log::info!("Initialized problem configuration. settings={:?}.", &problem.settings);

        Ok(problem)
    }

    /// Generate `num_roots` fire roots
    fn gen_fire_roots(&mut self) -> Vec<usize> {
        let mut rng = thread_rng();
        let roots = self.graph.nodes().iter()
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

        // For all undefended neighbours that are not already burning, check whether they have
        // to be added to `to_burn`
        self.is_active = false;
        for node_data in self.node_data.get_burning_node_data() {
            for edge in self.graph.get_outgoing_edges(node_data.node_id) {
                if self.node_data.is_undefended(&edge.tgt) {
                    // There is at least one node to be burned at some point in the future
                    if !self.is_active {
                        self.is_active = true;
                    }
                    // Burn the node if the global time exceeds the time at which the edge source
                    // started burning plus the edge weight
                    if self.global_time >= node_data.time + edge.dist as TimeUnit {
                        to_burn.push(edge.tgt);
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
            self.strategy.mut_inner().execute(&self.settings, &mut self.node_data, self.global_time);
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
        if !self.is_active {
            return;
        }

        log::info!("Starting problem simulation");

        let roots = self.gen_fire_roots();

        // Measure simulation time
        let start = Instant::now();

        self.strategy.initialize(&roots, &self.settings, &self.node_data);
        log::info!("Initialized fire containment strategy");

        while self.is_active {
            self.exec_step();
        }

        self.simulation_time_millis = start.elapsed().as_millis();

        log::info!("Finished problem simulation");
    }

    /// Generate the simulation response for this firefighter problem instance
    pub fn simulation_response(&self) -> OSMFSimulationResponse {
        log::info!("Generating simulation response");

        OSMFSimulationResponse {
            nodes_burned: self.node_data.burning.len(),
            nodes_defended: self.node_data.defended.len(),
            nodes_total: self.graph.num_nodes,
            end_time: self.global_time,
            simulation_time_millis: self.simulation_time_millis,
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
    use std::sync::Arc;

    use once_cell::sync::Lazy;

    use crate::firefighter::{problem::{OSMFProblem, OSMFSettings},
                             strategy::{OSMFStrategy,
                                        GreedyStrategy,
                                        MultiMinDistSetsStrategy,
                                        RandomStrategy,
                                        PriorityStrategy,
                                        Strategy}};
    use crate::firefighter::strategy::ScoreStrategy;
    use crate::graph::Graph;

    struct TestData {
        graph: Arc<Graph>,
        settings: OSMFSettings,
    }

    static TEST_DATA: Lazy<TestData> = Lazy::new(||
        TestData {
            graph: Arc::new(Graph::parse_from_file("data/bbgrund_undirected.fmi").unwrap()),
            settings: OSMFSettings {
                graph_name: "bbgrund".to_string(),
                strategy_name:"Greedy".to_string(),
                num_roots: 10,
                num_ffs: 2,
                strategy_every: 10,
            },
        });

    fn initialize(strategy: OSMFStrategy) -> OSMFProblem {
        OSMFProblem::new(TEST_DATA.graph.clone(), TEST_DATA.settings.clone(), strategy).unwrap()
    }

    #[test]
    fn test_roots() {
        let mut problem = initialize(OSMFStrategy::Random(
            RandomStrategy::new(TEST_DATA.graph.clone())));
        problem.simulate();

        let num_roots = problem.node_data.get_roots().len();
        let settings = &TEST_DATA.settings;
        assert_eq!(num_roots, settings.num_roots, "num burning at 0: {}, num roots: {}",
                   num_roots, settings.num_roots);
    }

    #[test]
    fn test_active() {
        let mut problem = initialize(OSMFStrategy::Random(
            RandomStrategy::new(TEST_DATA.graph.clone())));
        problem.simulate();

        assert!(!problem.is_active);
    }

    #[test]
    fn test_burned() {
        let mut problem = initialize(OSMFStrategy::Random(
            RandomStrategy::new(TEST_DATA.graph.clone())));
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
            GreedyStrategy::new(TEST_DATA.graph.clone())));
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
    fn test_score() {
        let mut problem = initialize(OSMFStrategy::Score(
            ScoreStrategy::new(TEST_DATA.graph.clone())));
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
            MultiMinDistSetsStrategy::new(TEST_DATA.graph.clone())));
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
            PriorityStrategy::new(TEST_DATA.graph.clone())));
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
            RandomStrategy::new(TEST_DATA.graph.clone())));
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