//presentation/dto/delete_user_request.rs
// ユーザー削除リクエストDTO
// 2025/7/8

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteUserRequest {
    // DELETEリクエストでは通常ボディは不要ですが、
    // 必要に応じて追加のパラメータを定義できます
}

impl DeleteUserRequest {
    pub fn validate(&self) -> Result<(), String> {
        // DELETEリクエストの場合は通常バリデーションは不要
        // 必要に応じて追加のバリデーションを実装
        Ok(())
    }
}
