//presentation/dto/login_response.rs
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_in: u64, // 秒数
}
