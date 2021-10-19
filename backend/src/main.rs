use std::{env,
          fs,
          sync::{Arc, Mutex, RwLock}};

use actix_web::{App, dev::HttpResponseBuilder, error::ResponseError, get, http::StatusCode, HttpMessage, HttpRequest, HttpResponse,
                HttpServer,
                middleware::Logger,
                post,
                Responder,
                web};
use derive_more::{Display, Error};
use log;
use qstring::QString;
use serde::Serialize;
use serde_json::json;

use crate::firefighter::{problem::{OSMFProblem, OSMFSettings},
                         strategy::{GreedyStrategy, OSMFStrategy, ShoDistStrategy, Strategy}};
use crate::graph::Graph;
use crate::session::OSMFSessionStorage;

mod graph;
mod session;
mod firefighter;

/// Storage for data associated to the web app
struct AppData {
    sessions: Mutex<OSMFSessionStorage>,
    graph: Arc<RwLock<Graph>>,
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
    #[display(fmt = "{}", message)]
    BadRequest { message: String },
}

impl OSMFError {
    /// Return the name of this error
    pub fn name(&self) -> String {
        match self {
            Self::Internal { .. } => "Internal Server Error".to_string(),
            Self::BadRequest { .. } => "Bad Request".to_string()
        }
    }
}

impl ResponseError for OSMFError {
    fn status_code(&self) -> StatusCode {
        match *self {
            Self::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::BadRequest { .. } => StatusCode::BAD_REQUEST
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
fn init_response(data: &web::Data<AppData>, req: &HttpRequest, mut res: HttpResponseBuilder) -> (HttpResponseBuilder, String) {
    let mut sessions = data.sessions.lock().unwrap();
    let sid = match req.cookie("sid") {
        Some(cur_cookie) => {
            if let Some(new_cookie) = sessions.refresh_session(cur_cookie.value()) {
                let sid = new_cookie.value().to_string();
                res.cookie(new_cookie);
                sid
            } else {
                cur_cookie.value().to_string()
            }
        }
        None => {
            let new_cookie = sessions.open_session();
            let sid = new_cookie.value().to_string();
            res.cookie(new_cookie);
            sid
        }
    };
    (res, sid)
}

/// Request to check whether the server is up and available
#[get("/ping")]
async fn ping(data: web::Data<AppData>, req: HttpRequest) -> impl Responder {
    let mut res = init_response(&data, &req, HttpResponse::Ok()).0;
    res.content_type("text/plain; charset=utf-8")
        .body("pong")
}

/// List all graph files that can be parsed by the server
#[get("/")]
async fn list_graphs(data: web::Data<AppData>, req: HttpRequest) -> Result<HttpResponse, OSMFError> {
    match fs::read_dir("resources/") {
        Ok(paths) => {
            let mut res = init_response(&data, &req, HttpResponse::Ok()).0;
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
    let mut res = init_response(&data, &req, HttpResponse::Ok()).0;
    let graph = data.graph.read().unwrap();
    res.json(&*graph)
}

/// Simulate a new firefighter problem instance
///
/// TODO send settings in query and parse them in this function
#[post("/simulate")]
async fn simulate_problem(data: web::Data<AppData>, req: HttpRequest) -> Result<HttpResponse, OSMFError> {
    let res_sid = init_response(&data, &req, HttpResponse::Created());
    let mut res = res_sid.0;
    let sid = res_sid.1;

    let query = QString::from(req.query_string());
    let graph = data.graph.clone();

    let strategy;
    if let Some(strategy_name) = query.get("strategy") {
        if strategy_name == "greedy" {
            strategy = OSMFStrategy::Greedy(GreedyStrategy::new(graph.clone()));
        } else if strategy_name == "sho_dist" {
            strategy = OSMFStrategy::ShortestDistance(ShoDistStrategy::new(graph.clone()));
        } else {
            log::warn!("Unknown strategy");
            return Err(OSMFError::BadRequest {
                message: format!("Unknown value for query parameter 'strategy': '{}'", strategy_name)
            });
        }
    } else {
        log::warn!("Strategy not specified");
        return Err(OSMFError::BadRequest {
            message: "Missing query parameter: 'strategy'".to_string()
        });
    }

    let num_roots;
    if let Some(num_str) = query.get("num_roots") {
        if let Ok(num) = num_str.parse::<usize>() {
            num_roots = num;
        } else {
            log::warn!("Number of fire roots no integer");
            return Err(OSMFError::BadRequest {
                message: format!("Invalid value for query parameter 'num_roots': {}", num_str)
            });
        }
    } else {
        log::warn!("Number of fire roots not specified");
        return Err(OSMFError::BadRequest {
            message: "Missing parameter: 'num_roots'".to_string()
        });
    }

    let num_ffs;
    if let Some(num_str) = query.get("num_ffs") {
        if let Ok(num) = num_str.parse::<usize>() {
            num_ffs = num;
        } else {
            log::warn!("Number of firefighters no integer");
            return Err(OSMFError::BadRequest {
                message: format!("Invalid value for query parameter 'num_ffs': {}", num_str)
            });
        }
    } else {
        log::warn!("Number of firefighters not specified");
        return Err(OSMFError::BadRequest {
            message: "Missing parameter: 'num_ffs'".to_string()
        });
    }

    let problem = Arc::new(RwLock::new(
        OSMFProblem::new(graph, OSMFSettings::new(num_roots, num_ffs), strategy)));

    {
        let mut sessions = data.sessions.lock().unwrap();
        let session = sessions.get_mut_session(&sid).unwrap();
        session.attach_problem(problem.clone());
    }

    let mut problem_ = problem.write().unwrap();
    problem_.simulate();
    Ok(res.json(problem_.node_data.direct()))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logger
    env::set_var("RUST_LOG", "debug");
    env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();

    let args: Vec<_> = env::args().collect();

    if args.len() < 2 {
        let err = "Missing argument: path to graph file";
        log::error!("{}", err);
        panic!("{}", err);
    }

    // Read in default graph
    let default_graph_file = &args[1];
    let default_graph = Graph::from_files(&default_graph_file);
    log::info!("Read in default graph file {}", default_graph_file);

    // Initialize app data
    let data = web::Data::new(AppData {
        sessions: Mutex::new(OSMFSessionStorage::new()),
        graph: Arc::new(RwLock::new(default_graph)),
    });

    // Initialize and start server
    let server = HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .wrap(Logger::default())
            .service(ping)
            .service(list_graphs)
            .service(send_graph)
            .service(simulate_problem)
    });
    server.bind("0.0.0.0:8080")?
        .run()
        .await
}