//presentation/dto/fortune_response.rs
use serde::Serialize;

#[derive(Serialize)]
pub struct FortuneResponse {
    pub message: String,
}
