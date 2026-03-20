use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::error::BlogError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StaticPage {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StaticAsset {
    pub source_path: String,
    pub output_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct StaticSiteBuild {
    pub pages: Vec<StaticPage>,
    pub assets: Vec<StaticAsset>,
}

#[async_trait]
pub trait StaticSiteGenerator: Send + Sync {
    async fn generate(&self) -> Result<StaticSiteBuild, BlogError>;
}

#[async_trait]
pub trait AssetStore: Send + Sync {
    async fn list_global_assets(&self) -> Result<Vec<StaticAsset>, BlogError>;
    async fn list_post_assets(&self, slug: &str) -> Result<Vec<StaticAsset>, BlogError>;
}

#[async_trait]
pub trait StaticSitePublisher: Send + Sync {
    async fn publish(&self, build: &StaticSiteBuild) -> Result<(), BlogError>;
}
