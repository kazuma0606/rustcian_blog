//domain/value_object/password.rs
// Password バリューオブジェクト
// 2025/7/8

use crate::shared::error::domain_error::{DomainError, DomainResult};

#[derive(Debug, Clone, PartialEq)]
pub struct Password(pub String);

impl Password {
    pub fn new(value: String) -> DomainResult<Self> {
        if value.len() < 8 {
            return Err(DomainError::InvalidPassword {
                reason: "Password must be at least 8 characters".to_string(),
            });
        }
        Ok(Self(value))
    }
}
