use axum::{
    Router,
    routing::{get, patch, post},
};
use sqlx::PgPool;

use crate::route::{admin::login_admin, company::create_company, employee::email_verification};

pub fn create_router(pool: PgPool) -> Router {
    Router::new()
    //admin
    .route("/auth/admin/login",post(login_admin))
    .route("/companies", post(create_company))
    .route("/email_verification",patch(email_verification))
    .with_state(pool)
}