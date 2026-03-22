use std::sync::Arc;

use chrono::Utc;
use serde::Serialize;
use serde_json::{Value, json};

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

// ---------------------------------------------------------------------------
// Noop
// ---------------------------------------------------------------------------

pub struct NoopObservabilitySink;

impl ObservabilitySink for NoopObservabilitySink {
    fn emit(&self, _event: AppEvent) {}
}

// ---------------------------------------------------------------------------
// Stdout (local dev default)
// ---------------------------------------------------------------------------

pub struct StdoutObservabilitySink;

impl ObservabilitySink for StdoutObservabilitySink {
    fn emit(&self, event: AppEvent) {
        let payload = json!({
            "timestamp": Utc::now().to_rfc3339(),
            "payload": event,
        });
        println!("{payload}");
    }
}

// ---------------------------------------------------------------------------
// Application Insights (Azure Monitor Track API)
// ---------------------------------------------------------------------------

pub struct ApplicationInsightsObservabilitySink {
    client: Arc<reqwest::Client>,
    endpoint: String,
    instrumentation_key: String,
}

impl ApplicationInsightsObservabilitySink {
    /// Build from an Application Insights connection string.
    /// Returns `None` if the string is missing or has no `InstrumentationKey`.
    pub fn from_connection_string(connection_string: &str) -> Option<Self> {
        let key = parse_cs_field(connection_string, "InstrumentationKey")?;
        let ingestion = parse_cs_field(connection_string, "IngestionEndpoint")
            .unwrap_or_else(|| "https://dc.services.visualstudio.com/".to_owned());
        let endpoint = format!("{}/v2/track", ingestion.trim_end_matches('/'));
        Some(Self {
            client: Arc::new(reqwest::Client::new()),
            endpoint,
            instrumentation_key: key,
        })
    }
}

impl ObservabilitySink for ApplicationInsightsObservabilitySink {
    fn emit(&self, event: AppEvent) {
        let body = build_telemetry_payload(&event, &self.instrumentation_key);
        let client = self.client.clone();
        let endpoint = self.endpoint.clone();
        tokio::spawn(async move {
            let _ = client.post(&endpoint).json(&body).send().await;
        });
    }
}

/// Parse a single `Key=Value` field from a semicolon-delimited connection string.
fn parse_cs_field(connection_string: &str, field: &str) -> Option<String> {
    connection_string.split(';').find_map(|part| {
        let (k, v) = part.split_once('=')?;
        if k.trim().eq_ignore_ascii_case(field) {
            Some(v.trim().to_owned())
        } else {
            None
        }
    })
}

/// Build the JSON array that the Application Insights Track API (`/v2/track`) expects.
/// All `AppEvent` variants are mapped to `EventData` (custom event telemetry).
pub fn build_telemetry_payload(event: &AppEvent, instrumentation_key: &str) -> Value {
    let (event_name, properties) = event_to_name_and_props(event);
    let envelope_name = format!(
        "Microsoft.ApplicationInsights.{}.Event",
        instrumentation_key.replace('-', "")
    );
    json!([{
        "name": envelope_name,
        "time": Utc::now().to_rfc3339(),
        "iKey": instrumentation_key,
        "data": {
            "baseType": "EventData",
            "baseData": {
                "ver": 2,
                "name": event_name,
                "properties": properties,
            }
        }
    }])
}

fn event_to_name_and_props(event: &AppEvent) -> (&'static str, serde_json::Map<String, Value>) {
    let mut props = serde_json::Map::new();
    let name = match event {
        AppEvent::PublicRequestServed { route, slug } => {
            props.insert("route".to_owned(), json!(route));
            if let Some(s) = slug {
                props.insert("slug".to_owned(), json!(s));
            }
            "public_request_served"
        }
        AppEvent::AdminAuthChecked { route, outcome } => {
            props.insert("route".to_owned(), json!(route));
            props.insert("outcome".to_owned(), json!(outcome));
            "admin_auth_checked"
        }
        AppEvent::AiMetadataGenerated {
            slug,
            outcome,
            source_model,
        } => {
            props.insert("slug".to_owned(), json!(slug));
            props.insert("outcome".to_owned(), json!(outcome));
            if let Some(m) = source_model {
                props.insert("source_model".to_owned(), json!(m));
            }
            "ai_metadata_generated"
        }
        AppEvent::StaticSitePublished {
            target,
            pages,
            assets,
        } => {
            props.insert("target".to_owned(), json!(target));
            props.insert("pages".to_owned(), json!(pages.to_string()));
            props.insert("assets".to_owned(), json!(assets.to_string()));
            "static_site_published"
        }
        AppEvent::ContentError { operation, error } => {
            props.insert("operation".to_owned(), json!(operation));
            props.insert("error".to_owned(), json!(error));
            "content_error"
        }
    };
    (name, props)
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

pub fn build_observability_sink(config: &AppConfig) -> Arc<dyn ObservabilitySink> {
    match config.observability_backend.as_str() {
        "noop" => Arc::new(NoopObservabilitySink),
        "appinsights" => config
            .application_insights_connection_string
            .as_deref()
            .and_then(ApplicationInsightsObservabilitySink::from_connection_string)
            .map(|s| Arc::new(s) as Arc<dyn ObservabilitySink>)
            .unwrap_or_else(|| {
                eprintln!(
                    "warn: OBSERVABILITY_BACKEND=appinsights but \
                     APPLICATIONINSIGHTS_CONNECTION_STRING is missing or invalid; \
                     falling back to stdout"
                );
                Arc::new(StdoutObservabilitySink)
            }),
        _ => Arc::new(StdoutObservabilitySink),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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

    #[test]
    fn parse_cs_field_extracts_instrumentation_key() {
        let cs = "InstrumentationKey=abc-123-def;\
                  IngestionEndpoint=https://eastus.in.applicationinsights.azure.com/;\
                  LiveEndpoint=https://eastus.livediagnostics.monitor.azure.com/";
        assert_eq!(
            parse_cs_field(cs, "InstrumentationKey").as_deref(),
            Some("abc-123-def")
        );
    }

    #[test]
    fn parse_cs_field_extracts_ingestion_endpoint() {
        let cs = "InstrumentationKey=k;IngestionEndpoint=https://example.com/;Other=x";
        assert_eq!(
            parse_cs_field(cs, "IngestionEndpoint").as_deref(),
            Some("https://example.com/")
        );
    }

    #[test]
    fn parse_cs_field_returns_none_for_missing_key() {
        let cs = "InstrumentationKey=k";
        assert!(parse_cs_field(cs, "IngestionEndpoint").is_none());
    }

    #[test]
    fn build_telemetry_payload_static_site_published() {
        let event = AppEvent::StaticSitePublished {
            target: "azurite:site".to_owned(),
            pages: 5,
            assets: 10,
        };
        let payload = build_telemetry_payload(&event, "test-ikey-999");

        assert_eq!(payload[0]["iKey"], "test-ikey-999");
        assert_eq!(payload[0]["data"]["baseType"], "EventData");
        assert_eq!(
            payload[0]["data"]["baseData"]["name"],
            "static_site_published"
        );
        assert_eq!(payload[0]["data"]["baseData"]["properties"]["pages"], "5");
        assert_eq!(payload[0]["data"]["baseData"]["properties"]["assets"], "10");
        assert_eq!(
            payload[0]["data"]["baseData"]["properties"]["target"],
            "azurite:site"
        );
    }

    #[test]
    fn build_telemetry_payload_public_request_served_with_slug() {
        let event = AppEvent::PublicRequestServed {
            route: "post_page",
            slug: Some("hello-rust".to_owned()),
        };
        let payload = build_telemetry_payload(&event, "ikey");

        assert_eq!(
            payload[0]["data"]["baseData"]["name"],
            "public_request_served"
        );
        assert_eq!(
            payload[0]["data"]["baseData"]["properties"]["route"],
            "post_page"
        );
        assert_eq!(
            payload[0]["data"]["baseData"]["properties"]["slug"],
            "hello-rust"
        );
    }

    #[test]
    fn build_telemetry_payload_public_request_served_without_slug() {
        let event = AppEvent::PublicRequestServed {
            route: "index_page",
            slug: None,
        };
        let payload = build_telemetry_payload(&event, "ikey");

        assert!(payload[0]["data"]["baseData"]["properties"]["slug"].is_null());
    }

    #[test]
    fn build_telemetry_payload_ai_metadata_generated() {
        let event = AppEvent::AiMetadataGenerated {
            slug: "my-post".to_owned(),
            outcome: "success",
            source_model: Some("gpt-4o".to_owned()),
        };
        let payload = build_telemetry_payload(&event, "ikey");

        assert_eq!(
            payload[0]["data"]["baseData"]["name"],
            "ai_metadata_generated"
        );
        assert_eq!(
            payload[0]["data"]["baseData"]["properties"]["source_model"],
            "gpt-4o"
        );
    }

    #[test]
    fn build_telemetry_payload_content_error() {
        let event = AppEvent::ContentError {
            operation: "load_post",
            error: "file not found".to_owned(),
        };
        let payload = build_telemetry_payload(&event, "ikey");

        assert_eq!(payload[0]["data"]["baseData"]["name"], "content_error");
        assert_eq!(
            payload[0]["data"]["baseData"]["properties"]["operation"],
            "load_post"
        );
    }

    #[test]
    fn appinsights_sink_is_built_from_valid_connection_string() {
        let cs = "InstrumentationKey=abc-123;\
                  IngestionEndpoint=https://eastus.in.applicationinsights.azure.com/";
        let sink = ApplicationInsightsObservabilitySink::from_connection_string(cs);
        assert!(sink.is_some());
        let sink = sink.unwrap();
        assert_eq!(sink.instrumentation_key, "abc-123");
        assert_eq!(
            sink.endpoint,
            "https://eastus.in.applicationinsights.azure.com/v2/track"
        );
    }

    #[test]
    fn appinsights_sink_returns_none_for_missing_instrumentation_key() {
        let cs = "IngestionEndpoint=https://example.com/";
        assert!(ApplicationInsightsObservabilitySink::from_connection_string(cs).is_none());
    }

    #[test]
    fn appinsights_sink_falls_back_to_default_ingestion_endpoint() {
        let cs = "InstrumentationKey=mykey";
        let sink = ApplicationInsightsObservabilitySink::from_connection_string(cs).unwrap();
        assert_eq!(
            sink.endpoint,
            "https://dc.services.visualstudio.com/v2/track"
        );
    }
}
