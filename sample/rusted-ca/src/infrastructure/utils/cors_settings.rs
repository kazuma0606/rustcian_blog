use axum::http::{HeaderName, Method};

pub fn allowed_origins() -> Vec<String> {
    vec![
        "http://localhost:3000".to_string(),
        "https://your-production-domain.com".to_string(),
    ]
}

pub fn allowed_methods() -> Vec<Method> {
    vec![
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::OPTIONS,
    ]
}

pub fn allowed_headers() -> Vec<HeaderName> {
    vec![
        HeaderName::from_static("authorization"),
        HeaderName::from_static("content-type"),
    ]
}
