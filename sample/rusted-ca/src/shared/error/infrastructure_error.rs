// src/shared/infrastructure_error.rs

use thiserror::Error;

#[derive(Error, Debug)]
pub enum InfrastructureError {
    // Database Errors
    #[error("Database connection failed: {message}")]
    DatabaseConnection { message: String },

    #[error("Database query failed: {query} - {message}")]
    DatabaseQuery { query: String, message: String },

    #[error("Database transaction failed: {message}")]
    DatabaseTransaction { message: String },

    #[error("Data serialization failed: {data_type} - {message}")]
    DataSerialization { data_type: String, message: String },

    // Network/External Service Errors
    #[error("Network error: {endpoint} - {message}")]
    Network { endpoint: String, message: String },

    #[error("External service error: {service} - {status}: {message}")]
    ExternalService {
        service: String,
        status: String,
        message: String,
    },

    #[error("API timeout: {service} - {timeout_ms}ms")]
    Timeout { service: String, timeout_ms: u64 },

    // Configuration Errors
    #[error("Configuration error: {key} - {message}")]
    Configuration { key: String, message: String },

    #[error("Environment variable missing: {var_name}")]
    EnvironmentVariable { var_name: String },

    // File/IO Errors
    #[error("File operation failed: {path} - {message}")]
    FileOperation { path: String, message: String },

    #[error("Resource not available: {resource} - {message}")]
    ResourceUnavailable { resource: String, message: String },
}

// Infrastructure Layer Result Type
pub type InfrastructureResult<T> = Result<T, InfrastructureError>;
