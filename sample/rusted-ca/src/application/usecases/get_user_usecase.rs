//application/usecases/get_user_usecase.rs
// ユーザー取得ユースケース
// 2025/7/8

use crate::application::dto::user_response_dto::UserResponseDto;
use crate::domain::repository::user_query_repository::UserQueryRepositoryInterface;
use crate::domain::value_object::user_id::UserId;
use crate::shared::error::application_error::{ApplicationError, ApplicationResult};
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait GetUserQueryUsecaseInterface: Send + Sync {
    async fn execute(&self, user_id: String) -> ApplicationResult<UserResponseDto>;
}

pub struct GetUserUseCase<T>
where
    T: UserQueryRepositoryInterface + Send + Sync,
{
    query_repository: Arc<T>,
}

impl<T> GetUserUseCase<T>
where
    T: UserQueryRepositoryInterface + Send + Sync,
{
    pub fn new(query_repository: Arc<T>) -> Self {
        Self { query_repository }
    }
}

#[async_trait]
impl<T> GetUserQueryUsecaseInterface for GetUserUseCase<T>
where
    T: UserQueryRepositoryInterface + Send + Sync,
{
    async fn execute(&self, user_id: String) -> ApplicationResult<UserResponseDto> {
        // 1. UserIdの構築
        let user_id = UserId::new(user_id.clone());

        // 2. ユーザー取得
        let user = self
            .query_repository
            .find_by_id(&user_id)
            .await
            .map_err(|e| ApplicationError::Infrastructure(crate::shared::error::infrastructure_error::InfrastructureError::ResourceUnavailable {
                resource: "user".to_string(),
                message: format!("{}", e),
            }))?
            .ok_or(ApplicationError::UserNotFound { id: user_id.0 })?;

        // 3. レスポンスDTO生成
        let response = UserResponseDto {
            id: user.id.0,
            email: user.email.0,
            name: user.name.0,
            phone: user.phone.map(|p| p.0),
            birth_date: user.birth_date.map(|b| b.0),
        };
        Ok(response)
    }
}
