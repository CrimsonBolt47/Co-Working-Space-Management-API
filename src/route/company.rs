use axum::{Json, extract::{State, Path, Query}, http::StatusCode};

use axum_extra::{TypedHeader, headers::{Authorization, authorization::Bearer}};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde_json::{json, Value};
use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;
use crate::{models::{company::{Company, CompanyQueryParams, CreateCompanyReq, UpdateCompanyReq}, employee::Role}, utils::{errorhandler::AppError, jwt::{AccessRole, Claims, verify_auth_token}}};

pub async fn create_company(
    State(pg): State<PgPool>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<CreateCompanyReq>
) -> Result<(StatusCode,Json<Value>), AppError> {

        //check if its accessed by admin only
        let claims = verify_auth_token(TypedHeader(auth))
            .await
            .map_err(|_| {
                warn!("Unauthorized company creation attempt - invalid token");
                AppError::unauthorized("invalid credentials")
            })?;

        if claims.role != AccessRole::Admin {
            warn!("Non-admin user attempted to create company");
            return Err(AppError::forbidden("only administrators have access"));
        }

        // Validate email 
        if !payload.manager.email.contains('@') || payload.manager.email.trim().is_empty() {
            return Err(AppError::bad_request("invalid email format"));
        }

        //add company
        let mut tx = pg.begin().await.map_err(|e| {
            warn!("Database error starting transaction: {}", e);
            AppError::database("Failed to create company")
        })?;

        let company_row = sqlx::query!("insert into companies (company_name, about) values ($1, $2) returning comp_id", payload.company_name, payload.about)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| {
                warn!("Database error inserting company: {}", e);
                AppError::database("Failed to create company")
            })?;

        //add manager
        let manager_row = sqlx::query!("insert into employees (name, email,comp_id, position,role) values ($1,$2,$3,$4,$5::employee_role) returning emp_id",
            payload.manager.name,
            payload.manager.email,
            company_row.comp_id,
            payload.manager.position,
            Role::MNG as Role,
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| {
                warn!("Database error inserting manager: {}", e);
                AppError::database("Failed to create company")
            })?;
        
        //create token
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
                id: manager_row.emp_id,
                sub: payload.manager.email.clone(),
                role: AccessRole::Manager,
                exp,
            };

            let token = encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(secret.as_bytes()),
            ).map_err(|e| {
                warn!("JWT encoding failed: {}", e);
                AppError::Unexpected
            })?;
        tx.commit().await.map_err(|e| {
            warn!("Database error committing transaction: {}", e);
            AppError::database("Failed to create company")
        })?;
    Ok((StatusCode::CREATED, Json(json!({
        "success": true,
        "data": {
            "token": token
        }
    }))))
}

pub async fn get_company_by_id(
    State(pg): State<PgPool>,
    Path(comp_id): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Value>, AppError> {

    let claims = verify_auth_token(TypedHeader(auth)).await.map_err(|_| {
        warn!("Unauthorized get company attempt - invalid token");
        AppError::unauthorized("invalid credentials")
    })?;
    if claims.role != AccessRole::Admin {
        warn!("Non-admin user attempted to get company by id");
        return Err(AppError::forbidden("only administrators have access"));
    }

    let company = sqlx::query_as!(Company, "select * from companies where comp_id = $1", comp_id)
        .fetch_one(&pg)
        .await
        .map_err(|e| {
            match e {
                sqlx::Error::RowNotFound => {
                    warn!("Company not found: {}", comp_id);
                    AppError::not_found("company not found")
                },
                _ => {
                    warn!("Database error fetching company: {}", e);
                    AppError::database("Failed to fetch company")
                }
            }
        })?;

    Ok(Json(json!({
        "success": true,
        "data": company
    })))
}


pub async fn get_companies(
    State(pg): State<PgPool>,
    Query(params): Query<CompanyQueryParams>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Value>, AppError> {
    
    let claims = verify_auth_token(TypedHeader(auth)).await.map_err(|_| {
        warn!("Unauthorized get companies attempt - invalid token");
        AppError::unauthorized("invalid credentials")
    })?;
    if claims.role != AccessRole::Admin {
        warn!("Non-admin user attempted to list companies");
        return Err(AppError::forbidden("only administrators have access"));
    }

    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(10);
    let offset = (page-1) * limit;

    let mut query_builder = sqlx::QueryBuilder::new("SELECT * from companies WHERE 1=1");

    //name filter
    if let Some(company_name) = params.company_name{
        query_builder.push(" AND company_name ILIKE ");
        query_builder.push_bind(format!("%{}%", company_name));
    };

    query_builder.push(" ORDER BY comp_id DESC ");
    query_builder.push(" LIMIT ");
    query_builder.push_bind(limit);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let query = query_builder.build_query_as::<Company>();

    let companies = query.fetch_all(&pg)
        .await
        .map_err(|e| {
            warn!("Database error fetching companies: {}", e);
            AppError::database("Failed to fetch companies")
        })?;

    let total_count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM companies")
        .fetch_one(&pg)
        .await
        .map_err(|e| {
            warn!("Database error counting companies: {}", e);
            AppError::database("Failed to count companies")
        })?
        .unwrap_or(0);

    let response = serde_json::json!({
        "success": true,
        "data": {
            "page": page,
            "limit": limit,
            "total": total_count,
            "items": companies
        }
    });
    Ok(Json(response))
}

pub async fn get_my_company(
    State(pg): State<PgPool>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>, 
) -> Result<Json<Value>, AppError> {

    let claims = verify_auth_token(TypedHeader(auth)).await
        .map_err(|_| {
            warn!("Unauthorized get my company attempt - invalid token");
            AppError::unauthorized("invalid credentials")
        })?;
        
    if claims.role == AccessRole::Admin {
        warn!("Admin attempted to get my company");
        return Err(AppError::forbidden("only employees and managers have access"))
    }

    let company_row = sqlx::query!("select comp_id from employees where emp_id = $1", claims.id)
        .fetch_one(&pg)
        .await
        .map_err(|e| {
            match e {
                sqlx::Error::RowNotFound => {
                    warn!("Employee not found: {}", claims.id);
                    AppError::not_found("employee not found")
                },
                _ => {
                    warn!("Database error fetching employee: {}", e);
                    AppError::database("Failed to fetch employee")
                }
            }
        })?;

    let company = sqlx::query_as!(
        Company,
        "SELECT * FROM companies WHERE comp_id = $1",
        company_row.comp_id
    )
    .fetch_one(&pg)
    .await
    .map_err(|e| {
        match e {
            sqlx::Error::RowNotFound => {
                warn!("Company not found for employee: {}", claims.id);
                AppError::not_found("company not found for this employee")
            },
            _ => {
                warn!("Database error fetching company: {}", e);
                AppError::database("Failed to fetch company")
            }
        }
    })?;

    Ok(Json(json!({
        "success": true,
        "data": company
    })))
}

pub async fn update_companies(
    State(pg): State<PgPool>,
    Path(comp_id): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<UpdateCompanyReq>
) -> Result<Json<Value>, AppError> {
    
    let claims = verify_auth_token(TypedHeader(auth)).await.map_err(|_| {
        warn!("Unauthorized update company attempt - invalid token");
        AppError::unauthorized("invalid credentials")
    })?;
    if claims.role != AccessRole::Admin {
        warn!("Non-admin user attempted to update company: {}", comp_id);
        return Err(AppError::forbidden("only administrators have access"));
    }

    let mut query_builder = sqlx::QueryBuilder::new("UPDATE companies");
    let mut separated = query_builder.separated("SET ");
    let mut has_update = false;

    if let Some(name) = payload.company_name {
        separated.push("company_name = ");
        separated.push_bind(name);
        has_update = true;
    }

    if let Some(about) = payload.about {
        separated.push("about = ");
        separated.push_bind(about);
        has_update = true;
    }

    if !has_update {
        return Err(AppError::bad_request("no parameters provided"));
    }

    separated.push("where comp_id = ");
    separated.push_bind(comp_id);

    let query = query_builder.build_query_as::<Company>();

    let mut tx = pg.begin().await.map_err(|e| {
        warn!("Database error starting transaction: {}", e);
        AppError::database("Failed to update company")
    })?;

    let companies = query.fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            match e {
                sqlx::Error::RowNotFound => {
                    warn!("Company not found for update: {}", comp_id);
                    AppError::not_found("company not found")
                },
                _ => {
                    warn!("Database error updating company: {}", e);
                    AppError::database("Failed to update company")
                }
            }
        })?;

    tx.commit().await.map_err(|e| {
        warn!("Database error committing transaction: {}", e);
        AppError::database("Failed to update company")
    })?;

    Ok(Json(json!({
        "success": true,
        "data": companies
    })))
}

pub async fn delete_company(
    State(pg): State<PgPool>,
    Path(comp_id): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<StatusCode, AppError> {

    let claims = verify_auth_token(TypedHeader(auth)).await.map_err(|_| {
        warn!("Unauthorized delete company attempt - invalid token");
        AppError::unauthorized("invalid credentials")
    })?;
    if claims.role != AccessRole::Admin {
        warn!("Non-admin user attempted to delete company: {}", comp_id);
        return Err(AppError::forbidden("only administrators have access"));
    }

    let result = sqlx::query!("delete from companies where comp_id = $1", comp_id)
        .execute(&pg)
        .await
        .map_err(|e| {
            warn!("Database error deleting company: {}", e);
            AppError::database("Failed to delete company")
        })?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("company not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}



