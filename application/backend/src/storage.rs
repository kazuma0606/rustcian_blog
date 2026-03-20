use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use hmac::{Hmac, Mac};
use pulldown_cmark::{Options, Parser, html};
use reqwest::{
    Client, Method, StatusCode,
    header::{CONTENT_LENGTH, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue},
};
use rustacian_blog_core::{
    BlogError, ChartPoint, Post, PostMetadata, PostRepository, PostStatus, PostSummary,
    PostVisibility, RenderedChart, TocItem,
};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

const AZURITE_ACCOUNT: &str = "devstoreaccount1";
const AZURITE_ACCOUNT_KEY: &str =
    "Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw==";
const AZURITE_CONTAINER: &str = "blog-content";
const AZURITE_API_VERSION: &str = "2023-11-03";
const INDEX_BLOB_NAME: &str = "posts/index.json";
const INLINE_MATH_OPEN_TOKEN: &str = "@@MATH_INLINE_OPEN@@";
const INLINE_MATH_CLOSE_TOKEN: &str = "@@MATH_INLINE_CLOSE@@";

pub struct LocalContentPostRepository {
    content_root: PathBuf,
}

impl LocalContentPostRepository {
    pub fn new(content_root: PathBuf) -> Self {
        Self { content_root }
    }

    fn posts_dir(&self) -> PathBuf {
        self.content_root.join("posts")
    }

    fn read_all_posts(&self) -> Result<Vec<Post>, BlogError> {
        load_posts_from_dir(&self.content_root, &self.posts_dir())
    }
}

#[async_trait]
impl PostRepository for LocalContentPostRepository {
    async fn list_posts(&self, visibility: PostVisibility) -> Result<Vec<PostSummary>, BlogError> {
        self.read_all_posts().map(|posts| {
            posts
                .into_iter()
                .filter(|post| visibility.allows(post.status))
                .map(|post| post.summary())
                .collect()
        })
    }

    async fn get_post(&self, slug: &str, visibility: PostVisibility) -> Result<Post, BlogError> {
        self.read_all_posts()?
            .into_iter()
            .find(|post| post.slug == slug && visibility.allows(post.status))
            .ok_or_else(|| BlogError::NotFound(slug.to_owned()))
    }
}

pub struct AzuritePostRepository {
    client: Client,
    blob_endpoint: String,
}

impl AzuritePostRepository {
    pub fn new(blob_endpoint: String) -> Self {
        Self {
            client: Client::new(),
            blob_endpoint,
        }
    }

    async fn read_manifest(&self) -> Result<Vec<ManifestEntry>, BlogError> {
        let response = self
            .request_blob(Method::GET, INDEX_BLOB_NAME, None, None)
            .await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(Vec::new());
        }

        let body = response
            .text()
            .await
            .map_err(|error| BlogError::Storage(error.to_string()))?;

        serde_json::from_str::<Vec<ManifestEntry>>(&body)
            .map_err(|error| BlogError::Parse(error.to_string()))
    }

    async fn read_post_blobs(
        &self,
        meta_blob_name: &str,
        markdown_blob_name: &str,
    ) -> Result<Post, BlogError> {
        let meta_response = self
            .request_blob(Method::GET, meta_blob_name, None, None)
            .await?;
        if meta_response.status() == StatusCode::NOT_FOUND {
            return Err(BlogError::NotFound(meta_blob_name.to_owned()));
        }

        let markdown_response = self
            .request_blob(Method::GET, markdown_blob_name, None, None)
            .await?;
        if markdown_response.status() == StatusCode::NOT_FOUND {
            return Err(BlogError::NotFound(markdown_blob_name.to_owned()));
        }

        let meta_raw = meta_response
            .text()
            .await
            .map_err(|error| BlogError::Storage(error.to_string()))?;
        let markdown_raw = markdown_response
            .text()
            .await
            .map_err(|error| BlogError::Storage(error.to_string()))?;

        let metadata = merge_supplemental_metadata(
            parse_metadata(&meta_raw)?,
            read_optional_metadata_blob(
                self,
                &format!("metadata/{}.json", parse_metadata_slug(&meta_raw)?),
            )
            .await?,
        );
        let slug = metadata.slug.clone();

        build_blob_post(
            self,
            metadata,
            normalize_markdown_body(&markdown_raw),
            &slug,
        )
        .await
    }

    async fn request_blob(
        &self,
        method: Method,
        blob_name: &str,
        body: Option<Vec<u8>>,
        extra_query: Option<BTreeMap<String, String>>,
    ) -> Result<reqwest::Response, BlogError> {
        let path = format!("/{AZURITE_CONTAINER}/{blob_name}");
        self.request(method, &path, body, extra_query, false).await
    }

    async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<Vec<u8>>,
        extra_query: Option<BTreeMap<String, String>>,
        is_container_request: bool,
    ) -> Result<reqwest::Response, BlogError> {
        let mut url = format!("{}{}", self.blob_endpoint.trim_end_matches('/'), path);

        if let Some(query) = &extra_query {
            let query_string = query
                .iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<_>>()
                .join("&");
            url = format!("{url}?{query_string}");
        }

        let body_bytes = body.unwrap_or_default();
        let content_length = if body_bytes.is_empty() {
            String::new()
        } else {
            body_bytes.len().to_string()
        };
        let request_date = chrono::Utc::now()
            .format("%a, %d %b %Y %H:%M:%S GMT")
            .to_string();

        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("x-ms-date"),
            HeaderValue::from_str(&request_date)
                .map_err(|error| BlogError::Storage(error.to_string()))?,
        );
        headers.insert(
            HeaderName::from_static("x-ms-version"),
            HeaderValue::from_static(AZURITE_API_VERSION),
        );
        if method == Method::PUT && !is_container_request {
            headers.insert(
                HeaderName::from_static("x-ms-blob-type"),
                HeaderValue::from_static("BlockBlob"),
            );
        }
        if !content_length.is_empty() {
            headers.insert(
                CONTENT_LENGTH,
                HeaderValue::from_str(&content_length)
                    .map_err(|error| BlogError::Storage(error.to_string()))?,
            );
        }
        if method == Method::PUT && !body_bytes.is_empty() {
            headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/octet-stream"),
            );
        }

        let authorization = build_authorization_header(
            &method,
            endpoint_path(&self.blob_endpoint),
            path,
            &headers,
            extra_query.as_ref(),
            &content_length,
            headers
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or(""),
        )?;
        headers.insert(
            HeaderName::from_static("authorization"),
            HeaderValue::from_str(&authorization)
                .map_err(|error| BlogError::Storage(error.to_string()))?,
        );

        self.client
            .request(method, &url)
            .headers(headers)
            .body(body_bytes)
            .send()
            .await
            .map_err(|error| BlogError::Storage(error.to_string()))
    }

    async fn read_chart_blob(&self, slug: &str, source: &str) -> Result<String, BlogError> {
        let blob_name = resolve_asset_blob_name(source, slug)?;
        let response = self
            .request_blob(Method::GET, &blob_name, None, None)
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Err(BlogError::Validation(format!(
                "chart source does not exist in storage: {source}"
            )));
        }

        response
            .text()
            .await
            .map_err(|error| BlogError::Storage(error.to_string()))
    }
}

#[async_trait]
impl PostRepository for AzuritePostRepository {
    async fn list_posts(&self, visibility: PostVisibility) -> Result<Vec<PostSummary>, BlogError> {
        let manifest = self.read_manifest().await?;
        let mut posts = Vec::new();

        for item in manifest
            .into_iter()
            .filter(|item| visibility.allows(item.status))
        {
            posts.push(
                self.read_post_blobs(&item.meta_blob_name, &item.markdown_blob_name)
                    .await?,
            );
        }

        posts.sort_by(|left, right| right.published_at.cmp(&left.published_at));
        Ok(posts.into_iter().map(|post| post.summary()).collect())
    }

    async fn get_post(&self, slug: &str, visibility: PostVisibility) -> Result<Post, BlogError> {
        let manifest = self.read_manifest().await?;
        let entry = manifest
            .into_iter()
            .find(|item| item.slug == slug && visibility.allows(item.status))
            .ok_or_else(|| BlogError::NotFound(slug.to_owned()))?;

        self.read_post_blobs(&entry.meta_blob_name, &entry.markdown_blob_name)
            .await
    }
}

pub async fn seed_azurite_from_local(
    content_root: PathBuf,
    blob_endpoint: &str,
) -> Result<(), BlogError> {
    let client = AzuritePostRepository::new(blob_endpoint.to_owned());
    create_container_if_needed(&client).await?;

    let article_dirs = article_directories(&content_root.join("posts"))?;
    let mut manifest = Vec::with_capacity(article_dirs.len());

    for article_dir in article_dirs {
        manifest.push(seed_post_dir_blob(&client, article_dir).await?);
    }

    let manifest_body = serde_json::to_vec_pretty(&manifest)
        .map_err(|error| BlogError::Storage(error.to_string()))?;
    upload_blob(&client, INDEX_BLOB_NAME, manifest_body, "application/json").await?;

    Ok(())
}

async fn create_container_if_needed(client: &AzuritePostRepository) -> Result<(), BlogError> {
    let mut query = BTreeMap::new();
    query.insert("restype".to_owned(), "container".to_owned());

    let response = client
        .request(
            Method::PUT,
            &format!("/{AZURITE_CONTAINER}"),
            None,
            Some(query),
            true,
        )
        .await?;

    match response.status() {
        StatusCode::CREATED | StatusCode::CONFLICT => Ok(()),
        status => Err(BlogError::Storage(format!(
            "failed to create Azurite container: {status}"
        ))),
    }
}

async fn seed_post_dir_blob(
    client: &AzuritePostRepository,
    path: PathBuf,
) -> Result<ManifestEntry, BlogError> {
    let metadata = parse_metadata(
        &fs::read_to_string(path.join("meta.yml"))
            .map_err(|error| BlogError::Storage(error.to_string()))?,
    )?;
    let body_markdown = fs::read_to_string(path.join("post.md"))
        .map_err(|error| BlogError::Storage(error.to_string()))?;
    let article_dir_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| BlogError::Storage("article directory name is invalid".to_owned()))?;

    let meta_blob_name = format!("posts/{article_dir_name}/meta.yml");
    let markdown_blob_name = format!("posts/{article_dir_name}/post.md");

    upload_blob(
        client,
        &meta_blob_name,
        serde_yaml::to_string(&metadata)
            .map_err(|error| BlogError::Storage(error.to_string()))?
            .into_bytes(),
        "application/yaml",
    )
    .await?;
    upload_blob(
        client,
        &markdown_blob_name,
        body_markdown.into_bytes(),
        "text/markdown",
    )
    .await?;
    upload_article_assets(client, &path, article_dir_name).await?;

    let supplemental_path =
        path.parent()
            .and_then(|posts_dir| posts_dir.parent())
            .map(|content_root| {
                content_root
                    .join("metadata")
                    .join(format!("{}.json", metadata.slug))
            });
    if let Some(supplemental_path) = supplemental_path.filter(|path| path.exists()) {
        let supplemental_blob_name = format!("metadata/{}.json", metadata.slug);
        upload_blob(
            client,
            &supplemental_blob_name,
            fs::read(&supplemental_path).map_err(|error| BlogError::Storage(error.to_string()))?,
            "application/json",
        )
        .await?;
    }

    Ok(ManifestEntry {
        slug: metadata.slug,
        status: metadata.status,
        meta_blob_name,
        markdown_blob_name,
    })
}

async fn upload_article_assets(
    client: &AzuritePostRepository,
    article_dir: &Path,
    article_dir_name: &str,
) -> Result<(), BlogError> {
    for asset in collect_article_assets(article_dir, article_dir)? {
        let relative = asset
            .strip_prefix(article_dir)
            .map_err(|error| BlogError::Storage(error.to_string()))?;
        let blob_name = format!(
            "posts/{article_dir_name}/{}",
            relative.to_string_lossy().replace('\\', "/")
        );
        let content_type = infer_content_type(&asset);
        upload_blob(
            client,
            &blob_name,
            fs::read(&asset).map_err(|error| BlogError::Storage(error.to_string()))?,
            content_type,
        )
        .await?;
    }

    Ok(())
}

fn collect_article_assets(root: &Path, current: &Path) -> Result<Vec<PathBuf>, BlogError> {
    let mut assets = Vec::new();

    for entry in fs::read_dir(current).map_err(|error| BlogError::Storage(error.to_string()))? {
        let path = entry
            .map_err(|error| BlogError::Storage(error.to_string()))?
            .path();
        if path.is_dir() {
            assets.extend(collect_article_assets(root, &path)?);
            continue;
        }

        if path == root.join("meta.yml") || path == root.join("post.md") {
            continue;
        }

        assets.push(path);
    }

    Ok(assets)
}

fn infer_content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
    {
        "csv" => "text/csv",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

async fn upload_blob(
    client: &AzuritePostRepository,
    blob_name: &str,
    body: Vec<u8>,
    content_type: &str,
) -> Result<(), BlogError> {
    let path = format!("/{AZURITE_CONTAINER}/{blob_name}");
    let mut url = format!("{}{}", client.blob_endpoint.trim_end_matches('/'), path);
    url = url.replace('\\', "/");

    let content_length = body.len().to_string();
    let request_date = chrono::Utc::now()
        .format("%a, %d %b %Y %H:%M:%S GMT")
        .to_string();

    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("x-ms-date"),
        HeaderValue::from_str(&request_date)
            .map_err(|error| BlogError::Storage(error.to_string()))?,
    );
    headers.insert(
        HeaderName::from_static("x-ms-version"),
        HeaderValue::from_static(AZURITE_API_VERSION),
    );
    headers.insert(
        HeaderName::from_static("x-ms-blob-type"),
        HeaderValue::from_static("BlockBlob"),
    );
    headers.insert(
        CONTENT_LENGTH,
        HeaderValue::from_str(&content_length)
            .map_err(|error| BlogError::Storage(error.to_string()))?,
    );
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_str(content_type)
            .map_err(|error| BlogError::Storage(error.to_string()))?,
    );

    let authorization = build_authorization_header(
        &Method::PUT,
        endpoint_path(&client.blob_endpoint),
        &path,
        &headers,
        None,
        &content_length,
        content_type,
    )?;
    headers.insert(
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&authorization)
            .map_err(|error| BlogError::Storage(error.to_string()))?,
    );

    let response = client
        .client
        .put(url)
        .headers(headers)
        .body(body)
        .send()
        .await
        .map_err(|error| BlogError::Storage(error.to_string()))?;

    match response.status() {
        StatusCode::CREATED => Ok(()),
        status => Err(BlogError::Storage(format!(
            "failed to upload blob {blob_name}: {status}"
        ))),
    }
}

fn build_authorization_header(
    method: &Method,
    endpoint_path: &str,
    path: &str,
    headers: &HeaderMap,
    query: Option<&BTreeMap<String, String>>,
    content_length: &str,
    content_type: &str,
) -> Result<String, BlogError> {
    let mut canonical_headers = headers
        .iter()
        .filter_map(|(name, value)| {
            let name = name.as_str().to_ascii_lowercase();
            if name.starts_with("x-ms-") {
                Some(format!("{}:{}\n", name, value.to_str().ok()?))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    canonical_headers.sort();

    let canonicalized_resource = {
        let mut resource = format!("/{AZURITE_ACCOUNT}{endpoint_path}{path}");
        if let Some(query) = query {
            for (key, value) in query {
                resource.push('\n');
                resource.push_str(&format!("{}:{}", key.to_ascii_lowercase(), value));
            }
        }
        resource
    };

    let string_to_sign = format!(
        "{method}\n\n\n{content_length}\n\n{content_type}\n\n\n\n\n\n\n{}{canonicalized_resource}",
        canonical_headers.concat()
    );

    let key = STANDARD
        .decode(AZURITE_ACCOUNT_KEY)
        .map_err(|error| BlogError::Storage(error.to_string()))?;
    let mut mac = Hmac::<Sha256>::new_from_slice(&key)
        .map_err(|error| BlogError::Storage(error.to_string()))?;
    mac.update(string_to_sign.as_bytes());
    let signature = STANDARD.encode(mac.finalize().into_bytes());

    Ok(format!("SharedKey {AZURITE_ACCOUNT}:{signature}"))
}

fn endpoint_path(blob_endpoint: &str) -> &str {
    blob_endpoint
        .strip_prefix("http://127.0.0.1:10000")
        .or_else(|| blob_endpoint.strip_prefix("http://localhost:10000"))
        .unwrap_or("")
}

fn load_posts_from_dir(content_root: &Path, posts_dir: &Path) -> Result<Vec<Post>, BlogError> {
    let article_dirs = article_directories(posts_dir)?;
    let tag_dictionary = load_tag_dictionary(posts_dir)?;
    let mut seen_slugs = BTreeSet::new();
    let mut posts = Vec::with_capacity(article_dirs.len());

    for article_dir in article_dirs {
        let post = load_post_dir(content_root, &article_dir)?;
        validate_tag_membership(&post.tags, tag_dictionary.as_ref())?;
        if !seen_slugs.insert(post.slug.clone()) {
            return Err(BlogError::Validation(format!(
                "duplicate slug found: {}",
                post.slug
            )));
        }
        posts.push(post);
    }

    posts.sort_by(|left, right| right.published_at.cmp(&left.published_at));
    Ok(posts)
}

fn article_directories(posts_dir: &Path) -> Result<Vec<PathBuf>, BlogError> {
    let mut article_dirs = fs::read_dir(posts_dir)
        .map_err(|error| BlogError::Storage(error.to_string()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    article_dirs.sort();
    Ok(article_dirs)
}

fn load_tag_dictionary(posts_dir: &Path) -> Result<Option<BTreeSet<String>>, BlogError> {
    let content_root = posts_dir
        .parent()
        .ok_or_else(|| BlogError::Storage("failed to resolve content root".to_owned()))?;
    let tags_path = content_root.join("tags.yml");

    if !tags_path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&tags_path).map_err(|error| {
        BlogError::Storage(format!("failed to read {}: {error}", tags_path.display()))
    })?;
    let definitions: Vec<TagDefinition> =
        serde_yaml::from_str(&raw).map_err(|error| BlogError::Parse(error.to_string()))?;

    let tags = definitions
        .into_iter()
        .map(|item| item.id)
        .collect::<BTreeSet<_>>();

    Ok(Some(tags))
}

fn load_post_dir(content_root: &Path, path: &Path) -> Result<Post, BlogError> {
    let metadata_path = path.join("meta.yml");
    let markdown_path = path.join("post.md");

    let metadata = parse_metadata(&fs::read_to_string(&metadata_path).map_err(|error| {
        BlogError::Storage(format!(
            "failed to read {}: {error}",
            metadata_path.display()
        ))
    })?)?;
    let body_markdown =
        normalize_markdown_body(&fs::read_to_string(&markdown_path).map_err(|error| {
            BlogError::Storage(format!(
                "failed to read {}: {error}",
                markdown_path.display()
            ))
        })?);
    let metadata = merge_supplemental_metadata(
        metadata,
        load_optional_supplemental_metadata(content_root, &metadata_path)?,
    );
    validate_metadata_assets(&metadata, path)?;

    build_post(metadata, body_markdown, path)
}

fn build_post(
    mut metadata: PostMetadata,
    body_markdown: String,
    article_dir: &Path,
) -> Result<Post, BlogError> {
    metadata.math = metadata.math || markdown_contains_math(&body_markdown);
    let chart_data = load_chart_data_from_fs(article_dir, &metadata)?;
    let toc_items = if metadata.toc {
        extract_toc_items(&body_markdown)
    } else {
        Vec::new()
    };
    let body_html = render_markdown(&body_markdown, &toc_items, metadata.math);
    let mut post = Post::new(metadata, toc_items, body_markdown, body_html)?;
    post.chart_data = chart_data;
    Ok(post)
}

async fn build_blob_post(
    repository: &AzuritePostRepository,
    mut metadata: PostMetadata,
    body_markdown: String,
    slug: &str,
) -> Result<Post, BlogError> {
    metadata.math = metadata.math || markdown_contains_math(&body_markdown);
    let chart_data = load_chart_data_from_blob(repository, slug, &metadata).await?;
    let toc_items = if metadata.toc {
        extract_toc_items(&body_markdown)
    } else {
        Vec::new()
    };
    let body_html = render_markdown(&body_markdown, &toc_items, metadata.math);
    let mut post = Post::new(metadata, toc_items, body_markdown, body_html)?;
    post.chart_data = chart_data;
    Ok(post)
}

fn parse_metadata(raw: &str) -> Result<PostMetadata, BlogError> {
    serde_yaml::from_str(raw).map_err(|error| BlogError::Parse(error.to_string()))
}

fn parse_metadata_slug(raw: &str) -> Result<String, BlogError> {
    #[derive(Deserialize)]
    struct MetadataSlug {
        slug: String,
    }

    let metadata: MetadataSlug =
        serde_yaml::from_str(raw).map_err(|error| BlogError::Parse(error.to_string()))?;
    Ok(metadata.slug)
}

fn normalize_markdown_body(raw: &str) -> String {
    raw.replace("\r\n", "\n").trim().to_owned()
}

fn merge_supplemental_metadata(
    mut metadata: PostMetadata,
    supplemental: Option<SupplementalMetadata>,
) -> PostMetadata {
    if let Some(supplemental) = supplemental
        && metadata.summary_ai.is_none()
    {
        metadata.summary_ai = supplemental.summary_ai;
    }

    metadata
}

fn load_optional_supplemental_metadata(
    content_root: &Path,
    metadata_path: &Path,
) -> Result<Option<SupplementalMetadata>, BlogError> {
    let slug = metadata_path
        .parent()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .ok_or_else(|| BlogError::Storage("failed to resolve article slug".to_owned()))?;
    let supplemental_path = content_root.join("metadata").join(format!("{slug}.json"));

    if !supplemental_path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&supplemental_path).map_err(|error| {
        BlogError::Storage(format!(
            "failed to read {}: {error}",
            supplemental_path.display()
        ))
    })?;
    let supplemental: SupplementalMetadata =
        serde_json::from_str(&raw).map_err(|error| BlogError::Parse(error.to_string()))?;
    Ok(Some(supplemental))
}

async fn read_optional_metadata_blob(
    client: &AzuritePostRepository,
    blob_name: &str,
) -> Result<Option<SupplementalMetadata>, BlogError> {
    let response = client
        .request_blob(Method::GET, blob_name, None, None)
        .await?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }

    let raw = response
        .text()
        .await
        .map_err(|error| BlogError::Storage(error.to_string()))?;
    let supplemental: SupplementalMetadata =
        serde_json::from_str(&raw).map_err(|error| BlogError::Parse(error.to_string()))?;
    Ok(Some(supplemental))
}

fn load_chart_data_from_fs(
    article_dir: &Path,
    metadata: &PostMetadata,
) -> Result<Vec<RenderedChart>, BlogError> {
    metadata
        .charts
        .iter()
        .map(|chart| {
            let raw = fs::read_to_string(resolve_asset_path(&chart.source, article_dir)?)
                .map_err(|error| BlogError::Storage(error.to_string()))?;
            parse_chart_data(chart, &raw)
        })
        .collect()
}

async fn load_chart_data_from_blob(
    repository: &AzuritePostRepository,
    slug: &str,
    metadata: &PostMetadata,
) -> Result<Vec<RenderedChart>, BlogError> {
    let mut rendered = Vec::with_capacity(metadata.charts.len());

    for chart in &metadata.charts {
        let raw = repository.read_chart_blob(slug, &chart.source).await?;
        rendered.push(parse_chart_data(chart, &raw)?);
    }

    Ok(rendered)
}

fn parse_chart_data(
    chart: &rustacian_blog_core::ChartDefinition,
    raw_csv: &str,
) -> Result<RenderedChart, BlogError> {
    let rows = parse_csv_rows(raw_csv)?;
    let points = rows
        .into_iter()
        .map(|row| {
            let x = row.get(&chart.x).cloned().ok_or_else(|| {
                BlogError::Validation(format!(
                    "chart x column '{}' was not found in {}",
                    chart.x, chart.source
                ))
            })?;
            let y_raw = row.get(&chart.y).cloned().ok_or_else(|| {
                BlogError::Validation(format!(
                    "chart y column '{}' was not found in {}",
                    chart.y, chart.source
                ))
            })?;
            let y = y_raw.parse::<f64>().map_err(|_| {
                BlogError::Validation(format!(
                    "chart y value '{}' is not numeric in {}",
                    y_raw, chart.source
                ))
            })?;

            Ok(ChartPoint { x, y })
        })
        .collect::<Result<Vec<_>, BlogError>>()?;

    if points.is_empty() {
        return Err(BlogError::Validation(format!(
            "chart csv has no data rows: {}",
            chart.source
        )));
    }

    Ok(RenderedChart {
        chart_type: chart.r#type.clone(),
        source: chart.source.clone(),
        x: chart.x.clone(),
        y: chart.y.clone(),
        title: chart.title.clone(),
        caption: chart.caption.clone(),
        points,
    })
}

fn parse_csv_rows(raw_csv: &str) -> Result<Vec<BTreeMap<String, String>>, BlogError> {
    let mut lines = raw_csv.lines().filter(|line| !line.trim().is_empty());
    let headers = lines
        .next()
        .ok_or_else(|| BlogError::Validation("chart csv is empty".to_owned()))
        .and_then(parse_csv_record)?;

    if headers.is_empty() {
        return Err(BlogError::Validation(
            "chart csv header is empty".to_owned(),
        ));
    }

    let mut rows = Vec::new();
    for line in lines {
        let values = parse_csv_record(line)?;
        if values.len() != headers.len() {
            return Err(BlogError::Validation(format!(
                "chart csv row has {} columns but expected {}",
                values.len(),
                headers.len()
            )));
        }

        let row = headers
            .iter()
            .cloned()
            .zip(values.into_iter())
            .collect::<BTreeMap<_, _>>();
        rows.push(row);
    }

    Ok(rows)
}

fn parse_csv_record(line: &str) -> Result<Vec<String>, BlogError> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                values.push(current.trim().to_owned());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if in_quotes {
        return Err(BlogError::Parse("unterminated csv quoted field".to_owned()));
    }

    values.push(current.trim().to_owned());
    Ok(values)
}

fn validate_metadata_assets(metadata: &PostMetadata, article_dir: &Path) -> Result<(), BlogError> {
    if let Some(hero_image) = &metadata.hero_image {
        let asset_path = resolve_asset_path(hero_image, article_dir)?;
        if !asset_path.exists() {
            return Err(BlogError::Validation(format!(
                "hero_image does not exist: {}",
                hero_image
            )));
        }
    }

    for chart in &metadata.charts {
        let asset_path = resolve_asset_path(&chart.source, article_dir)?;
        if !asset_path.exists() {
            return Err(BlogError::Validation(format!(
                "chart source does not exist: {}",
                chart.source
            )));
        }
    }

    Ok(())
}

fn validate_tag_membership(
    tags: &[String],
    tag_dictionary: Option<&BTreeSet<String>>,
) -> Result<(), BlogError> {
    let Some(dictionary) = tag_dictionary else {
        return Ok(());
    };

    for tag in tags {
        if !dictionary.contains(tag) {
            return Err(BlogError::Validation(format!(
                "tag is not defined in content/tags.yml: {}",
                tag
            )));
        }
    }

    Ok(())
}

fn resolve_asset_path(asset_ref: &str, article_dir: &Path) -> Result<PathBuf, BlogError> {
    if asset_ref.starts_with("/images/") {
        let content_root = article_dir
            .parent()
            .and_then(|posts_dir| posts_dir.parent())
            .ok_or_else(|| BlogError::Storage("failed to resolve content root".to_owned()))?;
        let relative = asset_ref.trim_start_matches('/');
        return Ok(content_root.join(relative));
    }

    if asset_ref.starts_with("./") || asset_ref.starts_with("../") {
        return Ok(article_dir.join(asset_ref));
    }

    Ok(article_dir.join(asset_ref))
}

fn resolve_asset_blob_name(asset_ref: &str, slug: &str) -> Result<String, BlogError> {
    let normalized = asset_ref.replace('\\', "/");
    if normalized.starts_with("/images/") {
        return Ok(normalized.trim_start_matches('/').to_owned());
    }

    let relative = normalized
        .strip_prefix("./")
        .unwrap_or(&normalized)
        .trim_start_matches('/');
    if relative.is_empty() {
        return Err(BlogError::Validation(format!(
            "asset reference is empty: {asset_ref}"
        )));
    }

    Ok(format!("posts/{slug}/{relative}"))
}

fn render_markdown(markdown: &str, toc_items: &[TocItem], enable_math: bool) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);

    let prepared = if enable_math {
        preprocess_markdown_math(markdown)
    } else {
        markdown.to_owned()
    };

    let parser = Parser::new_ext(&prepared, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    let html_output = attach_heading_ids(html_output, toc_items);
    if enable_math {
        finalize_math_placeholders(html_output)
    } else {
        html_output
    }
}

fn finalize_math_placeholders(html_output: String) -> String {
    html_output
        .replace(INLINE_MATH_OPEN_TOKEN, "\\(")
        .replace(INLINE_MATH_CLOSE_TOKEN, "\\)")
}

fn markdown_contains_math(markdown: &str) -> bool {
    if markdown.contains("$$") || markdown.contains("\\(") || markdown.contains("\\[") {
        return true;
    }

    let chars = markdown.chars().collect::<Vec<_>>();
    let mut index = 0;

    while index < chars.len() {
        if chars[index] == '$'
            && (index == 0 || chars[index - 1] != '\\')
            && index + 1 < chars.len()
            && chars[index + 1] != '$'
        {
            let mut end = index + 1;
            while end < chars.len() {
                if chars[end] == '$' && chars[end - 1] != '\\' {
                    return true;
                }
                end += 1;
            }
        }

        index += 1;
    }

    false
}

fn preprocess_markdown_math(markdown: &str) -> String {
    let with_block_math = wrap_block_math(markdown);
    wrap_inline_math(&with_block_math)
}

fn wrap_block_math(markdown: &str) -> String {
    let mut result = String::new();
    let mut lines = markdown.lines().peekable();
    let mut in_block_math = false;
    let mut block_lines = Vec::new();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed == "$$" {
            if in_block_math {
                let content = block_lines.join("\n");
                result.push_str("<div class=\"math-display\">\\[");
                result.push_str(&content);
                result.push_str("\\]</div>\n");
                block_lines.clear();
                in_block_math = false;
            } else {
                in_block_math = true;
            }
            continue;
        }

        if in_block_math {
            block_lines.push(line.to_owned());
        } else {
            result.push_str(line);
            if lines.peek().is_some() {
                result.push('\n');
            }
        }
    }

    if in_block_math {
        result.push_str("$$\n");
        result.push_str(&block_lines.join("\n"));
    }

    result
}

fn wrap_inline_math(markdown: &str) -> String {
    let mut result = String::new();
    let chars = markdown.chars().collect::<Vec<_>>();
    let mut index = 0;

    while index < chars.len() {
        if chars[index] == '$'
            && (index == 0 || chars[index - 1] != '\\')
            && index + 1 < chars.len()
            && chars[index + 1] != '$'
        {
            let mut end = index + 1;
            while end < chars.len() {
                if chars[end] == '$' && chars[end - 1] != '\\' {
                    break;
                }
                end += 1;
            }

            if end < chars.len() {
                let content = chars[index + 1..end].iter().collect::<String>();
                result.push_str("<span class=\"math-inline\">");
                result.push_str(INLINE_MATH_OPEN_TOKEN);
                result.push_str(&content);
                result.push_str(INLINE_MATH_CLOSE_TOKEN);
                result.push_str("</span>");
                index = end + 1;
                continue;
            }
        }

        result.push(chars[index]);
        index += 1;
    }

    result
}

fn extract_toc_items(markdown: &str) -> Vec<TocItem> {
    let mut toc_items = Vec::new();
    let mut seen_anchors = BTreeSet::new();

    for line in markdown.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('#') {
            continue;
        }

        let level = trimmed.chars().take_while(|ch| *ch == '#').count();
        if !(1..=3).contains(&level) {
            continue;
        }

        let title = trimmed[level..].trim();
        if title.is_empty() {
            continue;
        }

        let anchor = unique_anchor(slugify_heading(title), &mut seen_anchors);
        toc_items.push(TocItem {
            level: level as u8,
            title: title.to_owned(),
            anchor,
        });
    }

    toc_items
}

fn attach_heading_ids(mut html_output: String, toc_items: &[TocItem]) -> String {
    let mut search_from = 0;

    for item in toc_items {
        let open_tag = format!("<h{}>", item.level);
        let replacement = format!("<h{} id=\"{}\">", item.level, item.anchor);

        if let Some(relative_index) = html_output[search_from..].find(&open_tag) {
            let index = search_from + relative_index;
            html_output.replace_range(index..index + open_tag.len(), &replacement);
            search_from = index + replacement.len();
        }
    }

    html_output
}

fn slugify_heading(title: &str) -> String {
    let mut anchor = String::new();
    let mut prev_dash = false;

    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            anchor.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !anchor.is_empty() {
            anchor.push('-');
            prev_dash = true;
        }
    }

    anchor.trim_matches('-').to_owned()
}

fn unique_anchor(mut anchor: String, seen_anchors: &mut BTreeSet<String>) -> String {
    if anchor.is_empty() {
        anchor = "section".to_owned();
    }

    if seen_anchors.insert(anchor.clone()) {
        return anchor;
    }

    let base = anchor.clone();
    let mut index = 2;
    loop {
        let candidate = format!("{base}-{index}");
        if seen_anchors.insert(candidate.clone()) {
            return candidate;
        }
        index += 1;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManifestEntry {
    slug: String,
    status: PostStatus,
    meta_blob_name: String,
    markdown_blob_name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct TagDefinition {
    id: String,
    #[allow(dead_code)]
    name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SupplementalMetadata {
    #[serde(default)]
    summary_ai: Option<String>,
    #[serde(default)]
    suggested_tags: Vec<String>,
    #[serde(default)]
    intro_candidates: Vec<String>,
    #[serde(default)]
    source_model: Option<String>,
    #[serde(default)]
    generated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{
        AzuritePostRepository, LocalContentPostRepository, attach_heading_ids, build_post,
        extract_toc_items, load_post_dir, load_tag_dictionary, markdown_contains_math,
        parse_csv_rows, parse_metadata, render_markdown, resolve_asset_path,
        seed_azurite_from_local,
    };
    use rustacian_blog_core::{PostMetadata, PostRepository, PostStatus, PostVisibility, TocItem};
    use tempfile::tempdir;

    fn write_article(
        root: &std::path::Path,
        dir_name: &str,
        status: &str,
        body: &str,
    ) -> std::path::PathBuf {
        let article_dir = root.join(dir_name);
        fs::create_dir_all(&article_dir).unwrap();
        fs::write(
            article_dir.join("meta.yml"),
            format!(
                "title: Sample\nslug: {dir_name}\npublished_at: 2026-03-19T00:00:00Z\ntags:\n  - rust\nsummary: hello\nstatus: {status}\n"
            ),
        )
        .unwrap();
        fs::write(article_dir.join("post.md"), body).unwrap();
        article_dir
    }

    #[test]
    fn metadata_is_parsed_from_yaml() {
        let raw = r#"title: Sample
slug: sample
published_at: 2026-03-19T00:00:00Z
tags:
  - rust
summary: hello
status: draft
"#;

        let metadata = parse_metadata(raw).unwrap();

        assert_eq!(metadata.slug, "sample");
        assert_eq!(metadata.status, PostStatus::Draft);
    }

    #[test]
    fn load_post_dir_renders_html() {
        let temp = tempdir().unwrap();
        let content_root = temp.path();
        let article_dir = write_article(content_root, "sample", "published", "# Hello");

        let post = load_post_dir(content_root, &article_dir).unwrap();

        assert!(post.body_html.contains("<h1>"));
    }

    #[test]
    fn toc_items_are_extracted_from_markdown_headings() {
        let toc_items =
            extract_toc_items("# Title\n\n## First Section\n\n### Deep Dive\n\n## First Section\n");

        assert_eq!(toc_items.len(), 4);
        assert_eq!(toc_items[1].anchor, "first-section");
        assert_eq!(toc_items[3].anchor, "first-section-2");
    }

    #[test]
    fn heading_ids_are_attached_to_rendered_html() {
        let html = attach_heading_ids(
            "<h1>Title</h1><p>x</p><h2>Section</h2>".to_owned(),
            &[
                TocItem {
                    level: 1,
                    title: "Title".to_owned(),
                    anchor: "title".to_owned(),
                },
                TocItem {
                    level: 2,
                    title: "Section".to_owned(),
                    anchor: "section".to_owned(),
                },
            ],
        );

        assert!(html.contains("<h1 id=\"title\">"));
        assert!(html.contains("<h2 id=\"section\">"));
    }

    #[test]
    fn render_markdown_preserves_inline_math_markers() {
        let html = render_markdown("inline math $e^{i\\pi} + 1 = 0$", &[], true);

        assert!(html.contains("math-inline"));
        assert!(html.contains("\\("));
        assert!(html.contains("\\)"));
        assert!(html.contains("e^{i\\pi} + 1 = 0"));
    }

    #[test]
    fn render_markdown_preserves_block_math_markers() {
        let html = render_markdown("$$\n\\int_0^1 x^2 \\, dx = \\frac{1}{3}\n$$", &[], true);

        assert!(html.contains("<div class=\"math-display\">\\["));
        assert!(html.contains("\\int_0^1 x^2"));
    }

    #[test]
    fn detects_inline_math_usage_from_markdown() {
        assert!(markdown_contains_math("Euler: $e^{i\\pi} + 1 = 0$"));
    }

    #[test]
    fn ignores_escaped_dollar_without_math_pair() {
        assert!(!markdown_contains_math("price is \\$9.99 today"));
    }

    #[test]
    fn build_post_enables_math_when_markdown_contains_formula() {
        let temp = tempdir().unwrap();
        let article_dir = temp.path().join("math-sample");
        fs::create_dir_all(&article_dir).unwrap();
        let metadata = PostMetadata {
            title: "Math Sample".to_owned(),
            slug: "math-sample".to_owned(),
            published_at: chrono::DateTime::parse_from_rfc3339("2026-03-19T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: None,
            tags: vec!["rust".to_owned()],
            summary: "summary".to_owned(),
            hero_image: None,
            status: PostStatus::Published,
            toc: false,
            math: false,
            charts: Vec::new(),
            summary_ai: None,
        };

        let post =
            build_post(metadata, "Inline math $x^2 + y^2$".to_owned(), &article_dir).unwrap();

        assert!(post.math);
        assert!(post.body_html.contains("math-inline"));
    }

    #[test]
    fn parses_chart_csv_rows() {
        let rows = parse_csv_rows("step,ms\nbootstrap,38\napi,24\n").unwrap();

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].get("step"), Some(&"bootstrap".to_owned()));
        assert_eq!(rows[1].get("ms"), Some(&"24".to_owned()));
    }

    #[test]
    fn resolves_global_image_asset_path() {
        let temp = tempdir().unwrap();
        let content_root = temp.path();
        let article_dir = content_root.join("posts").join("sample");
        fs::create_dir_all(content_root.join("images")).unwrap();
        fs::create_dir_all(&article_dir).unwrap();

        let resolved = resolve_asset_path("/images/hero.svg", &article_dir).unwrap();

        assert_eq!(resolved, content_root.join("images").join("hero.svg"));
    }

    #[test]
    fn load_post_dir_rejects_missing_hero_image() {
        let temp = tempdir().unwrap();
        let content_root = temp.path();
        let article_dir = temp.path().join("sample");
        fs::create_dir_all(&article_dir).unwrap();
        fs::write(
            article_dir.join("meta.yml"),
            "title: Sample\nslug: sample\npublished_at: 2026-03-19T00:00:00Z\ntags:\n  - rust\nsummary: hello\nstatus: published\nhero_image: ./missing.svg\n",
        )
        .unwrap();
        fs::write(article_dir.join("post.md"), "# Hello").unwrap();

        let error = load_post_dir(content_root, &article_dir).unwrap_err();

        assert!(matches!(
            error,
            rustacian_blog_core::BlogError::Validation(_)
        ));
    }

    #[test]
    fn resolves_relative_chart_asset_path() {
        let temp = tempdir().unwrap();
        let article_dir = temp.path().join("sample");
        fs::create_dir_all(&article_dir).unwrap();

        let resolved = resolve_asset_path("./data.csv", &article_dir).unwrap();

        assert_eq!(resolved, article_dir.join("./data.csv"));
    }

    #[test]
    fn load_post_dir_rejects_missing_chart_source() {
        let temp = tempdir().unwrap();
        let content_root = temp.path();
        let article_dir = temp.path().join("sample");
        fs::create_dir_all(&article_dir).unwrap();
        fs::write(
            article_dir.join("meta.yml"),
            "title: Sample\nslug: sample\npublished_at: 2026-03-19T00:00:00Z\ntags:\n  - rust\nsummary: hello\nstatus: published\ncharts:\n  - type: line\n    source: ./missing.csv\n    x: date\n    y: value\n",
        )
        .unwrap();
        fs::write(article_dir.join("post.md"), "# Hello").unwrap();

        let error = load_post_dir(content_root, &article_dir).unwrap_err();

        assert!(matches!(
            error,
            rustacian_blog_core::BlogError::Validation(_)
        ));
    }

    #[test]
    fn load_post_dir_merges_optional_supplemental_metadata() {
        let temp = tempdir().unwrap();
        let content_root = temp.path();
        let posts_dir = content_root.join("posts");
        let metadata_dir = content_root.join("metadata");
        fs::create_dir_all(&posts_dir).unwrap();
        fs::create_dir_all(&metadata_dir).unwrap();
        let article_dir = write_article(&posts_dir, "sample", "published", "# Hello");
        fs::write(
            metadata_dir.join("sample.json"),
            r#"{"summary_ai":"supplemental summary"}"#,
        )
        .unwrap();

        let post = load_post_dir(content_root, &article_dir).unwrap();

        assert_eq!(post.summary_ai.as_deref(), Some("supplemental summary"));
    }

    #[test]
    fn load_post_dir_loads_chart_data_from_csv() {
        let temp = tempdir().unwrap();
        let content_root = temp.path();
        let article_dir = temp.path().join("sample");
        fs::create_dir_all(&article_dir).unwrap();
        fs::write(
            article_dir.join("meta.yml"),
            "title: Sample\nslug: sample\npublished_at: 2026-03-19T00:00:00Z\ntags:\n  - rust\nsummary: hello\nstatus: published\ncharts:\n  - type: line\n    source: ./metrics.csv\n    x: step\n    y: ms\n",
        )
        .unwrap();
        fs::write(article_dir.join("post.md"), "# Hello").unwrap();
        fs::write(
            article_dir.join("metrics.csv"),
            "step,ms\nbootstrap,38\napi,24\n",
        )
        .unwrap();

        let post = load_post_dir(content_root, &article_dir).unwrap();

        assert_eq!(post.chart_data.len(), 1);
        assert_eq!(post.chart_data[0].points.len(), 2);
        assert_eq!(post.chart_data[0].points[0].x, "bootstrap");
        assert_eq!(post.chart_data[0].points[1].y, 24.0);
    }

    #[test]
    fn load_post_dir_rejects_chart_csv_without_rows() {
        let temp = tempdir().unwrap();
        let content_root = temp.path();
        let article_dir = temp.path().join("sample");
        fs::create_dir_all(&article_dir).unwrap();
        fs::write(
            article_dir.join("meta.yml"),
            "title: Sample\nslug: sample\npublished_at: 2026-03-19T00:00:00Z\ntags:\n  - rust\nsummary: hello\nstatus: published\ncharts:\n  - type: line\n    source: ./metrics.csv\n    x: step\n    y: ms\n",
        )
        .unwrap();
        fs::write(article_dir.join("post.md"), "# Hello").unwrap();
        fs::write(article_dir.join("metrics.csv"), "step,ms\n").unwrap();

        let error = load_post_dir(content_root, &article_dir).unwrap_err();

        assert!(matches!(
            error,
            rustacian_blog_core::BlogError::Validation(_)
        ));
    }

    #[test]
    fn load_post_dir_rejects_chart_csv_without_required_column() {
        let temp = tempdir().unwrap();
        let content_root = temp.path();
        let article_dir = temp.path().join("sample");
        fs::create_dir_all(&article_dir).unwrap();
        fs::write(
            article_dir.join("meta.yml"),
            "title: Sample\nslug: sample\npublished_at: 2026-03-19T00:00:00Z\ntags:\n  - rust\nsummary: hello\nstatus: published\ncharts:\n  - type: line\n    source: ./metrics.csv\n    x: step\n    y: ms\n",
        )
        .unwrap();
        fs::write(article_dir.join("post.md"), "# Hello").unwrap();
        fs::write(
            article_dir.join("metrics.csv"),
            "step,value\nbootstrap,38\n",
        )
        .unwrap();

        let error = load_post_dir(content_root, &article_dir).unwrap_err();

        assert!(matches!(
            error,
            rustacian_blog_core::BlogError::Validation(_)
        ));
    }

    #[test]
    fn load_post_dir_rejects_chart_csv_with_non_numeric_y() {
        let temp = tempdir().unwrap();
        let content_root = temp.path();
        let article_dir = temp.path().join("sample");
        fs::create_dir_all(&article_dir).unwrap();
        fs::write(
            article_dir.join("meta.yml"),
            "title: Sample\nslug: sample\npublished_at: 2026-03-19T00:00:00Z\ntags:\n  - rust\nsummary: hello\nstatus: published\ncharts:\n  - type: line\n    source: ./metrics.csv\n    x: step\n    y: ms\n",
        )
        .unwrap();
        fs::write(article_dir.join("post.md"), "# Hello").unwrap();
        fs::write(article_dir.join("metrics.csv"), "step,ms\nbootstrap,fast\n").unwrap();

        let error = load_post_dir(content_root, &article_dir).unwrap_err();

        assert!(matches!(
            error,
            rustacian_blog_core::BlogError::Validation(_)
        ));
    }

    #[test]
    fn loads_optional_tag_dictionary() {
        let temp = tempdir().unwrap();
        let content_root = temp.path();
        fs::create_dir_all(content_root.join("posts")).unwrap();
        fs::write(
            content_root.join("tags.yml"),
            "- id: rust\n  name: Rust\n- id: actix-web\n  name: Actix Web\n",
        )
        .unwrap();

        let tags = load_tag_dictionary(&content_root.join("posts"))
            .unwrap()
            .unwrap();

        assert!(tags.contains("rust"));
        assert!(tags.contains("actix-web"));
    }

    #[tokio::test]
    async fn repository_rejects_undefined_tags_when_dictionary_exists() {
        let temp = tempdir().unwrap();
        let content_root = temp.path();
        let posts_dir = content_root.join("posts");
        fs::create_dir_all(&posts_dir).unwrap();
        fs::write(content_root.join("tags.yml"), "- id: rust\n  name: Rust\n").unwrap();
        let article_dir = posts_dir.join("sample");
        fs::create_dir_all(&article_dir).unwrap();
        fs::write(
            article_dir.join("meta.yml"),
            "title: Sample\nslug: sample\npublished_at: 2026-03-19T00:00:00Z\ntags:\n  - rust\n  - unknown\nsummary: hello\nstatus: published\n",
        )
        .unwrap();
        fs::write(article_dir.join("post.md"), "# Hello").unwrap();

        let repository = LocalContentPostRepository::new(content_root.to_path_buf());
        let error = repository
            .list_posts(PostVisibility::PublishedOnly)
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            rustacian_blog_core::BlogError::Validation(_)
        ));
    }

    #[tokio::test]
    async fn repository_lists_only_published_posts_from_content_directory() {
        let temp = tempdir().unwrap();
        let posts_dir = temp.path().join("posts");
        fs::create_dir_all(&posts_dir).unwrap();
        write_article(&posts_dir, "published-post", "published", "# Hello");
        write_article(&posts_dir, "draft-post", "draft", "# Draft");

        let repository = LocalContentPostRepository::new(temp.path().to_path_buf());
        let posts = repository
            .list_posts(PostVisibility::PublishedOnly)
            .await
            .unwrap();

        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0].slug, "published-post");
    }

    #[tokio::test]
    async fn repository_rejects_duplicate_slugs_across_article_directories() {
        let temp = tempdir().unwrap();
        let posts_dir = temp.path().join("posts");
        fs::create_dir_all(&posts_dir).unwrap();
        let first = posts_dir.join("first");
        let second = posts_dir.join("second");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        fs::write(
            first.join("meta.yml"),
            "title: First\nslug: duplicated\npublished_at: 2026-03-19T00:00:00Z\ntags:\n  - rust\nsummary: hello\nstatus: published\n",
        )
        .unwrap();
        fs::write(first.join("post.md"), "# First").unwrap();
        fs::write(
            second.join("meta.yml"),
            "title: Second\nslug: duplicated\npublished_at: 2026-03-19T01:00:00Z\ntags:\n  - rust\nsummary: hello\nstatus: published\n",
        )
        .unwrap();
        fs::write(second.join("post.md"), "# Second").unwrap();

        let repository = LocalContentPostRepository::new(temp.path().to_path_buf());
        let error = repository
            .list_posts(PostVisibility::PublishedOnly)
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            rustacian_blog_core::BlogError::Validation(_)
        ));
    }

    #[tokio::test]
    async fn repository_does_not_return_draft_post_detail() {
        let temp = tempdir().unwrap();
        let posts_dir = temp.path().join("posts");
        fs::create_dir_all(&posts_dir).unwrap();
        write_article(&posts_dir, "draft-post", "draft", "# Draft");

        let repository = LocalContentPostRepository::new(temp.path().to_path_buf());
        let error = repository
            .get_post("draft-post", PostVisibility::PublishedOnly)
            .await
            .unwrap_err();

        assert!(matches!(error, rustacian_blog_core::BlogError::NotFound(_)));
    }

    #[tokio::test]
    async fn azurite_repository_reads_seeded_posts_when_available() {
        let endpoint = "http://127.0.0.1:10000/devstoreaccount1";
        let require_azurite = std::env::var("RUN_AZURITE_TESTS").ok().as_deref() == Some("1");
        let temp = tempdir().unwrap();
        let posts_dir = temp.path().join("posts");
        fs::create_dir_all(&posts_dir).unwrap();
        write_article(&posts_dir, "sample", "published", "# Hello from Azurite");

        let seed_result = seed_azurite_from_local(temp.path().to_path_buf(), endpoint).await;

        if !require_azurite && seed_result.is_err() {
            return;
        }
        seed_result.unwrap();

        let repository = AzuritePostRepository::new(endpoint.to_owned());
        let post = repository
            .get_post("sample", PostVisibility::PublishedOnly)
            .await
            .unwrap();

        assert_eq!(post.slug, "sample");
        assert!(post.body_markdown.contains("Azurite"));
    }
}
