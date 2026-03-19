//domain/value_object/email.rs
// Email バリューオブジェクト
// 2025/7/8

use crate::shared::error::domain_error::{DomainError, DomainResult};

#[derive(Debug, Clone, PartialEq)]
pub struct Email(pub String);

impl Email {
    pub fn new(value: String) -> DomainResult<Self> {
        if value.is_empty() {
            return Err(DomainError::InvalidEmail {
                email: value,
                reason: "Email cannot be empty".to_string(),
            });
        }
        if !Self::is_valid_format(&value) {
            return Err(DomainError::InvalidEmail {
                email: value,
                reason: "Invalid email format".to_string(),
            });
        }
        Ok(Self(value))
    }
    fn is_valid_format(value: &str) -> bool {
        // 簡易なメールアドレス形式チェック
        value.contains('@') && value.contains('.')
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}
