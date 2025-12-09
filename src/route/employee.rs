use axum::{Json, extract::State, http::StatusCode};
use axum_extra::{TypedHeader, headers::{Authorization, authorization::Bearer}};
use chrono::{Utc, Duration};
use bcrypt::{verify, hash};
use jsonwebtoken::{EncodingKey, Header,encode};
use serde_json::{json,Value};
use sqlx::{PgPool};
use crate::{models::{admin::{Admin, AuthAdmin, LoginAdmin}, company::{Company, CreateCompanyReq}, employee::{Employee, EmployeePassword, Role}}, utils::{errorhandler::AppError, jwt::{AccessRole, Claims, verify_auth_token}}};


pub async fn email_verification(
    State(pg): State<PgPool>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<EmployeePassword>
) -> Result<(StatusCode,Json<Value>), AppError> {

    if payload.password.trim().is_empty() {
        return Err(AppError::bad_request("password is required"))
    }
    let mut tx = pg.begin().await.map_err(AppError::from)?;
    let claims = verify_auth_token(TypedHeader(auth)).await.map_err(|_| AppError::unauthorized("do not have access"))?;
    let employee_row = sqlx::query!("SELECT emp_id FROM employees WHERE email = $1 and password_hash is NULL", claims.sub)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| AppError::unauthorized("do not have access"))?;

    let hashed = hash(payload.password, 12)
        .map_err(|_| AppError::BadRequest("bad request".into()))?;

    sqlx::query!(
        r#"
            Update employees 
            set password_hash = $1
            where emp_id = $2
            and password_hash is NULL
        "#,
        hashed,
        employee_row.emp_id
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::from)?;

    tx.commit().await.map_err(AppError::from)?;
    Ok((StatusCode::ACCEPTED, Json(json!({"success": true}))))
}

