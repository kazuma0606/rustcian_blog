use std::sync::Arc;

use actix_files::Files;
use actix_web::{App, HttpServer, web};
use rustacian_blog_backend::{
    ai::{build_ai_metadata_generator, build_generated_metadata_store, build_vision_adapter},
    analytics_writer::AnalyticsWriter,
    auth::build_admin_auth_service,
    blob::AzuriteBlobAdapter,
    cloudflare::build_cloudflare_cache_client,
    comment_store::{
        AzuriteCommentRepository, AzuriteContactRepository, build_comment_repository,
        build_contact_repository,
    },
    config::AppConfig,
    notification::build_notification_sink,
    observability::{AppEvent, build_observability_sink},
    presentation,
    search::TantivySearchIndex,
    state::AppState,
    static_site::{LocalFileAssetStore, LocalStaticSiteGenerator, build_static_site_publisher},
    storage::{AzuritePostRepository, LocalContentPostRepository, seed_azurite_from_local},
    translator::build_translator,
};
use rustacian_blog_core::{
    GenerateAiMetadataUseCase, GetPostUseCase, ListPostsUseCase, PostRepository, PostVisibility,
    PublishStaticSiteUseCase,
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::from_filename(".env.local").ok();

    let config = AppConfig::from_env().expect("failed to load configuration");
    let repository: Arc<dyn PostRepository> = match config.storage_backend.as_str() {
        "azurite" => {
            let blob_endpoint = config
                .azurite_blob_endpoint
                .clone()
                .expect("AZURITE_BLOB_ENDPOINT is required when STORAGE_BACKEND=azurite");
            // Only seed from local filesystem when explicitly requested (local dev).
            // In production, CI uploads content directly to Blob Storage.
            if std::env::var("SEED_FROM_LOCAL").as_deref() == Ok("true") {
                seed_azurite_from_local(config.content_root.clone(), &blob_endpoint)
                    .await
                    .expect("failed to seed Azurite from local content");
            }
            Arc::new(AzuritePostRepository::new(blob_endpoint))
        }
        _ => Arc::new(LocalContentPostRepository::new(config.content_root.clone())),
    };

    // Initialize Azure Table Storage tables if needed.
    // Failures are non-fatal: tables may already exist, or Managed Identity
    // role propagation may be in progress. The tables will be created on first
    // successful write if they don't exist yet.
    if let Some(endpoint) = &config.azurite_table_endpoint {
        if let Err(e) = AzuriteCommentRepository::new(endpoint.clone()).init().await {
            eprintln!("warn: failed to init comments table (non-fatal): {e}");
        }
        if let Err(e) = AzuriteContactRepository::new(endpoint.clone()).init().await {
            eprintln!("warn: failed to init contacts table (non-fatal): {e}");
        }
    }

    // Build initial search index from all published posts.
    let search_index = Arc::new(TantivySearchIndex::new());
    {
        let slugs = repository
            .list_posts(PostVisibility::PublishedOnly)
            .await
            .unwrap_or_default();
        let mut posts = Vec::with_capacity(slugs.len());
        for s in &slugs {
            if let Ok(post) = repository
                .get_post(&s.slug, PostVisibility::PublishedOnly)
                .await
            {
                posts.push(post);
            }
        }
        if let Err(e) = search_index.rebuild(&posts) {
            eprintln!("warn: search index build failed: {e}");
        }
    }

    let translator = build_translator(&config).map(Arc::new);
    let static_generator = {
        let sg = LocalStaticSiteGenerator::new(
            repository.clone(),
            Arc::new(LocalFileAssetStore::new(config.content_root.clone())),
            config.base_url.clone(),
        );
        let sg = if let Some(t) = translator.clone() {
            sg.with_translator(t)
        } else {
            sg
        };
        Arc::new(sg)
    };
    let static_publisher = build_static_site_publisher(&config);
    let app_state = AppState {
        list_posts: ListPostsUseCase::new(repository.clone()),
        get_post: GetPostUseCase::new(repository.clone()),
        generate_ai_metadata: build_ai_metadata_generator(&config).map(|generator| {
            GenerateAiMetadataUseCase::new(
                repository.clone(),
                generator,
                build_generated_metadata_store(&config),
            )
        }),
        publish_static_site: Some(PublishStaticSiteUseCase::new(
            static_generator.clone(),
            static_publisher.clone(),
        )),
        admin_auth: build_admin_auth_service(&config),
        observability: build_observability_sink(&config),
        notification: build_notification_sink(&config),
        comment_repo: build_comment_repository(&config),
        contact_repo: build_contact_repository(&config),
        search_index,
        image_blob: config
            .azurite_blob_endpoint
            .clone()
            .map(AzuriteBlobAdapter::new),
        analytics: config
            .azurite_table_endpoint
            .clone()
            .map(|ep| Arc::new(AnalyticsWriter::new(ep))),
        cloudflare: build_cloudflare_cache_client(
            &reqwest::Client::new(),
            config.cloudflare_zone_id.as_deref(),
            config.cloudflare_api_token.as_deref(),
        ),
        http_client: reqwest::Client::new(),
        vision: build_vision_adapter(&config),
        translator: translator.clone(),
        config: config.clone(),
    };
    let bind_address = config.bind_address();
    let content_root = config.content_root.clone();
    let static_output_dir = config.static_output_dir.clone();

    if matches!(
        std::env::args().nth(1).as_deref(),
        Some("generate-static" | "publish-static" | "rebuild-static")
    ) {
        let build = app_state
            .publish_static_site
            .clone()
            .expect("static publishing is not configured")
            .execute()
            .await
            .expect("failed to generate static site");
        app_state.observability.emit(AppEvent::StaticSitePublished {
            target: match config.static_publish_backend.as_str() {
                "azurite" => format!("azurite:{}", config.static_publish_prefix),
                _ => format!("local:{}", static_output_dir.display()),
            },
            pages: build.pages.len(),
            assets: build.assets.len(),
        });
        match config.static_publish_backend.as_str() {
            "azurite" => println!(
                "static site published to Azurite prefix '{}' ({} pages, {} assets)",
                config.static_publish_prefix,
                build.pages.len(),
                build.assets.len()
            ),
            _ => println!(
                "static site written to {} ({} pages, {} assets)",
                static_output_dir.display(),
                build.pages.len(),
                build.assets.len()
            ),
        }
        return Ok(());
    }

    println!("listening on http://{}", bind_address);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .configure(presentation::routes)
            .service(Files::new("/assets", content_root.clone()).show_files_listing())
    })
    .bind(bind_address)?
    .run()
    .await
}
