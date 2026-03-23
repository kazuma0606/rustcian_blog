use crate::auth::{build_auth_redirect_url, exchange_code_for_token};
use crate::state::AppState;
use actix_multipart::Multipart;
use actix_web::{
    HttpRequest, HttpResponse, Result, cookie::Cookie, delete, get, http::header::ContentType,
    post, web,
};
use futures_util::TryStreamExt;
use rustacian_blog_core::{
    AdminAuthError, AiGenerationScope, BlogError, Comment, CommentStatus, ContactMessage, Post,
    PostSummary, PostVisibility, SearchQuery,
};
use rustacian_blog_frontend::{
    CommentView, GeneratedMetadataView, ImageView, PostSummaryView, SearchResultView,
    render_admin_comments, render_admin_dashboard, render_admin_image_gallery,
    render_admin_post_detail, render_admin_static_panel, render_comment_list, render_contact_page,
    render_en_post_page, render_en_posts_page, render_login_page, render_post_page,
    render_posts_page, render_search_page,
};
use std::{fs, path::Path};

use crate::comment_store::new_id;
use crate::observability::AppEvent;
use rustacian_blog_core::NotificationEvent;

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(health).service(
        web::scope("")
            .configure(public_routes)
            .service(web::scope("/admin").configure(admin_routes)),
    );
}

fn public_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_posts)
        .service(get_post)
        .service(get_image)
        .service(index_page)
        .service(post_page)
        .service(list_post_comments)
        .service(post_comment)
        .service(contact_page)
        .service(post_contact)
        .service(search_page)
        .service(en_index)
        .service(en_post_page);
}

fn admin_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(admin_login)
        .service(admin_callback)
        .service(admin_home)
        .service(admin_preview_placeholder)
        .service(admin_post_detail)
        .service(admin_static_panel)
        .service(generate_ai_metadata)
        .service(regenerate_static_site)
        .service(admin_comments)
        .service(approve_comment)
        .service(reject_comment)
        .service(admin_image_gallery)
        .service(admin_list_images)
        .service(admin_upload_image)
        .service(admin_delete_image)
        .service(admin_set_hero)
        .service(admin_describe_image);
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
    data.observability.emit(AppEvent::PublicRequestServed {
        route: "posts_api",
        slug: None,
    });

    Ok(HttpResponse::Ok().json(posts))
}

#[get("/posts/{slug}")]
async fn get_post(path: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let post = data
        .get_post
        .execute(&path.into_inner())
        .await
        .map_err(api_app_error)?;
    let slug = post.slug.clone();
    data.observability.emit(AppEvent::PublicRequestServed {
        route: "post_api",
        slug: Some(slug),
    });

    Ok(HttpResponse::Ok().json(post))
}

const PER_PAGE: usize = 10;

#[derive(serde::Deserialize, Default)]
struct PageQuery {
    page: Option<usize>,
}

#[get("/")]
async fn index_page(
    query: web::Query<PageQuery>,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    let all_posts = data
        .list_posts
        .execute()
        .await
        .map_err(internal_app_error)?;
    data.observability.emit(AppEvent::PublicRequestServed {
        route: "index_page",
        slug: None,
    });

    let total = all_posts.len();
    let total_pages = total.div_ceil(PER_PAGE);
    let page = query.page.unwrap_or(1).max(1).min(total_pages.max(1));
    let start = (page - 1) * PER_PAGE;
    let posts = all_posts.into_iter().skip(start).take(PER_PAGE).collect();

    Ok(html_response(render_posts_page(
        map_summaries(posts),
        page,
        total_pages,
    )))
}

#[get("/p/{slug}")]
async fn post_page(
    request: HttpRequest,
    path: web::Path<String>,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    let slug = path.into_inner();
    let post = data.get_post.execute(&slug).await.map_err(page_app_error)?;
    data.observability.emit(AppEvent::PublicRequestServed {
        route: "post_page",
        slug: Some(slug.clone()),
    });
    if let Some(analytics) = &data.analytics {
        let ip = peer_ip(&request);
        analytics.record_page_view(slug.clone(), ip.clone());
        analytics.record_session_step(slug.clone(), ip);
    }

    let en_url = data
        .translator
        .is_some()
        .then(|| format!("/en/posts/{}", &slug));
    Ok(html_response(render_post_page(
        map_post(post),
        en_url.as_deref(),
    )))
}

#[get("/images/{path:.*}")]
async fn get_image(path: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let relative = path.into_inner();
    if relative.is_empty() || relative.contains("..") || relative.contains('\\') {
        return Err(actix_web::error::ErrorNotFound("image not found"));
    }

    if data.config.storage_backend == "azurite"
        && let Some(blob) = &data.image_blob
        && let Some((bytes, content_type)) = blob
            .get_bytes(&format!("images/{relative}"))
            .await
            .map_err(internal_app_error)?
    {
        return Ok(binary_response(
            bytes,
            content_type
                .as_deref()
                .unwrap_or("application/octet-stream"),
        ));
    }

    let file_path = data.config.images_dir().join(&relative);
    if !file_path.exists() {
        return Err(actix_web::error::ErrorNotFound("image not found"));
    }
    let bytes = fs::read(&file_path)
        .map_err(|error| internal_app_error(BlogError::Storage(error.to_string())))?;

    Ok(binary_response(bytes, infer_content_type(&file_path)))
}

#[get("")]
async fn admin_home(request: HttpRequest, data: web::Data<AppState>) -> Result<HttpResponse> {
    authenticate_admin(&request, &data, "admin_home")
        .await
        .map_err(admin_auth_error)?;
    let posts = data
        .list_posts
        .execute_with_visibility(PostVisibility::IncludeDrafts)
        .await
        .map_err(internal_app_error)?;
    let mut response = html_response(render_admin_dashboard(map_summaries(posts)));
    attach_admin_session_cookie(&request, &mut response);
    Ok(response)
}

#[get("/preview/{slug}")]
async fn admin_preview_placeholder(
    path: web::Path<String>,
    request: HttpRequest,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    authenticate_admin(&request, &data, "admin_preview")
        .await
        .map_err(admin_auth_error)?;
    let slug = path.into_inner();
    let post = data
        .get_post
        .execute_with_visibility(&slug, PostVisibility::IncludeDrafts)
        .await
        .map_err(api_app_error)?;

    Ok(html_response(render_post_page(map_post(post), None)))
}

#[get("/posts/{slug}")]
async fn admin_post_detail(
    path: web::Path<String>,
    request: HttpRequest,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    authenticate_admin(&request, &data, "admin_post_detail")
        .await
        .map_err(admin_auth_error)?;
    let slug = path.into_inner();
    let post = data
        .get_post
        .execute_with_visibility(&slug, PostVisibility::IncludeDrafts)
        .await
        .map_err(api_app_error)?;

    // Try to load previously generated AI metadata from metadata_dir/<slug>.json
    let metadata = load_generated_metadata(&data.config, &slug).await;

    Ok(html_response(render_admin_post_detail(
        map_post(post),
        metadata,
    )))
}

#[get("/static")]
async fn admin_static_panel(
    request: HttpRequest,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    authenticate_admin(&request, &data, "admin_static_panel")
        .await
        .map_err(admin_auth_error)?;
    Ok(html_response(render_admin_static_panel()))
}

#[actix_web::post("/ai/{slug}/metadata")]
async fn generate_ai_metadata(
    path: web::Path<String>,
    request: HttpRequest,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    authenticate_admin(&request, &data, "admin_ai_metadata")
        .await
        .map_err(admin_auth_error)?;
    let use_case = data
        .generate_ai_metadata
        .clone()
        .ok_or_else(|| actix_web::error::ErrorNotImplemented("ai metadata is not configured"))?;
    let slug = path.into_inner();
    let generated = match use_case.execute(&slug, AiGenerationScope::default()).await {
        Ok(generated) => {
            data.observability.emit(AppEvent::AiMetadataGenerated {
                slug: slug.clone(),
                outcome: "success",
                source_model: generated.source_model.clone(),
            });
            let _ = data
                .notification
                .notify(NotificationEvent::AiMetadataGenerated {
                    slug: slug.clone(),
                    outcome: "success".to_owned(),
                })
                .await;
            generated
        }
        Err(error) => {
            data.observability.emit(AppEvent::AiMetadataGenerated {
                slug: slug.clone(),
                outcome: "error",
                source_model: None,
            });
            return Err(api_app_error(error));
        }
    };

    Ok(HttpResponse::Ok().json(generated))
}

#[actix_web::post("/static/regenerate")]
async fn regenerate_static_site(
    request: HttpRequest,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    authenticate_admin(&request, &data, "admin_static_regenerate")
        .await
        .map_err(admin_auth_error)?;
    let use_case = data.publish_static_site.clone().ok_or_else(|| {
        actix_web::error::ErrorNotImplemented("static publishing is not configured")
    })?;
    let build = use_case.execute().await.map_err(internal_app_error)?;
    let page_count = build.pages.len();

    // Rebuild the search index from all published posts.
    {
        let slugs = data.list_posts.execute().await.unwrap_or_default();
        let mut posts = Vec::with_capacity(slugs.len());
        for s in &slugs {
            if let Ok(post) = data.get_post.execute(&s.slug).await {
                posts.push(post);
            }
        }
        let _ = data.search_index.rebuild(&posts);
    }

    let _ = data
        .notification
        .notify(NotificationEvent::StaticSiteRebuilt {
            page_count,
            outcome: "success".to_owned(),
        })
        .await;

    // Purge Cloudflare cache after successful publish (best-effort)
    let cf_purged = if let Some(ref cf) = data.cloudflare {
        match cf.purge_all().await {
            Ok(()) => true,
            Err(e) => {
                eprintln!("cloudflare purge failed: {e}");
                false
            }
        }
    } else {
        false
    };

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "pages": page_count,
        "assets": build.assets.len(),
        "cloudflare_purged": cf_purged,
        "target": match data.config.static_publish_backend.as_str() {
            "azurite" => format!("azurite:{}", data.config.static_publish_prefix),
            _ => format!("local:{}", data.config.static_output_dir.display()),
        }
    })))
}

// ---------------------------------------------------------------------------
// Comment routes
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct CommentForm {
    author_name: String,
    content: String,
}

#[derive(serde::Deserialize)]
struct ContactForm {
    from_name: String,
    from_email: String,
    body: String,
}

#[get("/posts/{slug}/comments")]
async fn list_post_comments(
    path: web::Path<String>,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    let slug = path.into_inner();
    let comments = data
        .comment_repo
        .list_comments(&slug, false)
        .await
        .map_err(api_app_error)?;
    let views: Vec<CommentView> = comments
        .into_iter()
        .map(|c| CommentView {
            id: c.id,
            author_name: c.author_name,
            content: c.content,
            created_at: c.created_at.format("%Y-%m-%d %H:%M").to_string(),
        })
        .collect();
    Ok(html_response(render_comment_list(&slug, views)))
}

#[actix_web::post("/posts/{slug}/comments")]
async fn post_comment(
    path: web::Path<String>,
    form: web::Form<CommentForm>,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    let slug = path.into_inner();
    let author_name = sanitize(&form.author_name);
    let content = sanitize(&form.content);
    let comment = Comment {
        id: new_id(),
        post_slug: slug.clone(),
        author_name: author_name.clone(),
        content,
        created_at: chrono::Utc::now(),
        status: CommentStatus::Pending,
    };
    data.comment_repo
        .create_comment(&comment)
        .await
        .map_err(api_app_error)?;
    let _ = data
        .notification
        .notify(NotificationEvent::CommentReceived {
            slug: slug.clone(),
            author_name,
        })
        .await;
    Ok(HttpResponse::SeeOther()
        .insert_header(("Location", format!("/posts/{slug}/comments")))
        .finish())
}

#[get("/contact")]
async fn contact_page() -> HttpResponse {
    html_response(render_contact_page())
}

#[actix_web::post("/contact")]
async fn post_contact(
    form: web::Form<ContactForm>,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    let from_name = sanitize(&form.from_name);
    let from_email = sanitize(&form.from_email);
    let body = sanitize(&form.body);
    let msg = ContactMessage {
        id: new_id(),
        from_name: from_name.clone(),
        from_email,
        body,
        created_at: chrono::Utc::now(),
    };
    data.contact_repo
        .create_contact_message(&msg)
        .await
        .map_err(api_app_error)?;
    let _ = data
        .notification
        .notify(NotificationEvent::ContactFormSubmitted { from_name })
        .await;
    Ok(HttpResponse::SeeOther()
        .insert_header(("Location", "/contact?sent=1"))
        .finish())
}

// ---------------------------------------------------------------------------
// Search route
// ---------------------------------------------------------------------------

#[get("/search")]
async fn search_page(query: web::Query<SearchQuery>, data: web::Data<AppState>) -> HttpResponse {
    let q = query.q.trim();
    let results: Vec<SearchResultView> = if q.is_empty() {
        Vec::new()
    } else {
        data.search_index
            .search(q, 20)
            .unwrap_or_default()
            .into_iter()
            .map(|r| SearchResultView {
                slug: r.slug,
                title: r.title,
                excerpt: r.excerpt,
                tags: r.tags,
                date: r.date,
            })
            .collect()
    };
    if !q.is_empty()
        && let Some(analytics) = &data.analytics
    {
        analytics.record_search(q.to_owned(), results.len());
    }
    html_response(render_search_page(q, results))
}

// ---------------------------------------------------------------------------
// Admin comment moderation routes
// ---------------------------------------------------------------------------

#[get("/comments")]
async fn admin_comments(request: HttpRequest, data: web::Data<AppState>) -> Result<HttpResponse> {
    authenticate_admin(&request, &data, "admin_comments")
        .await
        .map_err(admin_auth_error)?;
    let pending = data
        .comment_repo
        .list_all_pending()
        .await
        .map_err(internal_app_error)?;
    let views: Vec<CommentView> = pending
        .into_iter()
        .map(|c| CommentView {
            id: c.id,
            author_name: c.author_name,
            content: c.content,
            created_at: c.created_at.format("%Y-%m-%d %H:%M").to_string(),
        })
        .collect();
    Ok(html_response(render_admin_comments(views)))
}

#[actix_web::post("/comments/{id}/approve")]
async fn approve_comment(
    path: web::Path<String>,
    request: HttpRequest,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    authenticate_admin(&request, &data, "admin_comments_approve")
        .await
        .map_err(admin_auth_error)?;
    let id = path.into_inner();
    data.comment_repo
        .update_status(&id, CommentStatus::Approved)
        .await
        .map_err(api_app_error)?;
    Ok(HttpResponse::SeeOther()
        .insert_header(("Location", "/admin/comments"))
        .finish())
}

#[actix_web::post("/comments/{id}/reject")]
async fn reject_comment(
    path: web::Path<String>,
    request: HttpRequest,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    authenticate_admin(&request, &data, "admin_comments_reject")
        .await
        .map_err(admin_auth_error)?;
    let id = path.into_inner();
    data.comment_repo
        .update_status(&id, CommentStatus::Rejected)
        .await
        .map_err(api_app_error)?;
    Ok(HttpResponse::SeeOther()
        .insert_header(("Location", "/admin/comments"))
        .finish())
}

// ---------------------------------------------------------------------------
// Admin helper: load previously generated AI metadata from disk
// ---------------------------------------------------------------------------

async fn load_generated_metadata(
    config: &crate::config::AppConfig,
    slug: &str,
) -> Option<GeneratedMetadataView> {
    let path = config.metadata_dir().join(format!("{slug}.json"));
    let bytes = std::fs::read(&path).ok()?;
    let val: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    Some(GeneratedMetadataView {
        summary_ai: val["summary_ai"].as_str().map(str::to_owned),
        suggested_tags: val["suggested_tags"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default(),
        intro_candidates: val["intro_candidates"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default(),
        generated_at: val["generated_at"].as_str().unwrap_or("").to_owned(),
        source_model: val["source_model"].as_str().map(str::to_owned),
    })
}

// ---------------------------------------------------------------------------
// Sanitization helper
// ---------------------------------------------------------------------------

fn sanitize(input: &str) -> String {
    ammonia::Builder::new()
        .tags(std::collections::HashSet::new())
        .clean(input)
        .to_string()
}

async fn authenticate_admin(
    request: &HttpRequest,
    data: &web::Data<AppState>,
    route: &'static str,
) -> Result<rustacian_blog_core::AdminIdentity, AdminAuthError> {
    if data.config.admin_auth_mode == "local-dev" {
        let identity = rustacian_blog_core::AdminIdentity {
            oid: Some("local-dev".to_owned()),
            preferred_username: Some("local-dev".to_owned()),
            groups: vec!["local-dev".to_owned()],
        };
        data.observability.emit(AppEvent::AdminAuthChecked {
            route,
            outcome: "local_dev",
        });
        return Ok(identity);
    }

    if data.config.admin_auth_mode == "disabled" {
        data.observability.emit(AppEvent::AdminAuthChecked {
            route,
            outcome: "disabled",
        });
        return Err(AdminAuthError::Disabled);
    }

    let raw = request
        .headers()
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
        .or_else(|| {
            request
                .cookie("admin_session")
                .map(|cookie| format!("Bearer {}", cookie.value()))
        })
        .ok_or(AdminAuthError::MissingBearerToken)?;
    let token = raw
        .strip_prefix("Bearer ")
        .ok_or(AdminAuthError::MissingBearerToken)?;
    let result = data.admin_auth.authenticate_bearer(token).await;
    data.observability.emit(AppEvent::AdminAuthChecked {
        route,
        outcome: match &result {
            Ok(_) => "success",
            Err(AdminAuthError::MissingBearerToken) => "missing_bearer",
            Err(AdminAuthError::InvalidToken(_)) => "invalid_token",
            Err(AdminAuthError::Forbidden(_)) => "forbidden",
            Err(AdminAuthError::Disabled) => "disabled",
            Err(AdminAuthError::MissingConfiguration(_)) => "misconfigured",
            Err(AdminAuthError::ProviderUnavailable(_)) => "provider_unavailable",
        },
    });
    result
}

/// Extract the peer IP address from the request, preferring X-Forwarded-For.
fn peer_ip(request: &HttpRequest) -> String {
    request
        .headers()
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_owned())
        .or_else(|| request.peer_addr().map(|addr| addr.ip().to_string()))
        .unwrap_or_else(|| "unknown".to_owned())
}

fn html_response(body: String) -> HttpResponse {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body)
}

fn binary_response(body: Vec<u8>, content_type: &str) -> HttpResponse {
    HttpResponse::Ok().content_type(content_type).body(body)
}

fn attach_admin_session_cookie(request: &HttpRequest, response: &mut HttpResponse) {
    let Some(raw) = request
        .headers()
        .get("authorization")
        .and_then(|value| value.to_str().ok())
    else {
        return;
    };
    let Some(token) = raw.strip_prefix("Bearer ") else {
        return;
    };

    let cookie = Cookie::build("admin_session", token.to_owned())
        .http_only(true)
        .path("/")
        .finish();
    let _ = response.add_cookie(&cookie);
}

fn internal_app_error(error: BlogError) -> actix_web::Error {
    log_content_error("internal_app_error", &error);
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
            log_content_error("page_app_error", &other);
            actix_web::error::ErrorInternalServerError("content is unavailable")
        }
    }
}

// ---------------------------------------------------------------------------
// Admin: Entra PKCE login / callback
// ---------------------------------------------------------------------------

#[get("/login")]
async fn admin_login(request: HttpRequest, data: web::Data<AppState>) -> Result<HttpResponse> {
    // If already authenticated, skip login
    if authenticate_admin(&request, &data, "admin_login")
        .await
        .is_ok()
    {
        return Ok(HttpResponse::Found()
            .insert_header(("Location", "/admin"))
            .finish());
    }

    if data.config.admin_auth_mode != "entra" && data.config.admin_auth_mode != "entra-oidc" {
        return Ok(html_response(render_login_page(Some(
            "管理者認証が設定されていません",
        ))));
    }

    match build_auth_redirect_url(&data.config) {
        Ok(pkce) => Ok(HttpResponse::Found()
            .insert_header(("Location", pkce.auth_url.as_str()))
            .finish()),
        Err(_) => Ok(html_response(render_login_page(Some(
            "Entra 設定が不足しています (ENTRA_TENANT_ID / ENTRA_CLIENT_ID / ENTRA_REDIRECT_URI)",
        )))),
    }
}

#[derive(serde::Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[get("/callback")]
async fn admin_callback(
    data: web::Data<AppState>,
    query: web::Query<CallbackQuery>,
) -> Result<HttpResponse> {
    if let Some(ref err) = query.error {
        let description = query.error_description.as_deref().unwrap_or(err.as_str());
        return Ok(html_response(render_login_page(Some(description))));
    }

    let code = query
        .code
        .as_deref()
        .ok_or_else(|| actix_web::error::ErrorBadRequest("missing code parameter"))?;
    let state = query
        .state
        .as_deref()
        .ok_or_else(|| actix_web::error::ErrorBadRequest("missing state parameter"))?;

    let id_token = exchange_code_for_token(&data.config, &data.http_client, code, state)
        .await
        .map_err(|e| {
            actix_web::error::ErrorUnauthorized(format!("token exchange failed: {e:?}"))
        })?;

    let cookie = Cookie::build("admin_session", id_token)
        .path("/admin")
        .http_only(true)
        .secure(data.config.app_env != "local")
        .max_age(actix_web::cookie::time::Duration::hours(8))
        .finish();

    Ok(HttpResponse::Found()
        .cookie(cookie)
        .insert_header(("Location", "/admin"))
        .finish())
}

// ---------------------------------------------------------------------------
// Admin: Image management
// ---------------------------------------------------------------------------

#[get("/images")]
async fn admin_image_gallery(
    request: HttpRequest,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    authenticate_admin(&request, &data, "admin_image_gallery")
        .await
        .map_err(admin_auth_error)?;
    let images = if let Some(blob) = data.image_blob.as_ref() {
        blob.list_blobs("images/")
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?
            .into_iter()
            .map(|item| {
                let name = item
                    .name
                    .strip_prefix("images/")
                    .unwrap_or(&item.name)
                    .to_owned();
                ImageView {
                    url: format!("/images/{name}"),
                    name,
                    content_type: item.content_type,
                    last_modified: item.last_modified,
                    size: item.size,
                }
            })
            .collect()
    } else {
        vec![]
    };
    Ok(html_response(render_admin_image_gallery(images)))
}

#[get("/images/list")]
async fn admin_list_images(
    request: HttpRequest,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    authenticate_admin(&request, &data, "admin_list_images")
        .await
        .map_err(admin_auth_error)?;
    let blob = data
        .image_blob
        .as_ref()
        .ok_or_else(|| actix_web::error::ErrorServiceUnavailable("blob storage not configured"))?;
    let items = blob
        .list_blobs("images/")
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    let json: Vec<serde_json::Value> = items
        .into_iter()
        .map(|item| {
            let filename = item
                .name
                .strip_prefix("images/")
                .unwrap_or(&item.name)
                .to_owned();
            serde_json::json!({
                "name": filename,
                "url": format!("/images/{filename}"),
                "content_type": item.content_type,
                "last_modified": item.last_modified,
                "size": item.size,
            })
        })
        .collect();
    Ok(HttpResponse::Ok().json(json))
}

#[post("/images")]
async fn admin_upload_image(
    request: HttpRequest,
    data: web::Data<AppState>,
    mut payload: Multipart,
) -> Result<HttpResponse> {
    authenticate_admin(&request, &data, "admin_upload_image")
        .await
        .map_err(admin_auth_error)?;
    let blob = data
        .image_blob
        .as_ref()
        .ok_or_else(|| actix_web::error::ErrorServiceUnavailable("blob storage not configured"))?;

    if let Some(mut field) = payload
        .try_next()
        .await
        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?
    {
        let content_type = field
            .content_type()
            .map(|m| m.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_owned());

        let filename = field
            .content_disposition()
            .and_then(|cd| cd.get_filename())
            .unwrap_or("upload")
            .to_owned();

        // Restrict to image MIME types
        if !content_type.starts_with("image/") {
            return Err(actix_web::error::ErrorUnsupportedMediaType(
                "only image/* content types are accepted",
            ));
        }

        let mut bytes = Vec::new();
        while let Some(chunk) = field
            .try_next()
            .await
            .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?
        {
            bytes.extend_from_slice(&chunk);
        }

        let blob_name = format!("images/{filename}");
        blob.put_bytes(&blob_name, bytes, &content_type)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

        return Ok(HttpResponse::Created().json(serde_json::json!({
            "name": filename,
            "url": format!("/images/{filename}"),
        })));
    }

    Err(actix_web::error::ErrorBadRequest(
        "no file field in multipart body",
    ))
}

#[delete("/images/{name}")]
async fn admin_delete_image(
    request: HttpRequest,
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    authenticate_admin(&request, &data, "admin_delete_image")
        .await
        .map_err(admin_auth_error)?;
    let blob = data
        .image_blob
        .as_ref()
        .ok_or_else(|| actix_web::error::ErrorServiceUnavailable("blob storage not configured"))?;
    let filename = path.into_inner();
    blob.delete_blob(&format!("images/{filename}"))
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(serde::Deserialize)]
struct SetHeroBody {
    image: String,
}

#[post("/posts/{slug}/hero")]
async fn admin_set_hero(
    request: HttpRequest,
    data: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<SetHeroBody>,
) -> Result<HttpResponse> {
    authenticate_admin(&request, &data, "admin_set_hero")
        .await
        .map_err(admin_auth_error)?;
    let slug = path.into_inner();
    let meta_path = data
        .config
        .content_root
        .join("posts")
        .join(&slug)
        .join("meta.yml");

    if !meta_path.exists() {
        return Err(actix_web::error::ErrorNotFound("post not found"));
    }

    let raw = fs::read_to_string(&meta_path)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    let mut value: serde_yaml::Value = serde_yaml::from_str(&raw)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    if let serde_yaml::Value::Mapping(ref mut map) = value {
        map.insert(
            serde_yaml::Value::String("hero_image".to_owned()),
            serde_yaml::Value::String(body.image.clone()),
        );
    }

    let updated = serde_yaml::to_string(&value)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    fs::write(&meta_path, updated)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"slug": slug, "hero_image": body.image})))
}

fn admin_auth_error(error: AdminAuthError) -> actix_web::Error {
    match error {
        AdminAuthError::MissingBearerToken | AdminAuthError::InvalidToken(_) => {
            // Return a redirect to the login page — actix wraps it in an error response
            actix_web::error::ErrorUnauthorized("admin authentication required")
        }
        AdminAuthError::Forbidden(_) => actix_web::error::ErrorForbidden("admin access denied"),
        AdminAuthError::Disabled | AdminAuthError::MissingConfiguration(_) => {
            actix_web::error::ErrorNotImplemented("admin preview is not configured")
        }
        AdminAuthError::ProviderUnavailable(_) => {
            actix_web::error::ErrorInternalServerError("admin authentication is unavailable")
        }
    }
}

fn log_content_error(_operation: &'static str, error: &BlogError) {
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
                description: post.description,
                hero_image: post
                    .hero_image
                    .map(|value| resolve_asset_url(&value, &post.slug)),
                toc: post.toc,
                math: post.math,
                summary_ai: post.summary_ai,
                read_minutes: post.read_minutes,
                status: match post.status {
                    rustacian_blog_core::PostStatus::Published => "published".to_owned(),
                    rustacian_blog_core::PostStatus::Draft => "draft".to_owned(),
                },
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// English translation routes
// ---------------------------------------------------------------------------

#[get("/en/")]
async fn en_index(data: web::Data<AppState>) -> Result<HttpResponse> {
    if data.translator.is_none() {
        return Err(actix_web::error::ErrorServiceUnavailable(
            "Translator not configured",
        ));
    }
    let summaries = data.list_posts.execute().await.map_err(page_app_error)?;
    let views: Vec<PostSummaryView> = summaries
        .into_iter()
        .map(|s| PostSummaryView {
            title: s.title,
            slug: s.slug,
            published_at: s.published_at.format("%Y-%m-%d").to_string(),
            updated_at: s.updated_at.map(|d| d.format("%Y-%m-%d").to_string()),
            tags: s.tags,
            summary: s.summary,
            description: s.description,
            hero_image: None,
            toc: false,
            math: false,
            summary_ai: s.summary_ai,
            read_minutes: s.read_minutes,
            status: "published".to_owned(),
        })
        .collect();
    Ok(html_response(render_en_posts_page(views)))
}

#[get("/en/posts/{slug}")]
async fn en_post_page(path: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let translator = data
        .translator
        .as_ref()
        .ok_or_else(|| actix_web::error::ErrorServiceUnavailable("Translator not configured"))?;
    let slug = path.into_inner();

    // Serve from blob cache if available.
    let cache_key = format!("translations/{slug}/index.html");
    if let Some(blob) = data.image_blob.as_ref()
        && let Ok(Some(cached)) = blob.get_text(&cache_key).await
    {
        return Ok(html_response(cached));
    }

    let post = data
        .get_post
        .execute(&slug)
        .await
        .map_err(|_| actix_web::error::ErrorNotFound("post not found"))?;

    let mut view = map_post(post);

    // Translate title and body.
    let en_title = translator
        .translate_text(&view.title)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    let en_body = translator
        .translate_html(&view.body_html)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    view.title = en_title;
    view.body_html = en_body;

    let ja_url = format!("/posts/{slug}");
    let html = render_en_post_page(view, &ja_url);

    // Cache the rendered page in blob storage (best-effort).
    if let Some(blob) = data.image_blob.as_ref() {
        let _ = blob
            .put_bytes(
                &cache_key,
                html.as_bytes().to_vec(),
                "text/html; charset=utf-8",
            )
            .await;
    }

    Ok(html_response(html))
}

// ---------------------------------------------------------------------------
// Vision — image alt-text generation
// ---------------------------------------------------------------------------

#[post("/images/{name}/describe")]
async fn admin_describe_image(
    request: HttpRequest,
    path: web::Path<String>,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    authenticate_admin(&request, &data, "admin_describe_image")
        .await
        .map_err(admin_auth_error)?;

    let name = path.into_inner();
    let vision = data.vision.as_ref().ok_or_else(|| {
        actix_web::error::ErrorServiceUnavailable("Vision adapter not configured")
    })?;

    let image_url = format!("{}/images/{}", data.config.base_url, name);
    let alt = vision
        .describe_image_url(&image_url)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"alt": alt})))
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
        description: post.description,
        hero_image: post
            .hero_image
            .map(|value| resolve_asset_url(&value, &slug)),
        toc: post.toc,
        math: post.math,
        summary_ai: post.summary_ai,
        read_minutes: post.read_minutes,
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
                table_headers: chart.table_headers,
                table_rows: chart.table_rows,
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
        status: match post.status {
            rustacian_blog_core::PostStatus::Published => "published".to_owned(),
            rustacian_blog_core::PostStatus::Draft => "draft".to_owned(),
        },
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

fn infer_content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
    {
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, sync::Arc};

    use actix_web::{App, http::StatusCode, test, web};
    use async_trait::async_trait;
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use chrono::{DateTime, Utc};
    use rustacian_blog_core::{
        AdminAuthService, AiAssistRequest, AiMetadataGenerator, Comment, CommentRepository,
        CommentStatus, ContactMessage, ContactRepository, GenerateAiMetadataUseCase,
        GeneratedMetadata, GeneratedMetadataStore, GetPostUseCase, ListPostsUseCase,
        NotificationEvent, NotificationSink, PostMetadata, PostRepository, PostStatus,
        PostVisibility, PublishStaticSiteUseCase, StaticSiteBuild, StaticSiteGenerator,
        StaticSitePublisher,
    };

    use super::*;
    use crate::{
        config::AppConfig,
        observability::{AppEvent, ObservabilitySink},
        search::TantivySearchIndex,
        state::AppState,
    };
    use tempfile::tempdir;

    struct MockRepository {
        list_result: Result<Vec<PostSummary>, BlogError>,
        get_result: Result<Post, BlogError>,
    }

    struct MockAdminAuthService {
        result: Result<rustacian_blog_core::AdminIdentity, AdminAuthError>,
    }

    struct MockAiMetadataGenerator;

    #[derive(Default)]
    struct MockGeneratedMetadataStore;

    struct MockObservabilitySink;
    struct MockNotificationSink;
    struct MockCommentRepository;
    struct MockContactRepository;
    struct MockStaticSiteGenerator;
    #[derive(Default)]
    struct MockStaticSitePublisher;

    #[async_trait]
    impl PostRepository for MockRepository {
        async fn list_posts(
            &self,
            visibility: PostVisibility,
        ) -> Result<Vec<PostSummary>, BlogError> {
            self.list_result.clone().map(|posts| {
                posts
                    .into_iter()
                    .filter(|post| visibility.allows(post.status))
                    .collect()
            })
        }

        async fn get_post(
            &self,
            slug: &str,
            visibility: PostVisibility,
        ) -> Result<Post, BlogError> {
            match self.get_result.clone() {
                Ok(post) if post.slug == slug && visibility.allows(post.status) => Ok(post),
                Ok(_) => Err(BlogError::NotFound(slug.to_owned())),
                Err(error) => Err(error),
            }
        }
    }

    #[async_trait]
    impl AdminAuthService for MockAdminAuthService {
        async fn authenticate_bearer(
            &self,
            _bearer_token: &str,
        ) -> Result<rustacian_blog_core::AdminIdentity, AdminAuthError> {
            self.result.clone()
        }
    }

    #[async_trait]
    impl AiMetadataGenerator for MockAiMetadataGenerator {
        async fn generate_metadata(
            &self,
            request: AiAssistRequest,
            _scope: AiGenerationScope,
        ) -> Result<GeneratedMetadata, BlogError> {
            Ok(GeneratedMetadata {
                summary_ai: Some(format!("AI summary for {}", request.slug)),
                suggested_tags: vec!["generated".to_owned()],
                intro_candidates: vec!["Generated intro".to_owned()],
                generated_at: Utc::now(),
                source_model: Some("mock-model".to_owned()),
            })
        }
    }

    #[async_trait]
    impl GeneratedMetadataStore for MockGeneratedMetadataStore {
        async fn save(&self, _slug: &str, _metadata: &GeneratedMetadata) -> Result<(), BlogError> {
            Ok(())
        }
    }

    impl ObservabilitySink for MockObservabilitySink {
        fn emit(&self, _event: AppEvent) {}
    }

    #[async_trait]
    impl NotificationSink for MockNotificationSink {
        async fn notify(&self, _event: NotificationEvent) -> Result<(), BlogError> {
            Ok(())
        }
    }

    #[async_trait]
    impl CommentRepository for MockCommentRepository {
        async fn create_comment(&self, _comment: &Comment) -> Result<(), BlogError> {
            Ok(())
        }

        async fn list_comments(
            &self,
            _slug: &str,
            _include_pending: bool,
        ) -> Result<Vec<Comment>, BlogError> {
            Ok(Vec::new())
        }

        async fn list_all_pending(&self) -> Result<Vec<Comment>, BlogError> {
            Ok(Vec::new())
        }

        async fn update_status(&self, _id: &str, _status: CommentStatus) -> Result<(), BlogError> {
            Ok(())
        }
    }

    #[async_trait]
    impl ContactRepository for MockContactRepository {
        async fn create_contact_message(&self, _msg: &ContactMessage) -> Result<(), BlogError> {
            Ok(())
        }
    }

    #[async_trait]
    impl StaticSiteGenerator for MockStaticSiteGenerator {
        async fn generate(&self) -> Result<StaticSiteBuild, BlogError> {
            Ok(StaticSiteBuild::default())
        }
    }

    #[async_trait]
    impl StaticSitePublisher for MockStaticSitePublisher {
        async fn publish(&self, _build: &StaticSiteBuild) -> Result<(), BlogError> {
            Ok(())
        }
    }

    fn app_state(repository: Arc<dyn PostRepository>) -> AppState {
        AppState {
            list_posts: ListPostsUseCase::new(repository.clone()),
            get_post: GetPostUseCase::new(repository),
            admin_auth: Arc::new(MockAdminAuthService {
                result: Err(AdminAuthError::Disabled),
            }),
            observability: Arc::new(MockObservabilitySink),
            notification: Arc::new(MockNotificationSink),
            comment_repo: Arc::new(MockCommentRepository),
            contact_repo: Arc::new(MockContactRepository),
            search_index: Arc::new(TantivySearchIndex::new()),
            image_blob: None,
            analytics: None,
            cloudflare: None,
            http_client: reqwest::Client::new(),
            vision: None,
            translator: None,
            generate_ai_metadata: None,
            publish_static_site: Some(PublishStaticSiteUseCase::new(
                Arc::new(MockStaticSiteGenerator),
                Arc::new(MockStaticSitePublisher),
            )),
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
                entra_oidc_metadata_url: None,
                entra_admin_group_id: None,
                entra_admin_user_oid: None,
                entra_redirect_uri: None,
                cloudflare_zone_id: None,
                cloudflare_api_token: None,
                azure_vision_endpoint: None,
                azure_vision_api_key: None,
                azure_translator_endpoint: None,
                azure_translator_api_key: None,
                acs_endpoint: None,
                acs_access_key: None,
                acs_sender_address: None,
                acs_recipient_address: None,
                static_output_dir: "./dist".into(),
                static_publish_backend: "local".to_owned(),
                static_publish_prefix: "site".to_owned(),
                observability_backend: "noop".to_owned(),
                application_insights_connection_string: None,
                base_url: "http://127.0.0.1:8080".to_owned(),
                slack_webhook_url: None,
            },
        }
    }

    fn app_state_with_admin_auth(repository: Arc<dyn PostRepository>) -> AppState {
        let mut state = app_state(repository);
        state.config.admin_auth_mode = "entra-poc".to_owned();
        state.config.entra_tenant_id = Some("tenant-123".to_owned());
        state.config.entra_client_id = Some("client-123".to_owned());
        state.config.entra_admin_group_id = Some("group-123".to_owned());
        state.admin_auth = Arc::new(MockAdminAuthService {
            result: Ok(rustacian_blog_core::AdminIdentity {
                oid: Some("user-1".to_owned()),
                preferred_username: None,
                groups: vec!["group-123".to_owned()],
            }),
        });
        state
    }

    fn app_state_with_local_dev_auth(repository: Arc<dyn PostRepository>) -> AppState {
        let mut state = app_state(repository);
        state.config.admin_auth_mode = "local-dev".to_owned();
        state
    }

    fn app_state_with_admin_auth_and_ai(repository: Arc<dyn PostRepository>) -> AppState {
        let mut state = app_state_with_admin_auth(repository.clone());
        state.generate_ai_metadata = Some(GenerateAiMetadataUseCase::new(
            repository,
            Arc::new(MockAiMetadataGenerator),
            Arc::new(MockGeneratedMetadataStore),
        ));
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
                description: None,
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
    async fn admin_home_requires_authentication_when_enabled() {
        let state = app_state_with_admin_auth(Arc::new(MockRepository {
            list_result: Ok(Vec::new()),
            get_result: Ok(sample_post()),
        }));

        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response =
            test::call_service(&app, test::TestRequest::get().uri("/admin").to_request()).await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn admin_home_renders_when_authenticated() {
        let state = app_state_with_admin_auth(Arc::new(MockRepository {
            list_result: Ok(Vec::new()),
            get_result: Ok(sample_post()),
        }));

        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/admin")
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
    async fn admin_home_renders_without_token_in_local_dev_mode() {
        let state = app_state_with_local_dev_auth(Arc::new(MockRepository {
            list_result: Ok(vec![sample_post().summary()]),
            get_result: Ok(sample_post()),
        }));

        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response =
            test::call_service(&app, test::TestRequest::get().uri("/admin").to_request()).await;

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn admin_ai_metadata_returns_not_implemented_when_ai_is_not_configured() {
        let state = app_state_with_admin_auth(Arc::new(MockRepository {
            list_result: Ok(Vec::new()),
            get_result: Ok(sample_post()),
        }));

        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/admin/ai/sample/metadata")
                .insert_header((
                    "authorization",
                    bearer_for(
                        r#"{"aud":"client-123","tid":"tenant-123","groups":["group-123"],"oid":"user-1"}"#,
                    ),
                ))
                .to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[actix_web::test]
    async fn admin_ai_metadata_generates_and_returns_json_for_draft_post() {
        let mut draft = sample_post();
        draft.status = PostStatus::Draft;
        let state = app_state_with_admin_auth_and_ai(Arc::new(MockRepository {
            list_result: Ok(Vec::new()),
            get_result: Ok(draft),
        }));

        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/admin/ai/sample/metadata")
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
        let body = test::read_body(response).await;
        let raw = String::from_utf8(body.to_vec()).unwrap();
        assert!(raw.contains("\"summary_ai\":\"AI summary for sample\""));
        assert!(raw.contains("\"source_model\":\"mock-model\""));
    }

    #[actix_web::test]
    async fn admin_static_regenerate_returns_success_when_authenticated() {
        let state = app_state_with_admin_auth(Arc::new(MockRepository {
            list_result: Ok(Vec::new()),
            get_result: Ok(sample_post()),
        }));

        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/admin/static/regenerate")
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
    async fn public_post_page_does_not_render_draft_post() {
        let mut draft = sample_post();
        draft.status = PostStatus::Draft;
        let state = app_state(Arc::new(MockRepository {
            list_result: Ok(Vec::new()),
            get_result: Ok(draft),
        }));

        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response =
            test::call_service(&app, test::TestRequest::get().uri("/p/sample").to_request()).await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
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

    #[actix_web::test]
    async fn admin_home_sets_cookie_for_follow_up_admin_actions() {
        let state = app_state_with_admin_auth(Arc::new(MockRepository {
            list_result: Ok(vec![sample_post().summary()]),
            get_result: Ok(sample_post()),
        }));

        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/admin")
                .insert_header((
                    "authorization",
                    bearer_for(
                        r#"{"aud":"client-123","tid":"tenant-123","groups":["group-123"],"oid":"user-1"}"#,
                    ),
                ))
                .to_request(),
        )
        .await;

        let cookies = response.response().cookies().collect::<Vec<_>>();
        assert!(
            cookies
                .iter()
                .any(|cookie| cookie.name() == "admin_session")
        );
    }

    #[actix_web::test]
    async fn image_route_serves_local_svg_asset() {
        let temp = tempdir().unwrap();
        let content_root = temp.path().join("content");
        fs::create_dir_all(content_root.join("images")).unwrap();
        fs::write(
            content_root.join("images").join("sample.svg"),
            "<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>",
        )
        .unwrap();

        let mut state = app_state(Arc::new(MockRepository {
            list_result: Ok(Vec::new()),
            get_result: Ok(sample_post()),
        }));
        state.config.storage_backend = "local".to_owned();
        state.config.content_root = content_root;

        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/images/sample.svg")
                .to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get("content-type")
                .and_then(|value| value.to_str().ok()),
            Some("image/svg+xml")
        );
    }

    // -----------------------------------------------------------------------
    // XSS / Injection sanitization tests
    // -----------------------------------------------------------------------

    #[actix_web::test]
    async fn sanitize_strips_script_tags() {
        let input = r#"<script>alert(1)</script>Hello"#;
        let output = sanitize(input);
        assert!(!output.contains("<script>"));
        assert!(output.contains("Hello"));
    }

    #[actix_web::test]
    async fn sanitize_strips_img_onerror() {
        let input = r#"<img src=x onerror="alert(1)">text"#;
        let output = sanitize(input);
        assert!(!output.contains("onerror"));
        assert!(output.contains("text"));
    }

    #[actix_web::test]
    async fn sanitize_strips_all_html_tags() {
        let input = r#"<b>bold</b> <i>italic</i> <a href="http://evil.com">click</a>"#;
        let output = sanitize(input);
        assert!(!output.contains('<'));
        assert!(output.contains("bold"));
        assert!(output.contains("italic"));
        assert!(output.contains("click"));
    }

    #[actix_web::test]
    async fn sanitize_preserves_plain_text() {
        let input = "普通のコメントです。Rust is great!";
        assert_eq!(sanitize(input), input);
    }

    // -----------------------------------------------------------------------
    // Comment route tests
    // -----------------------------------------------------------------------

    #[actix_web::test]
    async fn post_comment_redirects_after_success() {
        let state = app_state(Arc::new(MockRepository {
            list_result: Ok(Vec::new()),
            get_result: Ok(sample_post()),
        }));
        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/posts/sample/comments")
                .set_form([("author_name", "Alice"), ("content", "Great post!")])
                .to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok()),
            Some("/posts/sample/comments")
        );
    }

    #[actix_web::test]
    async fn list_post_comments_returns_html() {
        let state = app_state(Arc::new(MockRepository {
            list_result: Ok(Vec::new()),
            get_result: Ok(sample_post()),
        }));
        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/posts/sample/comments")
                .to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn contact_page_returns_html() {
        let state = app_state(Arc::new(MockRepository {
            list_result: Ok(Vec::new()),
            get_result: Ok(sample_post()),
        }));
        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response =
            test::call_service(&app, test::TestRequest::get().uri("/contact").to_request()).await;

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn post_contact_redirects_after_success() {
        let state = app_state(Arc::new(MockRepository {
            list_result: Ok(Vec::new()),
            get_result: Ok(sample_post()),
        }));
        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/contact")
                .set_form([
                    ("from_name", "Bob"),
                    ("from_email", "bob@example.com"),
                    ("body", "Hello!"),
                ])
                .to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
    }

    #[actix_web::test]
    async fn admin_comments_requires_auth() {
        let state = app_state(Arc::new(MockRepository {
            list_result: Ok(Vec::new()),
            get_result: Ok(sample_post()),
        }));
        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response = test::call_service(
            &app,
            test::TestRequest::get().uri("/admin/comments").to_request(),
        )
        .await;

        // auth disabled → 501 (admin not configured)
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    // -----------------------------------------------------------------------
    // Admin UI — Phase 6 tests
    // -----------------------------------------------------------------------

    #[actix_web::test]
    async fn admin_dashboard_contains_post_table() {
        let state = app_state_with_local_dev_auth(Arc::new(MockRepository {
            list_result: Ok(vec![sample_post().summary()]),
            get_result: Ok(sample_post()),
        }));
        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response =
            test::call_service(&app, test::TestRequest::get().uri("/admin").to_request()).await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = test::read_body(response).await;
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(html.contains("ダッシュボード") || html.contains("Rustacian"));
        assert!(html.contains("sample") || html.contains("Sample"));
    }

    #[actix_web::test]
    async fn admin_post_detail_returns_html_for_published_post() {
        let state = app_state_with_local_dev_auth(Arc::new(MockRepository {
            list_result: Ok(Vec::new()),
            get_result: Ok(sample_post()),
        }));
        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/admin/posts/sample")
                .to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = test::read_body(response).await;
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(html.contains("Sample"));
    }

    #[actix_web::test]
    async fn admin_post_detail_requires_auth() {
        let state = app_state_with_admin_auth(Arc::new(MockRepository {
            list_result: Ok(Vec::new()),
            get_result: Ok(sample_post()),
        }));
        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/admin/posts/sample")
                .to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn admin_static_panel_returns_html() {
        let state = app_state_with_local_dev_auth(Arc::new(MockRepository {
            list_result: Ok(Vec::new()),
            get_result: Ok(sample_post()),
        }));
        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response = test::call_service(
            &app,
            test::TestRequest::get().uri("/admin/static").to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = test::read_body(response).await;
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(html.contains("再生成") || html.contains("static/regenerate"));
    }

    #[actix_web::test]
    async fn admin_static_panel_requires_auth() {
        let state = app_state_with_admin_auth(Arc::new(MockRepository {
            list_result: Ok(Vec::new()),
            get_result: Ok(sample_post()),
        }));
        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response = test::call_service(
            &app,
            test::TestRequest::get().uri("/admin/static").to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn search_page_returns_html_for_empty_query() {
        let state = app_state(Arc::new(MockRepository {
            list_result: Ok(Vec::new()),
            get_result: Ok(sample_post()),
        }));
        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response =
            test::call_service(&app, test::TestRequest::get().uri("/search").to_request()).await;

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn search_page_returns_html_for_query_with_no_results() {
        let state = app_state(Arc::new(MockRepository {
            list_result: Ok(Vec::new()),
            get_result: Ok(sample_post()),
        }));
        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response = test::call_service(
            &app,
            test::TestRequest::get().uri("/search?q=rust").to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = test::read_body(response).await;
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(html.contains("検索"));
    }

    #[actix_web::test]
    async fn admin_comments_accessible_with_auth() {
        let state = app_state_with_admin_auth(Arc::new(MockRepository {
            list_result: Ok(Vec::new()),
            get_result: Ok(sample_post()),
        }));
        let app =
            test::init_service(App::new().app_data(web::Data::new(state)).configure(routes)).await;

        let response = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/admin/comments")
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
}
