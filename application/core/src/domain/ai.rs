use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::BlogError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiAssistRequest {
    pub slug: String,
    pub title: String,
    pub tags: Vec<String>,
    pub summary: String,
    pub body_markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiGenerationScope {
    pub summary: bool,
    pub suggested_tags: bool,
    pub intro_candidates: bool,
}

impl Default for AiGenerationScope {
    fn default() -> Self {
        Self {
            summary: true,
            suggested_tags: true,
            intro_candidates: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GeneratedMetadata {
    #[serde(default)]
    pub summary_ai: Option<String>,
    #[serde(default)]
    pub suggested_tags: Vec<String>,
    #[serde(default)]
    pub intro_candidates: Vec<String>,
    pub generated_at: DateTime<Utc>,
    #[serde(default)]
    pub source_model: Option<String>,
}

#[async_trait::async_trait]
pub trait AiMetadataGenerator: Send + Sync {
    async fn generate_metadata(
        &self,
        request: AiAssistRequest,
        scope: AiGenerationScope,
    ) -> Result<GeneratedMetadata, BlogError>;
}

#[async_trait::async_trait]
pub trait GeneratedMetadataStore: Send + Sync {
    async fn save(&self, slug: &str, metadata: &GeneratedMetadata) -> Result<(), BlogError>;
}
