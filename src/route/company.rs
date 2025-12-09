use axum::{Json, extract::{State, Path, Query}, http::StatusCode};

use axum_extra::{TypedHeader, headers::{Authorization, authorization::Bearer}};
use chrono::{Utc, Duration};
use bcrypt::{verify, hash};
use jsonwebtoken::{EncodingKey, Header,encode};
use serde_json::{json,Value};
use sqlx::{PgPool, QueryBuilder};
use uuid::Uuid;
use crate::{models::{admin::{Admin, AuthAdmin, LoginAdmin}, company::{Company, CompanyQueryParams, CreateCompanyReq}, employee::{Employee, EmployeePassword, Role}}, utils::{errorhandler::AppError, jwt::{AccessRole, Claims, verify_auth_token}}};

pub async fn create_company(
    State(pg): State<PgPool>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<CreateCompanyReq>
) -> Result<(StatusCode,Json<Value>), AppError> {

        //check if its accessed by admin only
        let claims = verify_auth_token(TypedHeader(auth))
            .await
            .map_err(|_| AppError::unauthorized("do not have access for token"))?;

        if claims.role != AccessRole::Admin {
            return Err(AppError::forbidden("only administrators have access"));
        }

        //add company
        let mut tx = pg.begin().await.map_err(AppError::from)?;

        let company_row = sqlx::query!("insert into companies (company_name, about) values ($1, $2) returning comp_id", payload.company_name, payload.about)
            .fetch_one(&mut *tx)
            .await
            .map_err(AppError::from)?;

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
            .map_err(AppError::from)?;
        
        //create token
        let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "mysecret".into());
            let exp = Utc::now() + Duration::hours(1);
        
            let claims = Claims{
                id: manager_row.emp_id,
                sub: payload.manager.email.clone(),
                role: AccessRole::Manager,
                exp: exp.timestamp() as usize,
            };

            let token = encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(secret.as_bytes()),
            ).map_err(|_| AppError::Unexpected)?;
        tx.commit().await.map_err(AppError::from)?;
    Ok((StatusCode::CREATED, Json(json!({"new_password token": token}))))
}

pub async fn get_company_by_id(
    State(pg): State<PgPool>,
    Path(comp_id): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Value>, AppError> {

    let claims = verify_auth_token(TypedHeader(auth)).await.map_err(|_| AppError::unauthorized("do not have access"))?;
    if claims.role != AccessRole::Admin {
        return Err(AppError::forbidden("only administrators have access"));
    }

    let company = sqlx::query_as!(Company, "select * from companies where comp_id = $1", comp_id)
        .fetch_one(&pg)
        .await
        .map_err(AppError::from)?;




    Ok(Json(json!(company)))
}


pub async fn get_companies(
    State(pg): State<PgPool>,
    Query(params): Query<CompanyQueryParams>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Value>, AppError> {
    
    let claims = verify_auth_token(TypedHeader(auth)).await.map_err(|_| AppError::unauthorized("do not have access"))?;
    if claims.role != AccessRole::Admin {
        return Err(AppError::forbidden("only administrators have access"));
    }

    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(10);
    let offset = (page-1) * limit;

    let mut query_builder = QueryBuilder::new("SELECT * from companies WHERE 1=1");

    //name filter
    if let Some(company_name) = params.company_name{
        query_builder.push(" AND company_name ILIKE ");
        query_builder.push_bind(format!("%{}%", company_name));
    };


    query_builder.push(" ORDER BY id DESC ");
    query_builder.push(" LIMIT ");
    query_builder.push_bind(limit);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let query = query_builder.build_query_as::<Company>();

    let companies = query.fetch_all(&pg)
        .await
    .map_err(|_| AppError::not_found("not found"))?;

    let total_count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM companies")
        .fetch_one(&pg)
        .await
    .map_err(|_| AppError::not_found("not found"))?
    .unwrap_or(0);

    //response
    let response = serde_json::json!({
        "page": page,
        "limit": limit,
        "total": total_count,
        "data": companies
    });
    Ok(Json(response))


}

pub async fn get_my_company(
    State(pg): State<PgPool>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>, 
) -> Result<(StatusCode, Json<Value>), AppError> {


    let claims = verify_auth_token(TypedHeader(auth)).await
        .map_err(|_| AppError::unauthorized("Invalid Token"))?;
        
    if claims.role==AccessRole::Admin {
        return Err(AppError::forbidden("this is for employees only"))
    }

    let company_row = sqlx::query!("select comp_id from employees where emp_id = $1", claims.id)
        .fetch_one(&pg)
        .await
        .map_err(AppError::from)?;



    let company = sqlx::query_as!(
        Company,
        "SELECT * FROM companies WHERE comp_id = $1",
        company_row.comp_id
    )
    .fetch_one(&pg)
    .await
    .map_err(|e| {
        if let sqlx::Error::RowNotFound = e {
            AppError::not_found("Company not found for this employee")
        } else {
            AppError::from(e)
        }
    })?;

    Ok((StatusCode::OK, Json(json!(company))))
}

pub async fn update_companies(

)


