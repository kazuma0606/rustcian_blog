use std::collections::BTreeMap;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use hmac::{Hmac, Mac};
use reqwest::{
    Client, Method, StatusCode,
    header::{CONTENT_LENGTH, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue},
};
use rustacian_blog_core::BlogError;
use sha2::Sha256;

const AZURITE_ACCOUNT: &str = "devstoreaccount1";
const AZURITE_ACCOUNT_KEY: &str =
    "Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw==";
const AZURITE_CONTAINER: &str = "blog-content";
const AZURITE_API_VERSION: &str = "2023-11-03";

#[derive(Debug, Clone)]
pub struct AzuriteBlobAdapter {
    client: Client,
    blob_endpoint: String,
}

impl AzuriteBlobAdapter {
    pub fn new(blob_endpoint: String) -> Self {
        Self {
            client: Client::new(),
            blob_endpoint,
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
        let authorization = build_authorization_header(
            &Method::PUT,
            endpoint_path(&self.blob_endpoint),
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
        let url = format!("{}{}", self.blob_endpoint.trim_end_matches('/'), path);
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
            StatusCode::CREATED | StatusCode::CONFLICT => Ok(()),
            status => Err(BlogError::Storage(format!(
                "failed to create Azurite container: {status}"
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

#[cfg(test)]
mod tests {
    use super::{AzuriteBlobAdapter, parse_blob_list_xml};

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
