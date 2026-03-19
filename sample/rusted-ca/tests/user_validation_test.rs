// tests/user_validation_test.rs

use rusted_ca::domain::entity::user::User;
use rusted_ca::domain::value_object::birth_date::BirthDate;
use rusted_ca::domain::value_object::email::Email;
use rusted_ca::domain::value_object::password::Password;
use rusted_ca::domain::value_object::phone::Phone;
use rusted_ca::domain::value_object::user_id::UserId;
use rusted_ca::domain::value_object::user_name::UserName;
use rusted_ca::shared::error::domain_error::DomainError;

#[test]
fn test_user_creation_success() {
    let id = UserId::new("user-1".to_string());
    let email = Email::new("user@example.com".to_string()).unwrap();
    let name = UserName::new("John Doe".to_string()).unwrap();
    let password = Password::new("SecurePassword123!".to_string()).unwrap();
    let phone = Some(Phone::new("+81-90-1234-5678".to_string()).unwrap());
    let birth_date = Some(BirthDate::new("1990-01-01".to_string()).unwrap());
    let user = User::new(id, email, name, password, phone, birth_date);
    assert!(user.is_ok());
}

#[test]
fn test_invalid_email() {
    let result = Email::new("invalid-email".to_string());
    assert!(matches!(result, Err(DomainError::InvalidEmail { .. })));
}

#[test]
fn test_empty_user_name() {
    let result = UserName::new("".to_string());
    assert!(matches!(result, Err(DomainError::InvalidUserName { .. })));
}

#[test]
fn test_short_password() {
    let result = Password::new("short".to_string());
    assert!(matches!(result, Err(DomainError::InvalidPassword { .. })));
}

#[test]
fn test_invalid_birth_date() {
    let result = BirthDate::new("01-01-1990".to_string());
    assert!(result.is_err());
}

#[test]
fn test_empty_phone() {
    let result = Phone::new("".to_string());
    assert!(result.is_err());
}
