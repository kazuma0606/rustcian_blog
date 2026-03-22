use std::sync::Arc;

use actix_web::{App, HttpServer, web};
use rustacian_blog_analytics::{
    config::AnalyticsConfig,
    presentation,
    store::AnalyticsStore,
    table::TableClient,
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::from_filename(".env.local").ok();

    let config = AnalyticsConfig::from_env();

    let table_endpoint = config
        .azurite_table_endpoint
        .clone()
        .expect("AZURITE_TABLE_ENDPOINT is required for analytics service");

    let client = Arc::new(TableClient::new(table_endpoint));
    let store = AnalyticsStore::new(client);

    store
        .init_tables()
        .await
        .expect("failed to initialize analytics tables");

    let store = web::Data::new(store);
    let bind_address = config.bind_address();

    println!("analytics listening on http://{bind_address}");

    HttpServer::new(move || {
        App::new()
            .app_data(store.clone())
            .configure(presentation::routes)
    })
    .bind(bind_address)?
    .run()
    .await
}
