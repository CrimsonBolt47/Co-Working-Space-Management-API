use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Deserialize, Serialize, FromRow)]
pub struct Space {
    pub space_id: Uuid,
    pub name: String,
    pub size: i32,
    pub description: Option<String>,
    pub created_at: OffsetDateTime,
}

#[derive(Deserialize, Serialize)]
pub struct CreateSpaceReq {
    pub name: String,
    pub size: i32,
    pub description: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct SpaceId {
    pub space_id: Uuid,
}

#[derive(Deserialize, Serialize)]
pub struct SpaceQueryParams {
     pub page: Option<i64>,
    pub limit: Option<i64>,
    pub name: Option<String>,
    pub size: Option<i32>
}

#[derive(Deserialize, Serialize)]
pub struct UpdateSpaceReq {
    pub name: Option<String>,
    pub size: Option<i32>
}

#[derive(Deserialize, Serialize)]
pub struct SpaceAvailableTimings {
    pub start_time: OffsetDateTime,
    pub end_time: OffsetDateTime,
}

#[derive(Deserialize, Serialize, FromRow)]
pub struct SpaceAvailable {
    pub space_id: Uuid,
    pub name: String,
    pub size: i32,
    pub description: Option<String>,
}