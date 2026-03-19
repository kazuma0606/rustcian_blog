// tests/auth_integration_test.rs
// JWT認証の統合テスト
// 2025/7/10
//
// ※ dev-dependenciesに以下が必要:
// reqwest = { version = "0.11", features = ["json"] }
// serde_json = "1.0"
// tokio = { version = "1.0", features = ["full"] }
// axum = "0.7"

use axum::Router;
use dotenvy::dotenv;
use reqwest::StatusCode;
use rusted_ca::infrastructure::config::app_config::DiscordConfig;
use rusted_ca::infrastructure::di::container::DIContainer;
use rusted_ca::presentation::router::app_router::create_app_router;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

type TestAddr = std::net::SocketAddr;

// テストサーバ起動ヘルパー（env::set_varは使わない）
async fn spawn_test_server(app: Router) -> TestAddr {
    use tokio::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    });
    addr
}

// .envファイルを明示的に読み込む
fn init_env() {
    let _ = dotenv();
    // 統合テスト用の環境変数を設定
    unsafe {
        std::env::set_var("TEST_MODE", "1");
    }
}

// テスト用のダミーDiscordConfig
fn dummy_discord_config() -> Arc<DiscordConfig> {
    Arc::new(DiscordConfig {
        webhook_url: String::new(),
        server_name: "test-server".to_string(),
        enabled: false,
        timeout: Duration::from_secs(1),
    })
}

#[tokio::test]
async fn test_login_success() {
    init_env();
    let di = DIContainer::new();
    let user_controller = di.build_user_controller().unwrap();
    let app = create_app_router(user_controller, dummy_discord_config());
    let addr = spawn_test_server(app).await;
    let client = reqwest::Client::new();
    let res = client
        .post(&format!("http://{}/api/auth/login", addr))
        .json(&json!({"username": "auth_user", "password": "auth_password"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    assert!(body["access_token"].is_string());
}

#[tokio::test]
async fn test_login_fail() {
    init_env();
    let di = DIContainer::new();
    let user_controller = di.build_user_controller().unwrap();
    let app = create_app_router(user_controller, dummy_discord_config());
    let addr = spawn_test_server(app).await;
    let client = reqwest::Client::new();
    let res = client
        .post(&format!("http://{}/api/auth/login", addr))
        .json(&json!({"username": "wrong", "password": "wrong"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_create_user_with_auth() {
    init_env();
    let di = DIContainer::new();
    let user_controller = di.build_user_controller().unwrap();
    let app = create_app_router(user_controller, dummy_discord_config());
    let addr = spawn_test_server(app).await;
    let client = reqwest::Client::new();
    // まずログインしてトークン取得
    let login_res = client
        .post(&format!("http://{}/api/auth/login", addr))
        .json(&json!({"username": "auth_user", "password": "auth_password"}))
        .send()
        .await
        .unwrap();
    assert_eq!(login_res.status(), StatusCode::OK);
    let body: serde_json::Value = login_res.json().await.unwrap();
    let token = body["access_token"].as_str().unwrap();
    // 認証付きでユーザー作成
    let res = client
        .post(&format!("http://{}/api/users", addr))
        .bearer_auth(token)
        .json(&json!({
            "email": "newuser@example.com",
            "name": "New User",
            "password": "Password123!"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
}

/// 認証なしでユーザー作成APIを叩くと失敗（401または400）することを確認
#[tokio::test]
async fn test_create_user_without_auth() {
    init_env();
    let di = DIContainer::new();
    let user_controller = di.build_user_controller().unwrap();
    let app = create_app_router(user_controller, dummy_discord_config());
    let addr = spawn_test_server(app).await;
    let client = reqwest::Client::new();
    let res = client
        .post(&format!("http://{}/api/users", addr))
        .json(&json!({
            "email": "failuser@example.com",
            "name": "Fail User",
            "password": "Password123!"
        }))
        .send()
        .await
        .unwrap();
    assert!(
        res.status() == StatusCode::UNAUTHORIZED || res.status() == StatusCode::BAD_REQUEST,
        "expected 401 or 400, got {}",
        res.status()
    );
}
