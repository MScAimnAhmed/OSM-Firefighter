use std::fmt::Formatter;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::num::{ParseIntError, ParseFloatError};

/// A graph node with id, latitude and longitude
#[derive(Debug)]
pub struct Node {
    pub id: usize,
    lat: f64,
    lon: f64
}

/// A directed graph edge with source and target
#[derive(Debug)]
pub struct Edge {
    pub src: usize,
    pub tgt: usize
}

/// A directed graph with nodes, edges and node offsets
#[derive(Debug)]
pub struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    offsets: Vec<usize>
}
impl Graph {
    /// Create a new directed graph without any nodes or edges
    pub fn new() -> Self {
        Graph {
            nodes: Vec::new(),
            edges: Vec::new(),
            offsets: Vec::new()
        }
    }

    fn parse_graph(file_path: &str) -> Result<Self, ParseGraphError> {
        if !file_path.ends_with(".fmi") {
            return Err(ParseGraphError::WrongFileFormat);
        }

        let graph_file = File::open(file_path)?;
        let graph_reader = BufReader::new(graph_file);

        let mut graph = Graph::new();
        let nodes = &mut graph.nodes;
        let edges = &mut graph.edges;
        let offsets = &mut graph.offsets;

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

        let num_nodes: usize = lines.next()
            .expect("Unexpected EOF while parsing number of nodes")?
            .parse()?;
        let num_edges: usize = lines.next()
            .expect("Unexpected EOF while parsing number of edges")?
            .parse()?;
        line_no += 3;

        nodes.reserve_exact(num_nodes);
        for i in 0..num_nodes {
            let line = lines.next()
                .expect(&format!("Unexpected EOF while parsing nodes in line {}", line_no))?;
            let mut split = line.split(" ");
            line_no += 1;
            split.next();

            let node = Node {
                id: i,
                lat: split.next()
                    .expect(&format!("Unexpected EOL while parsing node latitude in line {}",
                                     line_no))
                    .parse()?,
                lon: split.next()
                    .expect(&format!("Unexpected EOL while parsing node longitude in line {}",
                                     line_no))
                    .parse()?
            };
            nodes.push(node);
        }

        let mut last_src: i64 = -1;
        let mut offset: usize = 0;
        edges.reserve_exact(num_edges);
        offsets.resize(num_nodes+1, 0);
        for _ in 0..num_edges {
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
                    .parse()?
            };

            if edge.src as i64 > last_src {
                for j in (last_src+1) as usize..=edge.src {
                    offsets[j] = offset;
                }
                last_src = edge.src as i64;
            }
            offset += 1;

            edges.push(edge);
        }
        offsets[num_nodes] = num_edges;

        Ok(graph)
    }

    /// Create a directed graph from a file that contains node and edge data
    pub fn from_file(file_path: &str) -> Self {
        match Graph::parse_graph(file_path) {
            Ok(graph) => graph,
            Err(err) => panic!("Failed to create graph from file {}: {}", file_path,
                               err.to_string())
        }
    }
}
impl ToString for Graph {
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}

#[derive(Debug)]
enum ParseGraphError {
    IO(std::io::Error),
    ParseInt(ParseIntError),
    ParseFloat(ParseFloatError),
    WrongFileFormat
}
impl std::fmt::Display for ParseGraphError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseGraphError::IO(err) => write!(f, "{}", err.to_string()),
            ParseGraphError::ParseInt(err) => write!(f, "{}", err.to_string()),
            ParseGraphError::ParseFloat(err) => write!(f, "{}", err.to_string()),
            ParseGraphError::WrongFileFormat =>
                write!(f, "Graph files must have the '.fmi' file extension")
        }
    }
}
impl std::error::Error for ParseGraphError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            ParseGraphError::IO(ref err) => Some(err),
            ParseGraphError::ParseInt(ref err) => Some(err),
            ParseGraphError::ParseFloat(ref err) => Some(err),
            ParseGraphError::WrongFileFormat => None
        }
    }
}
impl From<std::io::Error> for ParseGraphError {
    fn from(err: std::io::Error) -> Self {
        ParseGraphError::IO(err)
    }
}
impl From<ParseIntError> for ParseGraphError {
    fn from(err: ParseIntError) -> Self {
        ParseGraphError::ParseInt(err)
    }
}
impl From<ParseFloatError> for ParseGraphError {
    fn from(err: ParseFloatError) -> Self {
        ParseGraphError::ParseFloat(err)
    }
}