use actix_web::http::Cookie;
use actix_web::cookie::SameSite;
use nanoid;
use transient_hashmap::TransientHashMap;

/// Container for OSM-Firefighter session data
pub struct OSMFSession {
    pub id: String,
}
impl OSMFSession {
    /// Create a new OSM-Firefighter session
    fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
        }
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

const SESSION_MAX_AGE_SECS: u32 = 60 * 60;

/// Storage for OSM-Firefighter sessions
pub struct OSMFSessionStorage {
    sessions: TransientHashMap<String, OSMFSession>,
}
impl OSMFSessionStorage {
    /// Create a new storage for OSM-Firefighter sessions
    pub fn new() -> Self {
        Self {
            sessions: TransientHashMap::new(SESSION_MAX_AGE_SECS),
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
        let string_id = &id.to_string();
        if self.sessions.contains_key(string_id) {
            OSMFSessionStatus::Got(self.sessions.get(string_id).unwrap())
        } else {
            self.open_session()
        }
    }
}