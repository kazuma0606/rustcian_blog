//domain/value_object/phone.rs
// Phone バリューオブジェクト
// 2025/7/8

use crate::shared::error::domain_error::{DomainError, DomainResult};

#[derive(Debug, Clone, PartialEq)]
pub struct Phone(pub String);

impl Phone {
    pub fn new(value: String) -> DomainResult<Self> {
        if value.is_empty() {
            return Err(DomainError::EntityValidationFailed {
                entity: "User".to_string(),
                field: "phone".to_string(),
                message: "Phone cannot be empty".to_string(),
            });
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}
