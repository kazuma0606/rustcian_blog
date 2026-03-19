//presentation/router/fortune_router.rs
use crate::presentation::controller::fortune_controller::get_fortune;
use axum::{Router, routing::get};

pub fn create_fortune_routes() -> Router {
    Router::new().route("/fortune", get(get_fortune))
}
