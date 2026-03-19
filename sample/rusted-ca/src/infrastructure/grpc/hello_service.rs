use axum::{
    body::Body,
    extract::Request,
    http::{Response, StatusCode},
    response::IntoResponse,
};
use prost::Message;
use std::sync::Arc;

// build.rsで生成されたコードをインポート
#[cfg(not(doctest))]
pub mod hello {
    include!(concat!(env!("OUT_DIR"), "/hello.rs"));
}

// 生成されたコードから型を再エクスポート
pub use hello::{HelloRequest, HelloResponse};

#[derive(Clone)]
pub struct HelloService;

impl HelloService {
    pub fn new() -> Self {
        Self
    }

    // Protocol Buffersの準拠性をテストする関数
    pub fn test_protobuf_compliance(&self) -> bool {
        // テスト用のリクエストを作成
        let test_request = HelloRequest {
            name: "Test".to_string(),
        };

        // エンコード
        let encoded = test_request.encode_to_vec();

        // デコード
        match HelloRequest::decode(encoded.as_ref()) {
            Ok(decoded) => {
                // 元のデータと一致するかチェック
                decoded.name == test_request.name
            }
            Err(_) => false,
        }
    }

    pub async fn handle_grpc_request(&self, request: Request<Body>) -> Response<Body> {
        // リクエストボディを取得
        let body_bytes = match axum::body::to_bytes(request.into_body(), usize::MAX).await {
            Ok(bytes) => bytes,
            Err(_) => {
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("Invalid request body"))
                    .unwrap();
            }
        };

        // リクエストボディが空の場合はデフォルトメッセージを返す
        let name = if body_bytes.is_empty() {
            "".to_string()
        } else {
            // JSONリクエストを試す
            if let Ok(json_value) = serde_json::from_slice::<serde_json::Value>(&body_bytes) {
                json_value["name"].as_str().unwrap_or("").to_string()
            } else {
                // Protocol Buffersのデシリアライゼーションを試す
                match HelloRequest::decode(body_bytes.as_ref()) {
                    Ok(req) => req.name,
                    Err(_) => {
                        return Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(Body::from("Invalid request format"))
                            .unwrap();
                    }
                }
            }
        };

        // ビジネスロジック
        let message = if name.is_empty() {
            "Hello, gRPC with PROST!".to_string()
        } else {
            format!("Hello, {}! gRPC with PROST!", name)
        };

        // JSONレスポンスを返す
        let response_json = serde_json::json!({
            "message": message,
            "protocol": "grpc-style"
        });

        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(Body::from(response_json.to_string()))
            .unwrap()
    }
}

// axumハンドラーとして使用
pub async fn grpc_hello_handler(
    axum::extract::State(service): axum::extract::State<Arc<HelloService>>,
    request: Request<Body>,
) -> impl IntoResponse {
    service.handle_grpc_request(request).await
}
