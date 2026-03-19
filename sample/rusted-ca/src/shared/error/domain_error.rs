// src/shared/domain_error.rs

use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum DomainError {
    // Value Object Validation Errors
    #[error("Invalid email format: '{email}' - {reason}")]
    InvalidEmail { email: String, reason: String },

    #[error("Invalid user name: '{name}' - {reason}")]
    InvalidUserName { name: String, reason: String },

    #[error("Invalid password: {reason}")]
    InvalidPassword { reason: String },

    // Business Rule Violations
    #[error("Business rule violation: {rule} - {message}")]
    BusinessRuleViolation { rule: String, message: String },

    #[error("Entity validation failed: {entity} - {field}: {message}")]
    EntityValidationFailed {
        entity: String,
        field: String,
        message: String,
    },

    // Domain Logic Errors
    #[error("Invariant violation: {message}")]
    InvariantViolation { message: String },
}

// Domain Layer Result Type
pub type DomainResult<T> = Result<T, DomainError>;
