use crate::infrastructure::config::app_config::DiscordConfig;
use crate::shared::notification::discord_notification::{notify_app_startup, notify_error};
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

/// Discord通知ミドルウェア
pub async fn discord_notification_middleware(
    State(config): State<Arc<DiscordConfig>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let start_time = std::time::Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();
    let response = next.run(request).await;

    // エラーが発生した場合のみ通知
    if response.status() >= StatusCode::BAD_REQUEST {
        let error_context = format!(
            "Method: {} | URI: {} | Status: {} | Duration: {:?}",
            method,
            uri,
            response.status(),
            start_time.elapsed()
        );
        let config_clone = config.clone();
        tokio::spawn(async move {
            let _ = notify_error(&config_clone, "HTTP Error", &error_context).await;
        });
    }
    response
}

/// 起動時Discord通知（必要な場合のみapp_routerから呼び出す）
pub async fn try_notify_startup(config: Arc<DiscordConfig>) {
    if config.enabled {
        let _ = notify_app_startup(&config).await;
    }
}
