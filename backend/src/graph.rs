use std::cmp::Ordering;
use std::fmt::Formatter;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::num::{ParseIntError, ParseFloatError};

use serde::Serialize;

use crate::binary_minheap::BinaryMinHeap;

/// Type alias for the result of a run of the Dijkstra algorithm
type DijkstraResult = Vec<usize>;

/// Struct to hold the grid bounds of a graph or part of a graph
#[derive(Debug, Serialize)]
pub(crate) struct GridBounds {
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
pub(crate) enum CompassDirection {
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

/// A graph node
///
/// # Attributes
/// * `id` - An id uniquely identifying the node
/// * `lat` - The nodes latitude coordinate
/// * `lon` - The nodes longitude coordinate
#[derive(Debug, Serialize, Default)]
pub struct Node {
    pub id: usize,
    pub lat: f64,
    pub lon: f64,
}

impl Node {
    /// Returns true if this node is located within the given grid bounds
    pub(crate) fn is_located_in(&self, gb: &GridBounds) -> bool {
        self.lat >= gb.min_lat && self.lat <= gb.max_lat
            && self.lon >= gb.min_lon && self.lon  <= gb.max_lon
    }

    /// Get the compass direction of this node relative to the given grid bounds
    pub(crate) fn get_relative_compass_direction(&self, gb: &GridBounds) -> CompassDirection {
        if self.lon >= gb.min_lon && self.lon <= gb.max_lon && self.lat > gb.max_lat {
            CompassDirection::North
        } else if self.lon > gb.max_lon && self.lat > gb.max_lat {
            CompassDirection::NorthEast
        } else if self.lon > gb.max_lon && self.lat >= gb.min_lat && self.lat <= gb.max_lat {
            CompassDirection::East
        } else if self.lon > gb.max_lon && self.lat < gb.min_lat {
            CompassDirection::SouthEast
        } else if self.lon >= gb.min_lon && self.lon <= gb.max_lon && self.lat < gb.min_lat {
            CompassDirection::South
        } else if self.lon < gb.min_lon && self.lat < gb.min_lat {
            CompassDirection::SouthWest
        } else if self.lon < gb.min_lon && self.lat >= gb.min_lat && self.lat <= gb.max_lat {
            CompassDirection::West
        } else if self.lon < gb.min_lon && self.lat > gb.max_lat {
            CompassDirection::NorthWest
        } else {
            CompassDirection::Zero
        }
    }
}

/// A directed and weighted graph edge
///
/// # Attributes
/// * `src` - The id of the source node
/// * `tgt` - The id of the target node
/// * `dist` - The distance between source and target
#[derive(Debug, Serialize, Default)]
pub struct Edge {
    pub src: usize,
    pub tgt: usize,
    pub dist: usize,
}

/// A directed and weighted graph with nodes and edges
#[derive(Debug, Serialize, Default)]
pub struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    offsets: Vec<usize>,
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
    /// Parse node and edge data from a file into a directed graph.
    /// Returns a `Result` containing the parsed graph if the operation succeeds, or an
    /// `Err` otherwise.
    pub fn parse_from_file(graph_file_path: &str) -> Result<Self, ParseError> {
        let graph_file = File::open(graph_file_path)?;
        let graph_reader = BufReader::new(graph_file);

        log::debug!("Start parsing graph: {}", graph_file_path);

        let mut lines = graph_reader.lines();
        let mut line_no = 0;

        loop {
            let line = lines.next()
                .expect(&format!("Unexpected EOF while parsing header after line {}", line_no))?;
            line_no += 1;

            if !line.starts_with("#") {
                break;
            }
        }

        let num_nodes = lines.next()
            .expect("Unexpected EOF while parsing number of nodes")?
            .parse()?;
        if num_nodes <= 0 {
            return Err(ParseError::EmptyNodes);
        }
        let num_edges = lines.next()
            .expect("Unexpected EOF while parsing number of edges")?
            .parse()?;
        line_no += 2;

        let mut nodes = Vec::with_capacity(num_nodes);
        for i in 0..num_nodes {
            let line = lines.next()
                .expect(&format!("Unexpected EOF while parsing nodes after line {}", line_no))?;
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
            };
            nodes.push(node);
        }
        log::debug!("Parsed {} nodes", num_nodes);

        let mut next_src: usize = 0;
        let mut offset: usize = 0;
        let mut edges = Vec::with_capacity(num_edges);
        let mut offsets = vec![0; num_nodes + 1];
        for _ in 0..num_edges {
            let line = lines.next()
                .expect(&format!("Unexpected EOF while parsing edges after line {}", line_no))?;
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

            if edge.src >= next_src {
                for j in next_src..=edge.src {
                    offsets[j] = offset;
                }
                next_src = edge.src + 1;
            }
            offset += 1;

            edges.push(edge);
        }
        for i in next_src..=num_nodes {
            offsets[i] = num_edges;
        }
        log::debug!("Parsed {} edges and computed node offsets", num_edges);

        Ok(Self {
            nodes,
            edges,
            offsets,
            num_nodes,
            num_edges,
        })
    }

    /// Returns a reference to the vector containing all graph nodes
    pub fn nodes(&self) -> &Vec<Node> {
        &self.nodes
    }

    /// Returns a reference to the node with id `node_id`
    pub fn get_node(&self, node_id: usize) -> &Node {
        &self.nodes[node_id]
    }

    /// Get the number of outgoing edges of the node with id `node_id`
    pub fn get_node_degree(&self, node_id: usize) -> usize {
        self.offsets[node_id + 1] - self.offsets[node_id]
    }

    /// Returns a reference to the vector containing all graph edges
    pub fn edges(&self) -> &Vec<Edge> {
        &self.edges
    }

    /// Get the outgoing edges of the node with id `node_id`
    pub fn get_outgoing_edges(&self, node_id: usize) -> &[Edge] {
        &self.edges[self.offsets[node_id]..self.offsets[node_id + 1]]
    }

    /// Run an one-to-all Dijkstra from the source node with id `src_id`
    pub fn run_dijkstra(&self, src_ids: &[usize]) -> DijkstraResult {
        let mut distances = vec![usize::MAX; self.num_nodes];
        for &src_id in src_ids {
            distances[src_id] = 0;
        }

        let mut pq = BinaryMinHeap::with_capacity(self.num_nodes);
        for &src_id in src_ids {
            pq.push(src_id, &distances);
        }

        while !pq.is_empty() {
            let node = pq.pop(&distances);

            for i in self.offsets[node]..self.offsets[node +1] {
                let edge = &self.edges[i];
                let dist = distances[node] + edge.dist;

                if dist < distances[edge.tgt] {
                    distances[edge.tgt] = dist;

                    if pq.contains(edge.tgt) {
                        pq.decrease_key(edge.tgt, &distances);
                    } else {
                        pq.push(edge.tgt, &distances);
                    }
                }
            }
        }

        distances
    }

    /// Returns this graphs grid bounds, i.e. the minimal/maximal latitude/longitude
    /// of this graph
    pub(crate) fn get_grid_bounds(&self) -> GridBounds {
        let latitudes: Vec<_> = self.nodes.iter()
            .map(|n| n.lat)
            .collect();
        let longitudes: Vec<_> = self.nodes.iter()
            .map(|n| n.lon)
            .collect();

        GridBounds {
            min_lat: *latitudes.iter()
                .min_by(|&lat1, &lat2| unstable_cmp_f64(*lat1, *lat2))
                // Calling unwrap is safe because the implementation of parse_graph ensures that the graph
                // consists of at least one node
                .unwrap(),
            max_lat: *latitudes.iter()
                .max_by(|&lat1, &lat2| unstable_cmp_f64(*lat1, *lat2))
                .unwrap(),
            min_lon: *longitudes.iter()
                .min_by(|&lon1, &lon2| unstable_cmp_f64(*lon1, *lon2))
                .unwrap(),
            max_lon: *longitudes.iter()
                .max_by(|&lon1, &lon2| unstable_cmp_f64(*lon1, *lon2))
                .unwrap(),
        }
    }
}

#[derive(Debug)]
pub enum ParseError {
    IO(std::io::Error),
    ParseInt(ParseIntError),
    ParseFloat(ParseFloatError),
    EmptyNodes,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IO(err) => write!(f, "{}", err.to_string()),
            Self::ParseInt(err) => write!(f, "{}", err.to_string()),
            Self::ParseFloat(err) => write!(f, "{}", err.to_string()),
            Self::EmptyNodes => write!(f, "Graph must consist of at least one node"),
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Self::IO(ref err) => Some(err),
            Self::ParseInt(ref err) => Some(err),
            Self::ParseFloat(ref err) => Some(err),
            Self::EmptyNodes => None,
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

#[cfg(test)]
mod test {
    use std::cmp::min;
    use rand::prelude::*;

    use crate::graph::Graph;

    #[test]
    fn test_nodes_edges() {
        let graph =
            Graph::parse_from_file("data/bbgrund_undirected.fmi").unwrap();

        assert_eq!(graph.nodes.len(), 350);
        assert_eq!(graph.edges.len(), 706);
    }

    #[test]
    fn test_grid_bounds() {
        let graph =
            Graph::parse_from_file("data/bbgrund_undirected.fmi").unwrap();

        let gb = graph.get_grid_bounds();
        assert!(gb.min_lat >= 48.67);
        assert!(gb.max_lat < 48.68);
        assert!(gb.min_lon >= 8.99);
        assert!(gb.max_lon < 9.02);
    }

    #[test]
    fn test_node() {
        let graph =
            Graph::parse_from_file("data/bbgrund_undirected.fmi").unwrap();

        let edges_with_src_70: Vec<_> = graph.edges.iter()
            .filter(|&e| e.src == 70)
            .collect();
        assert_eq!(edges_with_src_70.len(), 3);
    }

    #[test]
    fn test_dists() {
        let graph =
            Graph::parse_from_file("data/bbgrund_undirected.fmi").unwrap();

        let mut rng = thread_rng();
        let sources: Vec<_> = graph.nodes.iter()
            .map(|node| node.id)
            .choose_multiple(&mut rng, 2);
        let tgt = rng.gen_range(0..graph.num_nodes);

        let dists1 = graph.run_dijkstra(sources.as_slice());
        let dists2 = graph.run_dijkstra(&[sources[0]]);
        let dists3 = graph.run_dijkstra(&[sources[1]]);

        assert_eq!(min(dists2[tgt], dists3[tgt]), dists1[tgt]);
    }

    #[test]
    fn test_offsets() {
        let graph =
            Graph::parse_from_file("data/stgcenter_undirected.fmi").unwrap();

        let mut offsets_clone = graph.offsets.clone();
        offsets_clone.sort();
        assert_eq!(offsets_clone, graph.offsets);
    }
}