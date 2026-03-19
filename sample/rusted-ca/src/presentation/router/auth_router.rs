//presentation/router/auth_router.rs
use crate::presentation::controller::auth_controller;
use axum::{Router, routing::post};

pub fn create_auth_routes() -> Router {
    Router::new().route("/auth/login", post(auth_controller::login))
}
