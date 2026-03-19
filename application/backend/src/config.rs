use std::{env, path::PathBuf};

use rustacian_blog_core::BlogError;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub app_env: String,
    pub app_host: String,
    pub app_port: u16,
    pub storage_backend: String,
    pub content_root: PathBuf,
    pub azurite_blob_endpoint: Option<String>,
    pub azurite_table_endpoint: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, BlogError> {
        let storage_backend = env::var("STORAGE_BACKEND").unwrap_or_else(|_| "azurite".to_owned());

        Ok(Self {
            app_env: env::var("APP_ENV").unwrap_or_else(|_| "local".to_owned()),
            app_host: env::var("APP_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned()),
            app_port: env::var("APP_PORT")
                .ok()
                .and_then(|value| value.parse::<u16>().ok())
                .unwrap_or(8080),
            storage_backend: storage_backend.clone(),
            content_root: PathBuf::from(
                env::var("CONTENT_ROOT").unwrap_or_else(|_| "./content".to_owned()),
            ),
            azurite_blob_endpoint: Some(env::var("AZURITE_BLOB_ENDPOINT").unwrap_or_else(|_| {
                if storage_backend == "azurite" {
                    "http://127.0.0.1:10000/devstoreaccount1".to_owned()
                } else {
                    String::new()
                }
            }))
            .filter(|value| !value.is_empty()),
            azurite_table_endpoint: Some(env::var("AZURITE_TABLE_ENDPOINT").unwrap_or_else(|_| {
                if storage_backend == "azurite" {
                    "http://127.0.0.1:10002/devstoreaccount1".to_owned()
                } else {
                    String::new()
                }
            }))
            .filter(|value| !value.is_empty()),
        })
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.app_host, self.app_port)
    }

    pub fn images_dir(&self) -> PathBuf {
        self.content_root.join("images")
    }
}

#[cfg(test)]
mod tests {
    use super::AppConfig;

    #[test]
    fn default_config_is_local_and_azurite_ready() {
        let config = AppConfig::from_env().unwrap();

        assert_eq!(config.app_env, "local");
        assert_eq!(config.storage_backend, "azurite");
        assert_eq!(config.bind_address(), "127.0.0.1:8080");
    }
}
