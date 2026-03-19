//application/dto/user_request_dto.rs
// ユーザーリクエストDTO
// 2025/7/8

use serde::{Deserialize, Serialize};

/// ユーザー作成リクエストDTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequestDto {
    pub email: String,
    pub name: String,
    pub password: String,
    pub phone: Option<String>,
    pub birth_date: Option<String>,
}

impl CreateUserRequestDto {
    /// バリューオブジェクト変換＋バリデーション
    pub fn to_value_objects(&self) -> (String, String, String, Option<String>, Option<String>) {
        (
            self.email.clone(),
            self.name.clone(),
            self.password.clone(),
            self.phone.clone(),
            self.birth_date.clone(),
        )
    }
}

/// ユーザー更新リクエストDTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserRequestDto {
    pub id: String,
    pub name: Option<String>,
    pub phone: Option<String>,
    pub birth_date: Option<String>,
}

impl UpdateUserRequestDto {
    /// バリューオブジェクト変換用
    pub fn to_value_objects(&self) -> (String, Option<String>, Option<String>, Option<String>) {
        (
            self.id.clone(),
            self.name.clone(),
            self.phone.clone(),
            self.birth_date.clone(),
        )
    }
}

/// ユーザー削除リクエストDTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteUserRequestDto {
    pub id: String,
}
