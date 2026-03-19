//shared/middleware/cors_middleware.rs
// CORS設定
// 2025/7/12

use crate::infrastructure::utils::cors_settings;
use axum::http::HeaderValue;
use tower_http::cors::CorsLayer;

pub fn build_cors_layer() -> CorsLayer {
    let origins = cors_settings::allowed_origins()
        .into_iter()
        .filter_map(|o| o.parse::<HeaderValue>().ok())
        .collect::<Vec<_>>();
    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods(cors_settings::allowed_methods())
        .allow_headers(cors_settings::allowed_headers())
}
