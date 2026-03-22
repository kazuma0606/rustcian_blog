use std::sync::Arc;

use rustacian_blog_core::{
    AdminAuthService, GenerateAiMetadataUseCase, GetPostUseCase, ListPostsUseCase,
    PublishStaticSiteUseCase,
};

use rustacian_blog_core::{CommentRepository, ContactRepository, NotificationSink};

use crate::{
    blob::AzuriteBlobAdapter, config::AppConfig, observability::ObservabilitySink,
    search::TantivySearchIndex,
};

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
    pub search_index: Arc<TantivySearchIndex>,
    pub image_blob: Option<AzuriteBlobAdapter>,
    pub config: AppConfig,
}
