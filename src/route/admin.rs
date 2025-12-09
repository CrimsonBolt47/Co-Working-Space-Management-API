
use axum::{Json, extract::{State}};
use chrono::{Utc, Duration};
use bcrypt::{verify};
use jsonwebtoken::{EncodingKey, Header,encode};
use serde_json::{json,Value};
use sqlx::{PgPool};
use crate::{models::admin::{Admin, LoginAdmin}, utils::{errorhandler::AppError, jwt::{AccessRole, Claims}}};



pub async fn login_admin(
    State(pg): State<PgPool>,
    Json(payload): Json<LoginAdmin>
) -> Result<Json<Value>, AppError> {

    if payload.email.trim().is_empty() {
        return Err(AppError::bad_request("email is required"))
    }
    if payload.password.trim().is_empty() {
        return Err(AppError::bad_request("wrong credentials"))
    }

    let admin_opt = sqlx::query_as!(Admin, "SELECT * FROM admins WHERE email = $1", payload.email)
        .fetch_optional(&pg)
        .await
        .map_err(AppError::from)?;

    let admin = match admin_opt {
        Some(a) => a,
        None => return Err(AppError::not_found("admin not found"))
    };

    let valid = verify(&payload.password,&admin.password_hash)
        .map_err(|_| AppError::unauthorized("invalid credentials"))?;

    if !valid {
        return Err(AppError::unauthorized("invalid credentials"));
    }
    
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "mysecret".into());
    let exp = Utc::now() + Duration::hours(1);

    let claims = Claims{
        id: admin.admin_id,
        sub: admin.email,
        role: AccessRole::Admin,
        exp: exp.timestamp() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes())
    ).map_err(|_| AppError::Unexpected)?;

    Ok(Json(json!({"token":token})))
    
}