use std::{cmp::Ordering,
          fmt::Formatter,
          fs::File,
          io::{prelude::*, BufReader, Lines},
          num::{ParseIntError, ParseFloatError}};

use serde::Serialize;

/// Was the hub calculated via backward or forward search?
#[derive(Debug, Serialize)]
enum HubDirection {
    Backward,
    Forward,
}

/// Hub label for a node
#[derive(Debug, Serialize)]
struct HubLabel {
    hub_id: usize,
    dist: usize,
    dir: HubDirection,
}

/// Struct to hold the grid bounds of a graph or part of a graph
#[derive(Debug)]
pub struct GridBounds {
    pub min_lat: f64,
    pub max_lat: f64,
    pub min_lon: f64,
    pub max_lon: f64,
}

impl GridBounds {
    /// Returns true if this grid bounds are located within `other`
    pub fn is_located_in(&self, other: &GridBounds) -> bool {
        self.min_lat >= other.min_lat && self.max_lat <= other.max_lat
            && self.min_lon >= other.min_lon && self.max_lon <= other.max_lon
    }
}

/// Compass directions related to grid bounds
pub enum CompassDirection {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
    Zero,
}

/// A graph node with id, latitude and longitude
#[derive(Debug, Serialize, Default)]
pub struct Node {
    pub id: usize,
    pub lat: f64,
    pub lon: f64,
    bwd_hubs: Vec<HubLabel>,
    fwd_hubs: Vec<HubLabel>,
}

impl Node {
    /// Returns true if this node is located within the given grid bounds
    pub fn is_located_in(&self, gb: &GridBounds) -> bool {
        self.lat >= gb.min_lat && self.lat <= gb.max_lat
            && self.lon >= gb.min_lon && self.lon  <= gb.max_lon
    }

    /// Get the compass direction of this node relative to the given grid bounds
    pub fn get_relative_compass_direction(&self, gb: &GridBounds) -> CompassDirection {
        if self.lat >= gb.min_lat && self.lat <= gb.max_lat && self.lon > gb.max_lon {
            CompassDirection::North
        } else if self.lat > gb.max_lat && self.lon > gb.max_lon {
            CompassDirection::NorthEast
        } else if self.lat > gb.max_lat && self.lon >= gb.min_lon && self.lon <= gb.max_lon {
            CompassDirection::East
        } else if self.lat > gb.max_lat && self.lon < gb.min_lon {
            CompassDirection::SouthEast
        } else if self.lat >= gb.min_lat && self.lat <= gb.max_lat && self.lon < gb.min_lon {
            CompassDirection::South
        } else if self.lat < gb.min_lat && self.lon < gb.min_lon {
            CompassDirection::SouthWest
        } else if self.lat < gb.min_lat && self.lon >= gb.min_lon && self.lon <= gb.max_lon {
            CompassDirection::West
        } else if self.lat < gb.min_lat && self.lon > gb.max_lon {
            CompassDirection::NorthWest
        } else {
            CompassDirection::Zero
        }
    }
}

/// A directed graph edge with source and target
#[derive(Debug, Serialize, Default)]
pub struct Edge {
    pub src: usize,
    pub tgt: usize,
    pub dist: usize,
}

/// A directed graph with nodes, edges and node offsets
#[derive(Debug, Serialize, Default)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub offsets: Vec<usize>,
    pub num_nodes: usize,
    pub num_edges: usize,
}

/// Unstable float comparison.
/// # Returns
/// * `a < b`: `Ordering::Less`
/// * `a >= b`: `Ordering::Greater`
fn unstable_cmp_f64(a: f64, b: f64) -> Ordering {
    if a < b { Ordering::Less } else { Ordering::Greater }
}

impl Graph {
    /// Create a new directed graph without any nodes or edges
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            offsets: Vec::new(),
            num_nodes: 0,
            num_edges: 0,
        }
    }

    /// Parse node and edge data from one file and hub labels from another file
    /// into a directed graph
    fn parse_graph_with_hubs(&mut self, file_path: String) -> Result<(), ParseError> {
        self.parse_graph((file_path.clone() + ".fmi").as_str())?;
        self.parse_hubs((file_path + ".ch.hub").as_str())
    }

    /// Parse node and edge data from a file into a directed graph
    fn parse_graph(&mut self, graph_file_path: &str) -> Result<(), ParseError> {
        let graph_file = File::open(graph_file_path)?;
        let graph_reader = BufReader::new(graph_file);

        let mut lines = graph_reader.lines();
        let mut line_no = 0;

        loop {
            let line = lines.next()
                .expect(&format!("Unexpected EOF while parsing header in line {}", line_no))?;
            line_no += 1;

            if !line.starts_with("#") {
                break;
            }
        }

        self.num_nodes = lines.next()
            .expect("Unexpected EOF while parsing number of nodes")?
            .parse()?;
        self.num_edges = lines.next()
            .expect("Unexpected EOF while parsing number of edges")?
            .parse()?;
        line_no += 3;

        self.nodes.reserve_exact(self.num_nodes);
        for i in 0..self.num_nodes {
            let line = lines.next()
                .expect(&format!("Unexpected EOF while parsing nodes in line {}", line_no))?;
            let mut split = line.split(" ");
            line_no += 1;
            split.next(); // id
            split.next(); // second id

            let node = Node {
                id: i,
                lat: split.next()
                    .expect(&format!("Unexpected EOL while parsing node latitude in line {}",
                                     line_no))
                    .parse()?,
                lon: split.next()
                    .expect(&format!("Unexpected EOL while parsing node longitude in line {}",
                                     line_no))
                    .parse()?,
                bwd_hubs: Vec::new(),
                fwd_hubs: Vec::new(),
            };
            self.nodes.push(node);
        }

        let mut last_src: i64 = -1;
        let mut offset: usize = 0;
        self.edges.reserve_exact(self.num_edges);
        self.offsets.resize(self.num_nodes + 1, 0);
        for _ in 0..self.num_edges {
            let line = lines.next()
                .expect(&format!("Unexpected EOF while parsing edges in line {}", line_no))?;
            let mut split = line.split(" ");
            line_no += 1;

            let edge = Edge {
                src: split.next()
                    .expect(&format!("Unexpected EOL while parsing edge source in line {}",
                                     line_no))
                    .parse()?,
                tgt: split.next()
                    .expect(&format!("Unexpected EOL while parsing edge target in line {}",
                                     line_no))
                    .parse()?,
                dist: split.next()
                    .expect(&format!("Unexpected EOL while parsing edge weight in line {}",
                                     line_no))
                    .parse()?,
            };

            if edge.src as i64 > last_src {
                for j in (last_src + 1) as usize..=edge.src {
                    self.offsets[j] = offset;
                }
                last_src = edge.src as i64;
            }
            offset += 1;

            self.edges.push(edge);
        }
        self.offsets[self.num_nodes] = self.num_edges;

        Ok(())
    }

    /// Parse hub labels from a file and add them to their respective nodes
    fn parse_hubs(&mut self, hub_file_path: &str) -> Result<(), ParseError> {
        let hub_file = File::open(hub_file_path)?;
        let hub_reader = BufReader::new(hub_file);

        let mut lines = hub_reader.lines();
        let mut line_no = 0;

        let num_bwd_hubs: usize = lines.next()
            .expect("Unexpected EOF while parsing number of backward hubs")?
            .parse()?;
        let num_fwd_hubs: usize = lines.next()
            .expect("Unexpected EOF while parsing number of forward hubs")?
            .parse()?;
        line_no += 2;

        for _ in 0..num_bwd_hubs {
            line_no += 1;
            self.parse_hub(&mut lines, line_no, HubDirection::Backward)?;
        }
        for _ in 0..num_fwd_hubs {
            line_no += 1;
            self.parse_hub(&mut lines, line_no, HubDirection::Forward)?;
        }

        Ok(())
    }

    /// Parse a single hub label and add it to its respective node
    fn parse_hub(&mut self, lines: &mut Lines<BufReader<File>>, line_no: usize, direction: HubDirection) -> Result<(), ParseError> {
        let line = lines.next()
            .expect(&format!("Unexpected EOF while parsing hub label in line {}", line_no))?;
        let mut split = line.split(" ");

        let node_id: usize = split.next()
            .expect(&format!("Unexpected EOL while parsing node id in line {}",
                             line_no))
            .parse()?;

        let hub_label = HubLabel {
            hub_id: split.next()
                .expect(&format!("Unexpected EOL while parsing hub id in line {}",
                                 line_no))
                .parse()?,
            dist: split.next()
                .expect(&format!("Unexpected EOL while parsing distance to hub in line {}",
                                 line_no))
                .parse()?,
            dir: direction,
        };

        match self.nodes.get_mut(node_id) {
            Some(node) => {
                match hub_label.dir {
                    HubDirection::Backward => node.bwd_hubs.push(hub_label),
                    HubDirection::Forward => node.fwd_hubs.push(hub_label)
                }
                Ok(())
            },
            None => Err(ParseError::InvalidNode(node_id))
        }
    }

    /// Create a directed graph from a file that contains node and edge data
    pub fn from_files(file_path: &str) -> Self {
        let mut graph = Graph::new();
        match graph.parse_graph_with_hubs(file_path.to_string()) {
            Ok(_) => (),
            Err(err) => panic!("Failed to create graph from file {}: {}", file_path,
                               err.to_string())
        }
        graph
    }

    /// Get the number of outgoing edges of the node with id `node_id`
    pub fn get_out_degree(&self, node_id: usize) -> usize {
        self.offsets[node_id + 1] - self.offsets[node_id]
    }

    /// Get the shortest distance between the node with id `src_id` and the node with id `tgt_id`.
    /// Returns `usize::MAX` if no path exists.
    pub fn unchecked_get_shortest_dist(&self, src_id: usize, tgt_id: usize) -> usize {
        let src = &self.nodes[src_id];
        let tgt = &self.nodes[tgt_id];

        let mut ind_s = 0;
        let mut ind_t = 0;
        let sz_s = src.fwd_hubs.len();
        let sz_t = tgt.bwd_hubs.len();

        let mut best_dist = usize::MAX;

        while (ind_s < sz_s) && (ind_t < sz_t) {
            let src_hub = &src.fwd_hubs[ind_s];
            let tgt_hub = &tgt.bwd_hubs[ind_t];

            match src_hub.hub_id.cmp(&tgt_hub.hub_id) {
                Ordering::Equal => {
                    let hub_dist = src_hub.dist + tgt_hub.dist;
                    if best_dist > hub_dist {
                        best_dist = hub_dist;
                    }
                    ind_s += 1;
                    ind_t += 1;
                }
                Ordering::Less => {
                    ind_s += 1;
                }
                Ordering::Greater => {
                    ind_t += 1;
                }
            }
        }

        best_dist
    }

    /// Get the shortest distance between the node with id `src_id` and the node with id `tgt_id`.
    /// Returns error if no path exists.
    pub fn get_shortest_dist(&self, src_id: usize, tgt_id: usize) -> Result<usize, ComputationError> {
        let best_dist = self.unchecked_get_shortest_dist(src_id, tgt_id);
        if best_dist < usize::MAX {
            Ok(best_dist)
        } else {
            Err(ComputationError::NoPath(src_id, tgt_id))
        }
    }

    /// Returns this graphs grid bounds, i.e. the minimal/maximal latitude/longitude
    /// of this graph
    pub fn get_grid_bounds(&self) -> GridBounds {
        let latitudes: Vec<_> = self.nodes.iter()
            .map(|n| n.lat)
            .collect();
        let longitudes: Vec<_> = self.nodes.iter()
            .map(|n| n.lon)
            .collect();

        GridBounds {
            min_lat: *latitudes.iter()
                .min_by(|&lat1, &lat2| unstable_cmp_f64(*lat1, *lat2))
                .unwrap_or(&f64::NAN),
            max_lat: *latitudes.iter()
                .max_by(|&lat1, &lat2| unstable_cmp_f64(*lat1, *lat2))
                .unwrap_or(&f64::NAN),
            min_lon: *longitudes.iter()
                .min_by(|&lon1, &lon2| unstable_cmp_f64(*lon1, *lon2))
                .unwrap_or(&f64::NAN),
            max_lon: *longitudes.iter()
                .max_by(|&lon1, &lon2| unstable_cmp_f64(*lon1, *lon2))
                .unwrap_or(&f64::NAN),
        }
    }
}

#[derive(Debug)]
enum ParseError {
    IO(std::io::Error),
    ParseInt(ParseIntError),
    ParseFloat(ParseFloatError),
    InvalidNode(usize),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IO(err) => write!(f, "{}", err.to_string()),
            Self::ParseInt(err) => write!(f, "{}", err.to_string()),
            Self::ParseFloat(err) => write!(f, "{}", err.to_string()),
            Self::InvalidNode(node_id) => write!(f, "Invalid node {}", node_id)
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Self::IO(ref err) => Some(err),
            Self::ParseInt(ref err) => Some(err),
            Self::ParseFloat(ref err) => Some(err),
           _ => None
        }
    }
}

impl From<std::io::Error> for ParseError {
    fn from(err: std::io::Error) -> Self {
        Self::IO(err)
    }
}

impl From<ParseIntError> for ParseError {
    fn from(err: ParseIntError) -> Self {
        Self::ParseInt(err)
    }
}

impl From<ParseFloatError> for ParseError {
    fn from(err: ParseFloatError) -> Self {
        Self::ParseFloat(err)
    }
}

#[derive(Debug)]
pub enum ComputationError {
    NoPath(usize, usize),
}

impl std::fmt::Display for ComputationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoPath(src_id, tgt_id) =>
                write!(f, "No path between nodes {} and {}", src_id, tgt_id)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::graph::Graph;

    #[test]
    fn test_graph() {
        let graph = Graph::from_files("data/bbgrund");

        assert_eq!(graph.nodes.len(), 350);
        assert_eq!(graph.edges.len(), 685);

        let gb = graph.get_grid_bounds();
        assert!(gb.min_lat >= 48.67);
        assert!(gb.max_lat < 48.68);
        assert!(gb.min_lon >= 8.99);
        assert!(gb.max_lon < 9.02);

        for i in 0..graph.nodes.len() {
            let node = graph.nodes.get(i).unwrap();
            assert!(node.lat >= 48.67 && node.lat < 48.68);
            assert!(node.lon >= 8.99 && node.lon < 9.02);
        }

        let edges_with_src_70: Vec<_> = graph.edges.iter()
            .filter(|&e| e.src == 70)
            .collect();
        assert_eq!(edges_with_src_70.len(), 3);

        assert_eq!(graph.nodes[70].bwd_hubs.len(), 6);
        assert_eq!(graph.nodes[70].fwd_hubs.len(), 3);

        for node in &graph.nodes {
            if !node.bwd_hubs.is_empty() {
                assert!(node.bwd_hubs[0].hub_id <= node.bwd_hubs[node.bwd_hubs.len()-1].hub_id);
            }
            if !node.fwd_hubs.is_empty() {
                assert!(node.fwd_hubs[0].hub_id <= node.fwd_hubs[node.fwd_hubs.len()-1].hub_id);
            }
        }

        assert_eq!(graph.get_shortest_dist(1, 321).unwrap(), 822);
    }
}