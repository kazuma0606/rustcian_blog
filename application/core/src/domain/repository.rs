use async_trait::async_trait;

use super::{
    error::BlogError,
    post::{Post, PostSummary, PostVisibility},
};

#[async_trait]
pub trait PostRepository: Send + Sync {
    async fn list_posts(&self, visibility: PostVisibility) -> Result<Vec<PostSummary>, BlogError>;
    async fn get_post(&self, slug: &str, visibility: PostVisibility) -> Result<Post, BlogError>;
}
