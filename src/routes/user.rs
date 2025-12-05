use axum::{Json, extract::State, http::StatusCode};
use axum_extra::{TypedHeader, headers::{Authorization, authorization::Bearer}};
use bcrypt::{hash,verify};
use chrono::{Utc, Duration};
use jsonwebtoken::{EncodingKey, Header,encode};
use serde_json::{json,Value};
use sqlx::PgPool;

use crate::{models::user::{LoginUser, RegisterUser, User}, utils::jwt::{Claims, verify_auth_token}};


pub async fn register_user(
     State(pg): State<PgPool>,
    Json(payload): Json<RegisterUser>,
) -> Result<Json<User>, (StatusCode, String)> {

    let hashed = hash(payload.password, 12)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let user = sqlx::query_as!(
        User,
        r#"
            INSERT INTO users (name, email, password_hash)
            VALUES ($1, $2, $3)
            RETURNING id, name, email, password_hash
        "#,
        payload.name,
        payload.email,
        hashed
    )
    .fetch_one(&pg)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(user))
}

pub async fn login_user(
     State(pg): State<PgPool>,
    Json(payload): Json<LoginUser>,
) -> Result<Json<Value>, (StatusCode, String)> {

    let user_opt = sqlx::query_as!(User, "SELECT * FROM users WHERE email = $1", payload.email)
        .fetch_optional(&pg)
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid credentials".to_string()))?;

    let user = match user_opt {
        Some(u) => u,
        None => return Err((StatusCode::UNAUTHORIZED, "invalid credentials".into()))
    };

    let valid = verify(&payload.password, &user.password_hash)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid password".to_string()))?;

    if !valid {
        return Err((StatusCode::UNAUTHORIZED, "invalid password".to_string()));
    }

    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "mysecret".into());
    let exp = Utc::now() + Duration::hours(1);
 
    let claims = Claims{
        id: user.id,
        sub: user.email.clone(),
        exp: exp.timestamp() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    ).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({"token": token})))
}

pub async fn protected_route(
    State(pg): State<PgPool>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>
) -> Result<Json<User>, StatusCode>{
    let claims = verify_auth_token(TypedHeader(auth)).await?;
    println!("{:?}", claims);

    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE email = $1", claims.sub)
        .fetch_one(&pg)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    Ok(Json(user))

}