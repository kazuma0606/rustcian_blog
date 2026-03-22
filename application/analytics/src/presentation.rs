use actix_web::{HttpResponse, Responder, get, web};
use serde::Deserialize;

use crate::store::AnalyticsStore;

#[derive(Deserialize)]
pub struct DaysQuery {
    #[serde(default = "default_days")]
    days: u32,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_days() -> u32 {
    7
}
fn default_limit() -> usize {
    10
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(health)
        .service(popular)
        .service(summary)
        .service(gaps)
        .service(coread);
}

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "rustacian_blog_analytics"
    }))
}

#[get("/api/popular")]
async fn popular(
    store: web::Data<AnalyticsStore>,
    query: web::Query<DaysQuery>,
) -> impl Responder {
    match store.popular(query.days, query.limit).await {
        Ok(stats) => HttpResponse::Ok().json(stats),
        Err(e) => {
            eprintln!("popular error: {e}");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": e}))
        }
    }
}

#[get("/api/summary")]
async fn summary(
    store: web::Data<AnalyticsStore>,
    query: web::Query<DaysQuery>,
) -> impl Responder {
    match store.summary(query.days).await {
        Ok(s) => HttpResponse::Ok().json(s),
        Err(e) => {
            eprintln!("summary error: {e}");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": e}))
        }
    }
}

#[get("/api/gaps")]
async fn gaps(
    store: web::Data<AnalyticsStore>,
    query: web::Query<DaysQuery>,
) -> impl Responder {
    match store.gaps(query.days).await {
        Ok(g) => HttpResponse::Ok().json(g),
        Err(e) => {
            eprintln!("gaps error: {e}");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": e}))
        }
    }
}

#[get("/api/coread/{slug}")]
async fn coread(
    store: web::Data<AnalyticsStore>,
    slug: web::Path<String>,
) -> impl Responder {
    match store.coread(&slug).await {
        Ok(entries) => HttpResponse::Ok().json(entries),
        Err(e) => {
            eprintln!("coread error: {e}");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": e}))
        }
    }
}
