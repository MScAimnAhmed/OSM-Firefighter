use qstring::QString;
use std::str::FromStr;

use crate::OSMFError;

/// Thin wrapper for query strings with an API to get values
/// from the query easily
pub struct Query {
    qstring: QString,
}

impl From<&str> for Query {
    fn from(query_string: &str) -> Self {
        Self {
            qstring: QString::from(query_string),
        }
    }
}

impl Query {
    /// Get the value for `key` from the query.
    /// Returns an error if `key` is not specified
    pub fn get(&self, key: &str) -> Result<&str, OSMFError> {
        if let Some(val_str) = self.qstring.get(key) {
            Ok(val_str)
        } else {
            log::warn!("{} not specified", key);
            Err(OSMFError::BadRequest {
                message: format!("Missing parameter: '{}'", key)
            })
        }
    }

    /// Get the value for `key` from the query.
    /// Returns an error if `key` is not specified or
    /// if the value cannot be parsed to the specified type `F`.
    pub fn get_and_parse<F: FromStr>(&self, key: &str) -> Result<F, OSMFError> {
        let val_str = self.get(key)?;
        if let Ok(val) = val_str.parse::<F>() {
            Ok(val)
        } else {
            log::warn!("Cannot parse {} from string", key);
            Err(OSMFError::BadRequest {
                message: format!("Invalid value for parameter '{}': '{}'", key, val_str)
            })
        }
    }
}