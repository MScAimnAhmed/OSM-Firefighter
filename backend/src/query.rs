use qstring::QString;
use std::str::FromStr;

use crate::error::OSMFError;

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
    /// Try to get the value for `key` from the query.
    /// Returns `None` if `key` is not specified.
    pub fn try_get(&self, key: &str) -> Option<&str> {
        self.qstring.get(key)
    }

    /// Get the value for `key` from the query.
    /// Returns an error if `key` is not specified.
    pub fn get(&self, key: &str) -> Result<&str, OSMFError> {
        if let Some(val_str) = self.try_get(key) {
            Ok(val_str)
        } else {
            log::warn!("{} not specified", key);
            Err(OSMFError::BadRequest {
                message: format!("Missing parameter: '{}'", key)
            })
        }
    }

    /// Try to get the value for `key` from the query and parse it to type `F`.
    /// Returns `None` if `key` is not specified or
    /// `Some(Err(_))` if the value cannot be parsed to the specified type `F`.
    pub fn try_get_and_parse<F: FromStr>(&self, key: &str) -> Option<Result<F, OSMFError>> {
        let maybe_val = self.try_get(key);
        if let Some(val_str) = maybe_val {
            if let Ok(val) = val_str.parse::<F>() {
                Some(Ok(val))
            } else {
                log::warn!("Cannot parse {} from string", key);
                Some(Err(OSMFError::BadRequest {
                    message: format!("Invalid value for parameter '{}': '{}'", key, val_str)
                }))
            }
        } else {
            None
        }
    }

    /// Get the value for `key` from the query and parse it to type `F`.
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