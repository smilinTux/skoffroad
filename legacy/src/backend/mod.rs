mod routes;
mod models;
mod config;

pub use routes::*;
pub use models::*;
pub use config::*;

use warp::Filter;
use std::net::SocketAddr;

/// Start the backend server
pub async fn start_server(addr: SocketAddr) -> Result<(), anyhow::Error> {
    // Health check route
    let health_check = warp::path("health")
        .and(warp::get())
        .map(|| warp::reply::json(&serde_json::json!({ "status": "ok" })));

    // Combine all routes
    let routes = health_check;

    println!("Starting server on {}", addr);
    warp::serve(routes)
        .run(addr)
        .await;

    Ok(())
} 