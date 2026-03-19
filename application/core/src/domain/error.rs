use thiserror::Error;

#[derive(Debug, Error)]
pub enum BlogError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("post not found: {0}")]
    NotFound(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("parse error: {0}")]
    Parse(String),
}
