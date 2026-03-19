//application/usecases/update_user_usecase.rs
// ユーザー更新ユースケース
// 2025/7/8

use crate::application::dto::user_request_dto::UpdateUserRequestDto;
use crate::application::dto::user_response_dto::UserResponseDto;
use crate::domain::entity::user::User;
use crate::domain::repository::user_command_repository::UserCommandRepositoryInterface;
use crate::domain::repository::user_query_repository::UserQueryRepositoryInterface;
use crate::domain::value_object::{
    birth_date::BirthDate, email::Email, password::Password, phone::Phone, user_id::UserId,
    user_name::UserName,
};
use crate::shared::error::application_error::{ApplicationError, ApplicationResult};
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

#[async_trait]
pub trait UpdateUserUsecaseInterface: Send + Sync {
    async fn execute(
        &self,
        request_dto: UpdateUserRequestDto,
    ) -> ApplicationResult<UserResponseDto>;
}

pub struct UpdateUserUseCase {
    command_repository: Arc<dyn UserCommandRepositoryInterface + Send + Sync>,
    query_repository: Arc<dyn UserQueryRepositoryInterface + Send + Sync>,
}

impl UpdateUserUseCase {
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
impl UpdateUserUsecaseInterface for UpdateUserUseCase {
    async fn execute(
        &self,
        request_dto: UpdateUserRequestDto,
    ) -> ApplicationResult<UserResponseDto> {
        println!(
            "UpdateUserUseCase: Starting update for user ID: {}",
            request_dto.id
        );

        // 1. IDのバリデーション
        let user_id =
            Uuid::parse_str(&request_dto.id).map_err(|_| ApplicationError::InvalidInput {
                input: "user_id".to_string(),
                reason: "Invalid UUID format".to_string(),
            })?;
        let user_id_vo = UserId::new(user_id.to_string());
        println!("UpdateUserUseCase: User ID validated: {}", user_id_vo.0);

        // 2. 既存ユーザーの取得
        println!("UpdateUserUseCase: Fetching existing user...");
        let existing_user = self.query_repository
            .find_by_id(&user_id_vo)
            .await
            .map_err(|e| {
                println!("UpdateUserUseCase: Error fetching user: {}", e);
                ApplicationError::Infrastructure(
                    crate::shared::error::infrastructure_error::InfrastructureError::ResourceUnavailable {
                        resource: "user".to_string(),
                        message: format!("{}", e),
                    },
                )
            })?
            .ok_or_else(|| {
                println!("UpdateUserUseCase: User not found: {}", request_dto.id);
                ApplicationError::UserNotFound {
                    id: request_dto.id.clone(),
                }
            })?;
        println!(
            "UpdateUserUseCase: Existing user found: {}",
            existing_user.name().0
        );

        // 3. バリューオブジェクト変換＆バリデーション
        let name = if let Some(name) = request_dto.name {
            UserName::new(name).map_err(|e| ApplicationError::ValidationFailed {
                field: "name".to_string(),
                message: e.to_string(),
            })?
        } else {
            existing_user.name().clone()
        };
        let phone = if let Some(phone) = request_dto.phone {
            Some(
                Phone::new(phone).map_err(|e| ApplicationError::ValidationFailed {
                    field: "phone".to_string(),
                    message: e.to_string(),
                })?,
            )
        } else {
            existing_user.phone().cloned()
        };
        let birth_date = if let Some(birth_date) = request_dto.birth_date {
            Some(
                BirthDate::new(birth_date).map_err(|e| ApplicationError::ValidationFailed {
                    field: "birth_date".to_string(),
                    message: e.to_string(),
                })?,
            )
        } else {
            existing_user.birth_date().cloned()
        };

        // 4. ドメインエンティティ再生成（メール・パスワードは変更不可と仮定）
        let user = User::new(
            user_id_vo,
            existing_user.email().clone(),
            name,
            existing_user.password().clone(),
            phone,
            birth_date,
        )
        .map_err(|e| ApplicationError::InvalidInput {
            input: "user".to_string(),
            reason: format!("{}", e),
        })?;

        // 5. 保存（永続化）
        println!("UpdateUserUseCase: Updating user...");
        self.command_repository.update(&user).await.map_err(|e| {
            println!("UpdateUserUseCase: Error updating user: {}", e);
            ApplicationError::Infrastructure(
                crate::shared::error::infrastructure_error::InfrastructureError::ResourceUnavailable {
                    resource: "user".to_string(),
                    message: format!("{}", e),
                },
            )
        })?;
        println!("UpdateUserUseCase: User updated successfully");

        // 6. レスポンスDTO生成
        let response_dto = UserResponseDto {
            id: user.id().0.clone(),
            email: user.email().0.clone(),
            name: user.name().0.clone(),
            phone: user.phone().map(|p| p.0.clone()),
            birth_date: user.birth_date().map(|b| b.0.clone()),
        };
        Ok(response_dto)
    }
}
