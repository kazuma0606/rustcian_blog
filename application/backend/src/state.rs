use std::sync::Arc;

use rustacian_blog_core::{
    AdminAuthService, GenerateAiMetadataUseCase, GetPostUseCase, ListPostsUseCase,
    PublishStaticSiteUseCase,
};

use rustacian_blog_core::{CommentRepository, ContactRepository, NotificationSink};

use rustacian_blog_search::SearchEngine;

use crate::{
    ai::VisionAdapter, analytics_writer::AnalyticsWriter, blob::AzuriteBlobAdapter,
    cloudflare::CloudflareCacheClient, config::AppConfig, observability::ObservabilitySink,
    translator::AzureTranslatorAdapter,
};
use reqwest::Client;

#[derive(Clone)]
pub struct AppState {
    pub list_posts: ListPostsUseCase,
    pub get_post: GetPostUseCase,
    pub generate_ai_metadata: Option<GenerateAiMetadataUseCase>,
    pub publish_static_site: Option<PublishStaticSiteUseCase>,
    pub admin_auth: Arc<dyn AdminAuthService>,
    pub observability: Arc<dyn ObservabilitySink>,
    pub notification: Arc<dyn NotificationSink>,
    pub comment_repo: Arc<dyn CommentRepository>,
    pub contact_repo: Arc<dyn ContactRepository>,
    pub search_index: Arc<SearchEngine>,
    pub image_blob: Option<AzuriteBlobAdapter>,
    pub analytics: Option<Arc<AnalyticsWriter>>,
    pub cloudflare: Option<CloudflareCacheClient>,
    pub http_client: Client,
    pub vision: Option<VisionAdapter>,
    pub translator: Option<Arc<AzureTranslatorAdapter>>,
    pub config: AppConfig,
}
