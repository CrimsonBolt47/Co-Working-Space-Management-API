use axum::{Json, extract::{State}};
use bcrypt::{verify};
use jsonwebtoken::{EncodingKey, Header,encode};
use serde_json::{json,Value};
use sqlx::{PgPool};
use tracing::warn;
use crate::{models::admin::{Admin, LoginAdmin}, utils::{errorhandler::AppError, jwt::{AccessRole, Claims}}};



pub async fn login_admin(
    State(pg): State<PgPool>,
    Json(payload): Json<LoginAdmin>
) -> Result<Json<Value>, AppError> {

    //checking for empty fields
    if payload.email.trim().is_empty() {
        return Err(AppError::bad_request("invalid credentials"))
    }
    if payload.password.trim().is_empty() {
        return Err(AppError::bad_request("invalid credentials"))
    }

    let admin_opt = sqlx::query_as!(Admin, "SELECT * FROM admins WHERE email = $1", payload.email)
        .fetch_optional(&pg)
        .await
        .map_err(|e| {
            warn!("Database error checking conflicts: {}", e);
            AppError::database("Failed to check availability")})?;

    let admin = match admin_opt {
        Some(a) => a,
        None => {
            warn!("Failed login attempt: admin not found for email: {}", payload.email);
            return Err(AppError::unauthorized("invalid credentials"))
        }
    };

    let valid = verify(&payload.password,&admin.password_hash)
        .map_err(|_| AppError::unauthorized("invalid credentials"))?;

    if !valid {
        warn!("Failed login attempt: invalid password for email: {}", payload.email);
        return Err(AppError::unauthorized("invalid credentials"));
    }
    
    let secret = std::env::var("JWT_SECRET")
        .expect("JWT_SECRET environment variable must be set");
    let token_expiry_hours: u64 = std::env::var("TOKEN_EXPIRY_HOURS")
        .ok()
        .and_then(|h| h.parse().ok())
        .unwrap_or(1);
    
    let exp = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() + (token_expiry_hours * 3600)) as usize;

    let claims = Claims{
        id: admin.admin_id,
        sub: admin.email.clone(),
        role: AccessRole::Admin,
        exp,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes())
    ).map_err(|e| {
        warn!("JWT encoding failed: {}", e);
        AppError::Unexpected
    })?;

    Ok(Json(json!({
        "success": true,
        "data": {
            "token": token
        }
    })))
    
}