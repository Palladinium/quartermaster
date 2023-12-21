use std::fmt::Display;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use tracing::{error, info};

pub struct ErrorResponse {
    pub status: StatusCode,
    pub errors: Vec<ResponseError>,
}

impl ErrorResponse {
    pub fn not_found<E: Display>(error: E) -> Self {
        info!("Responding with 404 Not Found: {error}");
        Self {
            status: StatusCode::NOT_FOUND,
            errors: Vec::new(),
        }
    }

    pub fn internal_server_error<E: Display>(error: E) -> Self {
        error!("Responding with 500 Internal Server Error: {error}");
        Self {
            status: StatusCode::NOT_FOUND,
            errors: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponseBody {
    errors: Vec<ResponseError>,
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        let body = ErrorResponseBody {
            errors: self.errors,
        };

        IntoResponse::into_response((self.status, Json(body)))
    }
}

#[derive(Debug, Serialize)]
pub struct ResponseError {
    pub detail: String,
}
