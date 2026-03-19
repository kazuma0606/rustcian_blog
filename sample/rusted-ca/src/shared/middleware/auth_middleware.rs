use axum::{
    Json, RequestPartsExt, async_trait,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::LazyLock;
use thiserror::Error;
use uuid::Uuid;

// =============================================================================
// JWT Configuration
// =============================================================================

#[derive(Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub algorithm: Algorithm,
    pub expiration_hours: i64,
}

impl JwtConfig {
    pub fn new(secret: String) -> Self {
        Self {
            secret,
            algorithm: Algorithm::HS256,
            expiration_hours: 1,
        }
    }
    pub fn encoding_key(&self) -> EncodingKey {
        EncodingKey::from_secret(self.secret.as_ref())
    }
    pub fn decoding_key(&self) -> DecodingKey {
        DecodingKey::from_secret(self.secret.as_ref())
    }
}

pub static JWT_CONFIG: LazyLock<JwtConfig> = LazyLock::new(|| {
    #[cfg(feature = "testmode")]
    let secret = "test_jwt_secret".to_string();
    #[cfg(not(feature = "testmode"))]
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
        "mK8vL9jN2pQ3rS4tU5vW6xY7zA1bC2dE3fG4hI5jK6lM7nO8pQ9rS0tU1vW2xY3z4A5bC6dE7fG8h".to_string()
    });
    JwtConfig::new(secret)
});

// =============================================================================
// JWT Claims Definition
// =============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtClaims {
    pub sub: String,
    pub email: String,
    pub name: String,
    pub role: String,
    pub iat: i64,
    pub exp: i64,
    pub jti: String,
}

impl JwtClaims {
    pub fn new(user_id: String, email: String, name: String, role: String) -> Self {
        let now = chrono::Utc::now();
        let exp = now + chrono::Duration::hours(JWT_CONFIG.expiration_hours);
        Self {
            sub: user_id,
            email,
            name,
            role,
            iat: now.timestamp(),
            exp: exp.timestamp(),
            jti: Uuid::new_v4().to_string(),
        }
    }
    pub fn to_token(&self) -> Result<String, AuthError> {
        let header = Header::new(JWT_CONFIG.algorithm);
        encode(&header, self, &JWT_CONFIG.encoding_key()).map_err(|_| AuthError::TokenCreation)
    }
    pub fn from_token(token: &str) -> Result<Self, AuthError> {
        let validation = Validation::new(JWT_CONFIG.algorithm);
        let token_data = decode::<JwtClaims>(token, &JWT_CONFIG.decoding_key(), &validation)
            .map_err(|_| AuthError::InvalidToken)?;
        Ok(token_data.claims)
    }
    pub fn has_role(&self, required_role: &str) -> bool {
        match (self.role.as_str(), required_role) {
            ("superadmin", _) => true,
            ("admin", "admin" | "user") => true,
            ("user", "user") => true,
            _ => false,
        }
    }
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now().timestamp() > self.exp
    }
}

// =============================================================================
// Auth Error Types
// =============================================================================

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid token")]
    InvalidToken,
    #[error("Missing credentials")]
    MissingCredentials,
    #[error("Wrong credentials")]
    WrongCredentials,
    #[error("Token creation error")]
    TokenCreation,
    #[error("Insufficient permissions")]
    InsufficientPermissions,
    #[error("Token expired")]
    TokenExpired,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message, error_code) = match self {
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid token", "INVALID_TOKEN"),
            AuthError::MissingCredentials => (
                StatusCode::BAD_REQUEST,
                "Missing credentials",
                "MISSING_CREDENTIALS",
            ),
            AuthError::WrongCredentials => (
                StatusCode::UNAUTHORIZED,
                "Wrong credentials",
                "WRONG_CREDENTIALS",
            ),
            AuthError::TokenCreation => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Token creation error",
                "TOKEN_CREATION_ERROR",
            ),
            AuthError::InsufficientPermissions => (
                StatusCode::FORBIDDEN,
                "Insufficient permissions",
                "INSUFFICIENT_PERMISSIONS",
            ),
            AuthError::TokenExpired => (StatusCode::UNAUTHORIZED, "Token expired", "TOKEN_EXPIRED"),
        };
        let body = Json(json!({
            "success": false,
            "error": {
                "code": error_code,
                "message": error_message,
            }
        }));
        (status, body).into_response()
    }
}

// =============================================================================
// JWT Extractor - 基本認証
// =============================================================================

#[derive(Debug, Clone)]
pub struct AuthenticatedUser(pub JwtClaims);

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = AuthError;
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AuthError::MissingCredentials)?;
        let claims = JwtClaims::from_token(bearer.token())?;
        if claims.is_expired() {
            return Err(AuthError::TokenExpired);
        }
        Ok(AuthenticatedUser(claims))
    }
}

// =============================================================================
// Role-based Extractors
// =============================================================================

#[derive(Debug, Clone)]
pub struct AdminUser(pub JwtClaims);

#[async_trait]
impl<S> FromRequestParts<S> for AdminUser
where
    S: Send + Sync,
{
    type Rejection = AuthError;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let AuthenticatedUser(claims) = AuthenticatedUser::from_request_parts(parts, state).await?;
        if !claims.has_role("admin") {
            return Err(AuthError::InsufficientPermissions);
        }
        Ok(AdminUser(claims))
    }
}

#[derive(Debug, Clone)]
pub struct SuperAdminUser(pub JwtClaims);

#[async_trait]
impl<S> FromRequestParts<S> for SuperAdminUser
where
    S: Send + Sync,
{
    type Rejection = AuthError;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let AuthenticatedUser(claims) = AuthenticatedUser::from_request_parts(parts, state).await?;
        if claims.role != "superadmin" {
            return Err(AuthError::InsufficientPermissions);
        }
        Ok(SuperAdminUser(claims))
    }
}

// =============================================================================
// Optional Auth Extractor
// =============================================================================

#[derive(Debug, Clone)]
pub struct OptionalAuth(pub Option<JwtClaims>);

#[async_trait]
impl<S> FromRequestParts<S> for OptionalAuth
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match AuthenticatedUser::from_request_parts(parts, state).await {
            Ok(AuthenticatedUser(claims)) => Ok(OptionalAuth(Some(claims))),
            Err(_) => Ok(OptionalAuth(None)),
        }
    }
}

// =============================================================================
// JWT Service Implementation
// =============================================================================

#[derive(Clone)]
pub struct JwtService;

impl JwtService {
    pub fn new() -> Self {
        Self
    }
    pub fn generate_token_pair(
        &self,
        user_id: String,
        email: String,
        name: String,
        role: String,
    ) -> Result<(String, String), AuthError> {
        let access_claims = JwtClaims::new(user_id.clone(), email.clone(), name, role);
        let access_token = access_claims.to_token()?;
        let mut refresh_claims =
            JwtClaims::new(user_id, email, "refresh".to_string(), "refresh".to_string());
        let refresh_exp = chrono::Utc::now() + chrono::Duration::days(30);
        refresh_claims.exp = refresh_exp.timestamp();
        let refresh_token = refresh_claims.to_token()?;
        Ok((access_token, refresh_token))
    }
    pub fn refresh_access_token(
        &self,
        refresh_token: &str,
        name: String,
        role: String,
    ) -> Result<String, AuthError> {
        let refresh_claims = JwtClaims::from_token(refresh_token)?;
        if refresh_claims.role != "refresh" || refresh_claims.is_expired() {
            return Err(AuthError::InvalidToken);
        }
        let access_claims = JwtClaims::new(refresh_claims.sub, refresh_claims.email, name, role);
        access_claims.to_token()
    }
}
