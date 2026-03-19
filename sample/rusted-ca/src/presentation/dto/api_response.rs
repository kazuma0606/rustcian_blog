//presentation/dto/api_response.rs
// 統一APIレスポンス
// 2025/7/8

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: String,
    pub request_id: String,
    pub processing_time_ms: u64,
}
