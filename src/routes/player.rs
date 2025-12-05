use axum::{
    Json, extract::State, http::StatusCode 
};
use sqlx::{PgPool};
use serde_json::json;

use crate::models::player::{PlayerRow,CreatePlayerReq};
//get all players
pub async fn get_players(
    State(pg_connection_pool): State<PgPool>,
) -> Result<(StatusCode, String), (StatusCode,String)> {
    let rows = sqlx::query_as!(PlayerRow, r#"SELECT * FROM players ORDER BY player_id"#)
        .fetch_all(&pg_connection_pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"success": false, "message": e.to_string()}).to_string(),
            )
        })?;

    Ok((
        StatusCode::OK,
        json!({"success": true, "data": rows}).to_string(),
    ))
}

pub async fn create_player(
    State(pg_connection_pool): State<PgPool>,
    Json(player): Json<CreatePlayerReq>,
) ->  Result<(StatusCode, String), (StatusCode,String)> {
    let row = sqlx::query_as!(
        PlayerRow,
        "INSERT INTO players (name, age, wing) VALUES($1, $2, $3) RETURNING name, age, wing, player_id",
        player.name,
        player.age,
        player.wing
    )
    .fetch_one(&pg_connection_pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"success": false, "message": e.to_string()}).to_string(),
        )
    })?;

    Ok((
         StatusCode::OK,
        json!({"success": true, "player": row}).to_string(),
    ))
}