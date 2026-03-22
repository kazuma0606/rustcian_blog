use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use rustacian_blog_core::{
    BlogError, Comment, CommentRepository, CommentStatus, ContactMessage, ContactRepository,
};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{config::AppConfig, table::AzuriteTableClient};

const COMMENTS_TABLE: &str = "comments";
const CONTACTS_TABLE: &str = "contacts";

// ---------------------------------------------------------------------------
// ID helper
// ---------------------------------------------------------------------------

pub fn new_id() -> String {
    Uuid::new_v4().to_string()
}

// ---------------------------------------------------------------------------
// In-memory (local storage backend)
// ---------------------------------------------------------------------------

pub struct InMemoryCommentRepository {
    data: Arc<RwLock<Vec<Comment>>>,
}

impl Default for InMemoryCommentRepository {
    fn default() -> Self {
        Self { data: Arc::new(RwLock::new(Vec::new())) }
    }
}

#[async_trait]
impl CommentRepository for InMemoryCommentRepository {
    async fn create_comment(&self, comment: &Comment) -> Result<(), BlogError> {
        self.data.write().await.push(comment.clone());
        Ok(())
    }

    async fn list_comments(
        &self,
        slug: &str,
        include_pending: bool,
    ) -> Result<Vec<Comment>, BlogError> {
        let data = self.data.read().await;
        Ok(data
            .iter()
            .filter(|c| {
                c.post_slug == slug
                    && (include_pending || matches!(c.status, CommentStatus::Approved))
            })
            .cloned()
            .collect())
    }

    async fn list_all_pending(&self) -> Result<Vec<Comment>, BlogError> {
        let data = self.data.read().await;
        Ok(data
            .iter()
            .filter(|c| matches!(c.status, CommentStatus::Pending))
            .cloned()
            .collect())
    }

    async fn update_status(&self, id: &str, status: CommentStatus) -> Result<(), BlogError> {
        let mut data = self.data.write().await;
        data.iter_mut()
            .find(|c| c.id == id)
            .map(|c| c.status = status)
            .ok_or_else(|| BlogError::NotFound(id.to_owned()))
    }
}

pub struct InMemoryContactRepository {
    data: Arc<RwLock<Vec<ContactMessage>>>,
}

impl Default for InMemoryContactRepository {
    fn default() -> Self {
        Self { data: Arc::new(RwLock::new(Vec::new())) }
    }
}

#[async_trait]
impl ContactRepository for InMemoryContactRepository {
    async fn create_contact_message(&self, msg: &ContactMessage) -> Result<(), BlogError> {
        self.data.write().await.push(msg.clone());
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Azurite Table Storage
// ---------------------------------------------------------------------------

pub struct AzuriteCommentRepository {
    table: AzuriteTableClient,
}

impl AzuriteCommentRepository {
    pub fn new(table_endpoint: String) -> Self {
        Self { table: AzuriteTableClient::new(table_endpoint) }
    }

    pub async fn init(&self) -> Result<(), BlogError> {
        self.table.create_table_if_needed(COMMENTS_TABLE).await
    }
}

#[async_trait]
impl CommentRepository for AzuriteCommentRepository {
    async fn create_comment(&self, comment: &Comment) -> Result<(), BlogError> {
        let entity = serde_json::json!({
            "PartitionKey": comment.post_slug,
            "RowKey": comment.id,
            "author_name": comment.author_name,
            "content": comment.content,
            "created_at": comment.created_at.to_rfc3339(),
            "status": format!("{:?}", comment.status),
        });
        self.table.insert_entity(COMMENTS_TABLE, &entity).await
    }

    async fn list_comments(
        &self,
        slug: &str,
        include_pending: bool,
    ) -> Result<Vec<Comment>, BlogError> {
        let filter = if include_pending {
            format!("PartitionKey eq '{slug}'")
        } else {
            format!("PartitionKey eq '{slug}' and status eq 'Approved'")
        };
        let rows = self.table.query_entities(COMMENTS_TABLE, Some(&filter)).await?;
        rows.iter().map(row_to_comment).collect()
    }

    async fn list_all_pending(&self) -> Result<Vec<Comment>, BlogError> {
        let rows = self
            .table
            .query_entities(COMMENTS_TABLE, Some("status eq 'Pending'"))
            .await?;
        rows.iter().map(row_to_comment).collect()
    }

    async fn update_status(&self, id: &str, status: CommentStatus) -> Result<(), BlogError> {
        // Scan to find the entity's PartitionKey, then upsert.
        let rows = self
            .table
            .query_entities(COMMENTS_TABLE, Some(&format!("RowKey eq '{id}'")))
            .await?;
        let row = rows.first().ok_or_else(|| BlogError::NotFound(id.to_owned()))?;
        let mut updated = row.clone();
        updated["status"] = serde_json::json!(format!("{status:?}"));
        let pk = row["PartitionKey"].as_str().unwrap_or_default().to_owned();
        self.table.upsert_entity(COMMENTS_TABLE, &pk, id, &updated).await
    }
}

pub struct AzuriteContactRepository {
    table: AzuriteTableClient,
}

impl AzuriteContactRepository {
    pub fn new(table_endpoint: String) -> Self {
        Self { table: AzuriteTableClient::new(table_endpoint) }
    }

    pub async fn init(&self) -> Result<(), BlogError> {
        self.table.create_table_if_needed(CONTACTS_TABLE).await
    }
}

#[async_trait]
impl ContactRepository for AzuriteContactRepository {
    async fn create_contact_message(&self, msg: &ContactMessage) -> Result<(), BlogError> {
        let entity = serde_json::json!({
            "PartitionKey": "contact",
            "RowKey": msg.id,
            "from_name": msg.from_name,
            "from_email": msg.from_email,
            "body": msg.body,
            "created_at": msg.created_at.to_rfc3339(),
        });
        self.table.insert_entity(CONTACTS_TABLE, &entity).await
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn row_to_comment(row: &serde_json::Value) -> Result<Comment, BlogError> {
    let status_str = row["status"].as_str().unwrap_or("Pending");
    let status = match status_str {
        "Approved" => CommentStatus::Approved,
        "Rejected" => CommentStatus::Rejected,
        _ => CommentStatus::Pending,
    };
    Ok(Comment {
        id: row["RowKey"].as_str().unwrap_or_default().to_owned(),
        post_slug: row["PartitionKey"].as_str().unwrap_or_default().to_owned(),
        author_name: row["author_name"].as_str().unwrap_or_default().to_owned(),
        content: row["content"].as_str().unwrap_or_default().to_owned(),
        created_at: row["created_at"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now),
        status,
    })
}

// ---------------------------------------------------------------------------
// Factories
// ---------------------------------------------------------------------------

pub fn build_comment_repository(config: &AppConfig) -> Arc<dyn CommentRepository> {
    if let Some(endpoint) = &config.azurite_table_endpoint {
        Arc::new(AzuriteCommentRepository::new(endpoint.clone()))
    } else {
        Arc::new(InMemoryCommentRepository::default())
    }
}

pub fn build_contact_repository(config: &AppConfig) -> Arc<dyn ContactRepository> {
    if let Some(endpoint) = &config.azurite_table_endpoint {
        Arc::new(AzuriteContactRepository::new(endpoint.clone()))
    } else {
        Arc::new(InMemoryContactRepository::default())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_comment(id: &str, slug: &str, author: &str) -> Comment {
        Comment {
            id: id.to_owned(),
            post_slug: slug.to_owned(),
            author_name: author.to_owned(),
            content: "test content".to_owned(),
            created_at: Utc::now(),
            status: CommentStatus::Pending,
        }
    }

    #[tokio::test]
    async fn in_memory_only_approved_returned_to_public() {
        let repo = InMemoryCommentRepository::default();
        let c1 = make_comment("1", "hello", "Alice");
        let mut c2 = make_comment("2", "hello", "Bob");
        c2.status = CommentStatus::Approved;

        repo.create_comment(&c1).await.unwrap();
        repo.create_comment(&c2).await.unwrap();

        let public = repo.list_comments("hello", false).await.unwrap();
        assert_eq!(public.len(), 1);
        assert_eq!(public[0].author_name, "Bob");
    }

    #[tokio::test]
    async fn in_memory_list_all_pending() {
        let repo = InMemoryCommentRepository::default();
        repo.create_comment(&make_comment("1", "slug-a", "Alice")).await.unwrap();
        repo.create_comment(&make_comment("2", "slug-b", "Bob")).await.unwrap();

        let pending = repo.list_all_pending().await.unwrap();
        assert_eq!(pending.len(), 2);
    }

    #[tokio::test]
    async fn in_memory_update_status_approve() {
        let repo = InMemoryCommentRepository::default();
        repo.create_comment(&make_comment("id-1", "test", "Alice")).await.unwrap();

        repo.update_status("id-1", CommentStatus::Approved).await.unwrap();

        let approved = repo.list_comments("test", false).await.unwrap();
        assert_eq!(approved.len(), 1);
    }

    #[tokio::test]
    async fn in_memory_update_status_reject() {
        let repo = InMemoryCommentRepository::default();
        repo.create_comment(&make_comment("id-2", "test", "Bob")).await.unwrap();

        repo.update_status("id-2", CommentStatus::Rejected).await.unwrap();

        let pending = repo.list_all_pending().await.unwrap();
        assert!(pending.is_empty());
        let approved = repo.list_comments("test", false).await.unwrap();
        assert!(approved.is_empty());
    }

    #[tokio::test]
    async fn in_memory_update_status_not_found() {
        let repo = InMemoryCommentRepository::default();
        let result = repo.update_status("nonexistent", CommentStatus::Approved).await;
        assert!(matches!(result, Err(BlogError::NotFound(_))));
    }

    #[tokio::test]
    async fn in_memory_contact_repository_stores_message() {
        let repo = InMemoryContactRepository::default();
        let msg = ContactMessage {
            id: "msg-1".to_owned(),
            from_name: "Charlie".to_owned(),
            from_email: "charlie@example.com".to_owned(),
            body: "Hello from Charlie".to_owned(),
            created_at: Utc::now(),
        };
        repo.create_contact_message(&msg).await.unwrap();
        // No error = success
    }
}
