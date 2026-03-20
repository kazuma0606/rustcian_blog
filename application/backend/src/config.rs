use std::{
    env,
    path::{Path, PathBuf},
};

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
    pub azure_openai_endpoint: Option<String>,
    pub azure_openai_deployment: Option<String>,
    pub azure_openai_api_key: Option<String>,
    pub azure_openai_api_version: String,
    pub azure_openai_model_name: Option<String>,
    pub admin_auth_mode: String,
    pub entra_tenant_id: Option<String>,
    pub entra_client_id: Option<String>,
    pub entra_oidc_metadata_url: Option<String>,
    pub entra_admin_group_id: Option<String>,
    pub entra_admin_user_oid: Option<String>,
    pub static_output_dir: PathBuf,
    pub static_publish_backend: String,
    pub static_publish_prefix: String,
    pub observability_backend: String,
    pub application_insights_connection_string: Option<String>,
    pub base_url: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, BlogError> {
        let storage_backend = env::var("STORAGE_BACKEND").unwrap_or_else(|_| "azurite".to_owned());
        let workspace_root = workspace_root();

        Ok(Self {
            app_env: env::var("APP_ENV").unwrap_or_else(|_| "local".to_owned()),
            app_host: env::var("APP_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned()),
            app_port: env::var("APP_PORT")
                .ok()
                .and_then(|value| value.parse::<u16>().ok())
                .unwrap_or(8080),
            storage_backend: storage_backend.clone(),
            content_root: resolve_workspace_path(
                &workspace_root,
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
            azure_openai_endpoint: env::var("AZURE_OPENAI_ENDPOINT").ok(),
            azure_openai_deployment: env::var("AZURE_OPENAI_DEPLOYMENT").ok(),
            azure_openai_api_key: env::var("AZURE_OPENAI_API_KEY").ok(),
            azure_openai_api_version: env::var("AZURE_OPENAI_API_VERSION")
                .unwrap_or_else(|_| "2024-10-21".to_owned()),
            azure_openai_model_name: env::var("AZURE_OPENAI_MODEL_NAME").ok(),
            admin_auth_mode: env::var("ADMIN_AUTH_MODE").unwrap_or_else(|_| "disabled".to_owned()),
            entra_tenant_id: env::var("ENTRA_TENANT_ID").ok(),
            entra_client_id: env::var("ENTRA_CLIENT_ID").ok(),
            entra_oidc_metadata_url: env::var("ENTRA_OIDC_METADATA_URL").ok(),
            entra_admin_group_id: env::var("ENTRA_ADMIN_GROUP_ID").ok(),
            entra_admin_user_oid: env::var("ENTRA_ADMIN_USER_OID").ok(),
            static_output_dir: resolve_workspace_path(
                &workspace_root,
                env::var("STATIC_OUTPUT_DIR").unwrap_or_else(|_| "./dist".to_owned()),
            ),
            static_publish_backend: env::var("STATIC_PUBLISH_BACKEND")
                .unwrap_or_else(|_| "local".to_owned()),
            static_publish_prefix: env::var("STATIC_PUBLISH_PREFIX")
                .unwrap_or_else(|_| "site".to_owned())
                .trim_matches('/')
                .to_owned(),
            observability_backend: env::var("OBSERVABILITY_BACKEND")
                .unwrap_or_else(|_| "stdout".to_owned()),
            application_insights_connection_string: env::var(
                "APPLICATIONINSIGHTS_CONNECTION_STRING",
            )
            .ok(),
            base_url: env::var("BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned())
                .trim_end_matches('/')
                .to_owned(),
        })
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.app_host, self.app_port)
    }

    pub fn images_dir(&self) -> PathBuf {
        self.content_root.join("images")
    }

    pub fn metadata_dir(&self) -> PathBuf {
        self.content_root.join("metadata")
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn resolve_workspace_path(workspace_root: &Path, value: String) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        workspace_root.join(path)
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
