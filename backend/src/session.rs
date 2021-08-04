use std::collections::HashMap;
use std::time::{Instant, Duration};

use actix_web::http::Cookie;
use actix_web::cookie::SameSite;
use nanoid;

const SESSION_MAX_AGE: Duration = Duration::from_secs(60*60);

/// Container for OSM-Firefighter session data
pub struct OSMFSession {
    pub id: String,
    expires: Instant,
}
impl OSMFSession {
    /// Create a new OSM-Firefighter session
    fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            expires: Instant::now() + SESSION_MAX_AGE
        }
    }

    /// Check whether this OSM-Firefighter session is still valid, i.e. not
    /// expired yet
    fn is_valid(&self) -> bool {
        self.expires > Instant::now()
    }

    /// Build a session cookie from this OSM-Firefighter session
    pub fn build_cookie(&self) -> Cookie {
        Cookie::build("sid", &self.id)
            .secure(false)
            .same_site(SameSite::Strict)
            .finish()
    }
}

/// Tells whether a session has been opened, retrieved or whether no session
/// is available
pub enum OSMFSessionStatus<'a> {
    Opened(&'a OSMFSession),
    Got(&'a OSMFSession),
}

/// Storage for OSM-Firefighter sessions
pub struct OSMFSessionStorage {
    sessions: HashMap<String, OSMFSession>,
}
impl OSMFSessionStorage {
    /// Create a new storage for OSM-Firefighter sessions
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    /// Open a new session data container
    pub fn open_session(&mut self) -> OSMFSessionStatus {
        let id = nanoid::nanoid!();
        let session = OSMFSession::new(&id);
        self.sessions.insert(id.clone(), session);
        OSMFSessionStatus::Opened(self.sessions.get(&id).unwrap())
    }

    /// Get the session data container that matches `id` or open a new session
    /// if the old session is invalid or `id` matches none
    pub fn get_or_open_session(&mut self, id: &str) -> OSMFSessionStatus {
        match self.sessions.get(id) {
            Some(session) => {
                if session.is_valid() {
                    OSMFSessionStatus::Got(self.sessions.get(id).unwrap())
                } else {
                    self.sessions.remove(id);
                    self.open_session()
                }
            },
            None => self.open_session()
        }
    }
}