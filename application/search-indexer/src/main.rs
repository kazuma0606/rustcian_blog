use std::sync::Arc;

use rustacian_blog_backend::search_storage::BlobIndexStorage;
use rustacian_blog_core::{PostRepository, PostVisibility};
use rustacian_blog_search::{PostDoc, SearchEngine};

#[tokio::main]
async fn main() {
    dotenvy::from_filename(".env.local").ok();

    let blob_endpoint =
        std::env::var("AZURITE_BLOB_ENDPOINT").expect("AZURITE_BLOB_ENDPOINT must be set");

    println!("search-indexer: loading posts from blob storage...");

    let repository: Arc<dyn PostRepository> = Arc::new(
        rustacian_blog_backend::storage::AzuritePostRepository::new(blob_endpoint.clone()),
    );

    let summaries = repository
        .list_posts(PostVisibility::PublishedOnly)
        .await
        .expect("failed to list posts");

    println!("search-indexer: found {} published posts", summaries.len());

    let mut docs = Vec::with_capacity(summaries.len());
    for summary in &summaries {
        match repository
            .get_post(&summary.slug, PostVisibility::PublishedOnly)
            .await
        {
            Ok(post) => {
                docs.push(PostDoc {
                    slug: post.slug.clone(),
                    title: post.title.clone(),
                    body_text: ammonia::Builder::new()
                        .tags(std::collections::HashSet::new())
                        .clean(&post.body_html)
                        .to_string(),
                    tags: post.tags.clone(),
                    date: post.published_at.format("%Y-%m-%d").to_string(),
                });
            }
            Err(e) => {
                eprintln!(
                    "search-indexer: warn: failed to load post '{}': {e}",
                    summary.slug
                );
            }
        }
    }

    let engine = SearchEngine::new();
    engine.rebuild(&docs).expect("failed to build search index");

    let storage = BlobIndexStorage::new(blob_endpoint, "search/index.bin".to_owned());
    engine
        .save_to(&storage)
        .await
        .expect("failed to save search index to blob storage");

    println!(
        "search-indexer: index saved ({} docs) → search/index.bin",
        docs.len()
    );
}
