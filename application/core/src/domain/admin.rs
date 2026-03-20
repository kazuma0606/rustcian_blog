use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminIdentity {
    pub oid: Option<String>,
    pub preferred_username: Option<String>,
    pub groups: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdminAuthError {
    Disabled,
    MissingConfiguration(&'static str),
    ProviderUnavailable(&'static str),
    MissingBearerToken,
    InvalidToken(&'static str),
    Forbidden(&'static str),
}

#[async_trait]
pub trait AdminAuthService: Send + Sync {
    async fn authenticate_bearer(
        &self,
        bearer_token: &str,
    ) -> Result<AdminIdentity, AdminAuthError>;
}
