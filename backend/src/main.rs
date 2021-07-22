use crate::graph::Graph;

mod graph;

fn main() {
    let graph = Graph::from_file("resources/toy.fmi");
    println!("{:#?}", graph);
}
