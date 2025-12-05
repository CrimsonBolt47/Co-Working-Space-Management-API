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
    SqlxError(#[from] sqlx::Error),


    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Unexpected server error: {0}")]
    Unexpected(String),
}