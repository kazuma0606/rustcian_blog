use rustacian_blog_search::IndexStorage;

use crate::blob::AzuriteBlobAdapter;

/// Stores the serialized search index on the local filesystem.
pub struct LocalIndexStorage {
    pub path: std::path::PathBuf,
}

impl IndexStorage for LocalIndexStorage {
    async fn save(&self, data: &[u8]) -> Result<(), String> {
        std::fs::write(&self.path, data).map_err(|e| format!("LocalIndexStorage save: {e}"))
    }

    async fn load(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.path).map_err(|e| format!("LocalIndexStorage load: {e}"))
    }
}

/// Stores the serialized search index in Azure Blob Storage.
pub struct BlobIndexStorage {
    adapter: AzuriteBlobAdapter,
    blob_name: String,
}

impl BlobIndexStorage {
    pub fn new(blob_endpoint: String, blob_name: String) -> Self {
        Self {
            adapter: AzuriteBlobAdapter::new(blob_endpoint),
            blob_name,
        }
    }
}

impl IndexStorage for BlobIndexStorage {
    async fn save(&self, data: &[u8]) -> Result<(), String> {
        self.adapter
            .create_container_if_needed()
            .await
            .map_err(|e| format!("BlobIndexStorage create container: {e}"))?;
        self.adapter
            .put_bytes(&self.blob_name, data.to_vec(), "application/octet-stream")
            .await
            .map_err(|e| format!("BlobIndexStorage save: {e}"))
    }

    async fn load(&self) -> Result<Vec<u8>, String> {
        self.adapter
            .get_bytes(&self.blob_name)
            .await
            .map_err(|e| format!("BlobIndexStorage load: {e}"))?
            .map(|(bytes, _)| bytes)
            .ok_or_else(|| format!("search index blob not found: {}", self.blob_name))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn azurite_endpoint() -> Option<String> {
        std::env::var("AZURITE_BLOB_ENDPOINT").ok()
    }

    #[tokio::test]
    async fn blob_index_storage_save_and_load_roundtrip() {
        let Some(endpoint) = azurite_endpoint() else {
            eprintln!("skip: AZURITE_BLOB_ENDPOINT not set");
            return;
        };

        let storage = BlobIndexStorage::new(endpoint, "search/test-index.bin".to_owned());
        let payload = b"hello search index";

        storage.save(payload).await.expect("save failed");
        let loaded = storage.load().await.expect("load failed");
        assert_eq!(loaded, payload);
    }
}
