use std::sync::Arc;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::{Client, Method, StatusCode};
use rustacian_blog_core::BlogError;
use sha2::Sha256;
use tokio::sync::RwLock;

const AZURITE_ACCOUNT: &str = "devstoreaccount1";
const AZURITE_ACCOUNT_KEY: &str =
    "Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw==";
const TABLE_API_VERSION: &str = "2019-02-02";
const CONTENT_TYPE: &str = "application/json;odata=nometadata";
/// Azure IMDS endpoint for Managed Identity token acquisition.
const IMDS_TOKEN_URL: &str = "http://169.254.169.254/metadata/identity/oauth2/token\
    ?api-version=2018-02-01&resource=https%3A%2F%2Fstorage.azure.com%2F";
/// Renew the cached token this many seconds before actual expiry.
const TOKEN_EXPIRY_MARGIN_SECS: i64 = 300;

enum StorageCredential {
    SharedKey { key: String },
    ManagedIdentity,
}

struct CachedToken {
    token: String,
    expires_at: chrono::DateTime<Utc>,
}

struct CredentialState {
    kind: StorageCredential,
    token_cache: RwLock<Option<CachedToken>>,
}

/// Minimal Azure Table Storage REST client.
///
/// Auth is selected automatically at construction time:
/// - `AZURE_STORAGE_ACCOUNT_KEY` env var set → SharedKey with that key.
/// - Endpoint is localhost/Azurite → SharedKey with built-in Azurite dev key.
/// - Otherwise → Managed Identity (IMDS token with 5-min pre-expiry cache).
#[derive(Clone)]
pub struct AzuriteTableClient {
    client: Client,
    endpoint: String,
    /// The path portion of the endpoint URL (e.g. "/devstoreaccount1" for Azurite, "" for Azure).
    endpoint_path: String,
    account_name: String,
    cred: Arc<CredentialState>,
}

impl AzuriteTableClient {
    pub fn new(table_endpoint: impl Into<String>) -> Self {
        let endpoint = table_endpoint.into().trim_end_matches('/').to_owned();
        let endpoint_path = extract_endpoint_path(&endpoint).to_owned();

        let account_name = std::env::var("AZURE_STORAGE_ACCOUNT_NAME")
            .unwrap_or_else(|_| extract_account_name(&endpoint));

        let kind = if let Ok(key) = std::env::var("AZURE_STORAGE_ACCOUNT_KEY") {
            StorageCredential::SharedKey { key }
        } else if is_local_endpoint(&endpoint) {
            // Azurite dev — well-known default key, safe to hard-code.
            StorageCredential::SharedKey {
                key: AZURITE_ACCOUNT_KEY.to_owned(),
            }
        } else {
            StorageCredential::ManagedIdentity
        };

        Self {
            client: Client::new(),
            endpoint,
            endpoint_path,
            account_name,
            cred: Arc::new(CredentialState {
                kind,
                token_cache: RwLock::new(None),
            }),
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
        // GET has no body → Content-Type is empty in StringToSign.
        let auth = self.auth_header("GET", "", &date, &canonical).await?;

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
        let auth = self
            .auth_header(method.as_str(), CONTENT_TYPE, &date, &canonical)
            .await?;

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

    /// Build the Authorization header value.
    /// SharedKey: HMAC-SHA256 over StringToSign.
    /// ManagedIdentity: cached or freshly fetched Bearer token from IMDS.
    async fn auth_header(
        &self,
        verb: &str,
        content_type: &str,
        date: &str,
        canonical: &str,
    ) -> Result<String, BlogError> {
        match &self.cred.kind {
            StorageCredential::SharedKey { key } => {
                build_shared_key_auth(&self.account_name, key, verb, content_type, date, canonical)
            }
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
        // Double-check after acquiring write lock.
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

    fn canonical_resource(&self, path: &str) -> String {
        format!("/{}{}{path}", self.account_name, self.endpoint_path)
    }
}

/// Build a SharedKey Authorization header value for Azure Table Service.
///
/// StringToSign format (Table Service SharedKey):
/// ```text
/// VERB + "\n" + Content-MD5 + "\n" + Content-Type + "\n" + Date + "\n" + CanonicalizedResource
/// ```
fn build_shared_key_auth(
    account: &str,
    key: &str,
    verb: &str,
    content_type: &str,
    date: &str,
    canonicalized_resource: &str,
) -> Result<String, BlogError> {
    let string_to_sign = format!("{verb}\n\n{content_type}\n{date}\n{canonicalized_resource}");
    let key_bytes = STANDARD
        .decode(key)
        .map_err(|e| BlogError::Storage(e.to_string()))?;
    let mut mac = Hmac::<Sha256>::new_from_slice(&key_bytes)
        .map_err(|e| BlogError::Storage(e.to_string()))?;
    mac.update(string_to_sign.as_bytes());
    let sig = STANDARD.encode(mac.finalize().into_bytes());
    Ok(format!("SharedKey {account}:{sig}"))
}

fn is_local_endpoint(endpoint: &str) -> bool {
    endpoint.contains("127.0.0.1") || endpoint.contains("localhost")
}

/// Derive the storage account name from an endpoint URL.
/// - Azurite: `http://127.0.0.1:10002/devstoreaccount1` → `devstoreaccount1`
/// - Azure:   `https://myaccount.table.core.windows.net`  → `myaccount`
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
/// e.g. `"http://127.0.0.1:10002/devstoreaccount1"` → `"/devstoreaccount1"`
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

fn now_rfc1123() -> String {
    chrono::Utc::now()
        .format("%a, %d %b %Y %H:%M:%S GMT")
        .to_string()
}
