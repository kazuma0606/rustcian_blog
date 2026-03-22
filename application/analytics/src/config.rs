use std::env;

#[derive(Debug, Clone)]
pub struct AnalyticsConfig {
    pub app_host: String,
    pub app_port: u16,
    pub azurite_table_endpoint: Option<String>,
}

impl AnalyticsConfig {
    pub fn from_env() -> Self {
        Self {
            app_host: env::var("ANALYTICS_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned()),
            app_port: env::var("ANALYTICS_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8081),
            azurite_table_endpoint: env::var("AZURITE_TABLE_ENDPOINT").ok(),
        }
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.app_host, self.app_port)
    }
}
