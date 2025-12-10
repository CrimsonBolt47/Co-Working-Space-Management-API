use axum::{Json, extract::{State, Path}, http::StatusCode};

use axum_extra::{TypedHeader, headers::{Authorization, authorization::Bearer}};
use serde_json::{json,Value};
use sqlx::{PgPool};
use time::OffsetDateTime;
use tracing::warn;
use uuid::Uuid;
use crate::{models::{booking::{BookingId, CreateBookingReq, GetBooking, GetCompanyBooking, UpdateBookingReq}}, utils::{errorhandler::AppError, jwt::{AccessRole, verify_auth_token}}};


pub async fn create_booking(
    State(pg): State<PgPool>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<CreateBookingReq>
) -> Result<(StatusCode,Json<Value>), AppError> {

        //check if its accessed by admin only
        let claims = verify_auth_token(TypedHeader(auth))
            .await
            .map_err(|_| {
                warn!("Unauthorized booking attempt - invalid token");
                AppError::unauthorized("invalid credentials")
            })?;

        if claims.role == AccessRole::Admin {
            warn!("Admin attempted to create booking");
            return Err(AppError::forbidden("only employees have access"));
        }

        if OffsetDateTime::now_utc().date() != payload.start_time.date(){
            return Err(AppError::validation("you can only book for todays date"));
        }

        if payload.start_time > payload.end_time || payload.start_time + time::Duration::hours(2) > payload.end_time {
            return Err(AppError::validation("invalid timings"));
        }

        if payload.start_time <= OffsetDateTime::now_utc() {
            return Err(AppError::validation("booking time must be in the future"));
        }

        
        let conflict_exists = sqlx::query!(
                r#"
                SELECT EXISTS(
                    SELECT 1 FROM bookings
                    WHERE 
                        space_id = $1 AND 
                        (start_time, end_time) OVERLAPS ($2, $3)
                ) AS conflict
                "#,
                payload.space_id, 
                payload.start_time, 
                payload.end_time, 
            )
            .fetch_one(&pg)
            .await
            .map_err(|e| {
                warn!("Database error checking booking conflicts: {}", e);
                AppError::database("Failed to check availability")
            })?
            .conflict.unwrap_or(false);
        if conflict_exists {
            return Err(AppError::bad_request("slot is already filled"));
        }

        let mut tx = pg.begin().await.map_err(|e| {
            warn!("Database error starting transaction: {}", e);
            AppError::database("Failed to create booking")
        })?;

        let booking_row = sqlx::query_as!(BookingId,"insert into bookings (space_id, booked_by, start_time, end_time) values ($1, $2, $3, $4) returning booking_id",
            payload.space_id,
            claims.id,
            payload.start_time,
            payload.end_time)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| {
                warn!("Database error inserting booking: {}", e);
                AppError::database("Failed to create booking")
            })?;
        
        tx.commit().await.map_err(|e| {
            warn!("Database error committing transaction: {}", e);
            AppError::database("Failed to create booking")
        })?;
    Ok((StatusCode::CREATED, Json(json!({
        "success": true,
        "data": {
            "booking_id": booking_row.booking_id
        }
    }))))
}

pub async fn cancel_booking(
    State(pg): State<PgPool>,
    Path(booking_id): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Value>, AppError> {

    let claims = verify_auth_token(TypedHeader(auth)).await.map_err(|_| {
        warn!("Unauthorized booking cancellation attempt - invalid token");
        AppError::unauthorized("invalid credentials")
    })?;
    
    if claims.role == AccessRole::Admin {
        warn!("Admin attempted to cancel booking");
        return Err(AppError::forbidden("only employees have access"));
    }

    let result = sqlx::query!("delete from bookings where booking_id = $1 and booked_by = $2", booking_id, claims.id)
        .execute(&pg)
        .await
        .map_err(|e| {
            warn!("Database error deleting booking: {}", e);
            AppError::database("Failed to cancel booking")
        })?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("booking not found"));
    }

    Ok(Json(json!({
        "success": true,
        "data": {
            "message": "Booking cancelled successfully"
        }
    })))
}

pub async fn extend_booking(
    State(pg): State<PgPool>,
    Path(booking_id): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<UpdateBookingReq>
) -> Result<Json<Value>, AppError> {
    let claims = verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|_| {
            warn!("Unauthorized booking extension attempt - invalid token");
            AppError::unauthorized("invalid credentials")
        })?;
    
    if claims.role == AccessRole::Admin {
        warn!("Admin attempted to extend booking");
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
    .map_err(|e| {
        match e {
            sqlx::Error::RowNotFound => {
                warn!("Booking not found for extension: {} by user: {}", booking_id, claims.id);
                AppError::not_found("booking not found")
            },
            _ => {
                warn!("Database error fetching booking: {}", e);
                AppError::database("Failed to fetch booking")
            }
        }
    })?;
    
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
    .await
    .map_err(|e| {
        warn!("Database error checking conflicts: {}", e);
        AppError::database("Failed to check availability")
    })?
    .conflict
    .unwrap_or(false);
    
    if conflict_exists {
        return Err(AppError::bad_request(
            "The requested extension conflicts with an existing reservation."
        ));
    }
    
    let mut tx = pg.begin().await.map_err(|e| {
        warn!("Database error starting transaction: {}", e);
        AppError::database("Failed to extend booking")
    })?;
    
    sqlx::query!(
        "UPDATE bookings SET end_time = $1 WHERE booking_id = $2",
        new_end_time,
        booking_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        warn!("Database error updating booking: {}", e);
        AppError::database("Failed to extend booking")
    })?;
    
    tx.commit().await.map_err(|e| {
        warn!("Database error committing transaction: {}", e);
        AppError::database("Failed to extend booking")
    })?;
    
    Ok(Json(json!({
        "success": true,
        "data": {
            "booking_id": booking_id
        }
    })))
}

pub async fn get_own_bookings(
    State(pg): State<PgPool>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Value>, AppError> {

    let claims = verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|_| {
            warn!("Unauthorized get bookings attempt - invalid token");
            AppError::unauthorized("invalid credentials")
        })?;
    
    if claims.role == AccessRole::Admin {
        warn!("Admin attempted to get own bookings");
        return Err(AppError::forbidden("only employees have access"));
    }

    let booking_row = sqlx::query_as!(GetBooking, "select space_id, booked_by, start_time, end_time from bookings where booked_by = $1",
        claims.id)
        .fetch_all(&pg)
        .await
        .map_err(|e| {
            warn!("Database error fetching bookings: {}", e);
            AppError::database("Failed to fetch bookings")
        })?;

    Ok(Json(json!({
        "success": true,
        "data": booking_row
    })))
}

pub async fn get_booking_by_id(
    State(pg): State<PgPool>,
    Path(booking_id): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Value>, AppError> {

    let claims = verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|_| {
            warn!("Unauthorized get booking attempt - invalid token");
            AppError::unauthorized("invalid credentials")
        })?;
    
    if claims.role == AccessRole::Admin {
        warn!("Admin attempted to get booking by id");
        return Err(AppError::forbidden("only employees have access"));
    }

    let booking_row = sqlx::query_as!(GetBooking, "select space_id, booked_by, start_time, end_time from bookings where booked_by = $1 and booking_id = $2",
        claims.id,
        booking_id)
        .fetch_one(&pg)
        .await
        .map_err(|e| {
            match e {
                sqlx::Error::RowNotFound => {
                    warn!("Booking not found: {} for user: {}", booking_id, claims.id);
                    AppError::not_found("booking not found")
                },
                _ => {
                    warn!("Database error fetching booking: {}", e);
                    AppError::database("Failed to fetch booking")
                }
            }
        })?;

    Ok(Json(json!({
        "success": true,
        "data": booking_row
    })))
}

pub async fn get_company_bookings(
    State(pg): State<PgPool>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Value>, AppError> {

    let claims = verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|_| {
            warn!("Unauthorized get company bookings attempt - invalid token");
            AppError::unauthorized("invalid credentials")
        })?;
    
    if claims.role != AccessRole::Manager {
        warn!("Non-manager user attempted to get company bookings: {:?}", claims.role);
        return Err(AppError::forbidden("only managers have access"));
    }

    let booking_row = sqlx::query_as!(GetCompanyBooking, "SELECT
            b.booking_id,
            b.space_id,
            b.start_time,
            b.end_time,
            e.name AS employee_name,
            e.email,
            b.booked_by AS emp_id
        FROM bookings AS b
        INNER JOIN employees AS e ON b.booked_by = e.emp_id
        WHERE e.comp_id = $1;",
        claims.id)
        .fetch_all(&pg)
        .await
        .map_err(|e| {
            warn!("Database error fetching company bookings: {}", e);
            AppError::database("Failed to fetch company bookings")
        })?;

    Ok(Json(json!({
        "success": true,
        "data": booking_row
    })))
}

