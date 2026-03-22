use base64::{Engine as _, engine::general_purpose::STANDARD};
use hmac::{Hmac, Mac};
use reqwest::{Client, Method, StatusCode};
use sha2::Sha256;

const AZURITE_ACCOUNT: &str = "devstoreaccount1";
const AZURITE_ACCOUNT_KEY: &str =
    "Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw==";
const TABLE_API_VERSION: &str = "2019-02-02";
const CONTENT_TYPE: &str = "application/json;odata=nometadata";

#[derive(Debug, Clone)]
pub struct TableClient {
    client: Client,
    endpoint: String,
    endpoint_path: String,
}

impl TableClient {
    pub fn new(table_endpoint: impl Into<String>) -> Self {
        let endpoint = table_endpoint.into().trim_end_matches('/').to_owned();
        let endpoint_path = extract_endpoint_path(&endpoint).to_owned();
        Self { client: Client::new(), endpoint, endpoint_path }
    }

    pub async fn create_table_if_needed(&self, table_name: &str) -> Result<(), String> {
        let body = serde_json::json!({ "TableName": table_name });
        let resp = self.request(Method::POST, "/Tables", Some(body), None).await?;
        match resp.status() {
            StatusCode::CREATED | StatusCode::CONFLICT => Ok(()),
            s => Err(format!("create_table {table_name}: {s}")),
        }
    }

    pub async fn insert_entity(
        &self,
        table_name: &str,
        entity: &serde_json::Value,
    ) -> Result<(), String> {
        let path = format!("/{table_name}");
        let resp = self.request(Method::POST, &path, Some(entity.clone()), None).await?;
        match resp.status() {
            StatusCode::CREATED | StatusCode::NO_CONTENT => Ok(()),
            s => {
                let body = resp.text().await.unwrap_or_default();
                Err(format!("insert_entity: {s}: {body}"))
            }
        }
    }

    pub async fn query_entities(
        &self,
        table_name: &str,
        filter: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let path = format!("/{table_name}()");
        let query_suffix = filter
            .map(|f| format!("?$filter={}", urlencoding::encode(f)))
            .unwrap_or_default();
        let url = format!("{}{}{}", self.endpoint, path, query_suffix);

        let date = now_rfc1123();
        let canonical = self.canonical_resource(&path);
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
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            let s = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("query_entities: {s}: {body}"));
        }
        let json: serde_json::Value =
            resp.json().await.map_err(|e| e.to_string())?;
        Ok(json["value"].as_array().cloned().unwrap_or_default())
    }

    async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<serde_json::Value>,
        if_match: Option<&str>,
    ) -> Result<reqwest::Response, String> {
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
            let bytes =
                serde_json::to_vec(&json).map_err(|e| e.to_string())?;
            builder = builder.body(bytes);
        }
        builder.send().await.map_err(|e| e.to_string())
    }

    fn canonical_resource(&self, path: &str) -> String {
        format!("/{AZURITE_ACCOUNT}{}{path}", self.endpoint_path)
    }
}

fn build_auth(
    verb: &str,
    content_type: &str,
    date: &str,
    canonicalized_resource: &str,
) -> Result<String, String> {
    let string_to_sign =
        format!("{verb}\n\n{content_type}\n{date}\n{canonicalized_resource}");
    let key = STANDARD.decode(AZURITE_ACCOUNT_KEY).map_err(|e| e.to_string())?;
    let mut mac =
        Hmac::<Sha256>::new_from_slice(&key).map_err(|e| e.to_string())?;
    mac.update(string_to_sign.as_bytes());
    let sig = STANDARD.encode(mac.finalize().into_bytes());
    Ok(format!("SharedKey {AZURITE_ACCOUNT}:{sig}"))
}

fn extract_endpoint_path(endpoint: &str) -> &str {
    if let Some(after_scheme) = endpoint
        .strip_prefix("http://")
        .or_else(|| endpoint.strip_prefix("https://"))
    {
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
