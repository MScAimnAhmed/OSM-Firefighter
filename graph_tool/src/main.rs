use std::collections::BTreeMap;
use std::env;
use std::fmt::Formatter;
use std::fs::File;
use std::io::{BufRead, BufReader, LineWriter, Write};
use std::num::{ParseFloatError, ParseIntError};

#[derive(Debug)]
enum ParseError {
    IO(std::io::Error),
    ParseInt(ParseIntError),
    ParseFloat(ParseFloatError),
    //InvalidNode(usize),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IO(err) => write!(f, "{}", err.to_string()),
            Self::ParseInt(err) => write!(f, "{}", err.to_string()),
            Self::ParseFloat(err) => write!(f, "{}", err.to_string()),
            //Self::InvalidNode(node_id) => write!(f, "Invalid node {}", node_id)
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Self::IO(ref err) => Some(err),
            Self::ParseInt(ref err) => Some(err),
            Self::ParseFloat(ref err) => Some(err),
            //_ => None
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

/// An undirected graph edge between two nodes a and b
struct Edge {
    a: usize,
    b: usize,
    dist: usize,
    edge_type: String,
    maxspeed: String,
}

/// A graph node with id, latitude and longitude
struct Node {
    id: usize,
    id2: usize,
    lat: String,
    lon: String,
    elevation: String,
}

/// An undirected graph with nodes and edges
struct Graph {
    meta: String,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    new_edges: Vec<(usize, usize, usize, String, String)>,
    num_nodes: usize,
    num_edges: usize,
    new_num_edges: usize,
}

impl Graph {
    /// Parse node and edge data from a file into an undirected graph
    fn parse_graph (&mut self, graph_file_path: &str) -> Result<(), ParseError> {
        let graph_file = File::open(graph_file_path)?;
        let graph_reader = BufReader::new(graph_file);

        let mut lines = graph_reader.lines();
        let mut line_no = 0;

        loop {
            let line = lines.next()
                .expect(&format!("Unexpected EOF while parsing header in line {}", line_no))?;
            line_no += 1;

            self.meta.push_str(&line);
            self.meta.push_str("\n");

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

            let node = Node {
                id: i,
                id2: split.next()
                    .expect(&format!("Unexpected EOL while parsing node latitude in line {}",
                                     line_no))
                    .parse()?,
                lat: split.next()
                    .expect(&format!("Unexpected EOL while parsing node latitude in line {}",
                                     line_no))
                    .to_string(),
                lon: split.next()
                    .expect(&format!("Unexpected EOL while parsing node longitude in line {}",
                                     line_no))
                    .to_string(),
                elevation: split.next()
                    .expect(&format!("Unexpected EOL while parsing node latitude in line {}",
                                     line_no))
                    .to_string(),
            };
            self.nodes.push(node);
        }

        self.edges.reserve_exact(self.num_edges);
        let mut new_temp_edges: BTreeMap<(usize, usize), Vec<(usize, usize, usize, String, String)>> = BTreeMap::new();
        for _ in 0..self.num_edges {
            let line = lines.next()
                .expect(&format!("Unexpected EOF while parsing edges in line {}", line_no))?;
            let mut split = line.split(" ");
            line_no += 1;

            let edge = Edge {
                a: split.next()
                    .expect(&format!("Unexpected EOL while parsing edge source in line {}",
                                     line_no))
                    .parse()?,
                b: split.next()
                    .expect(&format!("Unexpected EOL while parsing edge target in line {}",
                                     line_no))
                    .parse()?,
                dist: split.next()
                    .expect(&format!("Unexpected EOL while parsing edge weight in line {}",
                                     line_no))
                    .parse()?,
                edge_type: split.next()
                    .expect(&format!("Unexpected EOL while parsing edge weight in line {}",
                                     line_no))
                    .to_string(),
                maxspeed: split.next()
                    .expect(&format!("Unexpected EOL while parsing edge weight in line {}",
                                     line_no))
                    .to_string(),
            };

            let min_vertex = edge.a.min(edge.b);
            let max_vertex = edge.a.max(edge.b);
            new_temp_edges.entry((min_vertex, max_vertex))
                .and_modify(|edges|{
                    if edge.dist < edges[0].2 {
                        edges[0].2 = edge.dist;
                        edges[1].2 = edge.dist;
                    }
                })
                .or_insert(vec![(edge.a, edge.b, edge.dist, edge.edge_type.clone(), edge.maxspeed.clone()), (edge.b, edge.a, edge.dist, edge.edge_type, edge.maxspeed)]);
        }

        self.new_edges.reserve_exact(new_temp_edges.len() * 2);
        for edge_values in new_temp_edges.values() {
            for (a, b, dist, edge_type, maxspeed) in edge_values {
                self.new_edges.push((*a, *b, *dist, edge_type.clone(), maxspeed.clone()));
            }
        }
        self.new_edges.sort_unstable_by(|e1, e2| {
            let id1 = e1.0;
            let id2 = e2.0;
            id1.cmp(&id2).then_with(||{
                let id1 = e1.1;
                let id2 = e2.1;
                id1.cmp(&id2)
            })
        });
        self.new_num_edges = self.new_edges.len();

        Ok(())
    }

    fn write_graph(&mut self, graph_file_path_out: &str) -> std::io::Result<()> {
        let file = File::create(graph_file_path_out)?;
        let mut file = LineWriter::new(file);

        file.write((format!("{}", self.meta)).as_bytes())?;
        file.write((format!("{}\n", self.num_nodes)).as_bytes())?;
        file.write((format!("{}\n", self.new_num_edges)).as_bytes())?;

        for node in &self.nodes {
            file.write((format!("{} {} {} {} {}\n", node.id, node.id2, node.lat, node.lon, node.elevation)).as_bytes())?;
        }

        for (a, b, dist, edge_type, maxspeed) in &self.new_edges {
            file.write((format!("{} {} {} {} {}\n", a, b, dist, edge_type, maxspeed)).as_bytes())?;
        }

        Ok(())
    }
}

/// Given arguments look like: "path/graphname.fmi path/new_graphname.fmi". Parses "graphname.fmi", creates an undirected graph and writes it in "new_graphname.fmi".
fn main() -> Result<(), ParseError> {
    let args: Vec<_> = env::args().collect();

    if args.len() < 3 {
        let err = "Missing argument: path to new undirected graph file";
        panic!("{}", err);
    }

    let in_graph = args[1].to_string();
    let out_graph = args[2].to_string();

    let mut graph = Graph {
        meta: "".to_string(),
        nodes: vec![],
        edges: vec![],
        new_edges: Default::default(),
        num_nodes: 0,
        num_edges: 0,
        new_num_edges: 0
    };
    graph.parse_graph(&in_graph)?;
    graph.write_graph(&out_graph)?;

    Ok(())
}
