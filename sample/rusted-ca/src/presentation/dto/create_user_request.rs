//presentation/dto/create_user_request.rs
// ユーザー作成リクエスト
// 2025/7/8

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub name: String,
    pub password: String,
    pub phone: Option<String>,
    pub birth_date: Option<String>,
}
