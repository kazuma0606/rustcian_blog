//domain/entity/user.rs
// User エンティティ + ビジネスロジック
// 2025/7/8

use crate::domain::value_object::{
    birth_date::BirthDate, email::Email, password::Password, phone::Phone, user_id::UserId,
    user_name::UserName,
};
use crate::shared::error::domain_error::DomainResult;

#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: UserId,
    pub email: Email,
    pub name: UserName,
    pub password: Password,
    pub phone: Option<Phone>,
    pub birth_date: Option<BirthDate>,
}

impl User {
    pub fn new(
        id: UserId,
        email: Email,
        name: UserName,
        password: Password,
        phone: Option<Phone>,
        birth_date: Option<BirthDate>,
    ) -> DomainResult<Self> {
        // 追加のビジネスルールやバリデーションがあればここで実施
        Ok(User {
            id,
            email,
            name,
            password,
            phone,
            birth_date,
        })
    }

    pub fn get_id(&self) -> &UserId {
        &self.id
    }

    pub fn id(&self) -> &UserId {
        &self.id
    }

    pub fn email(&self) -> &Email {
        &self.email
    }

    pub fn name(&self) -> &UserName {
        &self.name
    }

    pub fn password(&self) -> &Password {
        &self.password
    }

    pub fn phone(&self) -> Option<&Phone> {
        self.phone.as_ref()
    }

    pub fn birth_date(&self) -> Option<&BirthDate> {
        self.birth_date.as_ref()
    }
}
