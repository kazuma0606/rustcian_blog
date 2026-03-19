use async_trait::async_trait;

use super::{
    error::BlogError,
    post::{Post, PostSummary},
};

#[async_trait]
pub trait PostRepository: Send + Sync {
    async fn list_posts(&self) -> Result<Vec<PostSummary>, BlogError>;
    async fn get_post(&self, slug: &str) -> Result<Post, BlogError>;
}
