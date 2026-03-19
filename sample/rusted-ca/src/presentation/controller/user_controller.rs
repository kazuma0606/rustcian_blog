//presentation/controller/user_controller.rs
// ユーザー管理エンドポイント
// 2025/7/8

// =============================================================================
// CLEAN ARCHITECTURE - 正しい依存関係の流れ
// =============================================================================

// ┌─────────────────────────────────────────────────────────────┐
// │                    PRESENTATION LAYER                       │
// │  (HTTP, CLI, gRPC etc. - 外部インターフェース)                │
// └─────────────────────────────────────────────────────────────┘
//                                 ↓ 依存
// ┌─────────────────────────────────────────────────────────────┐
// │                    APPLICATION LAYER                        │
// │         (UseCase, DTO, Command/Query Handler)               │
// └─────────────────────────────────────────────────────────────┘
//                                 ↓ 依存
// ┌─────────────────────────────────────────────────────────────┐
// │                      DOMAIN LAYER                           │
// │     (Entity, ValueObject, Repository Interface)            │
// └─────────────────────────────────────────────────────────────┘
//                                 ↑ 実装 (依存性注入)
// ┌─────────────────────────────────────────────────────────────┐
// │                  INFRASTRUCTURE LAYER                       │
// │    (DB実装, 外部API, ファイルシステム etc.)                    │
// └─────────────────────────────────────────────────────────────┘

// =============================================================================
// 正しい配置: presentation/controller/user_controller.rs
// =============================================================================

use crate::application::dto::user_request_dto::{CreateUserRequestDto, UpdateUserRequestDto};
use crate::application::dto::user_response_dto::UserResponseDto;
use crate::application::usecases::create_user_usecase::CreateUserUsecaseInterface;
use crate::application::usecases::delete_user_usecase::DeleteUserUsecaseInterface;
use crate::application::usecases::get_user_usecase::GetUserQueryUsecaseInterface;
use crate::application::usecases::update_user_usecase::UpdateUserUsecaseInterface;
use crate::presentation::dto::api_response::ApiResponse;
use crate::presentation::dto::create_user_request::CreateUserRequest;
use crate::presentation::dto::update_user_request::UpdateUserRequest;
use crate::presentation::dto::user_response::UserResponse;
use crate::shared::error::application_error::ApplicationError;
use axum::{
    Json as JsonRequest,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde_json::{Value, json};
use std::sync::Arc;

/// ユーザー管理Controller
///
/// 責務:
/// 1. HTTPリクエストの受信とバリデーション
/// 2. Presentation DTOからApplication DTOへの変換
/// 3. UseCase実行
/// 4. Application DTOからPresentation DTOへの変換
/// 5. HTTPレスポンスの生成（ステータスコード + JSON）
pub struct UserController<T, U, V, W>
where
    T: CreateUserUsecaseInterface + Send + Sync,
    U: GetUserQueryUsecaseInterface + Send + Sync,
    V: UpdateUserUsecaseInterface + Send + Sync,
    W: DeleteUserUsecaseInterface + Send + Sync,
{
    create_user_usecase: Arc<T>,
    get_user_usecase: Arc<U>,
    update_user_usecase: Arc<V>,
    delete_user_usecase: Arc<W>,
}

impl<T, U, V, W> UserController<T, U, V, W>
where
    T: CreateUserUsecaseInterface + Send + Sync,
    U: GetUserQueryUsecaseInterface + Send + Sync,
    V: UpdateUserUsecaseInterface + Send + Sync,
    W: DeleteUserUsecaseInterface + Send + Sync,
{
    pub fn new(
        create_user_usecase: Arc<T>,
        get_user_usecase: Arc<U>,
        update_user_usecase: Arc<V>,
        delete_user_usecase: Arc<W>,
    ) -> Self {
        Self {
            create_user_usecase,
            get_user_usecase,
            update_user_usecase,
            delete_user_usecase,
        }
    }

    /// POST /api/users - ユーザー作成
    pub async fn create_user(
        &self,
        _auth: crate::shared::middleware::auth_middleware::AuthenticatedUser,
        JsonRequest(request): JsonRequest<CreateUserRequest>,
    ) -> Result<(StatusCode, Json<ApiResponse<UserResponse>>), (StatusCode, Json<Value>)> {
        // 1. プレゼンテーション層でのバリデーション
        if let Err(validation_error) = self.validate_create_user_request(&request) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": {
                        "code": "VALIDATION_ERROR",
                        "message": validation_error
                    }
                })),
            ));
        }

        // 2. Presentation DTO → Application DTO 変換
        let app_request = CreateUserRequestDto {
            email: request.email,
            name: request.name,
            password: request.password,
            phone: request.phone,
            birth_date: request.birth_date,
        };

        // 3. UseCase実行（Application層への依存）
        match self.create_user_usecase.execute(app_request).await {
            Ok(app_response) => {
                // 4. Application DTO → Presentation DTO 変換
                let presentation_response = UserResponse {
                    id: app_response.id,
                    email: app_response.email,
                    name: app_response.name,
                    phone: app_response.phone,
                    birth_date: app_response.birth_date,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    updated_at: chrono::Utc::now().to_rfc3339(),
                };

                // 5. HTTPレスポンス生成（Presentation層の責務）
                Ok((
                    StatusCode::CREATED,
                    Json(ApiResponse {
                        success: true,
                        data: Some(presentation_response),
                        message: "User created successfully".to_string(),
                        request_id: format!("req_{}", uuid::Uuid::new_v4()),
                        processing_time_ms: 0, // 実際はメトリクスから取得
                    }),
                ))
            }
            Err(error) => {
                // 6. エラーハンドリング（HTTP固有の処理）
                let (status_code, error_response) =
                    self.map_application_error_to_http_response(error);
                Err((status_code, Json(error_response)))
            }
        }
    }

    /// GET /api/users/{id} - ユーザー取得
    pub async fn get_user(
        &self,
        Path(user_id): Path<String>,
    ) -> Result<(StatusCode, Json<ApiResponse<UserResponse>>), (StatusCode, Json<Value>)> {
        // UUID形式チェック（Presentation層の責務）
        if !self.is_valid_uuid(&user_id) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": {
                        "code": "INVALID_UUID",
                        "message": "Invalid user ID format"
                    }
                })),
            ));
        }

        // UseCase実行
        match self.get_user_usecase.execute(user_id).await {
            Ok(app_response) => {
                let presentation_response = UserResponse {
                    id: app_response.id,
                    email: app_response.email,
                    name: app_response.name,
                    phone: app_response.phone,
                    birth_date: app_response.birth_date,
                    created_at: chrono::Utc::now().to_rfc3339(), // 実際はDBから
                    updated_at: chrono::Utc::now().to_rfc3339(),
                };

                Ok((
                    StatusCode::OK,
                    Json(ApiResponse {
                        success: true,
                        data: Some(presentation_response),
                        message: "User retrieved successfully".to_string(),
                        request_id: format!("req_{}", uuid::Uuid::new_v4()),
                        processing_time_ms: 0,
                    }),
                ))
            }
            Err(error) => {
                let (status_code, error_response) =
                    self.map_application_error_to_http_response(error);
                Err((status_code, Json(error_response)))
            }
        }
    }

    /// PUT /api/users/{id} - ユーザー更新
    pub async fn update_user(
        &self,
        _auth: crate::shared::middleware::auth_middleware::AuthenticatedUser,
        Path(user_id): Path<String>,
        JsonRequest(request): JsonRequest<UpdateUserRequest>,
    ) -> Result<(StatusCode, Json<ApiResponse<UserResponse>>), (StatusCode, Json<Value>)> {
        // 1. プレゼンテーション層でのバリデーション
        if let Err(validation_error) = request.validate() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": {
                        "code": "VALIDATION_ERROR",
                        "message": validation_error
                    }
                })),
            ));
        }

        // 2. UUID形式チェック
        if !self.is_valid_uuid(&user_id) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": {
                        "code": "INVALID_UUID",
                        "message": "Invalid user ID format"
                    }
                })),
            ));
        }

        // 3. Presentation DTO → Application DTO 変換
        let app_request = UpdateUserRequestDto {
            id: user_id,
            name: request.name,
            phone: request.phone,
            birth_date: request.birth_date,
        };

        // 4. UseCase実行
        match self.update_user_usecase.execute(app_request).await {
            Ok(app_response) => {
                // 5. Application DTO → Presentation DTO 変換
                let presentation_response = UserResponse {
                    id: app_response.id,
                    email: app_response.email,
                    name: app_response.name,
                    phone: app_response.phone,
                    birth_date: app_response.birth_date,
                    created_at: chrono::Utc::now().to_rfc3339(), // 実際はDBから
                    updated_at: chrono::Utc::now().to_rfc3339(),
                };

                // 6. HTTPレスポンス生成
                Ok((
                    StatusCode::OK,
                    Json(ApiResponse {
                        success: true,
                        data: Some(presentation_response),
                        message: "User updated successfully".to_string(),
                        request_id: format!("req_{}", uuid::Uuid::new_v4()),
                        processing_time_ms: 0,
                    }),
                ))
            }
            Err(error) => {
                // 7. エラーハンドリング
                let (status_code, error_response) =
                    self.map_application_error_to_http_response(error);
                Err((status_code, Json(error_response)))
            }
        }
    }

    /// DELETE /api/users/{id} - ユーザー削除
    pub async fn delete_user(
        &self,
        _auth: crate::shared::middleware::auth_middleware::AuthenticatedUser,
        Path(user_id): Path<String>,
    ) -> Result<(StatusCode, Json<ApiResponse<UserResponse>>), (StatusCode, Json<Value>)> {
        // 1. UUID形式チェック
        if !self.is_valid_uuid(&user_id) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": {
                        "code": "INVALID_UUID",
                        "message": "Invalid user ID format"
                    }
                })),
            ));
        }

        // 2. Presentation DTO → Application DTO 変換
        let app_request =
            crate::application::dto::user_request_dto::DeleteUserRequestDto { id: user_id };

        // 3. UseCase実行
        match self.delete_user_usecase.execute(app_request).await {
            Ok(app_response) => {
                // 4. Application DTO → Presentation DTO 変換
                let presentation_response = UserResponse {
                    id: app_response.id,
                    email: app_response.email,
                    name: app_response.name,
                    phone: app_response.phone,
                    birth_date: app_response.birth_date,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    updated_at: chrono::Utc::now().to_rfc3339(),
                };
                Ok((
                    StatusCode::OK,
                    Json(ApiResponse {
                        success: true,
                        data: Some(presentation_response),
                        message: "User deleted successfully".to_string(),
                        request_id: format!("req_{}", uuid::Uuid::new_v4()),
                        processing_time_ms: 0,
                    }),
                ))
            }
            Err(error) => {
                let (status_code, error_response) =
                    self.map_application_error_to_http_response(error);
                Err((status_code, Json(error_response)))
            }
        }
    }

    // =============================================================================
    // プライベートメソッド（Presentation層の責務）
    // =============================================================================

    /// HTTPレベルのバリデーション
    fn validate_create_user_request(&self, request: &CreateUserRequest) -> Result<(), String> {
        if request.email.is_empty() {
            return Err("Email is required".to_string());
        }
        if request.name.is_empty() {
            return Err("Name is required".to_string());
        }
        if request.password.len() < 8 {
            return Err("Password must be at least 8 characters".to_string());
        }
        Ok(())
    }

    /// UUID形式チェック
    fn is_valid_uuid(&self, uuid_str: &str) -> bool {
        uuid::Uuid::parse_str(uuid_str).is_ok()
    }

    /// ApplicationエラーをHTTPレスポンスにマッピング
    fn map_application_error_to_http_response(
        &self,
        error: ApplicationError,
    ) -> (StatusCode, Value) {
        match error {
            ApplicationError::EmailAlreadyExists { email } => (
                StatusCode::CONFLICT,
                json!({
                    "success": false,
                    "error": {
                        "code": "EMAIL_ALREADY_EXISTS",
                        "message": format!("Email '{}' is already in use", email),
                        "details": {
                            "layer": "application",
                            "operation": "create_user",
                            "timestamp": chrono::Utc::now().to_rfc3339()
                        }
                    }
                }),
            ),
            ApplicationError::UserNotFound { id } => (
                StatusCode::NOT_FOUND,
                json!({
                    "success": false,
                    "error": {
                        "code": "USER_NOT_FOUND",
                        "message": format!("User with ID '{}' not found", id),
                        "details": {
                            "layer": "application",
                            "operation": "get_user",
                            "timestamp": chrono::Utc::now().to_rfc3339()
                        }
                    }
                }),
            ),
            ApplicationError::Domain(domain_error) => (
                StatusCode::BAD_REQUEST,
                json!({
                    "success": false,
                    "error": {
                        "code": "DOMAIN_VALIDATION_ERROR",
                        "message": format!("Invalid input: {}", domain_error),
                        "details": {
                            "layer": "domain",
                            "operation": "validation",
                            "timestamp": chrono::Utc::now().to_rfc3339()
                        }
                    }
                }),
            ),
            ApplicationError::Infrastructure(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "success": false,
                    "error": {
                        "code": "INTERNAL_SERVER_ERROR",
                        "message": "An unexpected error occurred",
                        "details": {
                            "layer": "infrastructure",
                            "operation": "unknown",
                            "timestamp": chrono::Utc::now().to_rfc3339()
                        }
                    }
                }),
            ),
            ApplicationError::AuthorizationFailed { message } => (
                StatusCode::FORBIDDEN,
                json!({
                    "success": false,
                    "error": {
                        "code": "AUTHORIZATION_FAILED",
                        "message": format!("Authorization failed: {}", message),
                        "details": {
                            "layer": "application",
                            "operation": "authorization",
                            "timestamp": chrono::Utc::now().to_rfc3339()
                        }
                    }
                }),
            ),
            ApplicationError::OperationNotPermitted { operation, reason } => (
                StatusCode::FORBIDDEN,
                json!({
                    "success": false,
                    "error": {
                        "code": "OPERATION_NOT_PERMITTED",
                        "message": format!("Operation '{}' not permitted: {}", operation, reason),
                        "details": {
                            "layer": "application",
                            "operation": operation,
                            "timestamp": chrono::Utc::now().to_rfc3339()
                        }
                    }
                }),
            ),
            ApplicationError::ValidationFailed { field, message } => (
                StatusCode::BAD_REQUEST,
                json!({
                    "success": false,
                    "error": {
                        "code": "VALIDATION_FAILED",
                        "message": format!("Validation failed for field '{}': {}", field, message),
                        "details": {
                            "layer": "application",
                            "operation": "validation",
                            "timestamp": chrono::Utc::now().to_rfc3339()
                        }
                    }
                }),
            ),
            ApplicationError::InvalidInput { input, reason } => (
                StatusCode::BAD_REQUEST,
                json!({
                    "success": false,
                    "error": {
                        "code": "INVALID_INPUT",
                        "message": format!("Invalid input '{}': {}", input, reason),
                        "details": {
                            "layer": "application",
                            "operation": "validation",
                            "timestamp": chrono::Utc::now().to_rfc3339()
                        }
                    }
                }),
            ),
            ApplicationError::PreconditionFailed { condition } => (
                StatusCode::PRECONDITION_FAILED,
                json!({
                    "success": false,
                    "error": {
                        "code": "PRECONDITION_FAILED",
                        "message": format!("Precondition failed: {}", condition),
                        "details": {
                            "layer": "application",
                            "operation": "precondition_check",
                            "timestamp": chrono::Utc::now().to_rfc3339()
                        }
                    }
                }),
            ),
            ApplicationError::PostconditionFailed { condition } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "success": false,
                    "error": {
                        "code": "POSTCONDITION_FAILED",
                        "message": format!("Postcondition failed: {}", condition),
                        "details": {
                            "layer": "application",
                            "operation": "postcondition_check",
                            "timestamp": chrono::Utc::now().to_rfc3339()
                        }
                    }
                }),
            ),
        }
    }
}
