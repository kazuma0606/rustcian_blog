//presentation/dto/user_response.rs
// ユーザーレスポンス
// 2025/7/8

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub name: String,
    pub phone: Option<String>,
    pub birth_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
