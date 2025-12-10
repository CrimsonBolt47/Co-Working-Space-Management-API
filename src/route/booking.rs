use std::time::Duration;

use axum::{Json, extract::{State, Path, Query}, http::StatusCode};

use axum_extra::{TypedHeader, headers::{Authorization, authorization::Bearer}};
use bcrypt::{verify, hash};
use jsonwebtoken::{EncodingKey, Header,encode};
use serde_json::{json,Value};
use sqlx::{PgPool, QueryBuilder};
use time::OffsetDateTime;
use uuid::Uuid;
use crate::{models::{admin::{Admin, AuthAdmin, LoginAdmin}, booking::{BookingId, CreateBookingReq, GetBooking, GetCompanyBooking, UpdateBookingReq}, company::{Company, CompanyQueryParams, CreateCompanyReq, UpdateCompanyReq}, employee::{Employee, EmployeePassword, Role}}, utils::{errorhandler::AppError, jwt::{AccessRole, Claims, verify_auth_token}}};


pub async fn create_booking(
    State(pg): State<PgPool>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<CreateBookingReq>
) -> Result<(StatusCode,Json<Value>), AppError> {

        //check if its accessed by admin only
        let claims = verify_auth_token(TypedHeader(auth))
            .await
            .map_err(|_| AppError::unauthorized("do not have access for token"))?;

        if claims.role == AccessRole::Admin {
            return Err(AppError::forbidden("only employees have access"));
        }

        if OffsetDateTime::now_utc().date() != payload.start_time.date(){
            return Err(AppError::validation(" you can only book for todays date"));
        }

        if payload.start_time > payload.end_time || payload.start_time + Duration::from_hours(2) > payload.end_time {
            return Err(AppError::validation("Invalid timings"));
        }


        //check for overlaping
        let mut tx = pg.begin().await.map_err(AppError::from)?;
        let conflict_exists = sqlx::query!(
                r#"
                SELECT EXISTS(
                    SELECT 1 FROM bookings
                    WHERE 
                        space_id = $1 AND 
                        (start_time, end_time) OVERLAPS ($2, $3)
                ) AS conflict
                "#,
                payload.space_id, // $1
                payload.start_time, // $2
                payload.end_time, // $3
            )
            .fetch_one(&pg)
            .await?
            .conflict.unwrap_or(false);
        if conflict_exists {
            return Err(AppError::bad_request("slot is already filled"));
        }

        let booking_row = sqlx::query_as!(BookingId,"insert into bookings (space_id, booked_by, start_time, end_time) values ($1, $2, $3, $4) returning booking_id",
            payload.space_id,
            claims.id,
            payload.start_time,
            payload.end_time)
            .fetch_one(&mut *tx)
            .await
            .map_err(AppError::from)?;
        
        tx.commit().await.map_err(AppError::from)?;
    Ok((StatusCode::CREATED, Json(json!({
        "booking_id": booking_row.booking_id
    }))))
}

pub async fn cancel_booking(
    State(pg): State<PgPool>,
    Path(booking_id): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<StatusCode,AppError> {

    let claims = verify_auth_token(TypedHeader(auth)).await.map_err(|_| AppError::unauthorized("do not have access"))?;
    if claims.role == AccessRole::Admin {
        return Err(AppError::forbidden("only employees have access"));
    }

    sqlx::query!("delete from bookings where booking_id = $1 and booked_by = $2",booking_id, claims.id)
    .execute(&pg)
    .await
    .map_err(AppError::from)?;

    Ok(StatusCode::OK)
}

pub async fn extend_booking(
    State(pg): State<PgPool>,
    Path(booking_id): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<UpdateBookingReq>
) -> Result<Json<Value>, AppError> {
    let claims = verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|_| AppError::unauthorized("do not have access"))?;
    
    if claims.role == AccessRole::Admin {
        return Err(AppError::forbidden("only employees have access"));
    }
    
    let booking_row = sqlx::query_as!(CreateBookingReq,
        "select space_id, start_time, end_time from bookings 
         where booking_id = $1 and booked_by = $2",
        booking_id,
        claims.id
    )
    .fetch_one(&pg)
    .await
    .map_err(AppError::from)?;
    
    let new_end_time = booking_row.end_time
        .checked_add(payload.extra_time)
        .ok_or_else(|| AppError::validation("Invalid time calculation"))?;
    
    let max_end_time = booking_row.start_time
        .checked_add(time::Duration::hours(2))
        .ok_or_else(|| AppError::validation("Invalid time calculation"))?;
    
    if new_end_time > max_end_time {
        return Err(AppError::validation("you can only book for max 2 hours"));
    }
    
    let conflict_exists = sqlx::query!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM bookings 
            WHERE space_id = $1 
            AND booking_id != $4 
            AND (start_time, end_time) OVERLAPS ($2, $3)
        ) AS conflict
        "#,
        booking_row.space_id,
        booking_row.start_time,
        new_end_time,
        booking_id
    )
    .fetch_one(&pg)
    .await?
    .conflict
    .unwrap_or(false);
    
    if conflict_exists {
        return Err(AppError::bad_request(
            "The requested extension conflicts with an existing reservation."
        ));
    }
    
    let mut tx = pg.begin().await.map_err(AppError::from)?;
    
    sqlx::query!(
        "UPDATE bookings SET end_time = $1 WHERE booking_id = $2",
        new_end_time,
        booking_id
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::from)?;
    
    tx.commit().await?;
    
    Ok(Json(json!({"booking_id": booking_id})))
}

pub async fn get_own_bookings(
    State(pg): State<PgPool>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Value>, AppError> {

    let claims = verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|_| AppError::unauthorized("do not have access"))?;
    
    if claims.role == AccessRole::Admin {
        return Err(AppError::forbidden("only employees have access"));
    }

    let booking_row = sqlx::query_as!(GetBooking, "select space_id, booked_by, start_time, end_time from bookings where booked_by = $1",
        claims.id)
        .fetch_all(&pg)
        .await
        .map_err(AppError::from)?;

    Ok(Json(json!(booking_row)))
}

pub async fn get_booking_by_id (
     State(pg): State<PgPool>,
    Path(booking_id): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Value>, AppError> {

    let claims = verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|_| AppError::unauthorized("do not have access"))?;
    
    if claims.role == AccessRole::Admin {
        return Err(AppError::forbidden("only employees have access"));
    }

    let booking_row = sqlx::query_as!(GetBooking, "select space_id, booked_by, start_time, end_time from bookings where booked_by = $1 and booking_id = $2",
        claims.id,
        booking_id)
        .fetch_one(&pg)
        .await
        .map_err(AppError::from)?;

    Ok(Json(json!(booking_row)))
}

pub async fn get_company_bookings(
    State(pg): State<PgPool>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Value>, AppError> {

    let claims = verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|_| AppError::unauthorized("do not have access"))?;
    
    if claims.role != AccessRole::Manager {
        return Err(AppError::forbidden("only manager have access"));
    }

    let booking_row = sqlx::query_as!(GetCompanyBooking, "SELECT
            b.booking_id,
            b.space_id,
            b.start_time,
            b.end_time,
            e.name AS employee_name,  -- Rename to avoid ambiguity
            e.email,
            b.booked_by AS emp_id
        FROM bookings AS b
        INNER JOIN employees AS e ON b.booked_by = e.emp_id
        WHERE e.comp_id = $1;",
        claims.id)
        .fetch_all(&pg)
        .await
        .map_err(AppError::from)?;

    Ok(Json(json!(booking_row)))
}

