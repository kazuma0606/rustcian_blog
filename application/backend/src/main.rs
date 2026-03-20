use std::sync::Arc;

use actix_files::Files;
use actix_web::{App, HttpServer, web};
use rustacian_blog_backend::{
    ai::{build_ai_metadata_generator, build_generated_metadata_store},
    auth::build_admin_auth_service,
    blob::AzuriteBlobAdapter,
    config::AppConfig,
    observability::{AppEvent, build_observability_sink},
    presentation,
    state::AppState,
    static_site::{LocalFileAssetStore, LocalStaticSiteGenerator, build_static_site_publisher},
    storage::{AzuritePostRepository, LocalContentPostRepository, seed_azurite_from_local},
};
use rustacian_blog_core::{
    GenerateAiMetadataUseCase, GetPostUseCase, ListPostsUseCase, PostRepository,
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
            seed_azurite_from_local(config.content_root.clone(), &blob_endpoint)
                .await
                .expect("failed to seed Azurite from local content");
            Arc::new(AzuritePostRepository::new(blob_endpoint))
        }
        _ => Arc::new(LocalContentPostRepository::new(config.content_root.clone())),
    };

    let static_generator = Arc::new(LocalStaticSiteGenerator::new(
        repository.clone(),
        Arc::new(LocalFileAssetStore::new(config.content_root.clone())),
        config.base_url.clone(),
    ));
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
        image_blob: config
            .azurite_blob_endpoint
            .clone()
            .map(AzuriteBlobAdapter::new),
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
