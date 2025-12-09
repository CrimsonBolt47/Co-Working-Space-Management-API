use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer}};
use axum::http::StatusCode;
use serde::{Deserialize,Serialize};
use jsonwebtoken::{DecodingKey, Validation, decode};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum AccessRole {
    Admin,
    Manager,
    Employee
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub id: Uuid,
    pub sub: String,
    pub role: AccessRole,
    pub exp: usize,
}

pub async fn verify_auth_token(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>
) -> Result<Claims, StatusCode> {

    let token = auth.token();

    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "mysecret".into());

    let token_data = decode(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    Ok(token_data.claims)
}