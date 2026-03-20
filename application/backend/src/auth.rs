use actix_web::http::header::HeaderMap;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;

use crate::config::AppConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminIdentity {
    pub oid: Option<String>,
    pub preferred_username: Option<String>,
    pub groups: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdminAuthError {
    Disabled,
    MissingConfiguration(&'static str),
    MissingBearerToken,
    InvalidToken(&'static str),
    Forbidden(&'static str),
}

pub fn validate_admin_request(
    headers: &HeaderMap,
    config: &AppConfig,
) -> Result<AdminIdentity, AdminAuthError> {
    match config.admin_auth_mode.as_str() {
        "entra-poc" => validate_entra_poc_token(headers, config),
        _ => Err(AdminAuthError::Disabled),
    }
}

fn validate_entra_poc_token(
    headers: &HeaderMap,
    config: &AppConfig,
) -> Result<AdminIdentity, AdminAuthError> {
    let tenant_id = config
        .entra_tenant_id
        .as_deref()
        .ok_or(AdminAuthError::MissingConfiguration("ENTRA_TENANT_ID"))?;
    let client_id = config
        .entra_client_id
        .as_deref()
        .ok_or(AdminAuthError::MissingConfiguration("ENTRA_CLIENT_ID"))?;
    let raw = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .ok_or(AdminAuthError::MissingBearerToken)?;
    let token = raw
        .strip_prefix("Bearer ")
        .ok_or(AdminAuthError::MissingBearerToken)?;
    let claims = parse_jwt_claims(token)?;

    if claims.tid.as_deref() != Some(tenant_id) {
        return Err(AdminAuthError::Forbidden("tenant mismatch"));
    }
    if !claims.audience_matches(client_id) {
        return Err(AdminAuthError::Forbidden("audience mismatch"));
    }

    if let Some(required_group) = config.entra_admin_group_id.as_deref() {
        if !claims.groups.iter().any(|group| group == required_group) {
            return Err(AdminAuthError::Forbidden("admin group mismatch"));
        }
    } else if let Some(required_oid) = config.entra_admin_user_oid.as_deref() {
        if claims.oid.as_deref() != Some(required_oid) {
            return Err(AdminAuthError::Forbidden("admin user mismatch"));
        }
    } else {
        return Err(AdminAuthError::MissingConfiguration(
            "ENTRA_ADMIN_GROUP_ID or ENTRA_ADMIN_USER_OID",
        ));
    }

    Ok(AdminIdentity {
        oid: claims.oid,
        preferred_username: claims.preferred_username,
        groups: claims.groups,
    })
}

fn parse_jwt_claims(token: &str) -> Result<JwtClaims, AdminAuthError> {
    let mut segments = token.split('.');
    let _header = segments
        .next()
        .ok_or(AdminAuthError::InvalidToken("missing header"))?;
    let payload = segments
        .next()
        .ok_or(AdminAuthError::InvalidToken("missing payload"))?;

    let decoded = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| AdminAuthError::InvalidToken("payload is not base64url"))?;
    serde_json::from_slice::<JwtClaims>(&decoded)
        .map_err(|_| AdminAuthError::InvalidToken("payload is not valid json"))
}

#[derive(Debug, Clone, Deserialize)]
struct JwtClaims {
    #[serde(default)]
    aud: serde_json::Value,
    #[serde(default)]
    tid: Option<String>,
    #[serde(default)]
    oid: Option<String>,
    #[serde(default)]
    preferred_username: Option<String>,
    #[serde(default)]
    groups: Vec<String>,
}

impl JwtClaims {
    fn audience_matches(&self, client_id: &str) -> bool {
        match &self.aud {
            serde_json::Value::String(value) => value == client_id,
            serde_json::Value::Array(values) => {
                values.iter().any(|value| value.as_str() == Some(client_id))
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use actix_web::http::header::{AUTHORIZATION, HeaderMap, HeaderValue};
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

    use super::*;

    fn sample_config() -> AppConfig {
        AppConfig {
            app_env: "test".to_owned(),
            app_host: "127.0.0.1".to_owned(),
            app_port: 8080,
            storage_backend: "local".to_owned(),
            content_root: "./content".into(),
            azurite_blob_endpoint: None,
            azurite_table_endpoint: None,
            azure_openai_endpoint: None,
            azure_openai_deployment: None,
            azure_openai_api_key: None,
            azure_openai_api_version: "2024-10-21".to_owned(),
            azure_openai_model_name: None,
            admin_auth_mode: "entra-poc".to_owned(),
            entra_tenant_id: Some("tenant-123".to_owned()),
            entra_client_id: Some("client-123".to_owned()),
            entra_admin_group_id: Some("group-123".to_owned()),
            entra_admin_user_oid: None,
        }
    }

    fn bearer_for(payload: &str) -> String {
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none","typ":"JWT"}"#);
        let claims = URL_SAFE_NO_PAD.encode(payload);
        format!("Bearer {header}.{claims}.")
    }

    #[test]
    fn accepts_matching_entra_poc_claims() {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&bearer_for(
                r#"{"aud":"client-123","tid":"tenant-123","groups":["group-123"],"oid":"user-1"}"#,
            ))
            .unwrap(),
        );

        let identity = validate_admin_request(&headers, &sample_config()).unwrap();

        assert_eq!(identity.oid.as_deref(), Some("user-1"));
    }

    #[test]
    fn rejects_when_group_does_not_match() {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&bearer_for(
                r#"{"aud":"client-123","tid":"tenant-123","groups":["group-x"]}"#,
            ))
            .unwrap(),
        );

        let error = validate_admin_request(&headers, &sample_config()).unwrap_err();

        assert_eq!(error, AdminAuthError::Forbidden("admin group mismatch"));
    }
}
