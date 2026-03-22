use reqwest::Client;
use rustacian_blog_core::BlogError;

/// Cloudflare Cache API client.
/// Purges all cached files in a zone after static site publication.
#[derive(Debug, Clone)]
pub struct CloudflareCacheClient {
    client: Client,
    zone_id: String,
    api_token: String,
}

impl CloudflareCacheClient {
    pub fn new(client: Client, zone_id: String, api_token: String) -> Self {
        Self {
            client,
            zone_id,
            api_token,
        }
    }

    /// Purge everything cached in the zone.
    /// Cloudflare API: POST /zones/{zone_id}/purge_cache with {"purge_everything": true}
    pub async fn purge_all(&self) -> Result<(), BlogError> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/zones/{}/purge_cache",
            self.zone_id
        );
        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.api_token)
            .json(&serde_json::json!({"purge_everything": true}))
            .send()
            .await
            .map_err(|e| BlogError::Storage(format!("cloudflare request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BlogError::Storage(format!(
                "cloudflare purge failed ({status}): {body}"
            )));
        }

        Ok(())
    }
}

/// Build a `CloudflareCacheClient` from config, returning `None` when either
/// `cloudflare_zone_id` or `cloudflare_api_token` is absent.
pub fn build_cloudflare_cache_client(
    client: &Client,
    zone_id: Option<&str>,
    api_token: Option<&str>,
) -> Option<CloudflareCacheClient> {
    let zone_id = zone_id?.to_owned();
    let api_token = api_token?.to_owned();
    Some(CloudflareCacheClient::new(
        client.clone(),
        zone_id,
        api_token,
    ))
}
