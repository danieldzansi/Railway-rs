use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use super::models::ErrorResponse;

pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("{err:#}"),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(ErrorResponse {
            error: self.message,
        });
        (self.status, body).into_response()
    }
}
