use std::{
    collections::BTreeMap,
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
use rustacian_blog_core::{BlogError, Post, PostFrontmatter, PostRepository, PostSummary};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

const AZURITE_ACCOUNT: &str = "devstoreaccount1";
const AZURITE_ACCOUNT_KEY: &str =
    "Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw==";
const AZURITE_CONTAINER: &str = "blog-content";
const AZURITE_API_VERSION: &str = "2023-11-03";
const INDEX_BLOB_NAME: &str = "posts/index.json";

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
        load_posts_from_dir(&self.posts_dir())
    }
}

#[async_trait]
impl PostRepository for LocalContentPostRepository {
    async fn list_posts(&self) -> Result<Vec<PostSummary>, BlogError> {
        self.read_all_posts()
            .map(|posts| posts.into_iter().map(|post| post.summary()).collect())
    }

    async fn get_post(&self, slug: &str) -> Result<Post, BlogError> {
        self.read_all_posts()?
            .into_iter()
            .find(|post| post.slug == slug)
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

    async fn read_post_blob(&self, blob_name: &str) -> Result<Post, BlogError> {
        let response = self
            .request_blob(Method::GET, blob_name, None, None)
            .await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(BlogError::NotFound(blob_name.to_owned()));
        }

        let raw = response
            .text()
            .await
            .map_err(|error| BlogError::Storage(error.to_string()))?;

        let (frontmatter, body_markdown) = parse_frontmatter(&raw)?;
        let frontmatter = PostFrontmatter {
            title: frontmatter.title,
            slug: frontmatter.slug,
            published_at: frontmatter.published_at,
            tags: frontmatter.tags,
            summary: frontmatter.summary,
            hero_image: frontmatter.hero_image,
        };
        let body_html = render_markdown(&body_markdown);

        Post::new(frontmatter, body_markdown, body_html)
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
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/markdown"));
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

        let request = self
            .client
            .request(method, &url)
            .headers(headers)
            .body(body_bytes);

        request
            .send()
            .await
            .map_err(|error| BlogError::Storage(error.to_string()))
    }
}

#[async_trait]
impl PostRepository for AzuritePostRepository {
    async fn list_posts(&self) -> Result<Vec<PostSummary>, BlogError> {
        let manifest = self.read_manifest().await?;
        let mut posts = Vec::with_capacity(manifest.len());

        for item in manifest {
            posts.push(self.read_post_blob(&item.blob_name).await?);
        }

        posts.sort_by(|left, right| right.published_at.cmp(&left.published_at));
        Ok(posts.into_iter().map(|post| post.summary()).collect())
    }

    async fn get_post(&self, slug: &str) -> Result<Post, BlogError> {
        let manifest = self.read_manifest().await?;
        let entry = manifest
            .into_iter()
            .find(|item| item.slug == slug)
            .ok_or_else(|| BlogError::NotFound(slug.to_owned()))?;

        self.read_post_blob(&entry.blob_name).await
    }
}

pub async fn seed_azurite_from_local(
    content_root: PathBuf,
    blob_endpoint: &str,
) -> Result<(), BlogError> {
    let client = AzuritePostRepository::new(blob_endpoint.to_owned());
    create_container_if_needed(&client).await?;

    let entries = fs::read_dir(content_root.join("posts"))
        .map_err(|error| BlogError::Storage(error.to_string()))?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("md"))
        .map(|entry| seed_post_blob(&client, entry.path()))
        .collect::<Vec<_>>();

    let mut manifest = Vec::new();
    for entry in entries {
        manifest.push(entry.await?);
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

async fn seed_post_blob(
    client: &AzuritePostRepository,
    path: PathBuf,
) -> Result<ManifestEntry, BlogError> {
    let raw = fs::read_to_string(&path).map_err(|error| BlogError::Storage(error.to_string()))?;
    let (frontmatter, _) = parse_frontmatter(&raw)?;
    let blob_name = format!("posts/{}", path.file_name().unwrap().to_string_lossy());
    upload_blob(client, &blob_name, raw.into_bytes(), "text/markdown").await?;

    Ok(ManifestEntry {
        slug: frontmatter.slug,
        blob_name,
    })
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

fn load_posts_from_dir(posts_dir: &Path) -> Result<Vec<Post>, BlogError> {
    let mut posts = fs::read_dir(posts_dir)
        .map_err(|error| BlogError::Storage(error.to_string()))?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("md"))
        .map(|entry| load_post_file(&entry.path()))
        .collect::<Result<Vec<_>, _>>()?;

    posts.sort_by(|left, right| right.published_at.cmp(&left.published_at));
    Ok(posts)
}

#[derive(Debug, Deserialize)]
struct RawFrontmatter {
    title: String,
    slug: String,
    published_at: chrono::DateTime<chrono::Utc>,
    tags: Vec<String>,
    summary: String,
    #[serde(default)]
    hero_image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManifestEntry {
    slug: String,
    blob_name: String,
}

fn load_post_file(path: &Path) -> Result<Post, BlogError> {
    let raw = fs::read_to_string(path).map_err(|error| BlogError::Storage(error.to_string()))?;
    let (frontmatter, body_markdown) = parse_frontmatter(&raw)?;
    let frontmatter = PostFrontmatter {
        title: frontmatter.title,
        slug: frontmatter.slug,
        published_at: frontmatter.published_at,
        tags: frontmatter.tags,
        summary: frontmatter.summary,
        hero_image: frontmatter.hero_image,
    };
    let body_html = render_markdown(&body_markdown);

    Post::new(frontmatter, body_markdown, body_html)
}

fn parse_frontmatter(raw: &str) -> Result<(RawFrontmatter, String), BlogError> {
    let normalized = raw.replace("\r\n", "\n");

    if !normalized.starts_with("---\n") {
        return Err(BlogError::Parse(
            "markdown file must start with frontmatter delimiter".to_owned(),
        ));
    }

    let mut sections = normalized.splitn(3, "---\n");
    let _ = sections.next();
    let frontmatter = sections
        .next()
        .ok_or_else(|| BlogError::Parse("frontmatter block is missing".to_owned()))?;
    let body = sections
        .next()
        .ok_or_else(|| BlogError::Parse("markdown body is missing".to_owned()))?;

    let frontmatter: RawFrontmatter =
        serde_yaml::from_str(frontmatter).map_err(|error| BlogError::Parse(error.to_string()))?;

    Ok((frontmatter, body.trim().to_owned()))
}

fn render_markdown(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);

    let parser = Parser::new_ext(markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{
        AzuritePostRepository, LocalContentPostRepository, load_post_file, parse_frontmatter,
        seed_azurite_from_local,
    };
    use rustacian_blog_core::PostRepository;
    use tempfile::tempdir;

    #[test]
    fn frontmatter_is_parsed_from_markdown() {
        let raw = r#"---
title: Sample
slug: sample
published_at: 2026-03-19T00:00:00Z
tags:
  - rust
summary: hello
hero_image: /images/sample.svg
---

# Hello
"#;

        let (frontmatter, body) = parse_frontmatter(raw).unwrap();

        assert_eq!(frontmatter.slug, "sample");
        assert!(body.contains("# Hello"));
    }

    #[test]
    fn load_post_file_renders_html() {
        let temp = tempdir().unwrap();
        let file = temp.path().join("sample.md");
        fs::write(
            &file,
            r#"---
title: Sample
slug: sample
published_at: 2026-03-19T00:00:00Z
tags:
  - rust
summary: hello
---

# Hello
"#,
        )
        .unwrap();

        let post = load_post_file(&file).unwrap();

        assert!(post.body_html.contains("<h1>"));
    }

    #[tokio::test]
    async fn repository_lists_posts_from_content_directory() {
        let temp = tempdir().unwrap();
        let posts_dir = temp.path().join("posts");
        fs::create_dir_all(&posts_dir).unwrap();
        fs::write(
            posts_dir.join("sample.md"),
            r#"---
title: Sample
slug: sample
published_at: 2026-03-19T00:00:00Z
tags:
  - rust
summary: hello
---

# Hello
"#,
        )
        .unwrap();

        let repository = LocalContentPostRepository::new(temp.path().to_path_buf());
        let posts = repository.list_posts().await.unwrap();

        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0].slug, "sample");
    }

    #[tokio::test]
    async fn azurite_repository_reads_seeded_posts_when_available() {
        let endpoint = "http://127.0.0.1:10000/devstoreaccount1";
        let require_azurite = std::env::var("RUN_AZURITE_TESTS").ok().as_deref() == Some("1");
        let temp = tempdir().unwrap();
        let posts_dir = temp.path().join("posts");
        fs::create_dir_all(&posts_dir).unwrap();
        fs::write(
            posts_dir.join("sample.md"),
            r#"---
title: Sample
slug: sample
published_at: 2026-03-19T00:00:00Z
tags:
  - rust
summary: hello
---

# Hello from Azurite
"#,
        )
        .unwrap();

        let seed_result = seed_azurite_from_local(temp.path().to_path_buf(), endpoint).await;

        if !require_azurite && seed_result.is_err() {
            return;
        }
        seed_result.unwrap();

        let repository = AzuritePostRepository::new(endpoint.to_owned());
        let post = repository.get_post("sample").await.unwrap();

        assert_eq!(post.slug, "sample");
        assert!(post.body_markdown.contains("Azurite"));
    }
}
