//presentation/controller/auth_controller.rs
use crate::presentation::dto::login_request::LoginRequest;
use crate::shared::middleware::auth_middleware::{AuthError, JwtService};
use axum::Json;

#[derive(Debug, serde::Serialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub name: String,
    pub role: String,
}

#[derive(Debug, serde::Serialize)]
pub struct UserLoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user: UserInfo,
}

pub async fn login(
    Json(payload): Json<LoginRequest>,
) -> Result<Json<UserLoginResponse>, AuthError> {
    // 統合テスト用の認証情報設定
    let (auth_user, auth_pass) = if cfg!(test) || std::env::var("TEST_MODE").is_ok() {
        ("auth_user".to_string(), "auth_password".to_string())
    } else {
        (
            std::env::var("AUTH_USER").unwrap_or_else(|_| "auth_user".to_string()),
            std::env::var("AUTH_PASS").unwrap_or_else(|_| "auth_password".to_string()),
        )
    };

    if payload.username != auth_user || payload.password != auth_pass {
        return Err(AuthError::WrongCredentials);
    }

    // ダミー情報でJwtClaimsを生成（必要に応じてemail/name/roleを.envやDBから取得）
    let user_id = "dummy-id".to_string();
    let name = "Dummy User".to_string();
    let role = "user".to_string();

    let jwt_service = JwtService::new();
    let (access_token, refresh_token) = jwt_service.generate_token_pair(
        user_id.clone(),
        payload.username.clone(),
        name.clone(),
        role.clone(),
    )?;

    Ok(Json(UserLoginResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        user: UserInfo {
            id: user_id,
            email: payload.username,
            name,
            role,
        },
    }))
}
