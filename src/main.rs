mod db;
mod models;
mod routemount;
mod routes;
mod utils;

use db::init_db;

use crate::routemount::route::create_router;
#[tokio::main]
async fn main() {

    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("database_url is missing in env");
    let server_address = std::env::var("SERVER_ADDRESS").unwrap_or("127.0.0.1:7870".to_string());

    //connect to db
    let db_pool = init_db(&database_url).await;
    //connection
    let app = create_router(db_pool);

    let listener = tokio::net::TcpListener::bind(server_address).await.unwrap();
    println!("server running on port 7879");
    axum::serve(listener, app).await.unwrap();
}
