use axum::{Json, extract::{State, Path, Query}, http::StatusCode};

use axum_extra::{TypedHeader, headers::{Authorization, authorization::Bearer}};
use chrono::{Utc, Duration};
use bcrypt::{verify, hash};
use jsonwebtoken::{EncodingKey, Header,encode};
use serde_json::{json,Value};
use sqlx::{PgPool, QueryBuilder};
use uuid::Uuid;
use crate::{models::{admin::{Admin, AuthAdmin, LoginAdmin}, company::{Company, CompanyQueryParams, CreateCompanyReq, UpdateCompanyReq}, employee::{Employee, EmployeePassword, Role}, space::{self, CreateSpaceReq, Space, SpaceAvailableTimings, SpaceId, SpaceQueryParams, UpdateSpaceReq}}, utils::{errorhandler::AppError, jwt::{AccessRole, Claims, verify_auth_token}}};

pub async fn create_spaces(
    State(pg): State<PgPool>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<CreateSpaceReq>
) -> Result<(StatusCode,Json<Value>), AppError> {

        //check if its accessed by admin only
        let claims = verify_auth_token(TypedHeader(auth))
            .await
            .map_err(|_| AppError::unauthorized("do not have access for token"))?;

        if claims.role != AccessRole::Admin {
            return Err(AppError::forbidden("only administrators have access"));
        }

        //add space
        let mut tx = pg.begin().await.map_err(AppError::from)?;

        let space_row = sqlx::query_as!(SpaceId,"insert into spaces (name, size, description) values ($1, $2, $3) returning space_id", payload.name, payload.size, payload.description)
            .fetch_one(&mut *tx)
            .await
            .map_err(AppError::from)?;
        
        tx.commit().await.map_err(AppError::from)?;
    Ok((StatusCode::CREATED, Json(json!({"success": space_row}))))
}

pub async fn get_spaces_by_id(
    State(pg): State<PgPool>,
    Path(space_id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {

    let space = sqlx::query_as!(Space, "select * from spaces where space_id = $1", space_id)
        .fetch_one(&pg)
        .await
        .map_err(AppError::from)?;

    Ok(Json(json!(space)))
}


pub async fn get_spaces(
    State(pg): State<PgPool>,
    Query(params): Query<SpaceQueryParams>,
) -> Result<Json<Value>, AppError> {
    

    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(10);
    let offset = (page-1) * limit;

    let mut query_builder = QueryBuilder::new("SELECT * from spaces WHERE 1=1");

    //name filter
    if let Some(name) = params.name{
        query_builder.push(" AND name ILIKE ");
        query_builder.push_bind(format!("%{}%", name));
    };

    if let Some(size) = params.size{
        query_builder.push(" AND size = ");
        query_builder.push_bind(format!("%{}%", size));
    };


    query_builder.push(" ORDER BY id DESC ");
    query_builder.push(" LIMIT ");
    query_builder.push_bind(limit);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let query = query_builder.build_query_as::<Space>();

    let spaces = query.fetch_all(&pg)
        .await
    .map_err(|_| AppError::not_found("not found"))?;

    let total_count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM spaces")
        .fetch_one(&pg)
        .await
    .map_err(|_| AppError::not_found("not found"))?
    .unwrap_or(0);

    //response
    let response = serde_json::json!({
        "page": page,
        "limit": limit,
        "total": total_count,
        "data": spaces
    });
    Ok(Json(response))


}

pub async fn update_space(
    State(pg): State<PgPool>,
    Path(space_id): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<UpdateSpaceReq>
) -> Result<Json<Value>, AppError> 
{
    
    let claims = verify_auth_token(TypedHeader(auth)).await.map_err(|_| AppError::unauthorized("do not have access"))?;
    if claims.role != AccessRole::Admin {
        return Err(AppError::forbidden("only administrators have access"));
    }

    let mut query_builder = sqlx::QueryBuilder::new("UPDATE spaces");
    let mut separated = query_builder.separated("SET ");
    let mut has_update = false;

    let mut tx = pg.begin().await.map_err(AppError::from)?;

    if let Some(name) = payload.name {
        separated.push("name = ");
        separated.push_bind(name);
        has_update = true;
    }

    if let Some(size) = payload.size {
        separated.push("size = ");
        separated.push_bind(size);
        has_update = true;
    }

    if !has_update {
        return Err(AppError::bad_request("no parameters provided"));
    }

    separated.push("where space_id = ");
    separated.push_bind(space_id);

    let query = query_builder.build_query_as::<Space>();

    let spaces = query.fetch_all(&mut *tx)
        .await
    .map_err(|_| AppError::not_found("not found"))?;

    tx.commit().await?;

    Ok(Json(json!(spaces)))
}

pub async fn delete_space(
    State(pg): State<PgPool>,
    Path(space_id): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<StatusCode,AppError> {

    let claims = verify_auth_token(TypedHeader(auth)).await.map_err(|_| AppError::unauthorized("do not have access"))?;
    if claims.role != AccessRole::Admin {
        return Err(AppError::forbidden("only administrators have access"));
    }

    sqlx::query!("delete from spaces where space_id = $1",space_id)
    .execute(&pg)
    .await
    .map_err(AppError::from)?;

    Ok(StatusCode::OK)
}

pub async fn get_available_spaces_at_given_time(
    State(pg): State<PgPool>,
    Json(payload): Json<SpaceAvailableTimings>
) -> Result<StatusCode,AppError> {

    sqlx::query_as!("select space_id, size,description,")
    Ok(())
}



