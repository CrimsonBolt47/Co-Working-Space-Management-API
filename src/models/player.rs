use serde::{Deserialize, Serialize};
#[derive(Serialize)]
pub struct PlayerRow{
    pub player_id: i32,
    pub name: String,
    pub age: i32,
    pub wing: i32,
}

#[derive(Deserialize)]
pub struct CreatePlayerReq{
    pub name: String,
    pub age: i32,
    pub wing: i32,
}