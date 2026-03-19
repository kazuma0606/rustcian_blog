//domain/value_object/birth_date.rs
// BirthDate バリューオブジェクト
// 2025/7/8

use crate::shared::error::domain_error::{DomainError, DomainResult};
use chrono::NaiveDate;

#[derive(Debug, Clone, PartialEq)]
pub struct BirthDate(pub String);

impl BirthDate {
    pub fn new(value: String) -> DomainResult<Self> {
        // 厳密な日付フォーマットチェック（YYYY-MM-DD）
        if NaiveDate::parse_from_str(&value, "%Y-%m-%d").is_err() {
            return Err(DomainError::EntityValidationFailed {
                entity: "User".to_string(),
                field: "birth_date".to_string(),
                message: "Invalid birth date format (expected YYYY-MM-DD)".to_string(),
            });
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}
