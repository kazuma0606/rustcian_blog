//main.rs
// メインアプリケーション - Webサーバー起動
// 2025/7/8

use rusted_ca::infrastructure::config::app_config::AppConfig;
use rusted_ca::infrastructure::web::run::run;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let dotenv_result = dotenvy::from_path("C:/Users/yoshi/rusted-ca/rusted-ca/.env");
    println!("DEBUG: dotenvy result = {:?}", dotenv_result);

    for (key, value) in std::env::vars() {
        println!("ENV: {} = {}", key, value);
    }

    println!("🚀 クリーンアーキテクチャ + CQRS + DIコンテナのWebサーバーを起動します");
    println!("📋 実装内容:");
    println!("  - ドメイン層: エンティティ、バリューオブジェクト、リポジトリインターフェース");
    println!("  - アプリケーション層: ユースケース、DTO、コマンド/クエリ分離");
    println!("  - インフラストラクチャ層: SQLite実装、DIコンテナ、CQRS、Webサーバー");
    println!("  - プレゼンテーション層: コントローラー、ルーター、DTO");
    println!("  - gRPCサーバー: Protocol Buffers + Prost + Axum");
    println!("  - Discord通知: リアルタイム監視・アラート");

    // アプリケーション設定を読み込み
    let _app_config = AppConfig::from_env();

    // HTTP + gRPC統合サーバーを起動
    run().await?;

    Ok(())
}
