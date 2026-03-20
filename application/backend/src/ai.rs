use std::{fs, path::PathBuf, sync::Arc};

use chrono::Utc;
use reqwest::Client;
use rustacian_blog_core::{
    AiAssistRequest, AiGenerationScope, AiMetadataGenerator, BlogError, GeneratedMetadata,
    GeneratedMetadataStore,
};
use serde::{Deserialize, Serialize};

use crate::blob::AzuriteBlobAdapter;
use crate::config::AppConfig;

#[derive(Debug, Clone)]
pub struct AzureOpenAiConfig {
    pub endpoint: String,
    pub deployment: String,
    pub api_key: String,
    pub api_version: String,
    pub model_name: String,
}

pub struct AzureOpenAiMetadataGenerator {
    client: Client,
    config: AzureOpenAiConfig,
}

impl AzureOpenAiMetadataGenerator {
    pub fn new(config: AzureOpenAiConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    fn url(&self) -> String {
        format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.config.endpoint.trim_end_matches('/'),
            self.config.deployment,
            self.config.api_version
        )
    }

    fn build_prompt(request: &AiAssistRequest, scope: &AiGenerationScope) -> String {
        format!(
            "You are assisting a Rust technical blog workflow.\nReturn JSON with keys summary_ai, suggested_tags, intro_candidates.\nOnly include data for requested fields.\nsummary: {}\nsuggested_tags: {}\nintro_candidates: {}\nSlug: {}\nTitle: {}\nExisting tags: {}\nAuthor summary: {}\nMarkdown:\n{}",
            scope.summary,
            scope.suggested_tags,
            scope.intro_candidates,
            request.slug,
            request.title,
            request.tags.join(", "),
            request.summary,
            request.body_markdown
        )
    }
}

pub fn build_ai_metadata_generator(config: &AppConfig) -> Option<Arc<dyn AiMetadataGenerator>> {
    let endpoint = config.azure_openai_endpoint.clone()?;
    let deployment = config.azure_openai_deployment.clone()?;
    let api_key = config.azure_openai_api_key.clone()?;

    Some(Arc::new(AzureOpenAiMetadataGenerator::new(
        AzureOpenAiConfig {
            endpoint,
            deployment,
            api_key,
            api_version: config.azure_openai_api_version.clone(),
            model_name: config
                .azure_openai_model_name
                .clone()
                .unwrap_or_else(|| "azure-openai".to_owned()),
        },
    )))
}

#[async_trait::async_trait]
impl AiMetadataGenerator for AzureOpenAiMetadataGenerator {
    async fn generate_metadata(
        &self,
        request: AiAssistRequest,
        scope: AiGenerationScope,
    ) -> Result<GeneratedMetadata, BlogError> {
        let payload = ChatCompletionsRequest {
            messages: vec![
                ChatMessage {
                    role: "system".to_owned(),
                    content: "Return compact JSON only. Do not wrap in markdown.".to_owned(),
                },
                ChatMessage {
                    role: "user".to_owned(),
                    content: Self::build_prompt(&request, &scope),
                },
            ],
            response_format: JsonResponseFormat {
                r#type: "json_object".to_owned(),
            },
        };

        let response = self
            .client
            .post(self.url())
            .header("api-key", &self.config.api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|error| BlogError::Storage(error.to_string()))?;

        if !response.status().is_success() {
            return Err(BlogError::Storage(format!(
                "azure openai request failed with status {}",
                response.status()
            )));
        }

        let body: ChatCompletionsResponse = response
            .json()
            .await
            .map_err(|error| BlogError::Parse(error.to_string()))?;
        let content = body
            .choices
            .first()
            .map(|choice| choice.message.content.as_str())
            .ok_or_else(|| BlogError::Parse("azure openai returned no choices".to_owned()))?;
        let generated: GeneratedMetadataPayload =
            serde_json::from_str(content).map_err(|error| BlogError::Parse(error.to_string()))?;

        Ok(GeneratedMetadata {
            summary_ai: generated.summary_ai,
            suggested_tags: generated.suggested_tags,
            intro_candidates: generated.intro_candidates,
            generated_at: Utc::now(),
            source_model: Some(self.config.model_name.clone()),
        })
    }
}

#[derive(Debug, Clone)]
pub struct LocalGeneratedMetadataStore {
    metadata_dir: PathBuf,
}

impl LocalGeneratedMetadataStore {
    pub fn new(metadata_dir: PathBuf) -> Self {
        Self { metadata_dir }
    }
}

#[derive(Debug, Clone)]
pub struct AzuriteGeneratedMetadataStore {
    blob: AzuriteBlobAdapter,
}

impl AzuriteGeneratedMetadataStore {
    pub fn new(blob_endpoint: String) -> Self {
        Self {
            blob: AzuriteBlobAdapter::new(blob_endpoint),
        }
    }
}

pub fn build_generated_metadata_store(config: &AppConfig) -> Arc<dyn GeneratedMetadataStore> {
    match config.storage_backend.as_str() {
        "azurite" => Arc::new(AzuriteGeneratedMetadataStore::new(
            config
                .azurite_blob_endpoint
                .clone()
                .unwrap_or_else(|| "http://127.0.0.1:10000/devstoreaccount1".to_owned()),
        )),
        _ => Arc::new(LocalGeneratedMetadataStore::new(config.metadata_dir())),
    }
}

#[async_trait::async_trait]
impl GeneratedMetadataStore for LocalGeneratedMetadataStore {
    async fn save(&self, slug: &str, metadata: &GeneratedMetadata) -> Result<(), BlogError> {
        fs::create_dir_all(&self.metadata_dir)
            .map_err(|error| BlogError::Storage(error.to_string()))?;
        let path = self.metadata_dir.join(format!("{slug}.json"));
        let body = serde_json::to_string_pretty(metadata)
            .map_err(|error| BlogError::Parse(error.to_string()))?;
        fs::write(path, body).map_err(|error| BlogError::Storage(error.to_string()))
    }
}

#[async_trait::async_trait]
impl GeneratedMetadataStore for AzuriteGeneratedMetadataStore {
    async fn save(&self, slug: &str, metadata: &GeneratedMetadata) -> Result<(), BlogError> {
        let body = serde_json::to_vec_pretty(metadata)
            .map_err(|error| BlogError::Parse(error.to_string()))?;
        let blob_name = format!("metadata/{slug}.json");
        self.blob
            .put_bytes(&blob_name, body, "application/json")
            .await
    }
}

#[derive(Debug, Clone, Serialize)]
struct ChatCompletionsRequest {
    messages: Vec<ChatMessage>,
    response_format: JsonResponseFormat,
}

#[derive(Debug, Clone, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Clone, Serialize)]
struct JsonResponseFormat {
    r#type: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ChatCompletionsResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Clone, Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Debug, Clone, Deserialize)]
struct ChatResponseMessage {
    content: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GeneratedMetadataPayload {
    #[serde(default)]
    summary_ai: Option<String>,
    #[serde(default)]
    suggested_tags: Vec<String>,
    #[serde(default)]
    intro_candidates: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blob::AzuriteBlobAdapter;
    use crate::config::AppConfig;

    #[test]
    fn build_prompt_mentions_requested_fields() {
        let prompt = AzureOpenAiMetadataGenerator::build_prompt(
            &AiAssistRequest {
                slug: "sample".to_owned(),
                title: "Sample".to_owned(),
                tags: vec!["rust".to_owned()],
                summary: "summary".to_owned(),
                body_markdown: "# Hello".to_owned(),
            },
            &AiGenerationScope::default(),
        );

        assert!(prompt.contains("summary: true"));
        assert!(prompt.contains("suggested_tags: true"));
        assert!(prompt.contains("intro_candidates: true"));
        assert!(prompt.contains("Slug: sample"));
    }

    #[tokio::test]
    async fn local_generated_metadata_store_writes_json_file() {
        let temp = tempfile::tempdir().unwrap();
        let store = LocalGeneratedMetadataStore::new(temp.path().to_path_buf());
        let metadata = GeneratedMetadata {
            summary_ai: Some("summary".to_owned()),
            suggested_tags: vec!["rust".to_owned()],
            intro_candidates: vec!["intro".to_owned()],
            generated_at: Utc::now(),
            source_model: Some("model".to_owned()),
        };

        store.save("sample", &metadata).await.unwrap();

        let raw = fs::read_to_string(temp.path().join("sample.json")).unwrap();
        assert!(raw.contains("\"summary_ai\": \"summary\""));
        assert!(raw.contains("\"suggested_tags\""));
        assert!(raw.contains("\"intro_candidates\""));
        assert!(raw.contains("\"source_model\": \"model\""));
    }

    #[tokio::test]
    async fn generated_metadata_store_factory_returns_local_store_for_local_backend() {
        let temp = tempfile::tempdir().unwrap();
        let config = AppConfig {
            app_env: "test".to_owned(),
            app_host: "127.0.0.1".to_owned(),
            app_port: 8080,
            storage_backend: "local".to_owned(),
            content_root: temp.path().join("content"),
            azurite_blob_endpoint: None,
            azurite_table_endpoint: None,
            azure_openai_endpoint: None,
            azure_openai_deployment: None,
            azure_openai_api_key: None,
            azure_openai_api_version: "2024-10-21".to_owned(),
            azure_openai_model_name: None,
            admin_auth_mode: "disabled".to_owned(),
            entra_tenant_id: None,
            entra_client_id: None,
            entra_oidc_metadata_url: None,
            entra_admin_group_id: None,
            entra_admin_user_oid: None,
            static_output_dir: "./dist".into(),
            static_publish_backend: "local".to_owned(),
            static_publish_prefix: "site".to_owned(),
            observability_backend: "noop".to_owned(),
            application_insights_connection_string: None,
            base_url: "http://127.0.0.1:8080".to_owned(),
        };

        let store = build_generated_metadata_store(&config);
        let metadata = GeneratedMetadata {
            summary_ai: Some("summary".to_owned()),
            suggested_tags: vec!["rust".to_owned()],
            intro_candidates: vec!["intro".to_owned()],
            generated_at: Utc::now(),
            source_model: Some("model".to_owned()),
        };

        store.save("sample", &metadata).await.unwrap();

        let raw = fs::read_to_string(config.metadata_dir().join("sample.json")).unwrap();
        assert!(raw.contains("\"summary_ai\": \"summary\""));
    }

    #[tokio::test]
    async fn generated_metadata_store_factory_can_write_via_azurite_backend() {
        let endpoint = "http://127.0.0.1:10000/devstoreaccount1";
        let require_azurite = std::env::var("RUN_AZURITE_TESTS").ok().as_deref() == Some("1");
        let config = AppConfig {
            app_env: "test".to_owned(),
            app_host: "127.0.0.1".to_owned(),
            app_port: 8080,
            storage_backend: "azurite".to_owned(),
            content_root: "./content".into(),
            azurite_blob_endpoint: Some(endpoint.to_owned()),
            azurite_table_endpoint: None,
            azure_openai_endpoint: None,
            azure_openai_deployment: None,
            azure_openai_api_key: None,
            azure_openai_api_version: "2024-10-21".to_owned(),
            azure_openai_model_name: None,
            admin_auth_mode: "disabled".to_owned(),
            entra_tenant_id: None,
            entra_client_id: None,
            entra_oidc_metadata_url: None,
            entra_admin_group_id: None,
            entra_admin_user_oid: None,
            static_output_dir: "./dist".into(),
            static_publish_backend: "local".to_owned(),
            static_publish_prefix: "site".to_owned(),
            observability_backend: "noop".to_owned(),
            application_insights_connection_string: None,
            base_url: "http://127.0.0.1:8080".to_owned(),
        };
        let store = build_generated_metadata_store(&config);
        let metadata = GeneratedMetadata {
            summary_ai: Some("summary".to_owned()),
            suggested_tags: vec!["rust".to_owned()],
            intro_candidates: vec!["intro".to_owned()],
            generated_at: Utc::now(),
            source_model: Some("model".to_owned()),
        };

        let save_result = store.save("adapter-azurite", &metadata).await;
        if !require_azurite && save_result.is_err() {
            return;
        }
        save_result.unwrap();

        let blob = AzuriteBlobAdapter::new(endpoint.to_owned());
        let raw = blob.get_text("metadata/adapter-azurite.json").await;
        if !require_azurite && raw.is_err() {
            return;
        }
        let raw = raw.unwrap().unwrap();
        assert!(raw.contains("\"summary_ai\": \"summary\""));
    }

    #[test]
    fn ai_generator_factory_returns_none_when_configuration_is_missing() {
        let config = AppConfig {
            app_env: "test".to_owned(),
            app_host: "127.0.0.1".to_owned(),
            app_port: 8080,
            storage_backend: "local".to_owned(),
            content_root: "./content".into(),
            azurite_blob_endpoint: None,
            azurite_table_endpoint: None,
            azure_openai_endpoint: None,
            azure_openai_deployment: None,
            azure_openai_api_key: None,
            azure_openai_api_version: "2024-10-21".to_owned(),
            azure_openai_model_name: None,
            admin_auth_mode: "disabled".to_owned(),
            entra_tenant_id: None,
            entra_client_id: None,
            entra_oidc_metadata_url: None,
            entra_admin_group_id: None,
            entra_admin_user_oid: None,
            static_output_dir: "./dist".into(),
            static_publish_backend: "local".to_owned(),
            static_publish_prefix: "site".to_owned(),
            observability_backend: "noop".to_owned(),
            application_insights_connection_string: None,
            base_url: "http://127.0.0.1:8080".to_owned(),
        };

        assert!(build_ai_metadata_generator(&config).is_none());
    }
}
