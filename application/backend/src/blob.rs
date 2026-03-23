use std::{collections::BTreeMap, sync::Arc};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::{
    Client, Method, StatusCode,
    header::{CONTENT_LENGTH, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue},
};
use rustacian_blog_core::BlogError;
use sha2::Sha256;
use tokio::sync::RwLock;

const AZURITE_ACCOUNT: &str = "devstoreaccount1";
const AZURITE_ACCOUNT_KEY: &str =
    "Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw==";
pub const AZURITE_CONTAINER: &str = "blog-content";
const AZURITE_API_VERSION: &str = "2023-11-03";
/// Azure IMDS endpoint for Managed Identity token acquisition.
const IMDS_TOKEN_URL: &str = "http://169.254.169.254/metadata/identity/oauth2/token\
    ?api-version=2018-02-01&resource=https%3A%2F%2Fstorage.azure.com%2F";
/// Renew the cached token this many seconds before actual expiry.
const TOKEN_EXPIRY_MARGIN_SECS: i64 = 300;

#[derive(Debug)]
enum StorageCredential {
    SharedKey { key: String },
    ManagedIdentity,
}

#[derive(Debug)]
struct CachedToken {
    token: String,
    expires_at: chrono::DateTime<Utc>,
}

#[derive(Debug)]
struct CredentialState {
    kind: StorageCredential,
    token_cache: RwLock<Option<CachedToken>>,
}

/// Azure Blob Storage REST client.
///
/// Auth is selected automatically at construction time:
/// - `AZURE_STORAGE_ACCOUNT_KEY` env var set → SharedKey with that key.
/// - Endpoint is localhost/Azurite → SharedKey with built-in Azurite dev key.
/// - Otherwise → Managed Identity (IMDS token with 5-min pre-expiry cache).
#[derive(Clone, Debug)]
pub struct AzuriteBlobAdapter {
    client: Client,
    blob_endpoint: String,
    /// Path portion of the endpoint URL, prepended to blob paths in the canonical resource.
    /// Azurite: "/devstoreaccount1"; Azure: "".
    endpoint_path: String,
    account_name: String,
    cred: Arc<CredentialState>,
}

impl AzuriteBlobAdapter {
    pub fn new(blob_endpoint: String) -> Self {
        let blob_endpoint = blob_endpoint.trim_end_matches('/').to_owned();
        let endpoint_path = extract_endpoint_path(&blob_endpoint).to_owned();

        let account_name = std::env::var("AZURE_STORAGE_ACCOUNT_NAME")
            .unwrap_or_else(|_| extract_account_name(&blob_endpoint));

        let kind = if let Ok(key) = std::env::var("AZURE_STORAGE_ACCOUNT_KEY") {
            StorageCredential::SharedKey { key }
        } else if is_local_endpoint(&blob_endpoint) {
            StorageCredential::SharedKey {
                key: AZURITE_ACCOUNT_KEY.to_owned(),
            }
        } else {
            StorageCredential::ManagedIdentity
        };

        Self {
            client: Client::new(),
            blob_endpoint,
            endpoint_path,
            account_name,
            cred: Arc::new(CredentialState {
                kind,
                token_cache: RwLock::new(None),
            }),
        }
    }

    pub async fn get_text(&self, blob_name: &str) -> Result<Option<String>, BlogError> {
        let response = self.request_blob(Method::GET, blob_name, None).await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        response
            .text()
            .await
            .map(Some)
            .map_err(|error| BlogError::Storage(error.to_string()))
    }

    pub async fn get_bytes(
        &self,
        blob_name: &str,
    ) -> Result<Option<(Vec<u8>, Option<String>)>, BlogError> {
        let response = self.request_blob(Method::GET, blob_name, None).await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let bytes = response
            .bytes()
            .await
            .map_err(|error| BlogError::Storage(error.to_string()))?;

        Ok(Some((bytes.to_vec(), content_type)))
    }

    pub async fn put_bytes(
        &self,
        blob_name: &str,
        body: Vec<u8>,
        content_type: &str,
    ) -> Result<(), BlogError> {
        let path = format!("/{AZURITE_CONTAINER}/{blob_name}");
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
        let authorization = self
            .build_auth_header(
                &Method::PUT,
                &path,
                &headers,
                None,
                &content_length,
                content_type,
            )
            .await?;
        headers.insert(
            HeaderName::from_static("authorization"),
            HeaderValue::from_str(&authorization)
                .map_err(|error| BlogError::Storage(error.to_string()))?,
        );
        let url = format!("{}{}", self.blob_endpoint, path);
        let response = self
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

    pub async fn delete_blob(&self, blob_name: &str) -> Result<(), BlogError> {
        let response = self.request_blob(Method::DELETE, blob_name, None).await?;
        match response.status() {
            StatusCode::ACCEPTED | StatusCode::OK => Ok(()),
            StatusCode::NOT_FOUND => Ok(()),
            status => Err(BlogError::Storage(format!(
                "failed to delete blob {blob_name}: {status}"
            ))),
        }
    }

    pub async fn list_blobs(&self, prefix: &str) -> Result<Vec<BlobItem>, BlogError> {
        let mut query = BTreeMap::new();
        query.insert("restype".to_owned(), "container".to_owned());
        query.insert("comp".to_owned(), "list".to_owned());
        if !prefix.is_empty() {
            query.insert("prefix".to_owned(), prefix.to_owned());
        }
        let response = self
            .request(
                Method::GET,
                &format!("/{AZURITE_CONTAINER}"),
                None,
                Some(query),
                true,
            )
            .await?;

        if !response.status().is_success() {
            return Err(BlogError::Storage(format!(
                "failed to list blobs: {}",
                response.status()
            )));
        }

        let body = response
            .text()
            .await
            .map_err(|e| BlogError::Storage(e.to_string()))?;

        Ok(parse_blob_list_xml(&body))
    }

    pub async fn create_container_if_needed(&self) -> Result<(), BlogError> {
        let mut query = BTreeMap::new();
        query.insert("restype".to_owned(), "container".to_owned());
        let response = self
            .request(
                Method::PUT,
                &format!("/{AZURITE_CONTAINER}"),
                None,
                Some(query),
                true,
            )
            .await?;

        match response.status() {
            // CREATED = new container, CONFLICT = already exists, FORBIDDEN = pre-created by
            // infrastructure (Terraform) with permissions that block re-creation — treat as ok.
            StatusCode::CREATED | StatusCode::CONFLICT | StatusCode::FORBIDDEN => Ok(()),
            status => Err(BlogError::Storage(format!(
                "failed to create blob container: {status}"
            ))),
        }
    }

    async fn request_blob(
        &self,
        method: Method,
        blob_name: &str,
        body: Option<Vec<u8>>,
    ) -> Result<reqwest::Response, BlogError> {
        let path = format!("/{AZURITE_CONTAINER}/{blob_name}");
        self.request(method, &path, body, None, false).await
    }

    async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<Vec<u8>>,
        extra_query: Option<BTreeMap<String, String>>,
        is_container_request: bool,
    ) -> Result<reqwest::Response, BlogError> {
        let mut url = format!("{}{}", self.blob_endpoint, path);
        if let Some(query) = &extra_query {
            let query_string = query
                .iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<_>>()
                .join("&");
            url = format!("{url}?{query_string}");
        }

        let body_bytes = body.unwrap_or_default();
        // Azure Blob requires Content-Length on all PUT requests (even empty body = "0").
        let content_length = if body_bytes.is_empty() && method == Method::PUT {
            "0".to_owned()
        } else if body_bytes.is_empty() {
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

        let ct = headers
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_owned();
        let authorization = self
            .build_auth_header(
                &method,
                path,
                &headers,
                extra_query.as_ref(),
                &content_length,
                &ct,
            )
            .await?;
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

    /// Build the Authorization header value.
    /// SharedKey: HMAC-SHA256 over StringToSign (Blob service format).
    /// ManagedIdentity: cached or freshly fetched Bearer token from IMDS.
    async fn build_auth_header(
        &self,
        method: &Method,
        path: &str,
        headers: &HeaderMap,
        query: Option<&BTreeMap<String, String>>,
        content_length: &str,
        content_type: &str,
    ) -> Result<String, BlogError> {
        match &self.cred.kind {
            StorageCredential::SharedKey { key } => build_shared_key_auth(
                &self.account_name,
                key,
                method,
                &self.endpoint_path,
                path,
                headers,
                query,
                content_length,
                content_type,
            ),
            StorageCredential::ManagedIdentity => {
                let token = self.get_or_refresh_token().await?;
                Ok(format!("Bearer {token}"))
            }
        }
    }

    /// Return a valid Managed Identity token, refreshing via IMDS if the cache is stale.
    async fn get_or_refresh_token(&self) -> Result<String, BlogError> {
        {
            let cache = self.cred.token_cache.read().await;
            if let Some(cached) = cache.as_ref()
                && Utc::now() < cached.expires_at
            {
                return Ok(cached.token.clone());
            }
        }

        let mut cache = self.cred.token_cache.write().await;
        if let Some(cached) = cache.as_ref()
            && Utc::now() < cached.expires_at
        {
            return Ok(cached.token.clone());
        }

        let resp = self
            .client
            .get(IMDS_TOKEN_URL)
            .header("Metadata", "true")
            .send()
            .await
            .map_err(|e| BlogError::Storage(format!("IMDS request failed: {e}")))?;

        if !resp.status().is_success() {
            let s = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(BlogError::Storage(format!(
                "IMDS token fetch failed ({s}): {body}"
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| BlogError::Storage(format!("IMDS response parse failed: {e}")))?;

        let token = json["access_token"]
            .as_str()
            .ok_or_else(|| BlogError::Storage("IMDS: missing access_token".to_owned()))?
            .to_owned();

        let expires_on = json["expires_on"]
            .as_str()
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or_else(|| BlogError::Storage("IMDS: missing expires_on".to_owned()))?;

        let expires_at =
            chrono::DateTime::from_timestamp_secs(expires_on - TOKEN_EXPIRY_MARGIN_SECS)
                .ok_or_else(|| BlogError::Storage("IMDS: invalid expires_on".to_owned()))?;

        *cache = Some(CachedToken {
            token: token.clone(),
            expires_at,
        });
        Ok(token)
    }
}

#[derive(Debug, Clone)]
pub struct BlobItem {
    pub name: String,
    pub content_type: Option<String>,
    pub last_modified: Option<String>,
    pub size: Option<u64>,
}

/// Minimal XML parser for Azure Blob List response.
/// Extracts blob names and properties without pulling in an XML crate.
fn parse_blob_list_xml(xml: &str) -> Vec<BlobItem> {
    let mut items = Vec::new();
    for blob_block in xml.split("<Blob>").skip(1) {
        let name = extract_xml_tag(blob_block, "Name").unwrap_or_default();
        let content_type = extract_xml_tag(blob_block, "Content-Type");
        let last_modified = extract_xml_tag(blob_block, "Last-Modified");
        let size = extract_xml_tag(blob_block, "Content-Length").and_then(|v| v.parse().ok());
        items.push(BlobItem {
            name,
            content_type,
            last_modified,
            size,
        });
    }
    items
}

fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)?;
    Some(xml[start..start + end].to_owned())
}

/// Build a SharedKey Authorization header for Azure Blob Service.
///
/// StringToSign format (Blob Service SharedKey):
/// ```text
/// VERB + "\n" + Content-MD5 + "\n" + Content-Type + "\n" + Date + "\n"
/// + CanonicalizedAmzHeaders + CanonicalizedResource
/// ```
#[allow(clippy::too_many_arguments)]
fn build_shared_key_auth(
    account_name: &str,
    key: &str,
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
        // Azure SharedKey canonical resource: /{account}{endpoint_path}{path}
        // For Azurite (http://host:port/devstoreaccount1), endpoint_path = "/devstoreaccount1"
        // so the resource starts with /devstoreaccount1/devstoreaccount1/...
        // For Azure prod (https://account.blob.core.windows.net), endpoint_path = ""
        let mut resource = format!("/{account_name}{endpoint_path}{path}");
        if let Some(query) = query {
            for (key, value) in query {
                resource.push('\n');
                resource.push_str(&format!("{}:{}", key.to_ascii_lowercase(), value));
            }
        }
        resource
    };

    // Per Azure spec: Content-Length in StringToSign must be empty when value is 0 or absent.
    let cl_in_sign = if content_length == "0" || content_length.is_empty() {
        ""
    } else {
        content_length
    };
    let string_to_sign = format!(
        "{method}\n\n\n{cl_in_sign}\n\n{content_type}\n\n\n\n\n\n\n{}{canonicalized_resource}",
        canonical_headers.concat()
    );

    let key_bytes = STANDARD
        .decode(key)
        .map_err(|error| BlogError::Storage(error.to_string()))?;
    let mut mac = Hmac::<Sha256>::new_from_slice(&key_bytes)
        .map_err(|error| BlogError::Storage(error.to_string()))?;
    mac.update(string_to_sign.as_bytes());
    let signature = STANDARD.encode(mac.finalize().into_bytes());

    Ok(format!("SharedKey {account_name}:{signature}"))
}

fn is_local_endpoint(endpoint: &str) -> bool {
    endpoint.contains("127.0.0.1") || endpoint.contains("localhost")
}

/// Derive the storage account name from a blob endpoint URL.
/// - Azurite: `http://127.0.0.1:10000/devstoreaccount1` → `devstoreaccount1`
/// - Azure:   `https://myaccount.blob.core.windows.net`   → `myaccount`
fn extract_account_name(endpoint: &str) -> String {
    let path = extract_endpoint_path(endpoint);
    if !path.is_empty() {
        return path
            .trim_start_matches('/')
            .split('/')
            .next()
            .unwrap_or(AZURITE_ACCOUNT)
            .to_owned();
    }
    if let Some(after_scheme) = endpoint
        .strip_prefix("https://")
        .or_else(|| endpoint.strip_prefix("http://"))
        && let Some(dot_pos) = after_scheme.find('.')
    {
        return after_scheme[..dot_pos].to_owned();
    }
    AZURITE_ACCOUNT.to_owned()
}

/// Extract the URL path component from an endpoint URL.
/// e.g. `"http://127.0.0.1:10000/devstoreaccount1"` → `"/devstoreaccount1"`
fn extract_endpoint_path(endpoint: &str) -> &str {
    if let Some(after_scheme) = endpoint
        .strip_prefix("http://")
        .or_else(|| endpoint.strip_prefix("https://"))
        && let Some(slash) = after_scheme.find('/')
    {
        return &after_scheme[slash..];
    }
    ""
}

#[cfg(test)]
mod tests {
    use super::{AzuriteBlobAdapter, extract_account_name, parse_blob_list_xml};

    // ---------------------------------------------------------------------------
    // Pure-function unit tests (always run)
    // ---------------------------------------------------------------------------

    #[test]
    fn parse_blob_list_xml_returns_empty_for_no_blobs() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<EnumerationResults><Blobs></Blobs></EnumerationResults>"#;
        assert!(parse_blob_list_xml(xml).is_empty());
    }

    #[test]
    fn parse_blob_list_xml_extracts_name_and_size() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<EnumerationResults>
  <Blobs>
    <Blob>
      <Name>images/photo.jpg</Name>
      <Properties>
        <Content-Type>image/jpeg</Content-Type>
        <Content-Length>12345</Content-Length>
        <Last-Modified>Mon, 01 Jan 2026 00:00:00 GMT</Last-Modified>
      </Properties>
    </Blob>
    <Blob>
      <Name>images/logo.png</Name>
      <Properties>
        <Content-Type>image/png</Content-Type>
        <Content-Length>512</Content-Length>
        <Last-Modified>Tue, 02 Jan 2026 00:00:00 GMT</Last-Modified>
      </Properties>
    </Blob>
  </Blobs>
</EnumerationResults>"#;
        let items = parse_blob_list_xml(xml);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "images/photo.jpg");
        assert_eq!(items[0].content_type.as_deref(), Some("image/jpeg"));
        assert_eq!(items[0].size, Some(12345));
        assert_eq!(items[1].name, "images/logo.png");
        assert_eq!(items[1].size, Some(512));
    }

    #[test]
    fn parse_blob_list_xml_handles_missing_optional_fields() {
        let xml = r#"<EnumerationResults>
  <Blobs>
    <Blob><Name>bare.txt</Name><Properties></Properties></Blob>
  </Blobs>
</EnumerationResults>"#;
        let items = parse_blob_list_xml(xml);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "bare.txt");
        assert!(items[0].content_type.is_none());
        assert!(items[0].size.is_none());
    }

    #[test]
    fn extract_account_name_handles_azurite() {
        assert_eq!(
            extract_account_name("http://127.0.0.1:10000/devstoreaccount1"),
            "devstoreaccount1"
        );
    }

    #[test]
    fn extract_account_name_handles_azure() {
        assert_eq!(
            extract_account_name("https://rustacianprodst.blob.core.windows.net"),
            "rustacianprodst"
        );
    }

    // ---------------------------------------------------------------------------
    // Azurite integration tests (require RUN_AZURITE_TESTS=1)
    // ---------------------------------------------------------------------------

    fn azurite_adapter() -> AzuriteBlobAdapter {
        AzuriteBlobAdapter::new("http://127.0.0.1:10000/devstoreaccount1".to_owned())
    }

    fn require_azurite() -> bool {
        std::env::var("RUN_AZURITE_TESTS").ok().as_deref() == Some("1")
    }

    #[tokio::test]
    async fn put_bytes_uploads_and_get_bytes_retrieves() {
        let adapter = azurite_adapter();
        let init = adapter.create_container_if_needed().await;
        if !require_azurite() && init.is_err() {
            return;
        }
        init.unwrap();

        let blob_name = "test/put_bytes_test.txt";
        let content = b"hello azurite blob".to_vec();
        adapter
            .put_bytes(blob_name, content.clone(), "text/plain")
            .await
            .unwrap();

        let result = adapter.get_bytes(blob_name).await.unwrap();
        assert!(result.is_some());
        let (bytes, ct) = result.unwrap();
        assert_eq!(bytes, content);
        assert_eq!(ct.as_deref(), Some("text/plain"));

        adapter.delete_blob(blob_name).await.unwrap();
    }

    #[tokio::test]
    async fn list_blobs_returns_uploaded_blob() {
        let adapter = azurite_adapter();
        let init = adapter.create_container_if_needed().await;
        if !require_azurite() && init.is_err() {
            return;
        }
        init.unwrap();

        let blob_name = "test/list_blobs_test.png";
        adapter
            .put_bytes(blob_name, vec![1, 2, 3], "image/png")
            .await
            .unwrap();

        let items = adapter.list_blobs("test/").await.unwrap();
        let found = items.iter().any(|i| i.name == blob_name);
        assert!(found, "uploaded blob not found in list");

        adapter.delete_blob(blob_name).await.unwrap();
    }

    #[tokio::test]
    async fn delete_blob_removes_blob_from_list() {
        let adapter = azurite_adapter();
        let init = adapter.create_container_if_needed().await;
        if !require_azurite() && init.is_err() {
            return;
        }
        init.unwrap();

        let blob_name = "test/delete_test.txt";
        adapter
            .put_bytes(blob_name, b"bye".to_vec(), "text/plain")
            .await
            .unwrap();
        adapter.delete_blob(blob_name).await.unwrap();

        let result = adapter.get_bytes(blob_name).await.unwrap();
        assert!(result.is_none(), "blob should be gone after delete");
    }
}
