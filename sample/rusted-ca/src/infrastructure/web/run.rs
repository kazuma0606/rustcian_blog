//infrastructure/web/run.rs
// Webサーバー実行関数
// 2025/7/8

use axum::{
    Router,
    routing::{get, post},
    serve,
};
use std::net::SocketAddr;
use std::sync::Arc;

use crate::infrastructure::config::app_config::AppConfig;
use crate::infrastructure::di::container::DIContainer;
use crate::infrastructure::grpc::server::create_grpc_router;
use crate::infrastructure::utils::graceful_shutdown::shutdown_signal;
use crate::presentation::router::app_router::create_app_router;

/// Webサーバーを起動する
pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 1. DIコンテナの初期化
    let di_container = DIContainer::new();

    // 2. データベース接続のテスト
    let db_connection = di_container.create_database_connection()?;
    println!("✅ データベース接続の作成が完了しました");

    // 3. Repository実装のテスト
    let (command_repo, query_repo) = di_container.create_repositories()?;
    println!("✅ Repository実装の作成が完了しました");

    // 4. ID生成器のテスト
    let id_generator = di_container.create_id_generator();
    let test_id = id_generator();
    println!("✅ ID生成器のテストが完了しました: {}", test_id.0);

    // 5. アプリケーション設定の読み込み
    let app_config = AppConfig::from_env();
    let discord_config = Arc::new(app_config.discord);

    // 6. ルーティング設定（HTTP + gRPC統合）
    let user_controller = di_container.build_user_controller()?;
    let http_router = create_app_router(user_controller, discord_config);
    let grpc_router = create_grpc_router();

    // HTTPとgRPCルーターを統合
    let app = http_router.merge(grpc_router);

    // 6. サーバー起動
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("🚀 Server starting on {}", addr);
    println!("📋 利用可能なエンドポイント:");
    println!("  - POST /api/auth/login - ログイン(ユーザー認証)");
    println!("  - GET  /health - ヘルスチェック");
    println!("  - GET  /api/health - APIヘルスチェック");
    println!("  - POST /api/users - ユーザー作成");
    println!("  - GET  /api/users/:id - ユーザー取得");
    println!("  - PUT  /api/users/:id - ユーザー更新");
    println!("  - DELETE /api/users/:id - ユーザー削除");
    println!("  - GET  /api/fortune - ランダム癒し系おみくじ");
    println!("  - POST /grpc/hello - gRPC Hello Service (Protocol Buffers)");
    println!("  - Discord通知: エラー発生時に自動通知");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// APIヘルスチェックエンドポイント
async fn api_health_check() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "message": "API is running",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
