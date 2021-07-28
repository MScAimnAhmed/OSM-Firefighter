use actix_web::{get, HttpServer, App, Responder, HttpResponse};
use crate::graph::Graph;

mod graph;

/// Request to check whether the server is up and available
#[get("/ping")]
async fn ping() -> impl Responder {
    HttpResponse::Ok().body("pong")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let _graph = Graph::from_file("resources/toy.fmi");
    println!("Parsed graph toy.fmi");
    //println!("{:#?}", _graph);

    let server = HttpServer::new(|| {
        App::new()
            .service(ping)
    });
    println!("Starting web server on 127.0.0.1:8080");
    server.bind("127.0.0.1:8080")?
        .run()
        .await
}
