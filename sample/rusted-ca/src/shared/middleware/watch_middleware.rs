//shared/middleware/watch_middleware.rs
// 軽量ログ・メトリクス収集ミドルウェア
// 2025/1/27

use axum::{body::Body, extract::Request, http::Response, middleware::Next};
use chrono::{DateTime, Timelike, Utc};
use serde::Serialize;
use std::collections::HashMap;
use std::time::Instant;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

#[derive(Serialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
    pub context: HashMap<String, String>,
}

#[derive(Serialize)]
pub enum LogLevel {
    #[serde(rename = "Info")]
    Info,
    #[serde(rename = "Error")]
    Error,
}

#[derive(Serialize)]
struct RequestInfo {
    method: String,
    path: String,
    timestamp: DateTime<Utc>,
}

/// 軽量なリクエストID生成
fn generate_lightweight_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    format!("req_{:06}", rng.gen_range(0..999999))
}

/// ログファイルマネージャー
struct LogFileManager {
    base_path: String,
}

impl LogFileManager {
    fn new(base_path: &str) -> Self {
        Self {
            base_path: base_path.to_string(),
        }
    }

    fn get_error_log_path(&self) -> String {
        let now = Utc::now();
        let date = now.format("%Y%m%d");
        let hour = now.hour();

        format!("{}/{}_hour_{}_Error_log.jsonl", self.base_path, date, hour)
    }

    fn get_info_log_path(&self) -> String {
        let now = Utc::now();
        let date = now.format("%Y%m%d");
        let hour = now.hour();

        format!("{}/{}_hour_{}_logs.jsonl", self.base_path, date, hour)
    }
}

/// 非同期ログ保存
/// JSON Lines形式でログを保存（各行が独立したJSONオブジェクト）
async fn save_log_async(
    entry: LogEntry,
    base_path: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let file_manager = LogFileManager::new(base_path);

    let log_line = serde_json::to_string(&entry)? + "\n";

    // ファイルに追記（軽量な操作）
    let path = match entry.level {
        LogLevel::Error => file_manager.get_error_log_path(),
        _ => file_manager.get_info_log_path(),
    };

    // ディレクトリが存在しない場合は作成
    if let Some(parent) = std::path::Path::new(&path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // 非同期ファイル書き込み（JSON Lines形式）
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await?
        .write_all(log_line.as_bytes())
        .await?;

    Ok(())
}

/// 軽量ログ・メトリクス収集ミドルウェア（本番用: ./logs固定）
pub async fn watch_middleware(req: Request<Body>, next: Next) -> Response<Body> {
    watch_middleware_with_base_path(req, next, "./logs").await
}

/// テスト用: base_pathを指定できるミドルウェア
pub async fn watch_middleware_with_base_path(
    req: Request<Body>,
    next: Next,
    base_path: &str,
) -> Response<Body> {
    let start_time = Instant::now();
    let _request_id = generate_lightweight_id();

    // 最小限の情報のみ収集
    let request_info = RequestInfo {
        method: req.method().to_string(),
        path: req.uri().path().to_string(),
        timestamp: Utc::now(),
    };

    // レスポンス取得
    let response = next.run(req).await;

    // 軽量なログ記録（非同期、ブロックしない）
    let log_entry = LogEntry {
        timestamp: Utc::now(),
        level: if response.status().is_success() {
            LogLevel::Info
        } else {
            LogLevel::Error
        },
        message: format!(
            "{} {} - {}",
            request_info.method,
            request_info.path,
            response.status()
        ),
        context: HashMap::new(),
    };

    // 非同期でログ保存（レスポンスをブロックしない）
    let base_path = base_path.to_string();
    tokio::spawn(async move {
        if let Err(e) = save_log_async(log_entry, &base_path).await {
            eprintln!("Log save failed: {}", e);
        }
    });

    response
}
