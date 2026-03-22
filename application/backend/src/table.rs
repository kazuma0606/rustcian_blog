use base64::{Engine as _, engine::general_purpose::STANDARD};
use hmac::{Hmac, Mac};
use reqwest::{Client, Method, StatusCode};
use rustacian_blog_core::BlogError;
use sha2::Sha256;

const AZURITE_ACCOUNT: &str = "devstoreaccount1";
const AZURITE_ACCOUNT_KEY: &str =
    "Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw==";
const TABLE_API_VERSION: &str = "2019-02-02";
const CONTENT_TYPE: &str = "application/json;odata=nometadata";

/// Minimal Azurite Table Storage REST client (SharedKey auth).
#[derive(Debug, Clone)]
pub struct AzuriteTableClient {
    client: Client,
    endpoint: String, // e.g. "http://127.0.0.1:10002/devstoreaccount1"
    /// The path portion of the endpoint (e.g. "/devstoreaccount1").
    /// For Azurite, the canonical resource must include this prefix because
    /// the account name is embedded in the URL path rather than the hostname.
    endpoint_path: String,
}

impl AzuriteTableClient {
    pub fn new(table_endpoint: impl Into<String>) -> Self {
        let endpoint = table_endpoint.into().trim_end_matches('/').to_owned();
        let endpoint_path = extract_endpoint_path(&endpoint).to_owned();
        Self {
            client: Client::new(),
            endpoint,
            endpoint_path,
        }
    }

    pub async fn create_table_if_needed(&self, table_name: &str) -> Result<(), BlogError> {
        let body = serde_json::json!({ "TableName": table_name });
        let resp = self
            .request(Method::POST, "/Tables", Some(body), None)
            .await?;
        match resp.status() {
            StatusCode::CREATED | StatusCode::CONFLICT => Ok(()),
            s => Err(BlogError::Storage(format!(
                "create_table {table_name}: {s}"
            ))),
        }
    }

    pub async fn insert_entity(
        &self,
        table_name: &str,
        entity: &serde_json::Value,
    ) -> Result<(), BlogError> {
        let path = format!("/{table_name}");
        let resp = self
            .request(Method::POST, &path, Some(entity.clone()), None)
            .await?;
        match resp.status() {
            StatusCode::CREATED | StatusCode::NO_CONTENT => Ok(()),
            s => {
                let body = resp.text().await.unwrap_or_default();
                Err(BlogError::Storage(format!("insert_entity: {s}: {body}")))
            }
        }
    }

    pub async fn query_entities(
        &self,
        table_name: &str,
        filter: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, BlogError> {
        let path = format!("/{table_name}()");
        let query_suffix = filter
            .map(|f| format!("?$filter={}", urlencoding::encode(f)))
            .unwrap_or_default();
        let url = format!("{}{}{}", self.endpoint, path, query_suffix);

        let date = now_rfc1123();
        let canonical = self.canonical_resource(&path);
        // GET has no body → Content-Type is empty in StringToSign
        let auth = build_auth("GET", "", &date, &canonical)?;

        let resp = self
            .client
            .get(&url)
            .header("x-ms-date", &date)
            .header("Date", &date)
            .header("x-ms-version", TABLE_API_VERSION)
            .header("Accept", "application/json;odata=nometadata")
            .header("Authorization", &auth)
            .send()
            .await
            .map_err(|e| BlogError::Storage(e.to_string()))?;

        if !resp.status().is_success() {
            let s = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(BlogError::Storage(format!("query_entities: {s}: {body}")));
        }
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| BlogError::Storage(e.to_string()))?;
        Ok(json["value"].as_array().cloned().unwrap_or_default())
    }

    /// InsertOrReplace (PUT) — overwrites the whole entity.
    pub async fn upsert_entity(
        &self,
        table_name: &str,
        pk: &str,
        rk: &str,
        entity: &serde_json::Value,
    ) -> Result<(), BlogError> {
        let path = format!(
            "/{table_name}(PartitionKey='{}',RowKey='{}')",
            urlencoding::encode(pk),
            urlencoding::encode(rk)
        );
        let resp = self
            .request(Method::PUT, &path, Some(entity.clone()), Some("*"))
            .await?;
        match resp.status() {
            StatusCode::NO_CONTENT | StatusCode::CREATED => Ok(()),
            s => {
                let body = resp.text().await.unwrap_or_default();
                Err(BlogError::Storage(format!("upsert_entity: {s}: {body}")))
            }
        }
    }

    async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<serde_json::Value>,
        if_match: Option<&str>,
    ) -> Result<reqwest::Response, BlogError> {
        let url = format!("{}{}", self.endpoint, path);
        let date = now_rfc1123();
        let canonical = self.canonical_resource(path);
        let auth = build_auth(method.as_str(), CONTENT_TYPE, &date, &canonical)?;

        let mut builder = self
            .client
            .request(method, &url)
            .header("x-ms-date", &date)
            .header("Date", &date)
            .header("x-ms-version", TABLE_API_VERSION)
            .header("Accept", "application/json;odata=nometadata")
            .header("Content-Type", CONTENT_TYPE)
            .header("Authorization", &auth);

        if let Some(etag) = if_match {
            builder = builder.header("If-Match", etag);
        }
        if let Some(json) = body {
            let bytes = serde_json::to_vec(&json).map_err(|e| BlogError::Storage(e.to_string()))?;
            builder = builder.body(bytes);
        }
        builder
            .send()
            .await
            .map_err(|e| BlogError::Storage(e.to_string()))
    }

    /// Build the CanonicalizedResource string.
    ///
    /// For Azurite, the endpoint URL is `http://127.0.0.1:10002/devstoreaccount1`
    /// so `endpoint_path` is `/devstoreaccount1`.  The path argument already
    /// contains the table path (e.g. `/Tables`), so the full canonical resource
    /// is `/<account><endpoint_path><path>` which for Azurite gives
    /// `/devstoreaccount1/devstoreaccount1/Tables`.
    fn canonical_resource(&self, path: &str) -> String {
        format!("/{AZURITE_ACCOUNT}{}{path}", self.endpoint_path)
    }
}

/// Build a SharedKey Authorization header value for Azure Table Service.
///
/// StringToSign format (Table Service SharedKey):
/// ```text
/// VERB + "\n" + Content-MD5 + "\n" + Content-Type + "\n" + Date + "\n" + CanonicalizedResource
/// ```
fn build_auth(
    verb: &str,
    content_type: &str,
    date: &str,
    canonicalized_resource: &str,
) -> Result<String, BlogError> {
    // Content-MD5 is always empty (we don't send it)
    let string_to_sign = format!("{verb}\n\n{content_type}\n{date}\n{canonicalized_resource}");
    let key = STANDARD
        .decode(AZURITE_ACCOUNT_KEY)
        .map_err(|e| BlogError::Storage(e.to_string()))?;
    let mut mac =
        Hmac::<Sha256>::new_from_slice(&key).map_err(|e| BlogError::Storage(e.to_string()))?;
    mac.update(string_to_sign.as_bytes());
    let sig = STANDARD.encode(mac.finalize().into_bytes());
    Ok(format!("SharedKey {AZURITE_ACCOUNT}:{sig}"))
}

/// Extract the URL path component from an endpoint URL.
/// e.g. `"http://127.0.0.1:10002/devstoreaccount1"` → `"/devstoreaccount1"`
fn extract_endpoint_path(endpoint: &str) -> &str {
    // Strip scheme + authority (everything up to the third `/`)
    if let Some(after_scheme) = endpoint.strip_prefix("http://").or_else(|| endpoint.strip_prefix("https://")) {
        if let Some(slash) = after_scheme.find('/') {
            return &after_scheme[slash..];
        }
    }
    ""
}

fn now_rfc1123() -> String {
    chrono::Utc::now()
        .format("%a, %d %b %Y %H:%M:%S GMT")
        .to_string()
}
