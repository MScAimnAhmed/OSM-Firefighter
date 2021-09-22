use std::{sync::{Arc, RwLock},
          time::{Instant, Duration}};

use actix_web::{http::Cookie,
                cookie::SameSite};
use nanoid;
use transient_hashmap::TransientHashMap;

use crate::firefighter::OSMFProblem;
use crate::graph::Graph;

/// Container for OSM-Firefighter session data
pub struct OSMFSession {
    pub id: String,
    problem: OSMFProblem,
}

impl OSMFSession {
    /// Create a new `OSMFSession`
    fn new(id: String, graph: Arc<RwLock<Graph>>) -> Self {
        Self {
            id,
            problem: OSMFProblem::new(graph, 1),
        }
    }

    /// Build a session cookie for this `OSMFSession`
    fn build_cookie<'a, 'b: 'a>(&'a self) -> Cookie<'b> {
        Cookie::build("sid", self.id.clone())
            .secure(false)
            .same_site(SameSite::Strict)
            .finish()
    }
}

/// Time, after which to prune unused `OSMFSession` instances
const PRUNE_SESSIONS_AFTER_SECS: Duration = Duration::from_secs(60 * 60);

/// Storage for `OSMFSession` instances
pub struct OSMFSessionStorage {
    sessions: TransientHashMap<String, OSMFSession>,
    last_pruned: Instant,
}

impl OSMFSessionStorage {
    /// Create a new storage for `OSMFSession` instances
    pub fn new() -> Self {
        Self {
            sessions: TransientHashMap::new(PRUNE_SESSIONS_AFTER_SECS.as_secs() as u32),
            last_pruned: Instant::now(),
        }
    }

    /// Prune unused `OSMFSession` instances
    fn prune_sessions(&mut self) {
        if self.last_pruned.elapsed() >= PRUNE_SESSIONS_AFTER_SECS {
            self.sessions.prune();
            self.last_pruned = Instant::now();
        }
    }

    /// Open a new `OSMFSession`
    pub fn open_session(&mut self, graph: Arc<RwLock<Graph>>) -> Cookie {
        self.prune_sessions();
        let session = OSMFSession::new(nanoid::nanoid!(), graph);
        let cookie = session.build_cookie();
        self.sessions.insert(session.id.clone(), session);
        cookie
    }

    /// Refresh the `OSMFSession` with session id `id`
    pub fn refresh_session(&mut self, id: &str, graph: Arc<RwLock<Graph>>) -> Option<Cookie> {
        self.prune_sessions();
        let string_id = &id.to_string();
        if self.sessions.contains_key(string_id) {
            None
        } else {
            Some(self.open_session(graph))
        }
    }

    /// Get a mutable reference to the `OSMFSession` with session id `id`
    pub fn get_mut_session(&mut self, id: &str) -> Option<&mut OSMFSession> {
        self.prune_sessions();
        let string_id = &id.to_string();
        self.sessions.get_mut(string_id)
    }
}