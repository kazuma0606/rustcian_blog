//application/dto/user_response_dto.rs
// HTTP Response DTO
// 2025/7/8

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponseDto {
    pub id: String,
    pub email: String,
    pub name: String,
    pub phone: Option<String>,
    pub birth_date: Option<String>,
}
