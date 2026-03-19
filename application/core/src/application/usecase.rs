use std::sync::Arc;

use crate::domain::{
    error::BlogError,
    post::{Post, PostSummary},
    repository::PostRepository,
};

#[derive(Clone)]
pub struct ListPostsUseCase {
    repository: Arc<dyn PostRepository>,
}

impl ListPostsUseCase {
    pub fn new(repository: Arc<dyn PostRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self) -> Result<Vec<PostSummary>, BlogError> {
        self.repository.list_posts().await
    }
}

#[derive(Clone)]
pub struct GetPostUseCase {
    repository: Arc<dyn PostRepository>,
}

impl GetPostUseCase {
    pub fn new(repository: Arc<dyn PostRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, slug: &str) -> Result<Post, BlogError> {
        self.repository.get_post(slug).await
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;
    use crate::PostFrontmatter;

    struct MockRepository {
        posts: Vec<Post>,
    }

    #[async_trait::async_trait]
    impl PostRepository for MockRepository {
        async fn list_posts(&self) -> Result<Vec<PostSummary>, BlogError> {
            Ok(self.posts.iter().map(Post::summary).collect())
        }

        async fn get_post(&self, slug: &str) -> Result<Post, BlogError> {
            self.posts
                .iter()
                .find(|post| post.slug == slug)
                .cloned()
                .ok_or_else(|| BlogError::NotFound(slug.to_owned()))
        }
    }

    fn sample_post(slug: &str) -> Post {
        let frontmatter = PostFrontmatter {
            title: "Sample".to_owned(),
            slug: slug.to_owned(),
            published_at: DateTime::parse_from_rfc3339("2026-03-19T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            tags: vec!["rust".to_owned()],
            summary: "summary".to_owned(),
            hero_image: Some("/images/example.svg".to_owned()),
        };

        Post::new(
            frontmatter,
            "# Hello".to_owned(),
            "<h1>Hello</h1>".to_owned(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn list_posts_returns_summaries() {
        let use_case = ListPostsUseCase::new(Arc::new(MockRepository {
            posts: vec![sample_post("sample-post")],
        }));

        let posts = use_case.execute().await.unwrap();

        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0].slug, "sample-post");
    }

    #[tokio::test]
    async fn get_post_returns_matching_post() {
        let use_case = GetPostUseCase::new(Arc::new(MockRepository {
            posts: vec![sample_post("target-post")],
        }));

        let post = use_case.execute("target-post").await.unwrap();

        assert_eq!(post.slug, "target-post");
    }
}
