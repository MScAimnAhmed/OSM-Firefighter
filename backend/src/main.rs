use std::env;
use std::fs;
use std::sync::Mutex;

use actix_web::{get, HttpServer, App, HttpResponse, HttpRequest, HttpMessage, Responder};
use actix_web::dev::HttpResponseBuilder;
use actix_web::error::ResponseError;
use actix_web::http::StatusCode;
use actix_web::middleware::Logger;
use derive_more::{Display, Error};
use log;
use once_cell::sync::Lazy;
use serde::Serialize;
use serde_json::json;

mod graph;
mod session;

//use crate::graph::Graph;
use crate::session::{OSMFSessionStatus, OSMFSessionStorage};

static SESSIONS: Lazy<Mutex<OSMFSessionStorage>> =
    Lazy::new(|| Mutex::new(OSMFSessionStorage::new()));

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
        Self {
            status_code: status_code.as_u16(),
            error,
            message,
        }
    }
}

/// OSM-Firefighter custom error
#[derive(Debug, Display, Error)]
enum OSMFError {
    #[display(fmt = "{}", message)]
    Internal { message: String },
}
impl OSMFError {
    /// Return the name of this error
    pub fn name(&self) -> String {
        match self {
            Self::Internal { .. } => "Internal Server Error".to_string(),
        }
    }
}
impl ResponseError for OSMFError {
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

/// Common function to initialize a `HttpResponseBuilder` for an incoming `HttpRequest`.
/// This function must be called before retrieving session data.
fn init_response(req: HttpRequest, mut res: HttpResponseBuilder) -> HttpResponseBuilder {
    let mut sessions = SESSIONS.lock().unwrap();
    let session_status = match req.cookie("sid") {
        Some(cookie) => {
            let sid = cookie.value();
            sessions.get_or_open_session(sid)
        },
        None => sessions.open_session()
    };
    match session_status {
        OSMFSessionStatus::Opened(session) => {
            res.cookie(session.build_cookie());
        },
        OSMFSessionStatus::Got(..) => (),
    }
    res
}

/// Request to check whether the server is up and available
#[get("/ping")]
async fn ping(req: HttpRequest) -> impl Responder {
    let mut res = init_response(req, HttpResponse::Ok());
    res.content_type("text/plain; charset=utf-8")
        .body("pong")
}

/// List all graph files that can be parsed by the server
#[get("/")]
async fn list_graphs(req: HttpRequest) -> Result<HttpResponse, OSMFError> {
    let mut res = init_response(req, HttpResponse::Ok());
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
            Ok(res.json(json!(graphs)))
        },
        Err(err) => {
            log::warn!("Failed to list graph files: {}", err.to_string());
            Err(OSMFError::Internal {
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