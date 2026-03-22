use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::BlogError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CommentStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub post_slug: String,
    pub author_name: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub status: CommentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactMessage {
    pub id: String,
    pub from_name: String,
    pub from_email: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

#[async_trait]
pub trait CommentRepository: Send + Sync {
    async fn create_comment(&self, comment: &Comment) -> Result<(), BlogError>;
    async fn list_comments(
        &self,
        slug: &str,
        include_pending: bool,
    ) -> Result<Vec<Comment>, BlogError>;
    async fn list_all_pending(&self) -> Result<Vec<Comment>, BlogError>;
    async fn update_status(&self, id: &str, status: CommentStatus) -> Result<(), BlogError>;
}

#[async_trait]
pub trait ContactRepository: Send + Sync {
    async fn create_contact_message(&self, msg: &ContactMessage) -> Result<(), BlogError>;
}
