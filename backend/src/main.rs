use std::fs;

use actix_web::{get, HttpServer, App, Responder, HttpResponse};
use serde_json::json;

mod graph;

//use crate::graph::Graph;

/// Request to check whether the server is up and available
#[get("/ping")]
async fn ping() -> impl Responder {
    HttpResponse::Ok().body("pong")
}

/// List all graph files that can be parsed by the server
#[get("/")]
async fn list_graphs() -> impl Responder {
    match fs::read_dir("resources/") {
        Ok(paths) => {
            let mut graphs = Vec::new();
            for path in paths {
                let path = path.unwrap();
                let filetype = path.file_type().unwrap();
                if filetype.is_dir() {
                    continue;
                }
                let filename = String::from(
                    path.file_name()
                        .to_str()
                        .unwrap()
                );
                if !filename.ends_with(".fmi") {
                    continue;
                }
                graphs.push(filename);
            }
            HttpResponse::Ok().json(json!(graphs))
        },
        Err(err) => HttpResponse::InternalServerError().body(err.to_string())
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    //let graph = Graph::from_file("resources/toy.fmi");
    //println!("{:#?}", graph);

    let server = HttpServer::new(|| {
        App::new()
            .service(ping)
            .service(list_graphs)
    });
    println!("Starting web server on 127.0.0.1:8080");
    server.bind("127.0.0.1:8080")?
        .run()
        .await
}