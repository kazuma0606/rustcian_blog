use base64::{Engine as _, engine::general_purpose::STANDARD};
use hmac::{Hmac, Mac};
use reqwest::{Client, Method, StatusCode};
use rustacian_blog_core::BlogError;
use sha2::Sha256;

const AZURITE_ACCOUNT: &str = "devstoreaccount1";
const AZURITE_ACCOUNT_KEY: &str =
    "Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw==";
const TABLE_API_VERSION: &str = "2023-11-03";

/// Minimal Azurite Table Storage REST client (SharedKeyLite auth).
#[derive(Debug, Clone)]
pub struct AzuriteTableClient {
    client: Client,
    endpoint: String, // e.g. "http://127.0.0.1:10002/devstoreaccount1"
}

impl AzuriteTableClient {
    pub fn new(table_endpoint: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            endpoint: table_endpoint.into().trim_end_matches('/').to_owned(),
        }
    }

    pub async fn create_table_if_needed(&self, table_name: &str) -> Result<(), BlogError> {
        let body = serde_json::json!({ "TableName": table_name });
        let resp = self
            .request(Method::POST, "/Tables", Some(body), "application/json", None)
            .await?;
        match resp.status() {
            StatusCode::CREATED | StatusCode::CONFLICT => Ok(()),
            s => Err(BlogError::Storage(format!("create_table {table_name}: {s}"))),
        }
    }

    pub async fn insert_entity(
        &self,
        table_name: &str,
        entity: &serde_json::Value,
    ) -> Result<(), BlogError> {
        let path = format!("/{table_name}");
        let resp = self
            .request(Method::POST, &path, Some(entity.clone()), "application/json", None)
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
        let canonical = format!("/{AZURITE_ACCOUNT}{path}");
        let auth = build_auth(&date, &canonical)?;

        let resp = self
            .client
            .get(&url)
            .header("x-ms-date", &date)
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
        let json: serde_json::Value =
            resp.json().await.map_err(|e| BlogError::Storage(e.to_string()))?;
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
            .request(Method::PUT, &path, Some(entity.clone()), "application/json", Some("*"))
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
        content_type: &str,
        if_match: Option<&str>,
    ) -> Result<reqwest::Response, BlogError> {
        let url = format!("{}{}", self.endpoint, path);
        let date = now_rfc1123();
        let canonical = format!("/{AZURITE_ACCOUNT}{path}");
        let auth = build_auth(&date, &canonical)?;

        let mut builder = self
            .client
            .request(method, &url)
            .header("x-ms-date", &date)
            .header("x-ms-version", TABLE_API_VERSION)
            .header("Accept", "application/json;odata=nometadata")
            .header("Content-Type", content_type)
            .header("Authorization", &auth);

        if let Some(etag) = if_match {
            builder = builder.header("If-Match", etag);
        }
        if let Some(json) = body {
            let bytes =
                serde_json::to_vec(&json).map_err(|e| BlogError::Storage(e.to_string()))?;
            builder = builder.body(bytes);
        }
        builder.send().await.map_err(|e| BlogError::Storage(e.to_string()))
    }
}

fn build_auth(date: &str, canonicalized_resource: &str) -> Result<String, BlogError> {
    let string_to_sign = format!("{date}\n{canonicalized_resource}");
    let key = STANDARD
        .decode(AZURITE_ACCOUNT_KEY)
        .map_err(|e| BlogError::Storage(e.to_string()))?;
    let mut mac = Hmac::<Sha256>::new_from_slice(&key)
        .map_err(|e| BlogError::Storage(e.to_string()))?;
    mac.update(string_to_sign.as_bytes());
    let sig = STANDARD.encode(mac.finalize().into_bytes());
    Ok(format!("SharedKeyLite {AZURITE_ACCOUNT}:{sig}"))
}

fn now_rfc1123() -> String {
    chrono::Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string()
}
