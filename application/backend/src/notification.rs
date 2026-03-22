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
// Factory
// ---------------------------------------------------------------------------

pub fn build_notification_sink(config: &AppConfig) -> Arc<dyn NotificationSink> {
    match &config.slack_webhook_url {
        Some(url) => Arc::new(SlackNotificationSink::new(url.clone())),
        None => Arc::new(NoopNotificationSink),
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
}
