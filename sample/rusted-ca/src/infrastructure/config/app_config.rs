//infrastructure/config/app_config.rs
// アプリケーション設定
// 2025/7/8

use std::time::Duration;

/// Discord通知設定
#[derive(Clone, Debug)]
pub struct DiscordConfig {
    pub webhook_url: String,
    pub server_name: String,
    pub enabled: bool,
    pub timeout: Duration,
}

impl DiscordConfig {
    pub fn from_env() -> Self {
        Self {
            webhook_url: std::env::var("DISCORD_WEBHOOK_URL").unwrap_or_default(),
            server_name: std::env::var("DISCORD_SERVER_NAME")
                .unwrap_or_else(|_| "Rusted-CA Dev Alerts".to_string()),
            enabled: std::env::var("DISCORD_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .to_ascii_lowercase()
                == "true",
            timeout: Duration::from_secs(
                std::env::var("DISCORD_TIMEOUT")
                    .unwrap_or_else(|_| "5".to_string())
                    .parse()
                    .unwrap_or(5),
            ),
        }
    }
}

/// アプリケーション設定
#[derive(Clone, Debug)]
pub struct AppConfig {
    pub discord: DiscordConfig,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            discord: DiscordConfig::from_env(),
        }
    }
}
