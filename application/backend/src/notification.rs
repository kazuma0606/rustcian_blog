use std::sync::Arc;

use async_trait::async_trait;
use rustacian_blog_core::{BlogError, NotificationEvent, NotificationSink};

use crate::config::AppConfig;

// ---------------------------------------------------------------------------
// Noop
// ---------------------------------------------------------------------------

pub struct NoopNotificationSink;

#[async_trait]
impl NotificationSink for NoopNotificationSink {
    async fn notify(&self, _event: NotificationEvent) -> Result<(), BlogError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Slack
// ---------------------------------------------------------------------------

pub struct SlackNotificationSink {
    webhook_url: String,
    client: reqwest::Client,
}

impl SlackNotificationSink {
    pub fn new(webhook_url: String) -> Self {
        Self {
            webhook_url,
            client: reqwest::Client::new(),
        }
    }

    fn format_message(event: &NotificationEvent) -> String {
        match event {
            NotificationEvent::StaticSiteRebuilt {
                page_count,
                outcome,
            } => {
                format!("🔨 静的サイトを再生成しました（{page_count} ページ, outcome: {outcome}）")
            }
            NotificationEvent::CommentReceived { slug, author_name } => {
                format!("💬 新しいコメントが届きました — 記事: `{slug}`, 投稿者: {author_name}")
            }
            NotificationEvent::ContactFormSubmitted { from_name } => {
                format!("📩 お問い合わせが届きました — 送信者: {from_name}")
            }
            NotificationEvent::AiMetadataGenerated { slug, outcome } => {
                format!("🤖 AI メタデータを生成しました — 記事: `{slug}`, outcome: {outcome}")
            }
        }
    }
}

#[async_trait]
impl NotificationSink for SlackNotificationSink {
    async fn notify(&self, event: NotificationEvent) -> Result<(), BlogError> {
        let text = Self::format_message(&event);
        let body = serde_json::json!({ "text": text });
        self.client
            .post(&self.webhook_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| BlogError::Storage(e.to_string()))?
            .error_for_status()
            .map_err(|e| BlogError::Storage(e.to_string()))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ACS Email
// ---------------------------------------------------------------------------

pub struct AcsEmailNotificationSink {
    endpoint: String,
    access_key_bytes: Vec<u8>,
    sender_address: String,
    recipient_address: String,
    client: reqwest::Client,
}

impl AcsEmailNotificationSink {
    pub fn new(
        endpoint: String,
        access_key: &str,
        sender_address: String,
        recipient_address: String,
    ) -> Result<Self, BlogError> {
        use base64::{Engine, engine::general_purpose::STANDARD};
        let access_key_bytes = STANDARD
            .decode(access_key)
            .map_err(|e| BlogError::Storage(format!("invalid ACS access key: {e}")))?;
        Ok(Self {
            endpoint: endpoint.trim_end_matches('/').to_owned(),
            access_key_bytes,
            sender_address,
            recipient_address,
            client: reqwest::Client::new(),
        })
    }

    fn format_subject(event: &NotificationEvent) -> String {
        match event {
            NotificationEvent::StaticSiteRebuilt { .. } => "静的サイト再生成完了".to_owned(),
            NotificationEvent::CommentReceived { slug, .. } => {
                format!("新しいコメント — {slug}")
            }
            NotificationEvent::ContactFormSubmitted { from_name } => {
                format!("お問い合わせ — {from_name}")
            }
            NotificationEvent::AiMetadataGenerated { slug, .. } => {
                format!("AI メタデータ生成完了 — {slug}")
            }
        }
    }

    async fn send_email(&self, subject: &str, body_text: &str) -> Result<(), BlogError> {
        use base64::{Engine, engine::general_purpose::STANDARD};
        use hmac::{Hmac, Mac};
        use sha2::{Digest, Sha256};

        type HmacSha256 = Hmac<Sha256>;

        let api_version = "2023-03-31";
        let raw_url = format!("{}/emails:send?api-version={}", self.endpoint, api_version);
        let url = reqwest::Url::parse(&raw_url)
            .map_err(|e| BlogError::Storage(format!("invalid ACS endpoint: {e}")))?;

        let payload = serde_json::json!({
            "senderAddress": self.sender_address,
            "content": {
                "subject": subject,
                "plainText": body_text,
            },
            "recipients": {
                "to": [{"address": self.recipient_address}]
            }
        });
        let body_bytes =
            serde_json::to_vec(&payload).map_err(|e| BlogError::Storage(e.to_string()))?;

        let date = chrono::Utc::now()
            .format("%a, %d %b %Y %H:%M:%S GMT")
            .to_string();
        let content_hash = STANDARD.encode(Sha256::digest(&body_bytes));
        let host_header = match url.port() {
            Some(p) => format!("{}:{}", url.host_str().unwrap_or(""), p),
            None => url.host_str().unwrap_or("").to_owned(),
        };
        let path_and_query = match url.query() {
            Some(q) => format!("{}?{}", url.path(), q),
            None => url.path().to_owned(),
        };
        let string_to_sign = format!("POST\n{path_and_query}\n{date};{host_header};{content_hash}");

        let mut mac = HmacSha256::new_from_slice(&self.access_key_bytes)
            .map_err(|e| BlogError::Storage(format!("HMAC init error: {e}")))?;
        mac.update(string_to_sign.as_bytes());
        let signature = STANDARD.encode(mac.finalize().into_bytes());

        let authorization = format!(
            "HMAC-SHA256 SignedHeaders=x-ms-date;host;x-ms-content-sha256&Signature={signature}"
        );

        self.client
            .post(url)
            .header("Content-Type", "application/json")
            .header("x-ms-date", &date)
            .header("x-ms-content-sha256", &content_hash)
            .header("host", &host_header)
            .header("Authorization", authorization)
            .body(body_bytes)
            .send()
            .await
            .map_err(|e| BlogError::Storage(e.to_string()))?
            .error_for_status()
            .map_err(|e| BlogError::Storage(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl NotificationSink for AcsEmailNotificationSink {
    async fn notify(&self, event: NotificationEvent) -> Result<(), BlogError> {
        let subject = Self::format_subject(&event);
        let body = SlackNotificationSink::format_message(&event);
        self.send_email(&subject, &body).await
    }
}

// ---------------------------------------------------------------------------
// Multi (fan-out)
// ---------------------------------------------------------------------------

pub struct MultiNotificationSink {
    sinks: Vec<Arc<dyn NotificationSink>>,
}

impl MultiNotificationSink {
    pub fn new(sinks: Vec<Arc<dyn NotificationSink>>) -> Self {
        Self { sinks }
    }
}

#[async_trait]
impl NotificationSink for MultiNotificationSink {
    async fn notify(&self, event: NotificationEvent) -> Result<(), BlogError> {
        for sink in &self.sinks {
            if let Err(e) = sink.notify(event.clone()).await {
                eprintln!("notification sink error: {e}");
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

pub fn build_notification_sink(config: &AppConfig) -> Arc<dyn NotificationSink> {
    let slack: Option<Arc<dyn NotificationSink>> = config
        .slack_webhook_url
        .as_ref()
        .map(|url| Arc::new(SlackNotificationSink::new(url.clone())) as Arc<dyn NotificationSink>);

    let acs: Option<Arc<dyn NotificationSink>> = (|| {
        let endpoint = config.acs_endpoint.as_ref()?;
        let key = config.acs_access_key.as_ref()?;
        let sender = config.acs_sender_address.as_ref()?;
        let recipient = config.acs_recipient_address.as_ref()?;
        AcsEmailNotificationSink::new(endpoint.clone(), key, sender.clone(), recipient.clone())
            .ok()
            .map(|s| Arc::new(s) as Arc<dyn NotificationSink>)
    })();

    match (slack, acs) {
        (Some(s), Some(a)) => Arc::new(MultiNotificationSink::new(vec![s, a])),
        (Some(s), None) => s,
        (None, Some(a)) => a,
        (None, None) => Arc::new(NoopNotificationSink),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_site_rebuilt_message() {
        let msg = SlackNotificationSink::format_message(&NotificationEvent::StaticSiteRebuilt {
            page_count: 12,
            outcome: "success".to_owned(),
        });
        assert!(msg.contains("12"));
        assert!(msg.contains("success"));
    }

    #[test]
    fn comment_received_message() {
        let msg = SlackNotificationSink::format_message(&NotificationEvent::CommentReceived {
            slug: "hello-world".to_owned(),
            author_name: "Alice".to_owned(),
        });
        assert!(msg.contains("hello-world"));
        assert!(msg.contains("Alice"));
    }

    #[test]
    fn contact_form_submitted_message() {
        let msg = SlackNotificationSink::format_message(&NotificationEvent::ContactFormSubmitted {
            from_name: "Bob".to_owned(),
        });
        assert!(msg.contains("Bob"));
    }

    #[test]
    fn ai_metadata_generated_message() {
        let msg = SlackNotificationSink::format_message(&NotificationEvent::AiMetadataGenerated {
            slug: "rust-intro".to_owned(),
            outcome: "success".to_owned(),
        });
        assert!(msg.contains("rust-intro"));
        assert!(msg.contains("success"));
    }

    // -----------------------------------------------------------------------
    // ACS
    // -----------------------------------------------------------------------

    #[test]
    fn acs_rejects_invalid_base64_key() {
        let result = AcsEmailNotificationSink::new(
            "https://example.communication.azure.com".to_owned(),
            "not-valid-base64!!!",
            "sender@example.com".to_owned(),
            "recipient@example.com".to_owned(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn acs_subject_covers_all_events() {
        let cases = [
            (
                NotificationEvent::StaticSiteRebuilt {
                    page_count: 5,
                    outcome: "ok".to_owned(),
                },
                "再生成",
            ),
            (
                NotificationEvent::CommentReceived {
                    slug: "post-slug".to_owned(),
                    author_name: "Alice".to_owned(),
                },
                "post-slug",
            ),
            (
                NotificationEvent::ContactFormSubmitted {
                    from_name: "Bob".to_owned(),
                },
                "Bob",
            ),
            (
                NotificationEvent::AiMetadataGenerated {
                    slug: "my-post".to_owned(),
                    outcome: "ok".to_owned(),
                },
                "my-post",
            ),
        ];
        for (event, expected) in cases {
            assert!(
                AcsEmailNotificationSink::format_subject(&event).contains(expected),
                "subject missing '{expected}'"
            );
        }
    }

    #[tokio::test]
    async fn acs_sends_correct_hmac_signed_request() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/emails:send"))
            .respond_with(ResponseTemplate::new(202))
            .expect(1)
            .mount(&mock_server)
            .await;

        // ACS access keys are base64-encoded 32-byte values.
        use base64::{Engine, engine::general_purpose::STANDARD};
        let key_bytes = vec![0u8; 32];
        let key_b64 = STANDARD.encode(&key_bytes);

        let sink = AcsEmailNotificationSink::new(
            mock_server.uri(),
            &key_b64,
            "DoNotReply@example.com".to_owned(),
            "admin@example.com".to_owned(),
        )
        .expect("should build sink");

        sink.send_email("Test Subject", "Test body").await.unwrap();

        // wiremock verifies that exactly 1 matching request was received.
        mock_server.verify().await;
    }

    #[tokio::test]
    async fn acs_request_contains_required_hmac_headers() {
        use wiremock::matchers::{header_exists, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/emails:send"))
            .and(header_exists("x-ms-date"))
            .and(header_exists("x-ms-content-sha256"))
            .and(header_exists("Authorization"))
            .respond_with(ResponseTemplate::new(202))
            .expect(1)
            .mount(&mock_server)
            .await;

        use base64::{Engine, engine::general_purpose::STANDARD};
        let key_b64 = STANDARD.encode(vec![1u8; 32]);

        let sink = AcsEmailNotificationSink::new(
            mock_server.uri(),
            &key_b64,
            "DoNotReply@example.com".to_owned(),
            "admin@example.com".to_owned(),
        )
        .unwrap();

        let event = NotificationEvent::CommentReceived {
            slug: "rust-post".to_owned(),
            author_name: "Yoshi".to_owned(),
        };
        sink.notify(event).await.unwrap();

        mock_server.verify().await;
    }

    // -----------------------------------------------------------------------
    // Multi
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn multi_sink_calls_all_sinks() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct CountingSink(Arc<AtomicUsize>);

        #[async_trait]
        impl NotificationSink for CountingSink {
            async fn notify(&self, _: NotificationEvent) -> Result<(), BlogError> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        }

        let counter = Arc::new(AtomicUsize::new(0));
        let multi = MultiNotificationSink::new(vec![
            Arc::new(CountingSink(counter.clone())),
            Arc::new(CountingSink(counter.clone())),
        ]);

        multi
            .notify(NotificationEvent::ContactFormSubmitted {
                from_name: "Test".to_owned(),
            })
            .await
            .unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn multi_sink_continues_after_error() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct FailSink;
        #[async_trait]
        impl NotificationSink for FailSink {
            async fn notify(&self, _: NotificationEvent) -> Result<(), BlogError> {
                Err(BlogError::Storage("forced error".to_owned()))
            }
        }

        struct CountingSink(Arc<AtomicUsize>);
        #[async_trait]
        impl NotificationSink for CountingSink {
            async fn notify(&self, _: NotificationEvent) -> Result<(), BlogError> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        }

        let counter = Arc::new(AtomicUsize::new(0));
        let multi = MultiNotificationSink::new(vec![
            Arc::new(FailSink),
            Arc::new(CountingSink(counter.clone())),
        ]);

        // MultiSink should succeed even if one sink fails.
        multi
            .notify(NotificationEvent::ContactFormSubmitted {
                from_name: "Test".to_owned(),
            })
            .await
            .unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
