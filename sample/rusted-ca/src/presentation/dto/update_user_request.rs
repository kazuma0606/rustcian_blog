//presentation/dto/update_user_request.rs
// ユーザー更新リクエスト
// 2025/7/8

use serde::{Deserialize, Serialize};

/// ユーザー更新リクエストDTO
///
/// 責務:
/// 1. HTTPリクエストボディのデシリアライゼーション
/// 2. Presentation層でのバリデーション
/// 3. Application層へのデータ転送
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    /// ユーザー名（オプション）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// 電話番号（オプション）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,

    /// 生年月日（オプション）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birth_date: Option<String>,
}

impl UpdateUserRequest {
    /// バリデーション
    pub fn validate(&self) -> Result<(), String> {
        // 名前のバリデーション
        if let Some(ref name) = self.name {
            if name.trim().is_empty() {
                return Err("Name cannot be empty".to_string());
            }
            if name.len() > 100 {
                return Err("Name is too long (max 100 characters)".to_string());
            }
        }

        // 電話番号のバリデーション
        if let Some(ref phone) = self.phone {
            if phone.trim().is_empty() {
                return Err("Phone number cannot be empty".to_string());
            }
            // 簡単な電話番号形式チェック
            if !phone.chars().any(|c| c.is_digit(10)) {
                return Err("Phone number must contain digits".to_string());
            }
        }

        // 生年月日のバリデーション
        if let Some(ref birth_date) = self.birth_date {
            if birth_date.trim().is_empty() {
                return Err("Birth date cannot be empty".to_string());
            }
            // 日付形式チェック
            if chrono::NaiveDate::parse_from_str(birth_date, "%Y-%m-%d").is_err() {
                return Err("Birth date must be in YYYY-MM-DD format".to_string());
            }
        }

        // 少なくとも1つのフィールドが更新される必要がある
        if self.name.is_none() && self.phone.is_none() && self.birth_date.is_none() {
            return Err("At least one field must be provided for update".to_string());
        }

        Ok(())
    }
}
