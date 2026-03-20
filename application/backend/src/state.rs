use std::sync::Arc;

use rustacian_blog_core::{
    AdminAuthService, GenerateAiMetadataUseCase, GetPostUseCase, ListPostsUseCase,
    PublishStaticSiteUseCase,
};

use crate::{blob::AzuriteBlobAdapter, config::AppConfig, observability::ObservabilitySink};

#[derive(Clone)]
pub struct AppState {
    pub list_posts: ListPostsUseCase,
    pub get_post: GetPostUseCase,
    pub generate_ai_metadata: Option<GenerateAiMetadataUseCase>,
    pub publish_static_site: Option<PublishStaticSiteUseCase>,
    pub admin_auth: Arc<dyn AdminAuthService>,
    pub observability: Arc<dyn ObservabilitySink>,
    pub image_blob: Option<AzuriteBlobAdapter>,
    pub config: AppConfig,
}
