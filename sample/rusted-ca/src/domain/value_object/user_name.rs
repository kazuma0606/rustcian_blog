//domain/value_object/user_name.rs
// UserName バリューオブジェクト
// 2025/7/8

use crate::shared::error::domain_error::{DomainError, DomainResult};

#[derive(Debug, Clone, PartialEq)]
pub struct UserName(pub String);

impl UserName {
    pub fn new(value: String) -> DomainResult<Self> {
        if value.trim().is_empty() {
            return Err(DomainError::InvalidUserName {
                name: value,
                reason: "User name cannot be empty".to_string(),
            });
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}
