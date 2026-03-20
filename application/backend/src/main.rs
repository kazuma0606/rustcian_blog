use std::sync::Arc;

use actix_files::Files;
use actix_web::{App, HttpServer, web};
use rustacian_blog_backend::{
    config::AppConfig,
    presentation,
    state::AppState,
    storage::{AzuritePostRepository, LocalContentPostRepository, seed_azurite_from_local},
};
use rustacian_blog_core::{GetPostUseCase, ListPostsUseCase, PostRepository};

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

    let app_state = AppState {
        list_posts: ListPostsUseCase::new(repository.clone()),
        get_post: GetPostUseCase::new(repository),
        config: config.clone(),
    };
    let bind_address = config.bind_address();
    let content_root = config.content_root.clone();
    let static_images_dir = config.images_dir();

    println!("listening on http://{}", bind_address);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .configure(presentation::routes)
            .service(Files::new("/assets", content_root.clone()).show_files_listing())
            .service(Files::new("/images", static_images_dir.clone()).show_files_listing())
    })
    .bind(bind_address)?
    .run()
    .await
}
