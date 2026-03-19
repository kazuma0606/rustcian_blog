//application/commands/delete_user_command.rs
// ユーザー削除コマンド
// 2025/7/8

use crate::application::dto::user_request_dto::DeleteUserRequestDto;
use crate::application::dto::user_response_dto::UserResponseDto;
use crate::domain::repository::user_command_repository::UserCommandRepositoryInterface;
use crate::domain::repository::user_query_repository::UserQueryRepositoryInterface;
use crate::domain::value_object::user_id::UserId;
use crate::shared::error::application_error::{ApplicationError, ApplicationResult};
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

#[async_trait]
pub trait DeleteUserCommandInterface: Send + Sync {
    async fn execute(
        &self,
        request_dto: DeleteUserRequestDto,
    ) -> ApplicationResult<UserResponseDto>;
}

pub struct DeleteUserCommand {
    command_repository: Arc<dyn UserCommandRepositoryInterface + Send + Sync>,
    query_repository: Arc<dyn UserQueryRepositoryInterface + Send + Sync>,
}

impl DeleteUserCommand {
    pub fn new(
        command_repository: Arc<dyn UserCommandRepositoryInterface + Send + Sync>,
        query_repository: Arc<dyn UserQueryRepositoryInterface + Send + Sync>,
    ) -> Self {
        Self {
            command_repository,
            query_repository,
        }
    }
}

#[async_trait]
impl DeleteUserCommandInterface for DeleteUserCommand {
    async fn execute(
        &self,
        request_dto: DeleteUserRequestDto,
    ) -> ApplicationResult<UserResponseDto> {
        println!(
            "DeleteUserCommand: Starting delete for user ID: {}",
            request_dto.id
        );

        // 1. IDのバリデーション
        let user_id =
            Uuid::parse_str(&request_dto.id).map_err(|_| ApplicationError::InvalidInput {
                input: "user_id".to_string(),
                reason: "Invalid UUID format".to_string(),
            })?;
        let user_id_vo = UserId::new(user_id.to_string());
        println!("DeleteUserCommand: User ID validated: {}", user_id_vo.0);

        // 2. 既存ユーザーの存在確認
        println!("DeleteUserCommand: Checking if user exists...");
        let existing_user = self.query_repository
            .find_by_id(&user_id_vo)
            .await
            .map_err(|e| {
                println!("DeleteUserCommand: Error checking user existence: {}", e);
                ApplicationError::Infrastructure(
                    crate::shared::error::infrastructure_error::InfrastructureError::ResourceUnavailable {
                        resource: "user".to_string(),
                        message: format!("{}", e),
                    },
                )
            })?
            .ok_or_else(|| {
                println!("DeleteUserCommand: User not found: {}", request_dto.id);
                ApplicationError::UserNotFound {
                    id: request_dto.id.clone(),
                }
            })?;
        println!("DeleteUserCommand: User found: {}", existing_user.name().0);

        // 3. ユーザー削除
        println!("DeleteUserCommand: Deleting user...");
        self.command_repository.delete(&user_id_vo).await.map_err(|e| {
            println!("DeleteUserCommand: Error deleting user: {}", e);
            ApplicationError::Infrastructure(
                crate::shared::error::infrastructure_error::InfrastructureError::ResourceUnavailable {
                    resource: "user".to_string(),
                    message: format!("{}", e),
                },
            )
        })?;
        println!("DeleteUserCommand: User deleted successfully");

        // 4. 削除したユーザー情報をDTOで返す
        let response_dto = UserResponseDto {
            id: existing_user.id().0.clone(),
            email: existing_user.email().0.clone(),
            name: existing_user.name().0.clone(),
            phone: existing_user.phone().map(|p| p.0.clone()),
            birth_date: existing_user.birth_date().map(|b| b.0.clone()),
        };
        Ok(response_dto)
    }
}
