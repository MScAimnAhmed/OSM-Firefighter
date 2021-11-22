mod error;
mod graph;
mod session;
mod firefighter;
mod query;

use std::{collections::HashMap,
          env,
          fs,
          sync::{Arc, Mutex, RwLock}};

use actix_cors::Cors;
use actix_web::{App, dev::HttpResponseBuilder, get, HttpMessage, HttpRequest, HttpResponse, HttpServer, middleware::Logger, post, Responder, web, http};
use log;
use serde_json::json;

use crate::error::OSMFError;
use crate::firefighter::{problem::{OSMFProblem, OSMFSettings},
                         strategy::{GreedyStrategy, OSMFStrategy, MinDistGroupStrategy, Strategy},
                         TimeUnit};
use crate::graph::Graph;
use crate::query::Query;
use crate::session::OSMFSessionStorage;

/// Storage for data associated to the web app
struct AppData {
    sessions: Mutex<OSMFSessionStorage>,
    graphs_path: String,
    graphs: HashMap<String, Arc<RwLock<Graph>>>,
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
#[get("/graphs")]
async fn list_graphs(data: web::Data<AppData>, req: HttpRequest) -> Result<HttpResponse, OSMFError> {
    match fs::read_dir(&data.graphs_path) {
        Ok(paths) => {
            let (mut res, _) = init_response(&data, &req, HttpResponse::Ok());
            let mut graph_files = Vec::new();
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
                let mut ext_len;
                if filename.ends_with(".fmi") || filename.ends_with(".hub") {
                    ext_len = 4;
                    if filename.ends_with(".ch.hub") {
                        ext_len += 3;
                    }
                } else {
                    continue;
                }
                let graph_file = filename[0..filename.len()-ext_len].to_string();
                if !graph_files.contains(&graph_file) {
                    graph_files.push(graph_file);
                }
            }
            Ok(res.json(json!(graph_files)))
        }
        Err(err) => {
            log::warn!("Failed to list graph files: {}", err.to_string());
            Err(OSMFError::Internal {
                message: "Could not find graph file directory".to_string()
            })
        }
    }
}

/// List all available firefighter containment strategies
#[get("/strategies")]
async fn list_strategies(data: web::Data<AppData>, req: HttpRequest) -> impl Responder {
    let (mut res, _) = init_response(&data, &req, HttpResponse::Ok());
    res.json(json!(OSMFStrategy::available_strategies()))
}

/// Simulate a new firefighter problem instance
#[post("/simulate")]
async fn simulate_problem(data: web::Data<AppData>, settings: web::Json<OSMFSettings>, req: HttpRequest) -> Result<HttpResponse, OSMFError> {
    let (mut res, sid) = init_response(&data, &req, HttpResponse::Created());

    let graph = match data.graphs.get(&settings.graph_name) {
        Some(graph) => graph,
        None => {
            log::warn!("Unknown graph {}", settings.graph_name);
            return Err(OSMFError::BadRequest {
                message: format!("Unknown value for parameter 'graph': '{}'", settings.graph_name)
            });
        }
    };

    let strategy = match settings.strategy_name.as_str() {
        "greedy" => OSMFStrategy::Greedy(GreedyStrategy::new(graph.clone())),
        "min_distance_group" => OSMFStrategy::MinDistanceGroup(MinDistGroupStrategy::new(graph.clone())),
        _ => {
            log::warn!("Unknown strategy {}", settings.strategy_name);
            return Err(OSMFError::BadRequest {
                message: format!("Unknown value for parameter 'strategy': '{}'", settings.strategy_name)
            });
        }
    };

    let mut problem = OSMFProblem::new(
        graph.clone(),
        settings.into_inner(),
        strategy);
    problem.simulate();

    let res = res.json(problem.simulation_response());

    {
        let mut sessions = data.sessions.lock().unwrap();
        let session = sessions.get_mut_session(&sid).unwrap();
        session.attach_problem(problem);
    }

    Ok(res)
}

/// Display the view of a firefighter simulation
#[get("/view")]
async fn display_view(data: web::Data<AppData>, req: HttpRequest) -> Result<HttpResponse, OSMFError> {
    let (mut res, sid) = init_response(&data, &req, HttpResponse::Ok());

    let mut sessions = data.sessions.lock().unwrap();
    let session = sessions.get_mut_session(&sid).unwrap();
    let problem = match session.get_mut_problem() {
        Some(problem) => problem,
        None => {
            return Err(OSMFError::NoSimulation {
                message: "No simulation has been started yet".to_string()
            });
        }
    };

    let query = Query::from(req.query_string());
    let center_lat = query.try_get_and_parse::<f64>("clat");
    let center_lon = query.try_get_and_parse::<f64>("clon");
    let zoom = query.get_and_parse::<f64>("zoom")?;
    let time = query.get_and_parse::<TimeUnit>("time")?;

    if center_lat.is_some() && center_lon.is_some() {
        let center = (center_lat.unwrap()?, center_lon.unwrap()?);

        log::debug!("Computing view for center: {:?}, zoom: {} and time: {}", center, zoom, time);

        Ok(res.content_type("image/png")
            .body(problem.view_response(center, zoom, &time)))
    } else {
        log::debug!("Computing view for zoom: {} and time: {}", zoom, &time);

        Ok(res.content_type("image/png")
            .body(problem.view_response_alt(zoom, &time)))
    }
}

/// Get the metadata for a specific step of a firefighter simulation
#[get("/stepmeta")]
async fn get_sim_step_metadata(data: web::Data<AppData>, req: HttpRequest) -> Result<HttpResponse, OSMFError> {
    let (mut res, sid) = init_response(&data, &req, HttpResponse::Ok());

    let mut sessions = data.sessions.lock().unwrap();
    let session = sessions.get_session(&sid).unwrap();
    let problem = match session.get_problem() {
        Some(problem) => problem,
        None => {
            return Err(OSMFError::NoSimulation {
                message: "No simulation has been started yet".to_string()
            });
        }
    };

    let query = Query::from(req.query_string());
    let time = query.get_and_parse::<TimeUnit>("time")?;

    Ok(res.json(problem.sim_step_metadata_response(&time)))
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

    // Initialize graphs
    let graphs_path = args[1].to_string();
    let paths: Vec<_> = match fs::read_dir(&graphs_path) {
        Ok(paths) => paths.map(|path| path.unwrap()).collect(),
        Err(err) => panic!("{}", err.to_string())
    };
    let mut graphs = HashMap::with_capacity(graphs_path.len());
    for path in paths {
        let file_path = path.path().to_str().unwrap().split(".").collect::<Vec<_>>()[0].to_string();
        let file_name = path.file_name().to_str().unwrap().split(".").collect::<Vec<_>>()[0].to_string();
        graphs.entry(file_name.clone()).or_insert_with(|| {
            let graph = Arc::new(RwLock::new(Graph::from_files(&file_path)));

            log::info!("Loaded graph {}", file_name);

            graph
        });
    }

    // Initialize app data
    let data = web::Data::new(AppData {
        sessions: Mutex::new(OSMFSessionStorage::new()),
        graphs_path,
        graphs,
    });

    // Initialize and start server
    let server = HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allowed_methods(vec!["GET", "POST"])
            .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
            .allowed_header(http::header::CONTENT_TYPE)
            .supports_credentials()
            .max_age(3600);
        App::new()
            .app_data(data.clone())
            .wrap(cors)
            .wrap(Logger::default())
            .service(ping)
            .service(list_graphs)
            .service(list_strategies)
            .service(simulate_problem)
            .service(display_view)
            .service(get_sim_step_metadata)
    });
    server.bind("localhost:8080")?
        .run()
        .await
}
