use rustacian_blog_core::{GetPostUseCase, ListPostsUseCase};

use crate::config::AppConfig;

#[derive(Clone)]
pub struct AppState {
    pub list_posts: ListPostsUseCase,
    pub get_post: GetPostUseCase,
    pub config: AppConfig,
}
