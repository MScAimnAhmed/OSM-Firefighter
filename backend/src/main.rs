mod web_utils;

use std::{collections::HashMap, env, fs, sync::{Arc, Mutex}};

use actix_cors::Cors;
use actix_web::{App, get, http, HttpRequest, HttpResponse, HttpResponseBuilder, HttpServer, middleware::Logger, post, Responder, web};
use log;
use serde::{Serialize, Deserialize};
use serde_json::json;

use osmff_lib::firefighter::problem::{OSMFProblem, OSMFSettings};
use osmff_lib::firefighter::strategy::OSMFStrategy;
use osmff_lib::firefighter::TimeUnit;
use osmff_lib::graph::Graph;

use crate::web_utils::error::OSMFError;
use crate::web_utils::query::Query;
use crate::web_utils::session::OSMFSessionStorage;

/// Path to configuration file
const CONFIG_PATH: &str = "./config.json";

/// Server and backend service configuration
#[derive(Deserialize)]
struct Config {
    host: String,
    port: u16,
    log_level: String,
    graphs_path: String,
}

impl Config {
    /// Parses the configuration file at `file_path` into a new `Config` instance
    fn from_file(file_path: &str) -> Self {
        let data = fs::read_to_string(file_path)
            .expect("Could not find config file");
        serde_json::from_str(&data).expect("Config file does not contain valid JSON")
    }
}

/// Storage for data associated to the web app
struct AppData {
    sessions: Mutex<OSMFSessionStorage>,
    graphs: HashMap<String, Arc<Graph>>,
}

#[derive(Serialize)]
struct GraphData {
    name: String,
    num_of_nodes: usize
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
async fn list_graphs(data: web::Data<AppData>, req: HttpRequest) -> impl Responder {
    let (mut res, _) = init_response(&data, &req, HttpResponse::Ok());
    res.json(json!(
        data.graphs.iter()
        .map(|(graph_name, graph)| GraphData{name: graph_name.clone(), num_of_nodes: graph.num_nodes})
        .collect::<Vec<_>>()
    ))
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

    let strategy = match OSMFStrategy::from_name_and_graph(&settings.strategy_name, graph.clone()) {
        Some(s) => s,
        None => {
            log::warn!("Unknown strategy {}", settings.strategy_name);
            return Err(OSMFError::BadRequest {
                message: format!("Unknown value for parameter 'strategy': '{}'", settings.strategy_name)
            });
        }
    };

    let mut problem = match OSMFProblem::new(graph.clone(), settings.into_inner(), strategy) {
        Ok(problem) => problem,
        Err(err) => {
            return Err(err.into());
        }
    };
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
    // Parse config file
    let config = Config::from_file(CONFIG_PATH);

    // Initialize logger
    env::set_var("RUST_LOG", &config.log_level);
    env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();

    // Initialize graphs
    let graphs = match osmff_lib::load_graphs(&config.graphs_path) {
        Ok(graphs) => graphs,
        Err(err) => {
            panic!("Failed to load graphs: {}", err.to_string());
        }
    };

    // Initialize app data
    let data = web::Data::new(AppData {
        sessions: Mutex::new(OSMFSessionStorage::new()),
        graphs,
    });

    // Initialize and start server
    let server = HttpServer::new(move || {
        // Initialize cors settings
        let cors = Cors::default()
            .allow_any_origin()
            .allowed_methods(vec!["GET", "POST"])
            .allowed_headers(vec![http::header::ACCEPT, http::header::CONTENT_TYPE,
                                  http::header::AUTHORIZATION,])
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
    server.bind((config.host.as_str(), config.port))?
        .run()
        .await
}
