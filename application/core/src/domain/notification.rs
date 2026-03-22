use async_trait::async_trait;

use crate::domain::error::BlogError;

#[async_trait]
pub trait NotificationSink: Send + Sync {
    async fn notify(&self, event: NotificationEvent) -> Result<(), BlogError>;
}

#[derive(Debug, Clone)]
pub enum NotificationEvent {
    StaticSiteRebuilt {
        page_count: usize,
        outcome: String,
    },
    CommentReceived {
        slug: String,
        author_name: String,
    },
    ContactFormSubmitted {
        from_name: String,
    },
    AiMetadataGenerated {
        slug: String,
        outcome: String,
    },
}
