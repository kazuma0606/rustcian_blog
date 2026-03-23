use std::collections::HashMap;
use std::path::Path;

use crate::table::AzuriteTableClient;

pub struct AnalyticsStats {
    pub total_pvs: usize,
    pub unique_ips: usize,
    pub total_searches: usize,
    /// Top posts by PV count, descending, capped at 10.
    pub top_posts: Vec<(String, usize)>,
    /// Top search queries by count, descending, capped at 10.
    pub top_queries: Vec<(String, usize)>,
    /// Where the data came from: "csv" | "table"
    pub source: String,
}

/// Read analytics. Priority: CSV files → Table Storage → None.
pub async fn read_analytics(
    csv_dir: &Path,
    table_client: Option<&AzuriteTableClient>,
) -> Option<AnalyticsStats> {
    let pv_csv = csv_dir.join("pv.csv");
    if pv_csv.exists() {
        return read_from_csv(csv_dir);
    }
    if let Some(client) = table_client {
        return read_from_table(client).await;
    }
    None
}

fn read_from_csv(csv_dir: &Path) -> Option<AnalyticsStats> {
    let pv_path = csv_dir.join("pv.csv");
    let queries_path = csv_dir.join("queries.csv");

    let pv_rows = read_csv(&pv_path).unwrap_or_default();
    let query_rows = read_csv(&queries_path).unwrap_or_default();

    let total_pvs = pv_rows.len();
    let unique_ips: std::collections::HashSet<&str> = pv_rows
        .iter()
        .filter_map(|r| r.get("ip_hash").map(|s| s.as_str()))
        .collect();

    let mut post_counts: HashMap<String, usize> = HashMap::new();
    for row in &pv_rows {
        if let Some(slug) = row.get("slug") {
            *post_counts.entry(slug.clone()).or_default() += 1;
        }
    }

    let mut query_counts: HashMap<String, usize> = HashMap::new();
    for row in &query_rows {
        if let Some(q) = row.get("query") {
            *query_counts.entry(q.clone()).or_default() += 1;
        }
    }

    let top_posts = top_n(post_counts, 10);
    let top_queries = top_n(query_counts, 10);

    Some(AnalyticsStats {
        total_pvs,
        unique_ips: unique_ips.len(),
        total_searches: query_rows.len(),
        top_posts,
        top_queries,
        source: "csv".to_owned(),
    })
}

async fn read_from_table(client: &AzuriteTableClient) -> Option<AnalyticsStats> {
    let pv_rows = client.query_entities("analyticspv", None).await.ok()?;
    if pv_rows.is_empty() {
        return None;
    }

    let total_pvs = pv_rows.len();
    let unique_ips: std::collections::HashSet<String> = pv_rows
        .iter()
        .filter_map(|r| r["ip_hash"].as_str().map(|s| s.to_owned()))
        .collect();

    let mut post_counts: HashMap<String, usize> = HashMap::new();
    for row in &pv_rows {
        if let Some(slug) = row["slug"].as_str() {
            *post_counts.entry(slug.to_owned()).or_default() += 1;
        }
    }

    let query_rows = client
        .query_entities("analyticsqueries", None)
        .await
        .unwrap_or_default();
    let total_searches = query_rows.len();
    let mut query_counts: HashMap<String, usize> = HashMap::new();
    for row in &query_rows {
        if let Some(q) = row["query"].as_str() {
            *query_counts.entry(q.to_owned()).or_default() += 1;
        }
    }

    Some(AnalyticsStats {
        total_pvs,
        unique_ips: unique_ips.len(),
        total_searches,
        top_posts: top_n(post_counts, 10),
        top_queries: top_n(query_counts, 10),
        source: "table".to_owned(),
    })
}

/// Parse a simple CSV file with a header row. Returns a vec of row maps.
fn read_csv(path: &Path) -> Option<Vec<HashMap<String, String>>> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut lines = content.lines();
    let header: Vec<&str> = lines.next()?.split(',').collect();
    let rows = lines
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            header
                .iter()
                .zip(line.splitn(header.len(), ','))
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        })
        .collect();
    Some(rows)
}

fn top_n(counts: HashMap<String, usize>, n: usize) -> Vec<(String, usize)> {
    let mut items: Vec<(String, usize)> = counts.into_iter().collect();
    items.sort_by(|a, b| b.1.cmp(&a.1));
    items.truncate(n);
    items
}
