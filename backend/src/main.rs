mod error;
mod graph;
mod session;
mod firefighter;
mod query;

use std::{env,
          fs,
          sync::{Arc, Mutex, RwLock}};

use actix_web::{App,
                dev::HttpResponseBuilder,
                get,
                HttpMessage,
                HttpRequest,
                HttpResponse,
                HttpServer,
                middleware::Logger,
                post,
                Responder,
                web};
use log;
use serde_json::json;

use crate::error::OSMFError;
use crate::firefighter::{problem::{OSMFProblem, OSMFSettings},
                         strategy::{GreedyStrategy, OSMFStrategy, ShoDistStrategy, Strategy}};
use crate::graph::Graph;
use crate::session::OSMFSessionStorage;
use crate::query::Query;

/// Storage for data associated to the web app
struct AppData {
    sessions: Mutex<OSMFSessionStorage>,
    graph: Arc<RwLock<Graph>>,
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
#[post("/simulate")]
async fn simulate_problem(data: web::Data<AppData>, req: HttpRequest) -> Result<HttpResponse, OSMFError> {
    let res_sid = init_response(&data, &req, HttpResponse::Created());
    let mut res = res_sid.0;
    let sid = res_sid.1;

    let graph = data.graph.clone();
    let query = Query::from(req.query_string());

    let strategy_name = query.get("strategy")?;
    let strategy = match strategy_name {
        "greedy" => OSMFStrategy::Greedy(GreedyStrategy::new(graph.clone())),
        "sho_dist" => OSMFStrategy::ShortestDistance(ShoDistStrategy::new(graph.clone())),
        _ => {
            log::warn!("Unknown strategy {}", strategy_name);
            return Err(OSMFError::BadRequest {
                message: format!("Unknown value for parameter 'strategy': '{}'", strategy_name)
            });
        }
    };
    let num_roots = query.get_and_parse::<usize>("num_roots")?;
    let num_ffs = query.get_and_parse::<usize>("num_ffs")?;

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