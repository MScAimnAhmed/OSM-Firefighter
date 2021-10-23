use actix_web::{HttpResponse, http::StatusCode, ResponseError};
use derive_more::{Display, Error};
use serde::Serialize;

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
pub enum OSMFError {
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