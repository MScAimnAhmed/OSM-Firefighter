use std::{fmt::Formatter,
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
    pub hub_id: usize,
    dist: usize,
    dir: HubDirection,
}

/// A graph node with id, latitude and longitude
#[derive(Debug, Serialize)]
pub struct Node {
    pub id: usize,
    lat: f64,
    lon: f64,
    bwd_hubs: Vec<HubLabel>,
    fwd_hubs: Vec<HubLabel>,
}

/// A directed graph edge with source and target
#[derive(Debug, Serialize)]
pub struct Edge {
    pub src: usize,
    pub tgt: usize,
    pub dist: usize,
}

/// A directed graph with nodes, edges and node offsets
#[derive(Debug, Serialize)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub offsets: Vec<usize>,
    pub num_nodes: usize,
    pub num_edges: usize,
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
    fn parse_graph_with_hubs(&mut self, file_path: String) -> Result<(), ParseGraphError> {
        self.parse_graph((file_path.clone() + ".fmi").as_str())?;
        self.parse_hubs((file_path + ".hub").as_str())
    }

    /// Parse node and edge data from a file into a directed graph
    fn parse_graph(&mut self, graph_file_path: &str) -> Result<(), ParseGraphError> {
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
    fn parse_hubs(&mut self, hub_file_path: &str) -> Result<(), ParseGraphError> {
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
    fn parse_hub(&mut self, lines: &mut Lines<BufReader<File>>, line_no: usize, direction: HubDirection) -> Result<(), ParseGraphError> {
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
            None => Err(ParseGraphError::InvalidNodeId(node_id))
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
}

#[derive(Debug)]
enum ParseGraphError {
    IO(std::io::Error),
    ParseInt(ParseIntError),
    ParseFloat(ParseFloatError),
    InvalidNodeId(usize),
}

impl std::fmt::Display for ParseGraphError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IO(err) => write!(f, "{}", err.to_string()),
            Self::ParseInt(err) => write!(f, "{}", err.to_string()),
            Self::ParseFloat(err) => write!(f, "{}", err.to_string()),
            Self::InvalidNodeId(node_id) => write!(f, "Invalid node id {}", node_id)
        }
    }
}

impl std::error::Error for ParseGraphError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Self::IO(ref err) => Some(err),
            Self::ParseInt(ref err) => Some(err),
            Self::ParseFloat(ref err) => Some(err),
            Self::InvalidNodeId(..) => None,
        }
    }
}

impl From<std::io::Error> for ParseGraphError {
    fn from(err: std::io::Error) -> Self {
        Self::IO(err)
    }
}

impl From<ParseIntError> for ParseGraphError {
    fn from(err: ParseIntError) -> Self {
        Self::ParseInt(err)
    }
}

impl From<ParseFloatError> for ParseGraphError {
    fn from(err: ParseFloatError) -> Self {
        Self::ParseFloat(err)
    }
}

#[cfg(test)]
mod test {
    use crate::graph::Graph;

    #[test]
    fn test_graph() {
        let graph = Graph::from_files("resources/bbgrund_ch");

        assert_eq!(graph.nodes.len(), 350);
        assert_eq!(graph.edges.len(), 1087);

        for i in 0..graph.nodes.len() {
            let node = graph.nodes.get(i).unwrap();
            assert!(node.lat >= 48.67 && node.lat < 48.68);
            assert!(node.lon >= 8.99 && node.lon < 9.02);
        }

        let edges_with_src_70: Vec<_> = graph.edges.iter()
            .filter(|&e| e.src == 70)
            .collect();
        assert_eq!(edges_with_src_70.len(), 9);
    }
}