// tests/logs_integration_test.rs
// ログ機能の結合テスト
// 2025/1/27

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::get,
};
use chrono::{DateTime, Timelike, Utc};
use rand::Rng;
use rusted_ca::shared::middleware::watch_middleware::{
    watch_middleware, watch_middleware_with_base_path,
};
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tower::util::ServiceExt;

/// テスト用の簡単なエンドポイント
async fn test_endpoint() -> &'static str {
    "Hello, World!"
}

/// テスト用のエラーエンドポイント
async fn test_error_endpoint() -> (StatusCode, &'static str) {
    (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
}

/// テスト用のルーター作成
fn create_test_router(base_path: Arc<String>) -> Router {
    let base_path_static: &'static str = Box::leak(base_path.to_string().into_boxed_str());
    Router::new()
        .route("/test", get(test_endpoint))
        .route("/error", get(test_error_endpoint))
        .layer(axum::middleware::from_fn(move |req, next| {
            watch_middleware_with_base_path(req, next, base_path_static)
        }))
}

/// ログファイルの内容を読み込む
async fn read_log_file(path: &str) -> Vec<String> {
    if !Path::new(path).exists() {
        return Vec::new();
    }

    let content = fs::read_to_string(path).expect("Failed to read log file");
    content.lines().map(|line| line.to_string()).collect()
}

/// ログエントリを解析する
fn parse_log_entry(line: &str) -> Option<Value> {
    serde_json::from_str(line).ok()
}

#[tokio::test]
async fn test_watch_middleware_logs_successful_request() {
    let base_path = Arc::new(format!(
        "./test_logs/test_success_{}",
        rand::random::<u32>()
    ));
    let _ = fs::create_dir_all(&*base_path);
    let now = Utc::now();
    let date = now.format("%Y%m%d");
    let hour = now.hour();
    let log_file_path = format!("{}/{}_hour_{}_logs.jsonl", base_path, date, hour);
    let _ = fs::remove_file(&log_file_path);
    let app = create_test_router(base_path.clone());

    // テストリクエストを送信
    let request = Request::builder()
        .method("GET")
        .uri("/test")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // レスポンスが成功することを確認
    assert_eq!(response.status(), StatusCode::OK);

    // 少し待ってからログファイルを確認
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // ログファイルが生成されていることを確認
    assert!(
        Path::new(&log_file_path).exists(),
        "Log file should be created"
    );

    // ログファイルの内容を確認
    let log_lines = read_log_file(&log_file_path).await;
    assert!(!log_lines.is_empty(), "Log file should contain entries");

    // 最新のログエントリを解析
    if let Some(last_line) = log_lines.last() {
        let log_entry = parse_log_entry(last_line);
        assert!(log_entry.is_some(), "Log entry should be valid JSON");

        if let Some(entry) = log_entry {
            // ログエントリの内容を確認
            assert_eq!(entry["level"], "Info");
            assert!(entry["message"].as_str().unwrap().contains("GET /test"));
            assert!(entry["message"].as_str().unwrap().contains("200"));
            assert!(entry["timestamp"].is_string());
        }
    }

    // ログディレクトリをクリーンアップ
    let _ = fs::remove_dir_all(&*base_path);
}

#[tokio::test]
async fn test_watch_middleware_logs_error_request() {
    let base_path = Arc::new(format!("./test_logs/test_error_{}", rand::random::<u32>()));
    let _ = fs::create_dir_all(&*base_path);
    let now = Utc::now();
    let date = now.format("%Y%m%d");
    let hour = now.hour();
    let error_log_file_path = format!("{}/{}_hour_{}_Error_log.jsonl", base_path, date, hour);
    let _ = fs::remove_file(&error_log_file_path);
    let app = create_test_router(base_path.clone());

    // エラーを発生させるテストリクエストを送信
    let request = Request::builder()
        .method("GET")
        .uri("/error")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // レスポンスがエラーになることを確認
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    // 少し待ってからログファイルを確認
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // エラーログファイルが生成されていることを確認
    assert!(
        Path::new(&error_log_file_path).exists(),
        "Error log file should be created"
    );

    // エラーログファイルの内容を確認
    let log_lines = read_log_file(&error_log_file_path).await;
    assert!(
        !log_lines.is_empty(),
        "Error log file should contain entries"
    );

    // 最新のログエントリを解析
    if let Some(last_line) = log_lines.last() {
        let log_entry = parse_log_entry(last_line);
        assert!(log_entry.is_some(), "Log entry should be valid JSON");

        if let Some(entry) = log_entry {
            // エラーログエントリの内容を確認
            assert_eq!(entry["level"], "Error");
            assert!(entry["message"].as_str().unwrap().contains("GET /error"));
            assert!(entry["message"].as_str().unwrap().contains("500"));
            assert!(entry["timestamp"].is_string());
        }
    }

    let _ = fs::remove_dir_all(&*base_path);
}

#[tokio::test]
async fn test_watch_middleware_logs_multiple_requests() {
    let base_path = Arc::new(format!("./test_logs/test_multi_{}", rand::random::<u32>()));
    let _ = fs::create_dir_all(&*base_path);
    let now = Utc::now();
    let date = now.format("%Y%m%d");
    let hour = now.hour();
    let log_file_path = format!("{}/{}_hour_{}_logs.jsonl", base_path, date, hour);
    let error_log_file_path = format!("{}/{}_hour_{}_Error_log.jsonl", base_path, date, hour);
    let _ = fs::remove_file(&log_file_path);
    let _ = fs::remove_file(&error_log_file_path);
    let app = create_test_router(base_path.clone());

    // 複数のリクエストを送信
    for _i in 0..3 {
        let request = Request::builder()
            .method("GET")
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    // エラーリクエストも送信
    let error_request = Request::builder()
        .method("GET")
        .uri("/error")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(error_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    // 少し待ってからログファイルを確認
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 通常ログファイルの確認
    let log_lines = read_log_file(&log_file_path).await;
    assert_eq!(
        log_lines.len(),
        3,
        "Should have 3 log entries for successful requests"
    );

    // エラーログファイルの確認
    let error_log_lines = read_log_file(&error_log_file_path).await;
    assert_eq!(
        error_log_lines.len(),
        1,
        "Should have 1 log entry for error request"
    );

    let _ = fs::remove_dir_all(&*base_path);
}

#[tokio::test]
async fn test_watch_middleware_logs_file_format() {
    let base_path = Arc::new(format!("./test_logs/test_format_{}", rand::random::<u32>()));
    let _ = fs::create_dir_all(&*base_path);
    let now = Utc::now();
    let date = now.format("%Y%m%d");
    let hour = now.hour();
    let log_file_path = format!("{}/{}_hour_{}_logs.jsonl", base_path, date, hour);
    let _ = fs::remove_file(&log_file_path);
    let app = create_test_router(base_path.clone());

    // テストリクエストを送信
    let request = Request::builder()
        .method("GET")
        .uri("/test")
        .body(Body::empty())
        .unwrap();

    let _response = app.oneshot(request).await.unwrap();

    // 少し待ってからログファイルを確認
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // ログファイルの内容を確認
    let log_lines = read_log_file(&log_file_path).await;
    assert!(!log_lines.is_empty(), "Log file should contain entries");

    // 各行が有効なJSONであることを確認
    for line in log_lines {
        let parsed = parse_log_entry(&line);
        assert!(parsed.is_some(), "Each line should be valid JSON: {}", line);

        if let Some(entry) = parsed {
            // 必須フィールドの存在を確認
            assert!(
                entry["timestamp"].is_string(),
                "timestamp should be present"
            );
            assert!(entry["level"].is_string(), "level should be present");
            assert!(entry["message"].is_string(), "message should be present");
            assert!(entry["context"].is_object(), "context should be present");

            // levelの値が正しいことを確認
            let level = entry["level"].as_str().unwrap();
            assert!(
                level == "Info" || level == "Error",
                "level should be Info or Error"
            );
        }
    }

    let _ = fs::remove_dir_all(&*base_path);
}
