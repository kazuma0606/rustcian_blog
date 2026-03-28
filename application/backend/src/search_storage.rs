use rustacian_blog_search::IndexStorage;

/// Stores the serialized search index on the local filesystem.
pub struct LocalIndexStorage {
    pub path: std::path::PathBuf,
}

impl IndexStorage for LocalIndexStorage {
    fn save(&self, data: &[u8]) -> Result<(), String> {
        std::fs::write(&self.path, data).map_err(|e| format!("LocalIndexStorage save: {e}"))
    }

    fn load(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.path).map_err(|e| format!("LocalIndexStorage load: {e}"))
    }
}

/// Stores the serialized search index in Azure Blob Storage.
/// Placeholder for M3 — not yet wired to the live Blob adapter.
pub struct BlobIndexStorage {
    pub container: String,
    pub blob_name: String,
}

impl IndexStorage for BlobIndexStorage {
    fn save(&self, _data: &[u8]) -> Result<(), String> {
        // TODO(M3): upload `_data` to `self.container / self.blob_name` via Blob API.
        Err("BlobIndexStorage not yet implemented".to_owned())
    }

    fn load(&self) -> Result<Vec<u8>, String> {
        // TODO(M3): download from `self.container / self.blob_name` via Blob API.
        Err("BlobIndexStorage not yet implemented".to_owned())
    }
}
