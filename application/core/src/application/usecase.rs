use std::sync::Arc;

use crate::domain::{
    ai::{
        AiAssistRequest, AiGenerationScope, AiMetadataGenerator, GeneratedMetadata,
        GeneratedMetadataStore,
    },
    error::BlogError,
    post::{Post, PostSummary, PostVisibility},
    repository::PostRepository,
    static_site::{StaticSiteGenerator, StaticSitePublisher},
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
        self.repository
            .list_posts(PostVisibility::PublishedOnly)
            .await
    }

    pub async fn execute_with_visibility(
        &self,
        visibility: PostVisibility,
    ) -> Result<Vec<PostSummary>, BlogError> {
        self.repository.list_posts(visibility).await
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
        self.repository
            .get_post(slug, PostVisibility::PublishedOnly)
            .await
    }

    pub async fn execute_with_visibility(
        &self,
        slug: &str,
        visibility: PostVisibility,
    ) -> Result<Post, BlogError> {
        self.repository.get_post(slug, visibility).await
    }
}

#[derive(Clone)]
pub struct GenerateAiMetadataUseCase {
    repository: Arc<dyn PostRepository>,
    generator: Arc<dyn AiMetadataGenerator>,
    metadata_store: Arc<dyn GeneratedMetadataStore>,
}

#[derive(Clone)]
pub struct GenerateStaticSiteUseCase {
    generator: Arc<dyn StaticSiteGenerator>,
}

#[derive(Clone)]
pub struct PublishStaticSiteUseCase {
    generator: Arc<dyn StaticSiteGenerator>,
    publisher: Arc<dyn StaticSitePublisher>,
}

impl GenerateStaticSiteUseCase {
    pub fn new(generator: Arc<dyn StaticSiteGenerator>) -> Self {
        Self { generator }
    }

    pub async fn execute(&self) -> Result<crate::StaticSiteBuild, BlogError> {
        self.generator.generate().await
    }
}

impl PublishStaticSiteUseCase {
    pub fn new(
        generator: Arc<dyn StaticSiteGenerator>,
        publisher: Arc<dyn StaticSitePublisher>,
    ) -> Self {
        Self {
            generator,
            publisher,
        }
    }

    pub async fn execute(&self) -> Result<crate::StaticSiteBuild, BlogError> {
        let build = self.generator.generate().await?;
        self.publisher.publish(&build).await?;
        Ok(build)
    }
}

impl GenerateAiMetadataUseCase {
    pub fn new(
        repository: Arc<dyn PostRepository>,
        generator: Arc<dyn AiMetadataGenerator>,
        metadata_store: Arc<dyn GeneratedMetadataStore>,
    ) -> Self {
        Self {
            repository,
            generator,
            metadata_store,
        }
    }

    pub async fn execute(
        &self,
        slug: &str,
        scope: AiGenerationScope,
    ) -> Result<GeneratedMetadata, BlogError> {
        let post = self
            .repository
            .get_post(slug, PostVisibility::IncludeDrafts)
            .await?;
        let request = AiAssistRequest {
            slug: post.slug.clone(),
            title: post.title,
            tags: post.tags,
            summary: post.summary,
            body_markdown: post.body_markdown,
        };
        let generated = self.generator.generate_metadata(request, scope).await?;
        self.metadata_store.save(slug, &generated).await?;
        Ok(generated)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use chrono::{DateTime, Utc};

    use super::*;
    use crate::{AiGenerationScope, GeneratedMetadata, PostMetadata, PostStatus, PostVisibility};
    use crate::{StaticAsset, StaticPage, StaticSiteBuild};

    struct MockRepository {
        posts: Vec<Post>,
    }

    struct MockGenerator;
    struct MockStaticSiteGenerator;
    #[derive(Default)]
    struct MockStaticSitePublisher {
        published: std::sync::Mutex<Vec<StaticSiteBuild>>,
    }

    #[async_trait::async_trait]
    impl AiMetadataGenerator for MockGenerator {
        async fn generate_metadata(
            &self,
            request: AiAssistRequest,
            _scope: AiGenerationScope,
        ) -> Result<GeneratedMetadata, BlogError> {
            Ok(GeneratedMetadata {
                summary_ai: Some(format!("AI summary for {}", request.slug)),
                suggested_tags: vec!["generated".to_owned()],
                intro_candidates: vec!["Generated intro".to_owned()],
                generated_at: Utc.with_ymd_and_hms(2026, 3, 20, 12, 0, 0).unwrap(),
                source_model: Some("test-model".to_owned()),
            })
        }
    }

    #[async_trait::async_trait]
    impl StaticSiteGenerator for MockStaticSiteGenerator {
        async fn generate(&self) -> Result<StaticSiteBuild, BlogError> {
            Ok(StaticSiteBuild {
                pages: vec![StaticPage {
                    path: "index.html".to_owned(),
                    content: "<html></html>".to_owned(),
                }],
                assets: vec![StaticAsset {
                    source_path: "content/images/sample.png".to_owned(),
                    output_path: "images/sample.png".to_owned(),
                }],
            })
        }
    }

    #[async_trait::async_trait]
    impl StaticSitePublisher for MockStaticSitePublisher {
        async fn publish(&self, build: &StaticSiteBuild) -> Result<(), BlogError> {
            self.published.lock().unwrap().push(build.clone());
            Ok(())
        }
    }

    #[derive(Default)]
    struct MockMetadataStore {
        saved: std::sync::Mutex<Vec<(String, GeneratedMetadata)>>,
    }

    #[async_trait::async_trait]
    impl GeneratedMetadataStore for MockMetadataStore {
        async fn save(&self, slug: &str, metadata: &GeneratedMetadata) -> Result<(), BlogError> {
            self.saved
                .lock()
                .unwrap()
                .push((slug.to_owned(), metadata.clone()));
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl PostRepository for MockRepository {
        async fn list_posts(
            &self,
            visibility: PostVisibility,
        ) -> Result<Vec<PostSummary>, BlogError> {
            Ok(self
                .posts
                .iter()
                .filter(|post| visibility.allows(post.status))
                .map(Post::summary)
                .collect())
        }

        async fn get_post(
            &self,
            slug: &str,
            visibility: PostVisibility,
        ) -> Result<Post, BlogError> {
            self.posts
                .iter()
                .find(|post| post.slug == slug && visibility.allows(post.status))
                .cloned()
                .ok_or_else(|| BlogError::NotFound(slug.to_owned()))
        }
    }

    fn sample_post(slug: &str) -> Post {
        let metadata = PostMetadata {
            title: "Sample".to_owned(),
            slug: slug.to_owned(),
            published_at: DateTime::parse_from_rfc3339("2026-03-19T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            updated_at: None,
            tags: vec!["rust".to_owned()],
            summary: "summary".to_owned(),
            description: None,
            hero_image: Some("/images/example.svg".to_owned()),
            status: PostStatus::Published,
            toc: false,
            math: false,
            charts: Vec::new(),
            summary_ai: None,
        };

        Post::new(
            metadata,
            Vec::new(),
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

    #[tokio::test]
    async fn get_post_with_include_drafts_can_return_draft() {
        let mut draft = sample_post("draft-post");
        draft.status = PostStatus::Draft;

        let use_case = GetPostUseCase::new(Arc::new(MockRepository { posts: vec![draft] }));

        let post = use_case
            .execute_with_visibility("draft-post", PostVisibility::IncludeDrafts)
            .await
            .unwrap();

        assert_eq!(post.slug, "draft-post");
        assert_eq!(post.status, PostStatus::Draft);
    }

    #[tokio::test]
    async fn generate_ai_metadata_reads_draft_and_saves_json_payload() {
        let mut draft = sample_post("draft-post");
        draft.status = PostStatus::Draft;
        let repository = Arc::new(MockRepository { posts: vec![draft] });
        let store = Arc::new(MockMetadataStore::default());
        let use_case =
            GenerateAiMetadataUseCase::new(repository, Arc::new(MockGenerator), store.clone());

        let generated = use_case
            .execute("draft-post", AiGenerationScope::default())
            .await
            .unwrap();

        assert_eq!(
            generated.summary_ai.as_deref(),
            Some("AI summary for draft-post")
        );
        let saved = store.saved.lock().unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].0, "draft-post");
        assert_eq!(saved[0].1.source_model.as_deref(), Some("test-model"));
    }

    #[tokio::test]
    async fn generate_static_site_returns_pages_and_assets() {
        let use_case = GenerateStaticSiteUseCase::new(Arc::new(MockStaticSiteGenerator));

        let build = use_case.execute().await.unwrap();

        assert_eq!(build.pages.len(), 1);
        assert_eq!(build.pages[0].path, "index.html");
        assert_eq!(build.assets.len(), 1);
        assert_eq!(build.assets[0].output_path, "images/sample.png");
    }

    #[tokio::test]
    async fn publish_static_site_generates_and_publishes_build() {
        let publisher = Arc::new(MockStaticSitePublisher::default());
        let use_case =
            PublishStaticSiteUseCase::new(Arc::new(MockStaticSiteGenerator), publisher.clone());

        let build = use_case.execute().await.unwrap();

        assert_eq!(build.pages.len(), 1);
        let published = publisher.published.lock().unwrap();
        assert_eq!(published.len(), 1);
        assert_eq!(published[0].pages[0].path, "index.html");
    }
}
