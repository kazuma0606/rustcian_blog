use std::sync::Arc;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use reqwest::Client;
use rustacian_blog_core::{AdminAuthError, AdminAuthService, AdminIdentity};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;

use crate::config::AppConfig;

pub struct DisabledAdminAuthService;

pub struct EntraPocAdminAuthService {
    config: AppConfig,
}

pub struct EntraOidcAdminAuthService {
    config: AppConfig,
    client: Client,
    metadata_url: String,
    provider_state: RwLock<Option<OidcProviderState>>,
}

#[derive(Debug, Clone)]
struct OidcProviderState {
    issuer: String,
    jwks: JsonWebKeySet,
}

impl EntraPocAdminAuthService {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }
}

impl EntraOidcAdminAuthService {
    pub fn new(config: AppConfig) -> Result<Self, AdminAuthError> {
        let metadata_url = oidc_metadata_url(&config)?;
        Ok(Self {
            config,
            client: Client::new(),
            metadata_url,
            provider_state: RwLock::new(None),
        })
    }

    async fn provider_state(&self) -> Result<OidcProviderState, AdminAuthError> {
        if let Some(state) = self.provider_state.read().await.clone() {
            return Ok(state);
        }

        let discovery = self
            .client
            .get(&self.metadata_url)
            .send()
            .await
            .map_err(|_| AdminAuthError::ProviderUnavailable("oidc discovery request failed"))?;
        if !discovery.status().is_success() {
            return Err(AdminAuthError::ProviderUnavailable(
                "oidc discovery returned non-success status",
            ));
        }
        let discovery: OidcDiscoveryDocument = discovery
            .json()
            .await
            .map_err(|_| AdminAuthError::ProviderUnavailable("oidc discovery json was invalid"))?;

        let jwks = self
            .client
            .get(&discovery.jwks_uri)
            .send()
            .await
            .map_err(|_| AdminAuthError::ProviderUnavailable("jwks request failed"))?;
        if !jwks.status().is_success() {
            return Err(AdminAuthError::ProviderUnavailable(
                "jwks request returned non-success status",
            ));
        }
        let jwks: JsonWebKeySet = jwks
            .json()
            .await
            .map_err(|_| AdminAuthError::ProviderUnavailable("jwks json was invalid"))?;

        let state = OidcProviderState {
            issuer: discovery.issuer,
            jwks,
        };
        *self.provider_state.write().await = Some(state.clone());
        Ok(state)
    }
}

/// PKCE parameters returned by `build_auth_redirect_url`.
pub struct PkceParams {
    /// The full Azure AD authorization URL to redirect the browser to.
    pub auth_url: String,
    /// The code_verifier — must be presented at the token endpoint.
    /// We embed it in the `state` parameter so no server-side session is needed.
    pub state: String,
}

/// Build the Azure AD PKCE authorization URL.
/// The code_verifier is base64url-encoded and stored in `state` so that
/// the callback handler can retrieve it without server-side session storage.
pub fn build_auth_redirect_url(config: &AppConfig) -> Result<PkceParams, AdminAuthError> {
    let tenant_id = config
        .entra_tenant_id
        .as_deref()
        .ok_or(AdminAuthError::MissingConfiguration("ENTRA_TENANT_ID"))?;
    let client_id = config
        .entra_client_id
        .as_deref()
        .ok_or(AdminAuthError::MissingConfiguration("ENTRA_CLIENT_ID"))?;
    let redirect_uri = config
        .entra_redirect_uri
        .as_deref()
        .ok_or(AdminAuthError::MissingConfiguration("ENTRA_REDIRECT_URI"))?;

    // Generate code_verifier from two UUIDs (32 random bytes → 43-char base64url)
    let v1 = uuid::Uuid::new_v4();
    let v2 = uuid::Uuid::new_v4();
    let mut raw = Vec::with_capacity(32);
    raw.extend_from_slice(v1.as_bytes());
    raw.extend_from_slice(v2.as_bytes());
    let code_verifier = URL_SAFE_NO_PAD.encode(&raw);

    // code_challenge = BASE64URL(SHA256(code_verifier))
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let code_challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

    // Embed verifier in state (base64url) — stateless PKCE for server-side apps
    let state = URL_SAFE_NO_PAD.encode(code_verifier.as_bytes());

    let auth_url = format!(
        "https://login.microsoftonline.com/{tenant_id}/oauth2/v2.0/authorize\
         ?client_id={client_id}\
         &response_type=code\
         &redirect_uri={encoded_redirect}\
         &response_mode=query\
         &scope=openid+profile+email\
         &code_challenge={code_challenge}\
         &code_challenge_method=S256\
         &state={state}",
        encoded_redirect = urlencoding::encode(redirect_uri),
    );

    Ok(PkceParams { auth_url, state })
}

/// Exchange the authorization code for an id_token using PKCE.
/// `state` is the value returned by `build_auth_redirect_url`, which
/// encodes the code_verifier as base64url.
pub async fn exchange_code_for_token(
    config: &AppConfig,
    client: &Client,
    code: &str,
    state: &str,
) -> Result<String, AdminAuthError> {
    let tenant_id = config
        .entra_tenant_id
        .as_deref()
        .ok_or(AdminAuthError::MissingConfiguration("ENTRA_TENANT_ID"))?;
    let client_id = config
        .entra_client_id
        .as_deref()
        .ok_or(AdminAuthError::MissingConfiguration("ENTRA_CLIENT_ID"))?;
    let redirect_uri = config
        .entra_redirect_uri
        .as_deref()
        .ok_or(AdminAuthError::MissingConfiguration("ENTRA_REDIRECT_URI"))?;

    let verifier_bytes = URL_SAFE_NO_PAD
        .decode(state)
        .map_err(|_| AdminAuthError::InvalidToken("invalid state parameter"))?;
    let code_verifier = String::from_utf8(verifier_bytes)
        .map_err(|_| AdminAuthError::InvalidToken("state is not valid utf-8"))?;

    let token_url = format!("https://login.microsoftonline.com/{tenant_id}/oauth2/v2.0/token");
    let response = client
        .post(&token_url)
        .form(&[
            ("client_id", client_id),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
            ("code_verifier", code_verifier.as_str()),
        ])
        .send()
        .await
        .map_err(|_| AdminAuthError::ProviderUnavailable("token request failed"))?;

    if !response.status().is_success() {
        return Err(AdminAuthError::ProviderUnavailable(
            "token endpoint returned non-success status",
        ));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|_| AdminAuthError::ProviderUnavailable("token response is not valid json"))?;

    body["id_token"]
        .as_str()
        .map(str::to_owned)
        .ok_or(AdminAuthError::ProviderUnavailable(
            "no id_token in token response",
        ))
}

pub fn build_admin_auth_service(config: &AppConfig) -> Arc<dyn AdminAuthService> {
    match config.admin_auth_mode.as_str() {
        "entra-poc" => Arc::new(EntraPocAdminAuthService::new(config.clone())),
        "entra-oidc" => EntraOidcAdminAuthService::new(config.clone())
            .map(|service| Arc::new(service) as Arc<dyn AdminAuthService>)
            .unwrap_or_else(|_| Arc::new(DisabledAdminAuthService)),
        _ => Arc::new(DisabledAdminAuthService),
    }
}

#[async_trait::async_trait]
impl AdminAuthService for DisabledAdminAuthService {
    async fn authenticate_bearer(
        &self,
        _bearer_token: &str,
    ) -> Result<AdminIdentity, AdminAuthError> {
        Err(AdminAuthError::Disabled)
    }
}

#[async_trait::async_trait]
impl AdminAuthService for EntraPocAdminAuthService {
    async fn authenticate_bearer(
        &self,
        bearer_token: &str,
    ) -> Result<AdminIdentity, AdminAuthError> {
        let tenant_id = self
            .config
            .entra_tenant_id
            .as_deref()
            .ok_or(AdminAuthError::MissingConfiguration("ENTRA_TENANT_ID"))?;
        let client_id = self
            .config
            .entra_client_id
            .as_deref()
            .ok_or(AdminAuthError::MissingConfiguration("ENTRA_CLIENT_ID"))?;
        let claims = parse_jwt_claims(bearer_token)?;

        if claims.tid.as_deref() != Some(tenant_id) {
            return Err(AdminAuthError::Forbidden("tenant mismatch"));
        }
        if !claims.audience_matches(client_id) {
            return Err(AdminAuthError::Forbidden("audience mismatch"));
        }

        validate_admin_scope(&self.config, &claims)
    }
}

#[async_trait::async_trait]
impl AdminAuthService for EntraOidcAdminAuthService {
    async fn authenticate_bearer(
        &self,
        bearer_token: &str,
    ) -> Result<AdminIdentity, AdminAuthError> {
        let tenant_id = self
            .config
            .entra_tenant_id
            .as_deref()
            .ok_or(AdminAuthError::MissingConfiguration("ENTRA_TENANT_ID"))?;
        let client_id = self
            .config
            .entra_client_id
            .as_deref()
            .ok_or(AdminAuthError::MissingConfiguration("ENTRA_CLIENT_ID"))?;
        let header = decode_header(bearer_token)
            .map_err(|_| AdminAuthError::InvalidToken("invalid jwt header"))?;
        if header.alg != Algorithm::RS256 {
            return Err(AdminAuthError::InvalidToken("unsupported jwt algorithm"));
        }
        let kid = header
            .kid
            .ok_or(AdminAuthError::InvalidToken("missing kid header"))?;
        let provider = self.provider_state().await?;
        let key = provider
            .jwks
            .find_signing_key(&kid)
            .ok_or(AdminAuthError::InvalidToken("kid was not found in jwks"))?;
        let decoding_key = DecodingKey::from_rsa_components(&key.n, &key.e)
            .map_err(|_| AdminAuthError::InvalidToken("jwks key is invalid"))?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_nbf = true;
        validation.set_audience(&[client_id]);
        validation.set_issuer(&[provider.issuer.as_str()]);
        validation.set_required_spec_claims(&["exp", "iss", "aud"]);

        let decoded = decode::<JwtClaims>(bearer_token, &decoding_key, &validation)
            .map_err(|_| AdminAuthError::InvalidToken("jwt validation failed"))?;
        let claims = decoded.claims;

        if claims.tid.as_deref() != Some(tenant_id) {
            return Err(AdminAuthError::Forbidden("tenant mismatch"));
        }

        validate_admin_scope(&self.config, &claims)
    }
}

fn oidc_metadata_url(config: &AppConfig) -> Result<String, AdminAuthError> {
    if let Some(url) = config.entra_oidc_metadata_url.as_deref() {
        return Ok(url.to_owned());
    }

    let tenant_id = config
        .entra_tenant_id
        .as_deref()
        .ok_or(AdminAuthError::MissingConfiguration("ENTRA_TENANT_ID"))?;
    Ok(format!(
        "https://login.microsoftonline.com/{tenant_id}/v2.0/.well-known/openid-configuration"
    ))
}

fn validate_admin_scope(
    config: &AppConfig,
    claims: &JwtClaims,
) -> Result<AdminIdentity, AdminAuthError> {
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
        oid: claims.oid.clone(),
        preferred_username: claims.preferred_username.clone(),
        groups: claims.groups.clone(),
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

#[derive(Debug, Clone, Deserialize)]
struct OidcDiscoveryDocument {
    issuer: String,
    jwks_uri: String,
}

#[derive(Debug, Clone, Deserialize)]
struct JsonWebKeySet {
    keys: Vec<JsonWebKey>,
}

impl JsonWebKeySet {
    fn find_signing_key(&self, kid: &str) -> Option<&JsonWebKey> {
        self.keys.iter().find(|key| {
            key.kid.as_deref() == Some(kid)
                && key.kty == "RSA"
                && key.use_field.as_deref().unwrap_or("sig") == "sig"
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
struct JsonWebKey {
    kty: String,
    n: String,
    e: String,
    #[serde(default)]
    kid: Option<String>,
    #[serde(default, rename = "use")]
    use_field: Option<String>,
}

#[cfg(test)]
mod tests {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use chrono::Utc;
    use jsonwebtoken::{EncodingKey, Header, encode};
    use serde::Serialize;

    use super::*;

    const TEST_RSA_PRIVATE_KEY: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQDJETqse41HRBsc
7cfcq3ak4oZWFCoZlcic525A3FfO4qW9BMtRO/iXiyCCHn8JhiL9y8j5JdVP2Q9Z
IpfElcFd3/guS9w+5RqQGgCR+H56IVUyHZWtTJbKPcwWXQdNUX0rBFcsBzCRESJL
eelOEdHIjG7LRkx5l/FUvlqsyHDVJEQsHwegZ8b8C0fz0EgT2MMEdn10t6Ur1rXz
jMB/wvCg8vG8lvciXmedyo9xJ8oMOh0wUEgxziVDMMovmC+aJctcHUAYubwoGN8T
yzcvnGqL7JSh36Pwy28iPzXZ2RLhAyJFU39vLaHdljwthUaupldlNyCfa6Ofy4qN
ctlUPlN1AgMBAAECggEAdESTQjQ70O8QIp1ZSkCYXeZjuhj081CK7jhhp/4ChK7J
GlFQZMwiBze7d6K84TwAtfQGZhQ7km25E1kOm+3hIDCoKdVSKch/oL54f/BK6sKl
qlIzQEAenho4DuKCm3I4yAw9gEc0DV70DuMTR0LEpYyXcNJY3KNBOTjN5EYQAR9s
2MeurpgK2MdJlIuZaIbzSGd+diiz2E6vkmcufJLtmYUT/k/ddWvEtz+1DnO6bRHh
xuuDMeJA/lGB/EYloSLtdyCF6sII6C6slJJtgfb0bPy7l8VtL5iDyz46IKyzdyzW
tKAn394dm7MYR1RlUBEfqFUyNK7C+pVMVoTwCC2V4QKBgQD64syfiQ2oeUlLYDm4
CcKSP3RnES02bcTyEDFSuGyyS1jldI4A8GXHJ/lG5EYgiYa1RUivge4lJrlNfjyf
dV230xgKms7+JiXqag1FI+3mqjAgg4mYiNjaao8N8O3/PD59wMPeWYImsWXNyeHS
55rUKiHERtCcvdzKl4u35ZtTqQKBgQDNKnX2bVqOJ4WSqCgHRhOm386ugPHfy+8j
m6cicmUR46ND6ggBB03bCnEG9OtGisxTo/TuYVRu3WP4KjoJs2LD5fwdwJqpgtHl
yVsk45Y1Hfo+7M6lAuR8rzCi6kHHNb0HyBmZjysHWZsn79ZM+sQnLpgaYgQGRbKV
DZWlbw7g7QKBgQCl1u+98UGXAP1jFutwbPsx40IVszP4y5ypCe0gqgon3UiY/G+1
zTLp79GGe/SjI2VpQ7AlW7TI2A0bXXvDSDi3/5Dfya9ULnFXv9yfvH1QwWToySpW
Kvd1gYSoiX84/WCtjZOr0e0HmLIb0vw0hqZA4szJSqoxQgvF22EfIWaIaQKBgQCf
34+OmMYw8fEvSCPxDxVvOwW2i7pvV14hFEDYIeZKW2W1HWBhVMzBfFB5SE8yaCQy
pRfOzj9aKOCm2FjjiErVNpkQoi6jGtLvScnhZAt/lr2TXTrl8OwVkPrIaN0bG/AS
aUYxmBPCpXu3UjhfQiWqFq/mFyzlqlgvuCc9g95HPQKBgAscKP8mLxdKwOgX8yFW
GcZ0izY/30012ajdHY+/QK5lsMoxTnn0skdS+spLxaS5ZEO4qvPVb8RAoCkWMMal
2pOhmquJQVDPDLuZHdrIiKiDM20dy9sMfHygWcZjQ4WSxf/J7T9canLZIXFhHAZT
3wc9h4G8BBCtWN2TN/LsGZdB
-----END PRIVATE KEY-----"#;
    const TEST_JWK_N: &str = "yRE6rHuNR0QbHO3H3Kt2pOKGVhQqGZXInOduQNxXzuKlvQTLUTv4l4sggh5_CYYi_cvI-SXVT9kPWSKXxJXBXd_4LkvcPuUakBoAkfh-eiFVMh2VrUyWyj3MFl0HTVF9KwRXLAcwkREiS3npThHRyIxuy0ZMeZfxVL5arMhw1SRELB8HoGfG_AtH89BIE9jDBHZ9dLelK9a184zAf8LwoPLxvJb3Il5nncqPcSfKDDodMFBIMc4lQzDKL5gvmiXLXB1AGLm8KBjfE8s3L5xqi-yUod-j8MtvIj812dkS4QMiRVN_by2h3ZY8LYVGrqZXZTcgn2ujn8uKjXLZVD5TdQ";
    const TEST_JWK_E: &str = "AQAB";

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
            entra_oidc_metadata_url: None,
            entra_admin_group_id: Some("group-123".to_owned()),
            entra_admin_user_oid: None,
            entra_redirect_uri: Some("http://localhost:8080/admin/callback".to_owned()),
            static_output_dir: "./dist".into(),
            static_publish_backend: "local".to_owned(),
            static_publish_prefix: "site".to_owned(),
            observability_backend: "noop".to_owned(),
            application_insights_connection_string: None,
            base_url: "http://127.0.0.1:8080".to_owned(),
            slack_webhook_url: None,
        }
    }

    fn bearer_for(payload: &str) -> String {
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none","typ":"JWT"}"#);
        let claims = URL_SAFE_NO_PAD.encode(payload);
        format!("{header}.{claims}.")
    }

    #[derive(Debug, Clone, Serialize)]
    struct OidcTestClaims {
        aud: String,
        iss: String,
        tid: String,
        oid: String,
        groups: Vec<String>,
        exp: usize,
    }

    #[tokio::test]
    async fn accepts_matching_entra_poc_claims() {
        let service = EntraPocAdminAuthService::new(sample_config());

        let identity = service
            .authenticate_bearer(&bearer_for(
                r#"{"aud":"client-123","tid":"tenant-123","groups":["group-123"],"oid":"user-1"}"#,
            ))
            .await
            .unwrap();

        assert_eq!(identity.oid.as_deref(), Some("user-1"));
    }

    #[tokio::test]
    async fn rejects_when_group_does_not_match() {
        let service = EntraPocAdminAuthService::new(sample_config());

        let error = service
            .authenticate_bearer(&bearer_for(
                r#"{"aud":"client-123","tid":"tenant-123","groups":["group-x"]}"#,
            ))
            .await
            .unwrap_err();

        assert_eq!(error, AdminAuthError::Forbidden("admin group mismatch"));
    }

    #[tokio::test]
    async fn oidc_metadata_url_defaults_from_tenant() {
        let config = sample_config();

        let url = oidc_metadata_url(&config).unwrap();

        assert_eq!(
            url,
            "https://login.microsoftonline.com/tenant-123/v2.0/.well-known/openid-configuration"
        );
    }

    #[tokio::test]
    async fn accepts_matching_entra_oidc_claims() {
        let mut config = sample_config();
        config.admin_auth_mode = "entra-oidc".to_owned();
        let base_url = "https://login.microsoftonline.com/tenant-123/v2.0";
        let service = EntraOidcAdminAuthService {
            config,
            client: Client::new(),
            metadata_url: "https://example.invalid/.well-known/openid-configuration".to_owned(),
            provider_state: RwLock::new(Some(OidcProviderState {
                issuer: format!("{base_url}/issuer"),
                jwks: JsonWebKeySet {
                    keys: vec![JsonWebKey {
                        kty: "RSA".to_owned(),
                        n: TEST_JWK_N.to_owned(),
                        e: TEST_JWK_E.to_owned(),
                        kid: Some("rsa01".to_owned()),
                        use_field: Some("sig".to_owned()),
                    }],
                },
            })),
        };
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("rsa01".to_owned());
        let token = encode(
            &header,
            &OidcTestClaims {
                aud: "client-123".to_owned(),
                iss: format!("{base_url}/issuer"),
                tid: "tenant-123".to_owned(),
                oid: "user-1".to_owned(),
                groups: vec!["group-123".to_owned()],
                exp: (Utc::now().timestamp() + 3600) as usize,
            },
            &EncodingKey::from_rsa_pem(TEST_RSA_PRIVATE_KEY.as_bytes()).unwrap(),
        )
        .unwrap();

        let identity = service.authenticate_bearer(&token).await.unwrap();

        assert_eq!(identity.oid.as_deref(), Some("user-1"));
    }
}
