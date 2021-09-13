use std::{fmt::Formatter,
          fs::File,
          io::{prelude::*, BufReader},
          num::{ParseIntError, ParseFloatError}};

use serde::Serialize;

/// A graph node with id, latitude and longitude
#[derive(Debug, Serialize)]
pub struct Node {
    pub id: usize,
    lat: f64,
    lon: f64,
}

/// A directed graph edge with source and target
#[derive(Debug, Serialize)]
pub struct Edge {
    pub src: usize,
    pub tgt: usize,
}

/// A directed graph with nodes, edges and node offsets
#[derive(Debug, Serialize)]
pub struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    offsets: Vec<usize>,
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

    /// Parse node and edge data from a file into a directed graph
    fn parse_graph(&mut self, file_path: &str) -> Result<(), ParseGraphError> {
        if !file_path.ends_with(".fmi") {
            return Err(ParseGraphError::WrongFileFormat);
        }

        let graph_file = File::open(file_path)?;
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

    /// Create a directed graph from a file that contains node and edge data
    pub fn from_file(file_path: &str) -> Self {
        let mut graph = Graph::new();
        match graph.parse_graph(file_path) {
            Ok(graph) => graph,
            Err(err) => panic!("Failed to create graph from file {}: {}", file_path,
                               err.to_string())
        };
        graph
    }
}

#[derive(Debug)]
enum ParseGraphError {
    IO(std::io::Error),
    ParseInt(ParseIntError),
    ParseFloat(ParseFloatError),
    WrongFileFormat,
}

impl std::fmt::Display for ParseGraphError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IO(err) => write!(f, "{}", err.to_string()),
            Self::ParseInt(err) => write!(f, "{}", err.to_string()),
            Self::ParseFloat(err) => write!(f, "{}", err.to_string()),
            Self::WrongFileFormat => write!(f, "Graph files must have the '.fmi' file extension")
        }
    }
}

impl std::error::Error for ParseGraphError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Self::IO(ref err) => Some(err),
            Self::ParseInt(ref err) => Some(err),
            Self::ParseFloat(ref err) => Some(err),
            Self::WrongFileFormat => None
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
        let graph = Graph::from_file("resources/toy.fmi");

        assert_eq!(graph.nodes.len(), 5);
        assert_eq!(graph.edges.len(), 9);

        for i in 0..graph.nodes.len() {
            let node = graph.nodes.get(i).unwrap();
            assert_eq!(node.lat, (4900 + i) as f64 / 100.);
            assert_eq!(node.lon, (1000 + i) as f64 / 100.);
        }

        assert_eq!(graph.edges.get(0).unwrap().src, 0);
        assert_eq!(graph.edges.get(3).unwrap().tgt, 0);
        assert_eq!(graph.offsets.get(1).unwrap(), graph.offsets.get(2).unwrap());
    }
}