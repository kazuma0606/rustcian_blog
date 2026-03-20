use actix_web::{HttpRequest, HttpResponse, Result, get, http::header::ContentType, web};
use rustacian_blog_core::{BlogError, Post, PostSummary, PostVisibility};
use rustacian_blog_frontend::{render_post_page, render_posts_page};

use crate::auth::{AdminAuthError, validate_admin_request};
use crate::state::AppState;

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(health)
        .configure(public_routes)
        .configure(admin_routes);
}

fn public_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_posts)
        .service(get_post)
        .service(index_page)
        .service(post_page);
}

fn admin_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(admin_preview_placeholder);
}

#[get("/health")]
async fn health(data: web::Data<AppState>) -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "environment": data.config.app_env,
        "storage_backend": data.config.storage_backend,
        "azurite_blob_endpoint": data.config.azurite_blob_endpoint,
        "azurite_table_endpoint": data.config.azurite_table_endpoint,
    }))
}

#[get("/posts")]
async fn list_posts(data: web::Data<AppState>) -> Result<HttpResponse> {
    let posts = data
        .list_posts
        .execute()
        .await
        .map_err(internal_app_error)?;

    Ok(HttpResponse::Ok().json(posts))
}

#[get("/posts/{slug}")]
async fn get_post(path: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let post = data
        .get_post
        .execute(&path.into_inner())
        .await
        .map_err(api_app_error)?;

    Ok(HttpResponse::Ok().json(post))
}

#[get("/")]
async fn index_page(data: web::Data<AppState>) -> Result<HttpResponse> {
    let posts = data
        .list_posts
        .execute()
        .await
        .map_err(internal_app_error)?;

    Ok(html_response(render_posts_page(map_summaries(posts))))
}

#[get("/p/{slug}")]
async fn post_page(path: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let post = data
        .get_post
        .execute(&path.into_inner())
        .await
        .map_err(page_app_error)?;

    Ok(html_response(render_post_page(map_post(post))))
}

#[get("/admin/preview/{slug}")]
async fn admin_preview_placeholder(
    path: web::Path<String>,
    request: HttpRequest,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    validate_admin_request(request.headers(), &data.config).map_err(admin_auth_error)?;
    let slug = path.into_inner();
    let post = data
        .get_post
        .execute_with_visibility(&slug, PostVisibility::IncludeDrafts)
        .await
        .map_err(api_app_error)?;

    Ok(html_response(render_post_page(map_post(post))))
}

fn html_response(body: String) -> HttpResponse {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body)
}

fn internal_app_error(error: BlogError) -> actix_web::Error {
    log_content_error(&error);
    actix_web::error::ErrorInternalServerError("content is unavailable")
}

fn api_app_error(error: BlogError) -> actix_web::Error {
    match error {
        BlogError::NotFound(_) => actix_web::error::ErrorNotFound("post not found"),
        other => internal_app_error(other),
    }
}

fn page_app_error(error: BlogError) -> actix_web::Error {
    match error {
        BlogError::NotFound(_) => actix_web::error::ErrorNotFound("post not found"),
        other => {
            log_content_error(&other);
            actix_web::error::ErrorInternalServerError("content is unavailable")
        }
    }
}

fn admin_auth_error(error: AdminAuthError) -> actix_web::Error {
    match error {
        AdminAuthError::MissingBearerToken | AdminAuthError::InvalidToken(_) => {
            actix_web::error::ErrorUnauthorized("admin authentication required")
        }
        AdminAuthError::Forbidden(_) => actix_web::error::ErrorForbidden("admin access denied"),
        AdminAuthError::Disabled | AdminAuthError::MissingConfiguration(_) => {
            actix_web::error::ErrorNotImplemented("admin preview is not configured")
        }
    }
}

fn log_content_error(error: &BlogError) {
    eprintln!("content loading error: {error}");
}

fn map_summaries(posts: Vec<PostSummary>) -> Vec<rustacian_blog_frontend::PostSummaryView> {
    posts
        .into_iter()
        .map(|post| {
            let slug = post.slug.clone();
            rustacian_blog_frontend::PostSummaryView {
                title: post.title,
                slug,
                published_at: post.published_at.format("%Y-%m-%d").to_string(),
                updated_at: post
                    .updated_at
                    .map(|date| date.format("%Y-%m-%d").to_string()),
                tags: post.tags,
                summary: post.summary,
                hero_image: post
                    .hero_image
                    .map(|value| resolve_asset_url(&value, &post.slug)),
                toc: post.toc,
                math: post.math,
            }
        })
        .collect()
}

fn map_post(post: Post) -> rustacian_blog_frontend::PostView {
    let slug = post.slug.clone();
    rustacian_blog_frontend::PostView {
        title: post.title,
        slug: slug.clone(),
        published_at: post.published_at.format("%Y-%m-%d").to_string(),
        updated_at: post
            .updated_at
            .map(|date| date.format("%Y-%m-%d").to_string()),
        tags: post.tags,
        summary: post.summary,
        hero_image: post
            .hero_image
            .map(|value| resolve_asset_url(&value, &slug)),
        toc: post.toc,
        math: post.math,
        summary_ai: post.summary_ai,
        charts: post
            .chart_data
            .into_iter()
            .map(|chart| rustacian_blog_frontend::RenderedChartView {
                chart_type: chart.chart_type,
                source: resolve_asset_url(&chart.source, &slug),
                x: chart.x,
                y: chart.y,
                title: chart.title,
                caption: chart.caption,
                points: chart
                    .points
                    .into_iter()
                    .map(|point| rustacian_blog_frontend::ChartPointView {
                        x: point.x,
                        y: point.y,
                    })
                    .collect(),
            })
            .collect(),
        toc_items: post
            .toc_items
            .into_iter()
            .map(|item| rustacian_blog_frontend::TocItemView {
                level: item.level,
                title: item.title,
                anchor: item.anchor,
            })
            .collect(),
        body_html: resolve_body_asset_urls(&post.body_html, &slug),
    }
}

fn resolve_asset_url(asset_ref: &str, slug: &str) -> String {
    if asset_ref.starts_with("http://")
        || asset_ref.starts_with("https://")
        || asset_ref.starts_with('/')
    {
        return asset_ref.to_owned();
    }

    let normalized = asset_ref.replace('\\', "/");
    let relative = normalized.trim_start_matches("./");
    format!("/assets/posts/{slug}/{relative}")
}

fn resolve_body_asset_urls(body_html: &str, slug: &str) -> String {
    body_html
        .replace("src=\"./", &format!("src=\"/assets/posts/{slug}/"))
        .replace("src=\"../", &format!("src=\"/assets/posts/{slug}/../"))
        .replace("href=\"./", &format!("href=\"/assets/posts/{slug}/"))
        .replace("href=\"../", &format!("href=\"/assets/posts/{slug}/../"))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use actix_web::{App, http::StatusCode, test, web};
    use async_trait::async_trait;
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use chrono::{DateTime, Utc};
    use rustacian_blog_core::{
        GetPostUseCase, ListPostsUseCase, PostMetadata, PostRepository, PostStatus, PostVisibility,
    };

    use super::*;
    use crate::{config::AppConfig, state::AppState};

    struct MockRepository {
        list_result: Result<Vec<PostSummary>, BlogError>,
        get_result: Result<Post, BlogError>,
    }

    #[async_trait]
    impl PostRepository for MockRepository {
        async fn list_posts(
            &self,
            _visibility: PostVisibility,
        ) -> Result<Vec<PostSummary>, BlogError> {
            self.list_result.clone()
        }

        async fn get_post(
            &self,
            _slug: &str,
            _visibility: PostVisibility,
        ) -> Result<Post, BlogError> {
            self.get_result.clone()
        }
    }

    fn app_state(repository: Arc<dyn PostRepository>) -> AppState {
        AppState {
            list_posts: ListPostsUseCase::new(repository.clone()),
            get_post: GetPostUseCase::new(repository),
            config: AppConfig {
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
                admin_auth_mode: "disabled".to_owned(),
                entra_tenant_id: None,
                entra_client_id: None,
                entra_admin_group_id: None,
                entra_admin_user_oid: None,
            },
        }
    }

    fn app_state_with_admin_auth(repository: Arc<dyn PostRepository>) -> AppState {
        let mut state = app_state(repository);
        state.config.admin_auth_mode = "entra-poc".to_owned();
        state.config.entra_tenant_id = Some("tenant-123".to_owned());
        state.config.entra_client_id = Some("client-123".to_owned());
        state.config.entra_admin_group_id = Some("group-123".to_owned());
        state
    }

    fn sample_post() -> Post {
        Post::new(
            PostMetadata {
                title: "Sample".to_owned(),
                slug: "sample".to_owned(),
                published_at: DateTime::parse_from_rfc3339("2026-03-19T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                updated_at: None,
                tags: vec!["rust".to_owned()],
                summary: "summary".to_owned(),
                hero_image: None,
                status: PostStatus::Published,
                toc: false,
                math: false,
                summary_ai: None,
                charts: Vec::new(),
            },
            Vec::new(),
            "# Hello".to_owned(),
            "<h1>Hello</h1>".to_owned(),
        )
        .unwrap()
    }

    fn bearer_for(payload: &str) -> String {
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none","typ":"JWT"}"#);
        let claims = URL_SAFE_NO_PAD.encode(payload);
        format!("Bearer {header}.{claims}.")
    }

    #[actix_web::test]
    async fn post_api_returns_not_found_for_missing_post() {
        let state = app_state(Arc::new(MockRepository {
            list_result: Ok(Vec::new()),
            get_result: Err(BlogError::NotFound("missing".to_owned())),
        }));

        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response = test::call_service(
            &app,
            test::TestRequest::get().uri("/posts/missing").to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn posts_api_hides_validation_details_with_internal_server_error() {
        let state = app_state(Arc::new(MockRepository {
            list_result: Err(BlogError::Validation("broken content".to_owned())),
            get_result: Ok(sample_post()),
        }));

        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response =
            test::call_service(&app, test::TestRequest::get().uri("/posts").to_request()).await;

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[actix_web::test]
    async fn admin_preview_route_is_reserved_for_future_use() {
        let state = app_state(Arc::new(MockRepository {
            list_result: Ok(Vec::new()),
            get_result: Ok(sample_post()),
        }));

        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/admin/preview/sample")
                .to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[actix_web::test]
    async fn admin_preview_requires_bearer_token_when_entra_poc_is_enabled() {
        let state = app_state_with_admin_auth(Arc::new(MockRepository {
            list_result: Ok(Vec::new()),
            get_result: Ok(sample_post()),
        }));

        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/admin/preview/sample")
                .to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn admin_preview_can_render_draft_post_with_matching_group_claim() {
        let mut draft = sample_post();
        draft.status = PostStatus::Draft;
        let state = app_state_with_admin_auth(Arc::new(MockRepository {
            list_result: Ok(Vec::new()),
            get_result: Ok(draft),
        }));

        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/admin/preview/sample")
                .insert_header((
                    "authorization",
                    bearer_for(
                        r#"{"aud":"client-123","tid":"tenant-123","groups":["group-123"],"oid":"user-1"}"#,
                    ),
                ))
                .to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn resolves_article_asset_urls_for_post_view() {
        let mut post = sample_post();
        post.hero_image = Some("./hero.png".to_owned());
        post.body_html =
            "<p><img src=\"./diagram.svg\" alt=\"x\"></p><p><a href=\"./data.csv\">data</a></p>"
                .to_owned();

        let view = map_post(post);

        assert_eq!(
            view.hero_image.as_deref(),
            Some("/assets/posts/sample/hero.png")
        );
        assert!(
            view.body_html
                .contains("src=\"/assets/posts/sample/diagram.svg\"")
        );
        assert!(
            view.body_html
                .contains("href=\"/assets/posts/sample/data.csv\"")
        );
    }
}
