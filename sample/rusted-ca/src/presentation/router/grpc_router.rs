use crate::infrastructure::grpc::hello_service::{HelloService, grpc_hello_handler};
use axum::{Router, routing::post};
use std::sync::Arc;

/// gRPCルーターの作成
pub fn create_grpc_routes() -> Router {
    let hello_service = Arc::new(HelloService::new());

    Router::new()
        .route("/grpc/hello", post(grpc_hello_handler))
        .with_state(hello_service)
}
