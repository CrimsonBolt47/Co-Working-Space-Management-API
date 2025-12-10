use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response}
};

use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {

    #[error("Database query failed: {0}")]
    DatabaseError(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not Found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Unexpected server error")]
    Unexpected,
}

impl AppError {

    pub fn database<T: Into<String>>(msg: T) -> Self {
        AppError::DatabaseError(msg.into())
    }

    pub fn bad_request<T: Into<String>>(msg: T) -> Self {
        AppError::BadRequest(msg.into())
    }

    pub fn unauthorized<T: Into<String>>(msg: T) -> Self {
        AppError::Unauthorized(msg.into())
    }

    pub fn forbidden<T: Into<String>>(msg: T) -> Self {
        AppError::Forbidden(msg.into())
    }

    pub fn not_found<T: Into<String>>(msg: T) -> Self {
        AppError::NotFound(msg.into())
    }

    pub fn validation<T: Into<String>>(msg: T) -> Self {
        AppError::ValidationError(msg.into())
    }

}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::DatabaseError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),

            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),

            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),

            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),

            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),

            AppError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg.clone()),

            AppError::Unexpected => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),


        };

        let body = Json(json!({
            "success": false,
            "error": {
                "message": message,
                "kind": format!("{:?}",self)
            }
        }));

        (status, body).into_response()
    }
}