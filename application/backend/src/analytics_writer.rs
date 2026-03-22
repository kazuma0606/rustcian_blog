use std::sync::Arc;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::table::AzuriteTableClient;

const TABLE_PV: &str = "analyticspv";
const TABLE_QUERIES: &str = "analyticsqueries";
const TABLE_SESSIONS: &str = "analyticssessions";

/// Writes analytics events to Azure Table Storage (fire-and-forget).
/// Used by the backend to record page views, search queries, and reading sessions.
/// The analytics service reads from the same tables for aggregation.
#[derive(Clone)]
pub struct AnalyticsWriter {
    client: Arc<AzuriteTableClient>,
}

impl AnalyticsWriter {
    pub fn new(table_endpoint: impl Into<String>) -> Self {
        Self {
            client: Arc::new(AzuriteTableClient::new(table_endpoint)),
        }
    }

    /// Record a page view for `slug`. IP is hashed with a daily salt (no PII stored).
    pub fn record_page_view(&self, slug: String, ip: String) {
        let client = self.client.clone();
        tokio::spawn(async move {
            let date = Utc::now().format("%Y-%m-%d").to_string();
            let ip_hash = hash_ip(&ip, &date);
            let ts = Utc::now().timestamp_millis();
            let rk = format!("{slug}_{ts}");

            let entity = serde_json::json!({
                "PartitionKey": date,
                "RowKey": rk,
                "slug": slug,
                "ip_hash": ip_hash,
            });
            if let Err(e) = client.insert_entity(TABLE_PV, &entity).await {
                eprintln!("analytics pv write error: {e}");
            }
        });
    }

    /// Record a search query and its result count.
    pub fn record_search(&self, query: String, result_count: usize) {
        let client = self.client.clone();
        tokio::spawn(async move {
            let date = Utc::now().format("%Y-%m-%d").to_string();
            let ts = Utc::now().timestamp_millis();
            let rk = format!("{ts}");

            let entity = serde_json::json!({
                "PartitionKey": date,
                "RowKey": rk,
                "query": query,
                "result_count": result_count.to_string(),
            });
            if let Err(e) = client.insert_entity(TABLE_QUERIES, &entity).await {
                eprintln!("analytics query write error: {e}");
            }
        });
    }

    /// Record that `ip` visited `slug` (for co-reading graph).
    pub fn record_session_step(&self, slug: String, ip: String) {
        let client = self.client.clone();
        tokio::spawn(async move {
            let date = Utc::now().format("%Y-%m-%d").to_string();
            let ip_hash = hash_ip(&ip, &date);
            let pk = format!("{ip_hash}_{date}");
            let rk = Utc::now().timestamp_millis().to_string();

            let entity = serde_json::json!({
                "PartitionKey": pk,
                "RowKey": rk,
                "slug": slug,
            });
            if let Err(e) = client.insert_entity(TABLE_SESSIONS, &entity).await {
                eprintln!("analytics session write error: {e}");
            }
        });
    }
}

/// Hash IP with a daily rotating salt so the same IP looks different across days.
/// Returns a short hex string — no PII is stored.
fn hash_ip(ip: &str, date: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(ip.as_bytes());
    hasher.update(b"|");
    hasher.update(date.as_bytes());
    let result = hasher.finalize();
    // Take first 8 bytes → 11-character base64 (sufficient for deduplication)
    STANDARD.encode(&result[..8])
}
