use warp::{Filter, Rejection, Reply};
use serde_json::json;

/// Health check handler
pub async fn health_check() -> Result<impl Reply, Rejection> {
    Ok(warp::reply::json(&json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

/// Create all routes
pub fn routes() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let health = warp::path("health")
        .and(warp::get())
        .and_then(health_check);

    health
} 