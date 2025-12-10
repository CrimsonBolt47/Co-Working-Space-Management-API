use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::Duration;
use uuid::Uuid;

#[derive(Serialize, Deserialize,Debug, sqlx::FromRow)]
pub struct Booking {
    pub booking_id: Uuid,
    pub space_id: Uuid,
    pub booked_by: Uuid,
    pub start_time: OffsetDateTime,
    pub end_time: OffsetDateTime,
    created_at: OffsetDateTime
}

#[derive(Serialize, Deserialize)]
pub struct CreateBookingReq {
    pub space_id: Uuid,
    pub start_time: OffsetDateTime,
    pub end_time: OffsetDateTime,
}

#[derive(Serialize, Deserialize)]
pub struct BookingId {
    pub booking_id: Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateBookingReq {
    pub extra_time: Duration,
}

#[derive(Serialize, Deserialize,Debug)]
pub struct GetBooking {
    pub space_id: Uuid,
    pub booked_by: Uuid,
    pub start_time: OffsetDateTime,
    pub end_time: OffsetDateTime,
}

#[derive(Serialize, Deserialize)]
pub struct GetCompanyBooking {
    pub booking_id: Uuid,
    pub space_id: Uuid,
    pub emp_id: Uuid,
    pub employee_name: String,
    pub email: String,
    pub start_time: OffsetDateTime,
    pub end_time: OffsetDateTime,
}

#[derive(Serialize, Deserialize)]
pub struct AvailableBooking {
    pub start_time: OffsetDateTime,
    pub end_time: OffsetDateTime,
}