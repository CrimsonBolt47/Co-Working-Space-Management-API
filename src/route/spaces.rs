use axum::{Json, extract::{State, Path, Query}, http::StatusCode};

use axum_extra::{TypedHeader, headers::{Authorization, authorization::Bearer}};
use serde_json::{json, Value};
use sqlx::PgPool;
use time::OffsetDateTime;
use tracing::warn;
use uuid::Uuid;
use crate::{models::{booking::AvailableBooking, space::{CreateSpaceReq, Space, SpaceAvailable, SpaceAvailableTimings, SpaceId, SpaceQueryParams, UpdateSpaceReq}}, utils::{errorhandler::AppError, jwt::{AccessRole,verify_auth_token}}};

pub async fn create_spaces(
    State(pg): State<PgPool>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<CreateSpaceReq>
) -> Result<(StatusCode, Json<Value>), AppError> {

        //check if its accessed by admin only
        let claims = verify_auth_token(TypedHeader(auth))
            .await
            .map_err(|_| {
                warn!("Unauthorized space creation attempt - invalid token");
                AppError::unauthorized("invalid credentials")
            })?;

        if claims.role != AccessRole::Admin {
            warn!("Non-admin user attempted to create space");
            return Err(AppError::forbidden("only administrators have access"));
        }

        if payload.size <= 0 {
            return Err(AppError::bad_request("space size must be greater than 0"));
        }

        //add space
        let mut tx = pg.begin().await.map_err(|e| {
            warn!("Database error starting transaction: {}", e);
            AppError::database("Failed to create space")
        })?;

        let space_row = sqlx::query_as!(SpaceId, "insert into spaces (name, size, description) values ($1, $2, $3) returning space_id", payload.name, payload.size, payload.description)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| {
                warn!("Database error inserting space: {}", e);
                AppError::database("Failed to create space")
            })?;
        
        tx.commit().await.map_err(|e| {
            warn!("Database error committing transaction: {}", e);
            AppError::database("Failed to create space")
        })?;
    Ok((StatusCode::CREATED, Json(json!({
        "success": true,
        "data": {
            "space_id": space_row.space_id
        }
    }))))
}

pub async fn get_spaces_by_id(
    State(pg): State<PgPool>,
    Path(space_id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {

    let space = sqlx::query_as!(Space, "select * from spaces where space_id = $1", space_id)
        .fetch_one(&pg)
        .await
        .map_err(|e| {
            match e {
                sqlx::Error::RowNotFound => {
                    warn!("Space not found: {}", space_id);
                    AppError::not_found("space not found")
                },
                _ => {
                    warn!("Database error fetching space: {}", e);
                    AppError::database("Failed to fetch space")
                }
            }
        })?;

    Ok(Json(json!({
        "success": true,
        "data": space
    })))
}


pub async fn get_spaces(
    State(pg): State<PgPool>,
    Query(params): Query<SpaceQueryParams>,
) -> Result<Json<Value>, AppError> {
    
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(10);
    let offset = (page-1) * limit;

    let mut query_builder = sqlx::QueryBuilder::new("SELECT * from spaces WHERE 1=1");

    //name filter
    if let Some(name) = params.name{
        query_builder.push(" AND name ILIKE ");
        query_builder.push_bind(format!("%{}%", name));
    };

    if let Some(size) = params.size{
        query_builder.push(" AND size = ");
        query_builder.push_bind(size);
    };

    query_builder.push(" ORDER BY space_id DESC ");
    query_builder.push(" LIMIT ");
    query_builder.push_bind(limit);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let query = query_builder.build_query_as::<Space>();

    let spaces = query.fetch_all(&pg)
        .await
        .map_err(|e| {
            warn!("Database error fetching spaces: {}", e);
            AppError::database("Failed to fetch spaces")
        })?;

    let total_count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM spaces")
        .fetch_one(&pg)
        .await
        .map_err(|e| {
            warn!("Database error counting spaces: {}", e);
            AppError::database("Failed to count spaces")
        })?
        .unwrap_or(0);

    let response = serde_json::json!({
        "success": true,
        "data": {
            "page": page,
            "limit": limit,
            "total": total_count,
            "items": spaces
        }
    });
    Ok(Json(response))
}

pub async fn update_space(
    State(pg): State<PgPool>,
    Path(space_id): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<UpdateSpaceReq>
) -> Result<Json<Value>, AppError> {
    
    let claims = verify_auth_token(TypedHeader(auth)).await.map_err(|_| {
        warn!("Unauthorized update space attempt - invalid token");
        AppError::unauthorized("invalid credentials")
    })?;
    if claims.role != AccessRole::Admin {
        warn!("Non-admin user attempted to update space: {}", space_id);
        return Err(AppError::forbidden("only administrators have access"));
    }

    if let Some(size) = payload.size {
        if size <= 0 {
            return Err(AppError::bad_request("space size must be greater than 0"));
        }
    }

    let mut query_builder = sqlx::QueryBuilder::new("UPDATE spaces");
    let mut separated = query_builder.separated("SET ");
    let mut has_update = false;

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

    let mut tx = pg.begin().await.map_err(|e| {
        warn!("Database error starting transaction: {}", e);
        AppError::database("Failed to update space")
    })?;

    let space = query.fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            match e {
                sqlx::Error::RowNotFound => {
                    warn!("Space not found for update: {}", space_id);
                    AppError::not_found("space not found")
                },
                _ => {
                    warn!("Database error updating space: {}", e);
                    AppError::database("Failed to update space")
                }
            }
        })?;

    tx.commit().await.map_err(|e| {
        warn!("Database error committing transaction: {}", e);
        AppError::database("Failed to update space")
    })?;

    Ok(Json(json!({
        "success": true,
        "data": space
    })))
}

pub async fn delete_space(
    State(pg): State<PgPool>,
    Path(space_id): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<StatusCode, AppError> {

    let claims = verify_auth_token(TypedHeader(auth)).await.map_err(|_| {
        warn!("Unauthorized delete space attempt - invalid token");
        AppError::unauthorized("invalid credentials")
    })?;
    if claims.role != AccessRole::Admin {
        warn!("Non-admin user attempted to delete space: {}", space_id);
        return Err(AppError::forbidden("only administrators have access"));
    }

    let result = sqlx::query!("delete from spaces where space_id = $1", space_id)
        .execute(&pg)
        .await
        .map_err(|e| {
            warn!("Database error deleting space: {}", e);
            AppError::database("Failed to delete space")
        })?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("space not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_available_spaces_at_given_time(
    State(pg): State<PgPool>,
    Json(payload): Json<SpaceAvailableTimings>
) -> Result<Json<Value>, AppError> {

    let spaces_row = sqlx::query_as!(SpaceAvailable, r#"SELECT 
            s.space_id,
            s.name,
            s.size,
            s.description
            FROM 
            spaces s
            WHERE 
                NOT EXISTS (
                    SELECT 1
                    FROM bookings b
                    WHERE 
                        b.space_id = s.space_id
                        AND b.start_time < $2
                        AND b.end_time > $1
                )
            ORDER BY 
                s.name;
            "#,
            payload.start_time,
            payload.end_time)
                .fetch_all(&pg)
                .await
                .map_err(|e| {
                    warn!("Database error fetching available spaces: {}", e);
                    AppError::database("Failed to fetch available spaces")
                })?;

    Ok(Json(json!({
        "success": true,
        "data": spaces_row
    })))
}

pub async fn get_booked_time_spaces_by_id(
    State(pg): State<PgPool>,
    Path(space_id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {

    let today_date = OffsetDateTime::now_utc().date();
    let booking_row = sqlx::query_as!(AvailableBooking, r#"select 
                start_time,
                end_time
                from 
                bookings
                WHERE 
                space_id = $1
                AND start_time::date = $2
                ORDER BY 
                start_time;
            "#,
            space_id,
            today_date)
                .fetch_all(&pg)
                .await
                .map_err(|e| {
                    warn!("Database error fetching booked times: {}", e);
                    AppError::database("Failed to fetch booked times")
                })?;

    Ok(Json(json!({
        "success": true,
        "data": booking_row
    })))
}


