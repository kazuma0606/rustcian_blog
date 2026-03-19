// src/shared/presentation_error.rs

use super::application_error::ApplicationError;
use axum::http::StatusCode;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PresentationError {
    // HTTP Request Errors
    #[error("Bad request: {message}")]
    BadRequest { message: String },

    #[error("Unauthorized: {message}")]
    Unauthorized { message: String },

    #[error("Forbidden: {message}")]
    Forbidden { message: String },

    #[error("Not found: {resource}")]
    NotFound { resource: String },

    #[error("Method not allowed: {method} on {path}")]
    MethodNotAllowed { method: String, path: String },

    #[error("Request timeout: {message}")]
    RequestTimeout { message: String },

    #[error("Payload too large: {size} bytes (max: {max} bytes)")]
    PayloadTooLarge { size: usize, max: usize },

    // Content Type Errors
    #[error("Unsupported media type: {media_type}")]
    UnsupportedMediaType { media_type: String },

    #[error("Invalid content type: expected {expected}, got {actual}")]
    InvalidContentType { expected: String, actual: String },

    // Serialization Errors
    #[error("JSON serialization failed: {message}")]
    JsonSerialization { message: String },

    #[error("JSON deserialization failed: {message}")]
    JsonDeserialization { message: String },

    // Server Errors
    #[error("Internal server error: {message}")]
    InternalServer { message: String },

    #[error("Service unavailable: {message}")]
    ServiceUnavailable { message: String },

    // Dependency Error (auto-conversion from Application layer)
    #[error("Application error: {0}")]
    Application(#[from] ApplicationError),
}

// Presentation Layer Result Type
pub type PresentationResult<T> = Result<T, PresentationError>;

// HTTP Status Code Mapping
impl PresentationError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            PresentationError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            PresentationError::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
            PresentationError::Forbidden { .. } => StatusCode::FORBIDDEN,
            PresentationError::NotFound { .. } => StatusCode::NOT_FOUND,
            PresentationError::MethodNotAllowed { .. } => StatusCode::METHOD_NOT_ALLOWED,
            PresentationError::RequestTimeout { .. } => StatusCode::REQUEST_TIMEOUT,
            PresentationError::PayloadTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            PresentationError::UnsupportedMediaType { .. } => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            PresentationError::InvalidContentType { .. } => StatusCode::BAD_REQUEST,
            PresentationError::JsonSerialization { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            PresentationError::JsonDeserialization { .. } => StatusCode::BAD_REQUEST,
            PresentationError::InternalServer { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            PresentationError::ServiceUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            // Application layer error mapping
            PresentationError::Application(app_error) => match app_error {
                ApplicationError::UserNotFound { .. } => StatusCode::NOT_FOUND,
                ApplicationError::EmailAlreadyExists { .. } => StatusCode::CONFLICT,
                ApplicationError::AuthorizationFailed { .. } => StatusCode::FORBIDDEN,
                ApplicationError::ValidationFailed { .. } => StatusCode::BAD_REQUEST,
                ApplicationError::InvalidInput { .. } => StatusCode::BAD_REQUEST,
                ApplicationError::PreconditionFailed { .. } => StatusCode::PRECONDITION_FAILED,
                ApplicationError::OperationNotPermitted { .. } => StatusCode::FORBIDDEN,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }
}
