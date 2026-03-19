use tokio::signal;

/// グレースフルシャットダウン用シグナルハンドラ
pub async fn shutdown_signal() {
    // Ctrl+Cを待つ
    signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");
    println!("Good Bye.");
}
