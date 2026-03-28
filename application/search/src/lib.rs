use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use tantivy::{
    Index, IndexReader, TantivyDocument,
    collector::{Count, TopDocs},
    doc,
    query::QueryParser,
    schema::{Field, STORED, STRING, Schema, TEXT, Value},
};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Lightweight document fed into the search index.
/// The caller is responsible for stripping HTML from `body_text` before
/// passing it here; the search crate has no HTML-parsing dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostDoc {
    pub slug: String,
    pub title: String,
    /// Plain text — HTML must be stripped by the caller.
    pub body_text: String,
    pub tags: Vec<String>,
    /// ISO-8601 date string, e.g. "2024-01-15".
    pub date: String,
}

/// Query parameters for a search request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    pub q: String,
    /// 1-based page number (default 1).
    #[serde(default = "default_page")]
    pub page: usize,
    /// Results per page (default 20).
    #[serde(default = "default_per_page")]
    pub per_page: usize,
}

fn default_page() -> usize {
    1
}
fn default_per_page() -> usize {
    20
}

/// A single search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub slug: String,
    pub title: String,
    pub excerpt: String,
    pub tags: Vec<String>,
    pub date: String,
}

/// Paginated search results.
#[derive(Debug, Clone)]
pub struct SearchPage {
    pub hits: Vec<SearchHit>,
    pub total: usize,
    pub total_pages: usize,
    pub page: usize,
    pub per_page: usize,
}

// ---------------------------------------------------------------------------
// IndexStorage trait
// ---------------------------------------------------------------------------

/// Abstraction over where serialized index data is stored.
/// `async fn` in trait is allowed here; this trait is used with generics
/// (not `dyn`), so `Send` bounds are inferred from concrete implementations.
#[allow(async_fn_in_trait)]
pub trait IndexStorage: Send + Sync {
    async fn save(&self, data: &[u8]) -> Result<(), String>;
    async fn load(&self) -> Result<Vec<u8>, String>;
}

// ---------------------------------------------------------------------------
// SearchEngine
// ---------------------------------------------------------------------------

struct SearchInner {
    index: Index,
    reader: IndexReader,
    /// Original documents, kept for `save_to` serialization.
    docs: Vec<PostDoc>,
}

pub struct SearchEngine {
    schema: Schema,
    slug: Field,
    title: Field,
    body_text: Field,
    tags: Field,
    date: Field,
    inner: RwLock<Option<SearchInner>>,
}

impl SearchEngine {
    pub fn new() -> Self {
        let mut builder = Schema::builder();
        let slug = builder.add_text_field("slug", STRING | STORED);
        let title = builder.add_text_field("title", TEXT | STORED);
        let body_text = builder.add_text_field("body_text", TEXT | STORED);
        let tags = builder.add_text_field("tags", TEXT | STORED);
        let date = builder.add_text_field("date", STRING | STORED);
        let schema = builder.build();
        Self {
            schema,
            slug,
            title,
            body_text,
            tags,
            date,
            inner: RwLock::new(None),
        }
    }

    /// (Re)build the in-memory index from a slice of documents.
    pub fn rebuild(&self, docs: &[PostDoc]) -> Result<(), String> {
        let index = Index::create_in_ram(self.schema.clone());
        let mut writer = index
            .writer(16_000_000)
            .map_err(|e| format!("tantivy writer: {e}"))?;

        for doc in docs {
            let tags_text = doc.tags.join(" ");
            writer
                .add_document(doc!(
                    self.slug => doc.slug.as_str(),
                    self.title => doc.title.as_str(),
                    self.body_text => doc.body_text.as_str(),
                    self.tags => tags_text.as_str(),
                    self.date => doc.date.as_str(),
                ))
                .map_err(|e| format!("tantivy add_document: {e}"))?;
        }

        writer
            .commit()
            .map_err(|e| format!("tantivy commit: {e}"))?;

        let reader = index.reader().map_err(|e| format!("tantivy reader: {e}"))?;

        let mut guard = self
            .inner
            .write()
            .map_err(|e| format!("lock poisoned: {e}"))?;
        *guard = Some(SearchInner {
            index,
            reader,
            docs: docs.to_vec(),
        });
        Ok(())
    }

    /// Search the index and return a paginated result page.
    pub fn search(&self, query: &SearchQuery) -> Result<SearchPage, String> {
        let q = query.q.trim();
        let page = query.page.max(1);
        let per_page = query.per_page.max(1);

        if q.is_empty() {
            return Ok(SearchPage {
                hits: Vec::new(),
                total: 0,
                total_pages: 0,
                page,
                per_page,
            });
        }

        let guard = self
            .inner
            .read()
            .map_err(|e| format!("lock poisoned: {e}"))?;
        let Some(inner) = guard.as_ref() else {
            return Ok(SearchPage {
                hits: Vec::new(),
                total: 0,
                total_pages: 0,
                page,
                per_page,
            });
        };

        let searcher = inner.reader.searcher();
        let mut parser =
            QueryParser::for_index(&inner.index, vec![self.title, self.body_text, self.tags]);
        parser.set_conjunction_by_default();

        let parsed = match parser.parse_query(q) {
            Ok(q) => q,
            Err(_) => {
                return Ok(SearchPage {
                    hits: Vec::new(),
                    total: 0,
                    total_pages: 0,
                    page,
                    per_page,
                });
            }
        };

        let offset = (page - 1) * per_page;
        let (total, top_docs) = searcher
            .search(
                &parsed,
                &(Count, TopDocs::with_limit(per_page).and_offset(offset)),
            )
            .map_err(|e| format!("tantivy search: {e}"))?;

        let total_pages = total.div_ceil(per_page);

        let mut hits = Vec::with_capacity(top_docs.len());
        for (_score, addr) in top_docs {
            let retrieved: TantivyDocument = searcher
                .doc(addr)
                .map_err(|e| format!("tantivy doc retrieve: {e}"))?;

            let slug = retrieved
                .get_first(self.slug)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned();
            let title = retrieved
                .get_first(self.title)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned();
            let body = retrieved
                .get_first(self.body_text)
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let excerpt = truncate(body, 200);
            let tags_str = retrieved
                .get_first(self.tags)
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let tags = if tags_str.is_empty() {
                Vec::new()
            } else {
                tags_str.split_whitespace().map(str::to_owned).collect()
            };
            let date = retrieved
                .get_first(self.date)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned();

            hits.push(SearchHit {
                slug,
                title,
                excerpt,
                tags,
                date,
            });
        }

        Ok(SearchPage {
            hits,
            total,
            total_pages,
            page,
            per_page,
        })
    }

    /// Serialize the indexed documents to the given storage.
    pub async fn save_to<S: IndexStorage>(&self, storage: &S) -> Result<(), String> {
        let bytes = {
            let guard = self
                .inner
                .read()
                .map_err(|e| format!("lock poisoned: {e}"))?;
            let Some(inner) = guard.as_ref() else {
                return Ok(());
            };
            serde_json::to_vec(&inner.docs).map_err(|e| format!("serialize error: {e}"))?
        };
        storage.save(&bytes).await
    }

    /// Restore the index from the given storage.
    pub async fn load_from<S: IndexStorage>(&self, storage: &S) -> Result<(), String> {
        let bytes = storage.load().await?;
        let docs: Vec<PostDoc> =
            serde_json::from_slice(&bytes).map_err(|e| format!("deserialize error: {e}"))?;
        self.rebuild(&docs)
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn truncate(s: &str, max_chars: usize) -> String {
    let mut chars = s.chars();
    let taken: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{taken}…")
    } else {
        taken
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // --- helpers ---

    fn doc(slug: &str, title: &str, body_text: &str, tags: &[&str], date: &str) -> PostDoc {
        PostDoc {
            slug: slug.to_owned(),
            title: title.to_owned(),
            body_text: body_text.to_owned(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            date: date.to_owned(),
        }
    }

    fn q(s: &str) -> SearchQuery {
        SearchQuery {
            q: s.to_owned(),
            page: 1,
            per_page: 20,
        }
    }

    struct MockStorage {
        data: Mutex<Option<Vec<u8>>>,
    }

    impl MockStorage {
        fn empty() -> Self {
            Self {
                data: Mutex::new(None),
            }
        }
    }

    impl IndexStorage for MockStorage {
        async fn save(&self, data: &[u8]) -> Result<(), String> {
            *self.data.lock().unwrap() = Some(data.to_vec());
            Ok(())
        }
        async fn load(&self) -> Result<Vec<u8>, String> {
            self.data
                .lock()
                .unwrap()
                .clone()
                .ok_or_else(|| "no data in mock storage".to_owned())
        }
    }

    // --- ported from backend/src/search.rs ---

    #[test]
    fn index_returns_matching_post() {
        let engine = SearchEngine::new();
        let docs = vec![
            doc(
                "hello-rust",
                "Hello Rust",
                "Rust is amazing for systems programming",
                &["rust"],
                "2024-01-01",
            ),
            doc(
                "intro-actix",
                "Intro to Actix",
                "Actix Web framework tutorial",
                &["rust", "actix"],
                "2024-01-02",
            ),
        ];
        engine.rebuild(&docs).unwrap();

        let results = engine.search(&q("systems programming")).unwrap();
        assert_eq!(results.hits.len(), 1);
        assert_eq!(results.hits[0].slug, "hello-rust");
    }

    #[test]
    fn index_returns_empty_for_no_match() {
        let engine = SearchEngine::new();
        let docs = vec![doc("hello", "Hello World", "Hello", &[], "2024-01-01")];
        engine.rebuild(&docs).unwrap();

        let results = engine.search(&q("javascript typescript")).unwrap();
        assert!(results.hits.is_empty());
    }

    #[test]
    fn draft_posts_are_not_indexed() {
        // Caller is responsible for filtering drafts before calling rebuild.
        // This test verifies that only the docs passed in are indexed.
        let engine = SearchEngine::new();
        let docs = vec![doc(
            "published-post",
            "Published",
            "public content",
            &[],
            "2024-01-01",
        )];
        engine.rebuild(&docs).unwrap();

        let results = engine.search(&q("secret draft")).unwrap();
        assert!(results.hits.is_empty());

        let results = engine.search(&q("public content")).unwrap();
        assert_eq!(results.hits.len(), 1);
        assert_eq!(results.hits[0].slug, "published-post");
    }

    #[test]
    fn empty_query_returns_empty() {
        let engine = SearchEngine::new();
        let docs = vec![doc("p1", "Post 1", "content", &[], "2024-01-01")];
        engine.rebuild(&docs).unwrap();

        let results = engine.search(&q("")).unwrap();
        assert!(results.hits.is_empty());
        assert_eq!(results.total, 0);
    }

    #[test]
    fn rebuild_replaces_old_index() {
        let engine = SearchEngine::new();
        engine
            .rebuild(&[doc(
                "old-post",
                "Old Post",
                "old content",
                &[],
                "2024-01-01",
            )])
            .unwrap();

        engine
            .rebuild(&[doc(
                "new-post",
                "New Post",
                "new content",
                &[],
                "2024-01-02",
            )])
            .unwrap();

        let old = engine.search(&q("old content")).unwrap();
        assert!(
            old.hits.is_empty(),
            "old content should be gone after rebuild"
        );

        let new = engine.search(&q("new content")).unwrap();
        assert_eq!(new.hits.len(), 1);
    }

    // --- pagination ---

    fn make_docs(n: usize) -> Vec<PostDoc> {
        (1..=n)
            .map(|i| {
                doc(
                    &format!("post-{i}"),
                    &format!("Post {i}"),
                    "rust content",
                    &["rust"],
                    "2024-01-01",
                )
            })
            .collect()
    }

    #[test]
    fn pagination_total_and_total_pages() {
        let engine = SearchEngine::new();
        engine.rebuild(&make_docs(25)).unwrap();

        let result = engine
            .search(&SearchQuery {
                q: "rust".to_owned(),
                page: 1,
                per_page: 10,
            })
            .unwrap();

        assert_eq!(result.total, 25);
        assert_eq!(result.total_pages, 3);
        assert_eq!(result.hits.len(), 10);
    }

    #[test]
    fn pagination_page2_returns_correct_offset() {
        let engine = SearchEngine::new();
        engine.rebuild(&make_docs(25)).unwrap();

        let page1 = engine
            .search(&SearchQuery {
                q: "rust".to_owned(),
                page: 1,
                per_page: 10,
            })
            .unwrap();
        let page2 = engine
            .search(&SearchQuery {
                q: "rust".to_owned(),
                page: 2,
                per_page: 10,
            })
            .unwrap();

        assert_eq!(page2.hits.len(), 10);
        // page 1 and page 2 must not overlap
        let slugs1: std::collections::HashSet<_> = page1.hits.iter().map(|h| &h.slug).collect();
        let slugs2: std::collections::HashSet<_> = page2.hits.iter().map(|h| &h.slug).collect();
        assert!(
            slugs1.is_disjoint(&slugs2),
            "page 1 and page 2 should not share results"
        );
    }

    // --- AND / OR search ---

    #[test]
    fn and_search_returns_only_both_terms() {
        let engine = SearchEngine::new();
        engine
            .rebuild(&[
                doc(
                    "rust-azure",
                    "Rust on Azure",
                    "rust and azure cloud",
                    &[],
                    "2024-01-01",
                ),
                doc(
                    "rust-only",
                    "Rust Only",
                    "rust systems programming",
                    &[],
                    "2024-01-02",
                ),
                doc(
                    "azure-only",
                    "Azure Only",
                    "azure cloud services",
                    &[],
                    "2024-01-03",
                ),
            ])
            .unwrap();

        // conjunction is default; space means AND
        let results = engine.search(&q("rust azure")).unwrap();
        assert_eq!(results.hits.len(), 1);
        assert_eq!(results.hits[0].slug, "rust-azure");
    }

    #[test]
    fn or_search_returns_either_term() {
        let engine = SearchEngine::new();
        engine
            .rebuild(&[
                doc(
                    "rust-post",
                    "Rust Post",
                    "rust systems programming",
                    &[],
                    "2024-01-01",
                ),
                doc(
                    "leptos-post",
                    "Leptos Post",
                    "leptos web framework",
                    &[],
                    "2024-01-02",
                ),
                doc(
                    "other-post",
                    "Other Post",
                    "something else entirely",
                    &[],
                    "2024-01-03",
                ),
            ])
            .unwrap();

        let results = engine.search(&q("rust OR leptos")).unwrap();
        let slugs: Vec<_> = results.hits.iter().map(|h| h.slug.as_str()).collect();
        assert!(slugs.contains(&"rust-post"), "rust-post should match");
        assert!(slugs.contains(&"leptos-post"), "leptos-post should match");
        assert!(
            !slugs.contains(&"other-post"),
            "other-post should not match"
        );
    }

    // --- tags field search ---

    #[test]
    fn tags_field_query_matches_tagged_posts() {
        let engine = SearchEngine::new();
        engine
            .rebuild(&[
                doc(
                    "tagged-rust",
                    "Rust Post",
                    "general content",
                    &["rust"],
                    "2024-01-01",
                ),
                doc(
                    "tagged-go",
                    "Go Post",
                    "general content",
                    &["go"],
                    "2024-01-02",
                ),
                doc(
                    "no-tags",
                    "No Tag Post",
                    "general content",
                    &[],
                    "2024-01-03",
                ),
            ])
            .unwrap();

        let results = engine.search(&q("tags:rust")).unwrap();
        assert_eq!(results.hits.len(), 1);
        assert_eq!(results.hits[0].slug, "tagged-rust");
    }

    // --- save_to / load_from ---

    #[tokio::test]
    async fn save_and_load_produces_same_results() {
        let engine = SearchEngine::new();
        engine
            .rebuild(&[
                doc(
                    "post-a",
                    "Post A",
                    "rust and tantivy",
                    &["rust"],
                    "2024-01-01",
                ),
                doc(
                    "post-b",
                    "Post B",
                    "leptos web ui",
                    &["leptos"],
                    "2024-01-02",
                ),
            ])
            .unwrap();

        let storage = MockStorage::empty();
        engine.save_to(&storage).await.unwrap();

        let engine2 = SearchEngine::new();
        engine2.load_from(&storage).await.unwrap();

        let results = engine2.search(&q("tantivy")).unwrap();
        assert_eq!(results.hits.len(), 1);
        assert_eq!(results.hits[0].slug, "post-a");

        let results2 = engine2.search(&q("leptos")).unwrap();
        assert_eq!(results2.hits.len(), 1);
        assert_eq!(results2.hits[0].slug, "post-b");
    }
}
