use std::{collections::HashMap, sync::Arc};

use chrono::{Duration, Utc};
use serde::Serialize;

use crate::table::TableClient;

pub const TABLE_PV: &str = "analyticspv";
pub const TABLE_QUERIES: &str = "analyticsqueries";
pub const TABLE_SESSIONS: &str = "analyticssessions";

#[derive(Debug, Clone, Serialize)]
pub struct PageViewStat {
    pub slug: String,
    pub pv: usize,
    /// Estimated unique visitors (distinct ip_hash per day)
    pub unique: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub total_pv: usize,
    pub total_queries: usize,
    pub zero_hit_queries: usize,
    pub days: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchGap {
    pub query: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoReadEntry {
    pub slug: String,
    pub count: usize,
}

pub struct AnalyticsStore {
    client: Arc<TableClient>,
}

impl AnalyticsStore {
    pub fn new(client: Arc<TableClient>) -> Self {
        Self { client }
    }

    /// Initialize all analytics tables.
    pub async fn init_tables(&self) -> Result<(), String> {
        for table in [TABLE_PV, TABLE_QUERIES, TABLE_SESSIONS] {
            self.client.create_table_if_needed(table).await?;
        }
        Ok(())
    }

    /// Top pages by PV over the last `days` days.
    pub async fn popular(&self, days: u32, limit: usize) -> Result<Vec<PageViewStat>, String> {
        let dates = date_range(days);
        let filter = date_filter(&dates);
        let rows = self.client.query_entities(TABLE_PV, Some(&filter)).await?;

        // Aggregate: pv per slug, unique = distinct ip_hash per (slug, date)
        let mut pv_map: HashMap<String, usize> = HashMap::new();
        let mut unique_map: HashMap<String, std::collections::HashSet<String>> = HashMap::new();

        for row in &rows {
            let slug = row["slug"].as_str().unwrap_or("").to_owned();
            let ip_hash = row["ip_hash"].as_str().unwrap_or("").to_owned();
            if slug.is_empty() {
                continue;
            }
            *pv_map.entry(slug.clone()).or_insert(0) += 1;
            unique_map.entry(slug).or_default().insert(ip_hash);
        }

        let mut stats: Vec<PageViewStat> = pv_map
            .into_iter()
            .map(|(slug, pv)| {
                let unique = unique_map.get(&slug).map(|s| s.len()).unwrap_or(0);
                PageViewStat { slug, pv, unique }
            })
            .collect();
        stats.sort_by(|a, b| b.pv.cmp(&a.pv));
        stats.truncate(limit);
        Ok(stats)
    }

    /// Overall summary stats over `days` days.
    pub async fn summary(&self, days: u32) -> Result<Summary, String> {
        let dates = date_range(days);
        let filter = date_filter(&dates);

        let pv_rows = self.client.query_entities(TABLE_PV, Some(&filter)).await?;
        let q_rows = self.client.query_entities(TABLE_QUERIES, Some(&filter)).await?;

        let zero_hit = q_rows
            .iter()
            .filter(|r| r["result_count"].as_str().unwrap_or("1") == "0")
            .count();

        Ok(Summary {
            total_pv: pv_rows.len(),
            total_queries: q_rows.len(),
            zero_hit_queries: zero_hit,
            days,
        })
    }

    /// Search queries with zero results over `days` days, sorted by frequency.
    pub async fn gaps(&self, days: u32) -> Result<Vec<SearchGap>, String> {
        let dates = date_range(days);
        let filter = date_filter(&dates);
        let rows = self.client.query_entities(TABLE_QUERIES, Some(&filter)).await?;

        let mut counts: HashMap<String, usize> = HashMap::new();
        for row in &rows {
            if row["result_count"].as_str().unwrap_or("1") == "0" {
                let query = row["query"].as_str().unwrap_or("").trim().to_lowercase();
                if !query.is_empty() {
                    *counts.entry(query).or_insert(0) += 1;
                }
            }
        }

        let mut gaps: Vec<SearchGap> = counts
            .into_iter()
            .map(|(query, count)| SearchGap { query, count })
            .collect();
        gaps.sort_by(|a, b| b.count.cmp(&a.count));
        Ok(gaps)
    }

    /// Pages co-read with `slug` (appeared in same session), sorted by frequency.
    pub async fn coread(&self, slug: &str) -> Result<Vec<CoReadEntry>, String> {
        // Find all sessions that contain this slug
        let all_rows = self.client.query_entities(TABLE_SESSIONS, None).await?;

        // Group rows by session (PartitionKey)
        let mut sessions: HashMap<String, Vec<String>> = HashMap::new();
        for row in &all_rows {
            let pk = row["PartitionKey"].as_str().unwrap_or("").to_owned();
            let row_slug = row["slug"].as_str().unwrap_or("").to_owned();
            if !pk.is_empty() && !row_slug.is_empty() {
                sessions.entry(pk).or_default().push(row_slug);
            }
        }

        // Count co-occurrences
        let mut counts: HashMap<String, usize> = HashMap::new();
        for slugs in sessions.values() {
            if slugs.contains(&slug.to_owned()) {
                for other in slugs {
                    if other != slug {
                        *counts.entry(other.clone()).or_insert(0) += 1;
                    }
                }
            }
        }

        let mut result: Vec<CoReadEntry> = counts
            .into_iter()
            .map(|(s, count)| CoReadEntry { slug: s, count })
            .collect();
        result.sort_by(|a, b| b.count.cmp(&a.count));
        Ok(result)
    }
}

/// Generate a list of `YYYY-MM-DD` strings for the last `days` days.
fn date_range(days: u32) -> Vec<String> {
    let today = Utc::now().date_naive();
    (0..days as i64)
        .map(|i| (today - Duration::days(i)).format("%Y-%m-%d").to_string())
        .collect()
}

/// Build an OData filter expression matching any of the given PartitionKey dates.
fn date_filter(dates: &[String]) -> String {
    dates
        .iter()
        .map(|d| format!("PartitionKey eq '{d}'"))
        .collect::<Vec<_>>()
        .join(" or ")
}
