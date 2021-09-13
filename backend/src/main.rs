use std::{env,
          fs,
          sync::{Mutex, RwLock}};

use actix_web::{get, HttpServer, App, HttpRequest, HttpResponse, Responder, HttpMessage,
                dev::HttpResponseBuilder,
                error::ResponseError,
                http::StatusCode,
                middleware::Logger,
                web};
use derive_more::{Display, Error};
use log;
use serde::Serialize;
use serde_json::json;

mod graph;
mod session;
mod firefighter;

use crate::graph::Graph;
use crate::session::OSMFSessionStorage;

/// Storage for data associated to the web app
struct AppData {
    sessions: Mutex<OSMFSessionStorage>,
    graph: RwLock<Graph>,
}

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
            Self::Internal { .. } => "Internal Server Error".to_string()
        }
    }
}

impl ResponseError for OSMFError {
    fn status_code(&self) -> StatusCode {
        match *self {
            Self::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR
        }
    }
    fn error_response(&self) -> HttpResponse {
        let res = ErrorResponse::new(
            self.status_code(),
            self.name(),
            self.to_string(),
        );
        HttpResponse::build(self.status_code()).json(res)
    }
}

/// Common function to initialize a `HttpResponseBuilder` for an incoming `HttpRequest`.
/// This function must be called before retrieving session data.
fn init_response(data: &web::Data<AppData>, req: &HttpRequest, mut res: HttpResponseBuilder) -> HttpResponseBuilder {
    let mut sessions = data.sessions.lock().unwrap();
    let new_cookie = match req.cookie("sid") {
        Some(cookie) => sessions.refresh_session(cookie.value()),
        None => Some(sessions.open_session())
    };
    match new_cookie {
        Some(cookie) => {
            res.cookie(cookie);
        }
        None => ()
    }
    res
}

/// Request to check whether the server is up and available
#[get("/ping")]
async fn ping(data: web::Data<AppData>, req: HttpRequest) -> impl Responder {
    let mut res = init_response(&data, &req, HttpResponse::Ok());
    res.content_type("text/plain; charset=utf-8")
        .body("pong")
}

/// List all graph files that can be parsed by the server
#[get("/")]
async fn list_graphs(data: web::Data<AppData>, req: HttpRequest) -> Result<HttpResponse, OSMFError> {
    match fs::read_dir("resources/") {
        Ok(paths) => {
            let mut res = init_response(&data, &req, HttpResponse::Ok());
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
        }
        Err(err) => {
            log::warn!("Failed to list graph files: {}", err.to_string());
            Err(OSMFError::Internal {
                message: "Could not find graph file directory".to_string()
            })
        }
    }
}

/// Send the currently loaded graph
#[get("/graph")]
async fn send_graph(data: web::Data<AppData>, req: HttpRequest) -> impl Responder {
    let mut res = init_response(&data, &req, HttpResponse::Ok());
    let graph = data.graph.read().unwrap();
    res.json(&*graph)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logger
    env::set_var("RUST_LOG", "info");
    env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();

    // Read in default graph
    let default_graph_file = env::var("OSMF_DEFAULT_GRAPH")
        .expect(&format!("Could not find environment variable 'OSMF_DEFAULT_GRAPH'"));
    let default_graph = Graph::from_file(&default_graph_file);
    log::info!("Read in default graph file {}", default_graph_file);

    // Initialize app data
    let data = web::Data::new(AppData {
        sessions: Mutex::new(OSMFSessionStorage::new()),
        graph: RwLock::new(default_graph),
    });

    // Initialize and start server
    let server = HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .wrap(Logger::default())
            .service(ping)
            .service(list_graphs)
            .service(send_graph)
    });
    server.bind("0.0.0.0:8080")?
        .run()
        .await
}