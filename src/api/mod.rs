pub mod errors;
pub mod models;
pub mod routes;

use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use bollard::Docker;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub fn router(docker: Docker) -> Router {
    let state = Arc::new(docker);

    Router::new()
        .route("/health", get(routes::health))
        .route("/deploy", post(routes::deploy))
        .route("/containers", get(routes::list_containers))
        .route("/containers/{id}", get(routes::get_container))
        .route("/containers/{id}/stop", post(routes::stop_container))
        .route("/containers/{id}/logs", get(routes::get_logs))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
