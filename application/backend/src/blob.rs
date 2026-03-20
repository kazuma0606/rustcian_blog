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
