use std::env;
use std::fs;

use actix_web::{get, error::ResponseError, http::StatusCode, middleware::Logger, HttpServer, App,
                Responder, HttpResponse};
use derive_more::{Display, Error};
use log;
use serde::Serialize;
use serde_json::json;

mod graph;

//use crate::graph::Graph;

/// Blueprint for error responses
#[derive(Serialize)]
struct ErrorResponse {
    status_code: u16,
    error: String,
    message: String,
}
impl ErrorResponse {
    /// Create a new error response
    fn new(status_code: StatusCode, error: String, message: String) -> Self {
        ErrorResponse {
            status_code: status_code.as_u16(),
            error,
            message,
        }
    }
}

/// OSM-Firefighter custom error
#[derive(Debug, Display, Error)]
enum OSMFirefighterError {
    #[display(fmt = "{}", message)]
    Internal { message: String },
}
impl OSMFirefighterError {
    /// Return the name of this error
    pub fn name(&self) -> String {
        match self {
            Self::Internal { .. } => "Internal Server Error".to_string(),
        }
    }
}
impl ResponseError for OSMFirefighterError {
    fn status_code(&self) -> StatusCode {
        match *self {
            Self::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
    fn error_response(&self) -> HttpResponse {
        let res = ErrorResponse::new(
            self.status_code(),
            self.name(),
            self.to_string()
        );
        HttpResponse::build(self.status_code()).json(res)
    }
}

/// Request to check whether the server is up and available
#[get("/ping")]
async fn ping() -> impl Responder {
    HttpResponse::Ok().body("pong")
}

/// List all graph files that can be parsed by the server
#[get("/")]
async fn list_graphs() -> Result<HttpResponse, OSMFirefighterError> {
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
            Ok(HttpResponse::Ok().json(json!(graphs)))
        },
        Err(err) => {
            log::warn!("Failed to list graph files: {}", err.to_string());
            Err(OSMFirefighterError::Internal {
                message: "Could not find graph file directory".to_string()
            })
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logger
    env::set_var("RUST_LOG", "info");
    env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();

    // Initialize and start server
    let server = HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .service(ping)
            .service(list_graphs)
    });
    server.bind("127.0.0.1:8080")?
        .run()
        .await
}