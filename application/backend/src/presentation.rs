use actix_web::{
    HttpResponse, Result, error::ErrorInternalServerError, get, http::header::ContentType, web,
};
use rustacian_blog_core::{Post, PostSummary};
use rustacian_blog_frontend::{render_post_page, render_posts_page};

use crate::state::AppState;

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(health)
        .service(list_posts)
        .service(get_post)
        .service(index_page)
        .service(post_page);
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
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(posts))
}

#[get("/posts/{slug}")]
async fn get_post(path: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let post = data
        .get_post
        .execute(&path.into_inner())
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(post))
}

#[get("/")]
async fn index_page(data: web::Data<AppState>) -> Result<HttpResponse> {
    let posts = data
        .list_posts
        .execute()
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(html_response(render_posts_page(map_summaries(posts))))
}

#[get("/p/{slug}")]
async fn post_page(path: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let post = data
        .get_post
        .execute(&path.into_inner())
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(html_response(render_post_page(map_post(post))))
}

fn html_response(body: String) -> HttpResponse {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body)
}

fn map_summaries(posts: Vec<PostSummary>) -> Vec<rustacian_blog_frontend::PostSummaryView> {
    posts
        .into_iter()
        .map(|post| rustacian_blog_frontend::PostSummaryView {
            title: post.title,
            slug: post.slug,
            published_at: post.published_at.format("%Y-%m-%d").to_string(),
            tags: post.tags,
            summary: post.summary,
            hero_image: post.hero_image,
        })
        .collect()
}

fn map_post(post: Post) -> rustacian_blog_frontend::PostView {
    rustacian_blog_frontend::PostView {
        title: post.title,
        slug: post.slug,
        published_at: post.published_at.format("%Y-%m-%d").to_string(),
        tags: post.tags,
        summary: post.summary,
        hero_image: post.hero_image,
        body_html: post.body_html,
    }
}
