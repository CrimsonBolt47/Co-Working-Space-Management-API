use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Spaces {
    pub space_id: Uuid,
    pub name: String,
    pub size: usize,
    pub desciption: String,
       pub created_at: OffsetDateTime,
}

