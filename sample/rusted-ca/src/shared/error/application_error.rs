// src/shared/application_error.rs

use super::domain_error::DomainError;
use super::infrastructure_error::InfrastructureError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApplicationError {
    // UseCase Specific Errors
    #[error("User not found: {id}")]
    UserNotFound { id: String },

    #[error("Email already exists: {email}")]
    EmailAlreadyExists { email: String },

    #[error("Authorization failed: {message}")]
    AuthorizationFailed { message: String },

    #[error("Operation not permitted: {operation} - {reason}")]
    OperationNotPermitted { operation: String, reason: String },

    // Input Validation Errors
    #[error("Validation failed: {field} - {message}")]
    ValidationFailed { field: String, message: String },

    #[error("Invalid input: {input} - {reason}")]
    InvalidInput { input: String, reason: String },

    // UseCase Flow Errors
    #[error("Precondition failed: {condition}")]
    PreconditionFailed { condition: String },

    #[error("Postcondition failed: {condition}")]
    PostconditionFailed { condition: String },

    // Dependency Errors (auto-conversion from lower layers)
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),

    #[error("Infrastructure error: {0}")]
    Infrastructure(#[from] InfrastructureError),
}

// Application Layer Result Type
pub type ApplicationResult<T> = Result<T, ApplicationError>;
