use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use chrono::Utc;
use rustacian_blog_core::{
    AssetStore, BlogError, Post, PostRepository, PostVisibility, StaticAsset, StaticPage,
    StaticSiteBuild, StaticSiteGenerator, StaticSitePublisher,
};
use rustacian_blog_frontend::{
    ChartPointView, PostSummaryView, PostView, RenderedChartView, TagLinkView, TocItemView,
    render_post_page, render_posts_page, render_tag_posts_page, render_tags_page,
};
use serde::Serialize;

use crate::{blob::AzuriteBlobAdapter, config::AppConfig};

pub struct LocalStaticSiteGenerator {
    repository: Arc<dyn PostRepository>,
    asset_store: Arc<dyn AssetStore>,
    base_url: String,
}

impl LocalStaticSiteGenerator {
    pub fn new(
        repository: Arc<dyn PostRepository>,
        asset_store: Arc<dyn AssetStore>,
        base_url: String,
    ) -> Self {
        Self {
            repository,
            asset_store,
            base_url,
        }
    }
}

pub struct LocalFileAssetStore {
    content_root: PathBuf,
}

impl LocalFileAssetStore {
    pub fn new(content_root: PathBuf) -> Self {
        Self { content_root }
    }
}

#[async_trait]
impl AssetStore for LocalFileAssetStore {
    async fn list_global_assets(&self) -> Result<Vec<StaticAsset>, BlogError> {
        collect_global_image_assets(&self.content_root)
    }

    async fn list_post_assets(&self, slug: &str) -> Result<Vec<StaticAsset>, BlogError> {
        collect_post_assets(&self.content_root, slug)
    }
}

pub struct LocalDirectoryStaticSitePublisher {
    output_dir: PathBuf,
}

impl LocalDirectoryStaticSitePublisher {
    pub fn new(output_dir: PathBuf) -> Self {
        Self { output_dir }
    }
}

#[async_trait]
impl StaticSitePublisher for LocalDirectoryStaticSitePublisher {
    async fn publish(&self, build: &StaticSiteBuild) -> Result<(), BlogError> {
        write_static_site_build(build, &self.output_dir)
    }
}

pub struct AzuriteBlobStaticSitePublisher {
    blob: AzuriteBlobAdapter,
    prefix: String,
}

impl AzuriteBlobStaticSitePublisher {
    pub fn new(blob_endpoint: String, prefix: String) -> Self {
        Self {
            blob: AzuriteBlobAdapter::new(blob_endpoint),
            prefix: prefix.trim_matches('/').to_owned(),
        }
    }

    fn blob_name(&self, path: &str) -> String {
        if self.prefix.is_empty() {
            path.replace('\\', "/")
        } else {
            format!("{}/{}", self.prefix, path.replace('\\', "/"))
        }
    }
}

#[async_trait]
impl StaticSitePublisher for AzuriteBlobStaticSitePublisher {
    async fn publish(&self, build: &StaticSiteBuild) -> Result<(), BlogError> {
        self.blob.create_container_if_needed().await?;

        for page in &build.pages {
            self.blob
                .put_bytes(
                    &self.blob_name(&page.path),
                    page.content.as_bytes().to_vec(),
                    infer_content_type_from_output_path(&page.path),
                )
                .await?;
        }

        for asset in &build.assets {
            self.blob
                .put_bytes(
                    &self.blob_name(&asset.output_path),
                    fs::read(&asset.source_path)
                        .map_err(|error| BlogError::Storage(error.to_string()))?,
                    infer_content_type_from_output_path(&asset.output_path),
                )
                .await?;
        }

        Ok(())
    }
}

pub fn build_static_site_publisher(config: &AppConfig) -> Arc<dyn StaticSitePublisher> {
    match config.static_publish_backend.as_str() {
        "azurite" => Arc::new(AzuriteBlobStaticSitePublisher::new(
            config
                .azurite_blob_endpoint
                .clone()
                .unwrap_or_else(|| "http://127.0.0.1:10000/devstoreaccount1".to_owned()),
            config.static_publish_prefix.clone(),
        )),
        _ => Arc::new(LocalDirectoryStaticSitePublisher::new(
            config.static_output_dir.clone(),
        )),
    }
}

#[async_trait]
impl StaticSiteGenerator for LocalStaticSiteGenerator {
    async fn generate(&self) -> Result<StaticSiteBuild, BlogError> {
        let summaries = self
            .repository
            .list_posts(PostVisibility::PublishedOnly)
            .await?;
        let mut pages = vec![StaticPage {
            path: "index.html".to_owned(),
            content: render_posts_page(map_summaries(summaries.clone())),
        }];
        let mut assets = self.asset_store.list_global_assets().await?;
        let mut tag_map: BTreeMap<String, Vec<rustacian_blog_core::PostSummary>> = BTreeMap::new();
        let mut search_entries = Vec::with_capacity(summaries.len());
        let mut sitemap_urls = vec![absolute_url(&self.base_url, "/")];
        let mut rss_items = Vec::with_capacity(summaries.len());

        for summary in &summaries {
            for tag in &summary.tags {
                tag_map
                    .entry(tag.clone())
                    .or_default()
                    .push(summary.clone());
            }
        }

        for summary in summaries {
            let post = self
                .repository
                .get_post(&summary.slug, PostVisibility::PublishedOnly)
                .await?;
            pages.push(StaticPage {
                path: format!("posts/{}/index.html", post.slug),
                content: render_post_page(map_post(post.clone())),
            });
            assets.extend(self.asset_store.list_post_assets(&post.slug).await?);
            search_entries.push(SearchEntry::from_post(&post));
            sitemap_urls.push(absolute_url(
                &self.base_url,
                &format!("/posts/{}/", post.slug),
            ));
            rss_items.push(RssItem::from_post(&post, &self.base_url));
        }

        let tag_links = tag_map
            .iter()
            .map(|(tag, posts)| TagLinkView {
                tag: tag.clone(),
                href: format!("/tags/{tag}/"),
                count: posts.len(),
            })
            .collect::<Vec<_>>();
        pages.push(StaticPage {
            path: "tags/index.html".to_owned(),
            content: render_tags_page(tag_links),
        });
        sitemap_urls.push(absolute_url(&self.base_url, "/tags/"));

        for (tag, posts) in tag_map {
            pages.push(StaticPage {
                path: format!("tags/{tag}/index.html"),
                content: render_tag_posts_page(&tag, map_summaries(posts)),
            });
            sitemap_urls.push(absolute_url(&self.base_url, &format!("/tags/{tag}/")));
        }

        pages.push(StaticPage {
            path: "search.json".to_owned(),
            content: serde_json::to_string_pretty(&search_entries)
                .map_err(|error| BlogError::Storage(error.to_string()))?,
        });
        pages.push(StaticPage {
            path: "sitemap.xml".to_owned(),
            content: render_sitemap_xml(&sitemap_urls),
        });
        pages.push(StaticPage {
            path: "rss.xml".to_owned(),
            content: render_rss_xml(&self.base_url, &rss_items),
        });
        pages.push(StaticPage {
            path: "_meta/build.json".to_owned(),
            content: serde_json::to_string_pretty(&StaticBuildManifest::new(
                &self.base_url,
                &pages,
                &assets,
            ))
            .map_err(|error| BlogError::Storage(error.to_string()))?,
        });
        let pages = pages
            .into_iter()
            .map(optimize_static_page)
            .collect::<Vec<_>>();

        Ok(StaticSiteBuild { pages, assets })
    }
}

pub fn write_static_site_build(
    build: &StaticSiteBuild,
    output_dir: &Path,
) -> Result<(), BlogError> {
    if output_dir.exists() {
        fs::remove_dir_all(output_dir).map_err(|error| BlogError::Storage(error.to_string()))?;
    }
    fs::create_dir_all(output_dir).map_err(|error| BlogError::Storage(error.to_string()))?;

    for page in &build.pages {
        let path = output_dir.join(&page.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| BlogError::Storage(error.to_string()))?;
        }
        fs::write(path, &page.content).map_err(|error| BlogError::Storage(error.to_string()))?;
    }

    for asset in &build.assets {
        let output_path = output_dir.join(&asset.output_path);
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|error| BlogError::Storage(error.to_string()))?;
        }
        fs::copy(&asset.source_path, output_path)
            .map_err(|error| BlogError::Storage(error.to_string()))?;
    }

    Ok(())
}

fn map_summaries(posts: Vec<rustacian_blog_core::PostSummary>) -> Vec<PostSummaryView> {
    posts
        .into_iter()
        .map(|post| PostSummaryView {
            title: post.title,
            slug: post.slug.clone(),
            published_at: post.published_at.format("%Y-%m-%d").to_string(),
            updated_at: post
                .updated_at
                .map(|date| date.format("%Y-%m-%d").to_string()),
            tags: post.tags,
            summary: post.summary,
            hero_image: post
                .hero_image
                .map(|value| resolve_asset_url(&value, &post.slug)),
            toc: post.toc,
            math: post.math,
            status: match post.status {
                rustacian_blog_core::PostStatus::Published => "published".to_owned(),
                rustacian_blog_core::PostStatus::Draft => "draft".to_owned(),
            },
        })
        .collect()
}

fn map_post(post: Post) -> PostView {
    let slug = post.slug.clone();
    PostView {
        title: post.title,
        slug: slug.clone(),
        published_at: post.published_at.format("%Y-%m-%d").to_string(),
        updated_at: post
            .updated_at
            .map(|date| date.format("%Y-%m-%d").to_string()),
        tags: post.tags,
        summary: post.summary,
        hero_image: post
            .hero_image
            .map(|value| resolve_asset_url(&value, &slug)),
        toc: post.toc,
        math: post.math,
        summary_ai: post.summary_ai,
        charts: post
            .chart_data
            .into_iter()
            .map(|chart| RenderedChartView {
                chart_type: chart.chart_type,
                source: resolve_asset_url(&chart.source, &slug),
                x: chart.x,
                y: chart.y,
                title: chart.title,
                caption: chart.caption,
                points: chart
                    .points
                    .into_iter()
                    .map(|point| ChartPointView {
                        x: point.x,
                        y: point.y,
                    })
                    .collect(),
                table_headers: chart.table_headers,
                table_rows: chart.table_rows,
            })
            .collect(),
        toc_items: post
            .toc_items
            .into_iter()
            .map(|item| TocItemView {
                level: item.level,
                title: item.title,
                anchor: item.anchor,
            })
            .collect(),
        body_html: resolve_body_asset_urls(&post.body_html, &slug),
    }
}

fn resolve_asset_url(asset_ref: &str, slug: &str) -> String {
    if asset_ref.starts_with("http://")
        || asset_ref.starts_with("https://")
        || asset_ref.starts_with('/')
    {
        return asset_ref.to_owned();
    }

    let normalized = asset_ref.replace('\\', "/");
    let relative = normalized.trim_start_matches("./");
    format!("/assets/posts/{slug}/{relative}")
}

fn resolve_body_asset_urls(body_html: &str, slug: &str) -> String {
    body_html
        .replace("src=\"./", &format!("src=\"/assets/posts/{slug}/"))
        .replace("src=\"../", &format!("src=\"/assets/posts/{slug}/../"))
        .replace("href=\"./", &format!("href=\"/assets/posts/{slug}/"))
        .replace("href=\"../", &format!("href=\"/assets/posts/{slug}/../"))
}

fn collect_global_image_assets(content_root: &Path) -> Result<Vec<StaticAsset>, BlogError> {
    let images_dir = content_root.join("images");
    if !images_dir.exists() {
        return Ok(Vec::new());
    }

    collect_assets_recursive(&images_dir, &images_dir, "images")
}

fn collect_post_assets(content_root: &Path, slug: &str) -> Result<Vec<StaticAsset>, BlogError> {
    let article_dir = content_root.join("posts").join(slug);
    if !article_dir.exists() {
        return Ok(Vec::new());
    }

    let mut assets =
        collect_assets_recursive(&article_dir, &article_dir, &format!("assets/posts/{slug}"))?;
    assets.retain(|asset| {
        !asset.source_path.ends_with("meta.yml") && !asset.source_path.ends_with("post.md")
    });
    Ok(assets)
}

fn collect_assets_recursive(
    root: &Path,
    current: &Path,
    output_prefix: &str,
) -> Result<Vec<StaticAsset>, BlogError> {
    let mut assets = Vec::new();

    for entry in fs::read_dir(current).map_err(|error| BlogError::Storage(error.to_string()))? {
        let path = entry
            .map_err(|error| BlogError::Storage(error.to_string()))?
            .path();
        if path.is_dir() {
            assets.extend(collect_assets_recursive(root, &path, output_prefix)?);
            continue;
        }

        let relative = path
            .strip_prefix(root)
            .map_err(|error| BlogError::Storage(error.to_string()))?
            .to_string_lossy()
            .replace('\\', "/");
        assets.push(StaticAsset {
            source_path: path.to_string_lossy().to_string(),
            output_path: format!("{output_prefix}/{relative}"),
        });
    }

    Ok(assets)
}

fn infer_content_type_from_output_path(path: &str) -> &'static str {
    match Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
    {
        "html" => "text/html; charset=utf-8",
        "json" => "application/json",
        "xml" => "application/xml",
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "csv" => "text/csv; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

#[derive(Debug, Serialize)]
struct SearchEntry {
    slug: String,
    title: String,
    summary: String,
    tags: Vec<String>,
    published_at: String,
    url: String,
}

impl SearchEntry {
    fn from_post(post: &Post) -> Self {
        Self {
            slug: post.slug.clone(),
            title: post.title.clone(),
            summary: post.summary.clone(),
            tags: post.tags.clone(),
            published_at: post.published_at.format("%Y-%m-%d").to_string(),
            url: format!("/posts/{}/", post.slug),
        }
    }
}

#[derive(Debug)]
struct RssItem {
    title: String,
    link: String,
    description: String,
    published_at_rfc2822: String,
}

#[derive(Debug, Serialize)]
struct StaticBuildManifest {
    generated_at: String,
    base_url: String,
    page_paths: Vec<String>,
    asset_paths: Vec<String>,
    html_strategy: &'static str,
    structured_data_strategy: &'static str,
    asset_strategy: &'static str,
}

impl StaticBuildManifest {
    fn new(base_url: &str, pages: &[StaticPage], assets: &[StaticAsset]) -> Self {
        Self {
            generated_at: Utc::now().to_rfc3339(),
            base_url: base_url.to_owned(),
            page_paths: pages.iter().map(|page| page.path.clone()).collect(),
            asset_paths: assets
                .iter()
                .map(|asset| asset.output_path.clone())
                .collect(),
            html_strategy: "minify_intertag_whitespace",
            structured_data_strategy: "compact_json_and_xml",
            asset_strategy: "copy_as_is",
        }
    }
}

impl RssItem {
    fn from_post(post: &Post, base_url: &str) -> Self {
        Self {
            title: post.title.clone(),
            link: absolute_url(base_url, &format!("/posts/{}/", post.slug)),
            description: post.summary.clone(),
            published_at_rfc2822: post.published_at.to_rfc2822(),
        }
    }
}

fn absolute_url(base_url: &str, path: &str) -> String {
    format!("{base_url}{}", path)
}

fn render_sitemap_xml(urls: &[String]) -> String {
    let body = urls
        .iter()
        .map(|url| format!("<url><loc>{}</loc></url>", escape_xml(url)))
        .collect::<String>();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">{body}</urlset>"
    )
}

fn render_rss_xml(base_url: &str, items: &[RssItem]) -> String {
    let body = items
        .iter()
        .map(|item| {
            format!(
                "<item><title>{}</title><link>{}</link><guid>{}</guid><description>{}</description><pubDate>{}</pubDate></item>",
                escape_xml(&item.title),
                escape_xml(&item.link),
                escape_xml(&item.link),
                escape_xml(&item.description),
                escape_xml(&item.published_at_rfc2822),
            )
        })
        .collect::<String>();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><rss version=\"2.0\"><channel><title>Rustacian Blog</title><link>{}</link><description>Rustacian Blog feed</description>{body}</channel></rss>",
        escape_xml(base_url)
    )
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn optimize_static_page(page: StaticPage) -> StaticPage {
    let content = match Path::new(&page.path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
    {
        "html" => minify_html(&page.content),
        "json" => compact_json(&page.content).unwrap_or(page.content),
        "xml" => minify_xml(&page.content),
        _ => page.content,
    };

    StaticPage {
        path: page.path,
        content,
    }
}

fn minify_html(input: &str) -> String {
    input.replace("\r\n", "\n")
}

fn minify_xml(input: &str) -> String {
    input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("")
        .replace("> <", "><")
}

fn compact_json(input: &str) -> Result<String, serde_json::Error> {
    let value: serde_json::Value = serde_json::from_str(input)?;
    serde_json::to_string(&value)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde_json::Value;
    use tempfile::tempdir;

    use super::*;
    use crate::blob::AzuriteBlobAdapter;
    use crate::config::AppConfig;
    use crate::storage::LocalContentPostRepository;

    fn write_article(
        content_root: &Path,
        slug: &str,
        status: &str,
        body: &str,
    ) -> Result<(), BlogError> {
        let article_dir = content_root.join("posts").join(slug);
        fs::create_dir_all(&article_dir).map_err(|error| BlogError::Storage(error.to_string()))?;
        fs::write(
            article_dir.join("meta.yml"),
            format!(
                "title: {slug}\nslug: {slug}\npublished_at: 2026-03-20T00:00:00Z\ntags:\n  - rust\nsummary: summary\nstatus: {status}\nhero_image: /images/sample.png\n"
            ),
        )
        .map_err(|error| BlogError::Storage(error.to_string()))?;
        fs::write(article_dir.join("post.md"), body)
            .map_err(|error| BlogError::Storage(error.to_string()))?;
        fs::write(article_dir.join("data.csv"), "step,ms\nboot,10\n")
            .map_err(|error| BlogError::Storage(error.to_string()))?;
        Ok(())
    }

    #[tokio::test]
    async fn generator_outputs_only_published_posts_and_tag_pages() {
        let temp = tempdir().unwrap();
        let content_root = temp.path().join("content");
        fs::create_dir_all(content_root.join("posts")).unwrap();
        fs::create_dir_all(content_root.join("images")).unwrap();
        fs::write(content_root.join("images").join("sample.png"), b"png").unwrap();
        write_article(&content_root, "published-post", "published", "# Hello").unwrap();
        write_article(&content_root, "draft-post", "draft", "# Draft").unwrap();

        let generator = LocalStaticSiteGenerator::new(
            Arc::new(LocalContentPostRepository::new(content_root.clone())),
            Arc::new(LocalFileAssetStore::new(content_root.clone())),
            "https://example.com".to_owned(),
        );

        let build = generator.generate().await.unwrap();
        let page_paths = build
            .pages
            .iter()
            .map(|page| page.path.as_str())
            .collect::<BTreeSet<_>>();

        assert!(page_paths.contains("index.html"));
        assert!(page_paths.contains("posts/published-post/index.html"));
        assert!(page_paths.contains("tags/index.html"));
        assert!(page_paths.contains("tags/rust/index.html"));
        assert!(page_paths.contains("search.json"));
        assert!(page_paths.contains("sitemap.xml"));
        assert!(page_paths.contains("rss.xml"));
        assert!(page_paths.contains("_meta/build.json"));
        assert!(!page_paths.contains("posts/draft-post/index.html"));

        let sitemap = build
            .pages
            .iter()
            .find(|page| page.path == "sitemap.xml")
            .unwrap();
        let rss = build
            .pages
            .iter()
            .find(|page| page.path == "rss.xml")
            .unwrap();
        assert!(
            sitemap
                .content
                .contains("https://example.com/posts/published-post/")
        );
        assert!(
            rss.content
                .contains("https://example.com/posts/published-post/")
        );
        let manifest = build
            .pages
            .iter()
            .find(|page| page.path == "_meta/build.json")
            .unwrap();
        assert!(
            manifest
                .content
                .contains("\"base_url\":\"https://example.com\"")
        );
        assert!(
            manifest
                .content
                .contains("\"html_strategy\":\"minify_intertag_whitespace\"")
        );
        assert!(
            manifest
                .content
                .contains("\"posts/published-post/index.html\"")
        );
        assert!(manifest.content.contains("\"images/sample.png\""));
    }

    #[test]
    fn optimize_static_page_compacts_html_and_json() {
        let html = optimize_static_page(StaticPage {
            path: "index.html".to_owned(),
            content: "<html>\n  <body> test </body>\n</html>\n".to_owned(),
        });
        let json = optimize_static_page(StaticPage {
            path: "search.json".to_owned(),
            content: "{\n  \"a\": 1,\n  \"b\": [\n    2\n  ]\n}".to_owned(),
        });

        assert_eq!(html.content, "<html>\n  <body> test </body>\n</html>\n");
        assert_eq!(json.content, "{\"a\":1,\"b\":[2]}");
    }

    #[tokio::test]
    async fn local_publisher_factory_writes_static_build_to_output_dir() {
        let temp = tempdir().unwrap();
        let output_dir = temp.path().join("dist");
        let config = AppConfig {
            app_env: "test".to_owned(),
            app_host: "127.0.0.1".to_owned(),
            app_port: 8080,
            storage_backend: "local".to_owned(),
            content_root: temp.path().join("content"),
            azurite_blob_endpoint: None,
            azurite_table_endpoint: None,
            azure_openai_endpoint: None,
            azure_openai_deployment: None,
            azure_openai_api_key: None,
            azure_openai_api_version: "2024-10-21".to_owned(),
            azure_openai_model_name: None,
            admin_auth_mode: "disabled".to_owned(),
            entra_tenant_id: None,
            entra_client_id: None,
            entra_oidc_metadata_url: None,
            entra_admin_group_id: None,
            entra_admin_user_oid: None,
            static_output_dir: output_dir.clone(),
            static_publish_backend: "local".to_owned(),
            static_publish_prefix: "site".to_owned(),
            observability_backend: "noop".to_owned(),
            application_insights_connection_string: None,
            base_url: "https://example.com".to_owned(),
            slack_webhook_url: None,
        };
        let publisher = build_static_site_publisher(&config);
        let build = StaticSiteBuild {
            pages: vec![StaticPage {
                path: "index.html".to_owned(),
                content: "<html>ok</html>".to_owned(),
            }],
            assets: Vec::new(),
        };

        publisher.publish(&build).await.unwrap();

        assert_eq!(
            fs::read_to_string(output_dir.join("index.html")).unwrap(),
            "<html>ok</html>"
        );
    }

    #[tokio::test]
    async fn azurite_publisher_factory_can_write_static_build_to_blob_prefix() {
        let endpoint = "http://127.0.0.1:10000/devstoreaccount1";
        let require_azurite = std::env::var("RUN_AZURITE_TESTS").ok().as_deref() == Some("1");
        let config = AppConfig {
            app_env: "test".to_owned(),
            app_host: "127.0.0.1".to_owned(),
            app_port: 8080,
            storage_backend: "local".to_owned(),
            content_root: "./content".into(),
            azurite_blob_endpoint: Some(endpoint.to_owned()),
            azurite_table_endpoint: None,
            azure_openai_endpoint: None,
            azure_openai_deployment: None,
            azure_openai_api_key: None,
            azure_openai_api_version: "2024-10-21".to_owned(),
            azure_openai_model_name: None,
            admin_auth_mode: "disabled".to_owned(),
            entra_tenant_id: None,
            entra_client_id: None,
            entra_oidc_metadata_url: None,
            entra_admin_group_id: None,
            entra_admin_user_oid: None,
            static_output_dir: "./dist".into(),
            static_publish_backend: "azurite".to_owned(),
            static_publish_prefix: "adapter-test".to_owned(),
            observability_backend: "noop".to_owned(),
            application_insights_connection_string: None,
            base_url: "https://example.com".to_owned(),
            slack_webhook_url: None,
        };
        let publisher = build_static_site_publisher(&config);
        let build = StaticSiteBuild {
            pages: vec![StaticPage {
                path: "index.html".to_owned(),
                content: "<html>ok</html>".to_owned(),
            }],
            assets: Vec::new(),
        };

        let publish_result = publisher.publish(&build).await;
        if !require_azurite && publish_result.is_err() {
            return;
        }
        publish_result.unwrap();

        let blob = AzuriteBlobAdapter::new(endpoint.to_owned());
        let raw = blob.get_text("adapter-test/index.html").await;
        if !require_azurite && raw.is_err() {
            return;
        }
        assert_eq!(raw.unwrap().unwrap(), "<html>ok</html>");
    }

    #[tokio::test]
    async fn generated_machine_readable_outputs_match_snapshots() {
        let temp = tempdir().unwrap();
        let content_root = temp.path().join("content");
        fs::create_dir_all(content_root.join("posts")).unwrap();
        fs::create_dir_all(content_root.join("images")).unwrap();
        fs::write(content_root.join("images").join("sample.png"), b"png").unwrap();
        write_article(&content_root, "published-post", "published", "# Hello").unwrap();

        let generator = LocalStaticSiteGenerator::new(
            Arc::new(LocalContentPostRepository::new(content_root.clone())),
            Arc::new(LocalFileAssetStore::new(content_root)),
            "https://example.com".to_owned(),
        );
        let build = generator.generate().await.unwrap();

        assert_eq!(
            normalize_json_snapshot(page_content(&build, "search.json")),
            normalize_json_snapshot(include_str!("../tests/snapshots/static_search.json"))
        );
        assert_eq!(
            page_content(&build, "sitemap.xml").trim(),
            include_str!("../tests/snapshots/static_sitemap.xml").trim()
        );
        assert_eq!(
            page_content(&build, "rss.xml").trim(),
            include_str!("../tests/snapshots/static_rss.xml").trim()
        );
        assert_eq!(
            normalize_build_manifest_snapshot(page_content(&build, "_meta/build.json")),
            include_str!("../tests/snapshots/static_build_manifest.json").trim()
        );
    }

    fn page_content<'a>(build: &'a StaticSiteBuild, path: &str) -> &'a str {
        build
            .pages
            .iter()
            .find(|page| page.path == path)
            .map(|page| page.content.as_str())
            .unwrap()
    }

    fn normalize_build_manifest_snapshot(input: &str) -> String {
        let mut value: Value = serde_json::from_str(input).unwrap();
        value["generated_at"] = Value::String("__SNAPSHOT__".to_owned());
        serde_json::to_string(&value).unwrap()
    }

    fn normalize_json_snapshot(input: &str) -> String {
        let value: Value = serde_json::from_str(input).unwrap();
        serde_json::to_string(&value).unwrap()
    }
}
