use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct Admin {
    pub admin_id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub created_at: OffsetDateTime,
}

#[derive(Serialize, Deserialize)]
pub struct LoginAdmin {
    pub email: String,
    pub password: String
}

pub struct AuthAdmin {
    pub email: String,
}