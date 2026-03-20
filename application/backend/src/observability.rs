use std::sync::Arc;

use chrono::Utc;
use serde::Serialize;

use crate::config::AppConfig;

pub trait ObservabilitySink: Send + Sync {
    fn emit(&self, event: AppEvent);
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum AppEvent {
    PublicRequestServed {
        route: &'static str,
        slug: Option<String>,
    },
    AdminAuthChecked {
        route: &'static str,
        outcome: &'static str,
    },
    AiMetadataGenerated {
        slug: String,
        outcome: &'static str,
        source_model: Option<String>,
    },
    StaticSitePublished {
        target: String,
        pages: usize,
        assets: usize,
    },
    ContentError {
        operation: &'static str,
        error: String,
    },
}

pub struct NoopObservabilitySink;

pub struct StdoutObservabilitySink;

pub fn build_observability_sink(config: &AppConfig) -> Arc<dyn ObservabilitySink> {
    match config.observability_backend.as_str() {
        "noop" => Arc::new(NoopObservabilitySink),
        _ => Arc::new(StdoutObservabilitySink),
    }
}

impl ObservabilitySink for NoopObservabilitySink {
    fn emit(&self, _event: AppEvent) {}
}

impl ObservabilitySink for StdoutObservabilitySink {
    fn emit(&self, event: AppEvent) {
        let payload = serde_json::json!({
            "timestamp": Utc::now().to_rfc3339(),
            "payload": event,
        });
        println!("{payload}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_event_serializes_with_event_name() {
        let raw = serde_json::to_string(&AppEvent::StaticSitePublished {
            target: "azurite:site".to_owned(),
            pages: 2,
            assets: 3,
        })
        .unwrap();

        assert!(raw.contains("\"event\":\"static_site_published\""));
        assert!(raw.contains("\"pages\":2"));
    }
}
