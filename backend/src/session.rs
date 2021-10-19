use std::{sync::{Arc, RwLock},
          time::{Instant, Duration}};

use actix_web::{http::Cookie,
                cookie::SameSite};
use nanoid;
use transient_hashmap::TransientHashMap;

use crate::firefighter::problem::OSMFProblem;

/// Container for OSM-Firefighter session data
pub struct OSMFSession {
    pub id: String,
    problem: Option<Arc<RwLock<OSMFProblem>>>,
}

impl OSMFSession {
    /// Create a new `OSMFSession`
    fn new(id: String) -> Self {
        Self {
            id,
            problem: None,
        }
    }

    /// Build a session cookie for this `OSMFSession`
    fn build_cookie<'a, 'b: 'a>(&'a self) -> Cookie<'b> {
        Cookie::build("sid", self.id.clone())
            .secure(false)
            .same_site(SameSite::Strict)
            .finish()
    }

    /// Attach a firefighter problem instance to this `OSMFSession`
    pub fn attach_problem(&mut self, problem: Arc<RwLock<OSMFProblem>>) {
        self.problem = Some(problem);
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
    pub fn open_session(&mut self) -> Cookie {
        self.prune_sessions();
        let session = OSMFSession::new(nanoid::nanoid!());
        let cookie = session.build_cookie();
        self.sessions.insert(session.id.clone(), session);
        cookie
    }

    /// Refresh the `OSMFSession` with session id `id`
    pub fn refresh_session(&mut self, id: &str) -> Option<Cookie> {
        self.prune_sessions();
        let string_id = &id.to_string();
        if self.sessions.contains_key(string_id) {
            None
        } else {
            Some(self.open_session())
        }
    }

    /// Get a mutable reference to the `OSMFSession` with session id `id`
    pub fn get_mut_session(&mut self, id: &str) -> Option<&mut OSMFSession> {
        self.prune_sessions();
        let string_id = &id.to_string();
        self.sessions.get_mut(string_id)
    }
}