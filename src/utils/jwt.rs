use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer}};
use axum::http::StatusCode;
use serde::{Deserialize,Serialize};
use jsonwebtoken::{DecodingKey, Validation, decode};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub id: i32,
    pub sub: String,
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