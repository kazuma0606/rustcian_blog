use std::{collections::HashSet, sync::RwLock};

use rustacian_blog_core::{BlogError, Post, PostStatus, SearchResult};
use tantivy::{
    Index, IndexReader, TantivyDocument,
    collector::TopDocs,
    doc,
    query::QueryParser,
    schema::{Field, STORED, STRING, Schema, TEXT, Value},
};

struct SearchInner {
    index: Index,
    reader: IndexReader,
}

pub struct TantivySearchIndex {
    schema: Schema,
    slug: Field,
    title: Field,
    body_text: Field,
    tags: Field,
    date: Field,
    inner: RwLock<Option<SearchInner>>,
}

impl TantivySearchIndex {
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

    /// (Re)build the in-memory index from a list of posts.
    /// Only `Published` posts are indexed; drafts are silently skipped.
    pub fn rebuild(&self, posts: &[Post]) -> Result<(), BlogError> {
        let index = Index::create_in_ram(self.schema.clone());
        let mut writer = index
            .writer(16_000_000) // 16 MB heap
            .map_err(|e| BlogError::Storage(format!("tantivy writer: {e}")))?;

        for post in posts {
            if !matches!(post.status, PostStatus::Published) {
                continue;
            }
            let body_plain = strip_html(&post.body_html);
            let tags_text = post.tags.join(" ");
            let date_str = post.published_at.format("%Y-%m-%d").to_string();

            writer
                .add_document(doc!(
                    self.slug => post.slug.as_str(),
                    self.title => post.title.as_str(),
                    self.body_text => body_plain.as_str(),
                    self.tags => tags_text.as_str(),
                    self.date => date_str.as_str(),
                ))
                .map_err(|e| BlogError::Storage(format!("tantivy add_document: {e}")))?;
        }

        writer
            .commit()
            .map_err(|e| BlogError::Storage(format!("tantivy commit: {e}")))?;

        let reader = index
            .reader()
            .map_err(|e| BlogError::Storage(format!("tantivy reader: {e}")))?;

        let mut guard = self
            .inner
            .write()
            .map_err(|e| BlogError::Storage(format!("lock poisoned: {e}")))?;
        *guard = Some(SearchInner { index, reader });
        Ok(())
    }

    /// Execute a search query and return up to `limit` results.
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<SearchResult>, BlogError> {
        let guard = self
            .inner
            .read()
            .map_err(|e| BlogError::Storage(format!("lock poisoned: {e}")))?;
        let Some(inner) = guard.as_ref() else {
            return Ok(Vec::new());
        };

        let searcher = inner.reader.searcher();
        let mut parser =
            QueryParser::for_index(&inner.index, vec![self.title, self.body_text, self.tags]);
        parser.set_conjunction_by_default();

        let query = match parser.parse_query(query_str) {
            Ok(q) => q,
            Err(_) => return Ok(Vec::new()),
        };

        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(limit))
            .map_err(|e| BlogError::Storage(format!("tantivy search: {e}")))?;

        let mut results = Vec::with_capacity(top_docs.len());
        for (_score, addr) in top_docs {
            let retrieved: TantivyDocument = searcher
                .doc(addr)
                .map_err(|e| BlogError::Storage(format!("tantivy doc retrieve: {e}")))?;

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

            results.push(SearchResult {
                slug,
                title,
                excerpt,
                tags,
                date,
            });
        }
        Ok(results)
    }
}

impl Default for TantivySearchIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Strip all HTML tags from a string and return plain text.
fn strip_html(html: &str) -> String {
    ammonia::Builder::new()
        .tags(HashSet::new())
        .clean(html)
        .to_string()
}

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
    use chrono::Utc;
    use rustacian_blog_core::{PostMetadata, PostStatus, TocItem};

    fn make_post(slug: &str, title: &str, body_html: &str, status: PostStatus) -> Post {
        Post::new(
            PostMetadata {
                title: title.to_owned(),
                slug: slug.to_owned(),
                published_at: Utc::now(),
                updated_at: None,
                tags: vec!["rust".to_owned()],
                summary: "summary".to_owned(),
                hero_image: None,
                status,
                toc: false,
                math: false,
                charts: Vec::new(),
                summary_ai: None,
            },
            Vec::<TocItem>::new(),
            body_html.to_owned(),
            body_html.to_owned(),
        )
        .unwrap()
    }

    #[test]
    fn index_returns_matching_post() {
        let idx = TantivySearchIndex::new();
        let posts = vec![
            make_post(
                "hello-rust",
                "Hello Rust",
                "<p>Rust is amazing for systems programming</p>",
                PostStatus::Published,
            ),
            make_post(
                "intro-actix",
                "Intro to Actix",
                "<p>Actix Web framework tutorial</p>",
                PostStatus::Published,
            ),
        ];
        idx.rebuild(&posts).unwrap();

        let results = idx.search("systems programming", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].slug, "hello-rust");
    }

    #[test]
    fn index_returns_empty_for_no_match() {
        let idx = TantivySearchIndex::new();
        let posts = vec![make_post(
            "hello",
            "Hello World",
            "<p>Hello</p>",
            PostStatus::Published,
        )];
        idx.rebuild(&posts).unwrap();

        let results = idx.search("javascript typescript", 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn draft_posts_are_not_indexed() {
        let idx = TantivySearchIndex::new();
        let posts = vec![
            make_post(
                "published-post",
                "Published",
                "<p>public content</p>",
                PostStatus::Published,
            ),
            make_post(
                "draft-post",
                "Draft Secret",
                "<p>secret draft content</p>",
                PostStatus::Draft,
            ),
        ];
        idx.rebuild(&posts).unwrap();

        let results = idx.search("secret draft", 10).unwrap();
        assert!(
            results.is_empty(),
            "draft should not appear in search results"
        );

        let results = idx.search("public content", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].slug, "published-post");
    }

    #[test]
    fn empty_query_returns_empty() {
        let idx = TantivySearchIndex::new();
        let posts = vec![make_post(
            "p1",
            "Post 1",
            "<p>content</p>",
            PostStatus::Published,
        )];
        idx.rebuild(&posts).unwrap();

        let results = idx.search("", 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn rebuild_replaces_old_index() {
        let idx = TantivySearchIndex::new();
        let v1 = vec![make_post(
            "old-post",
            "Old Post",
            "<p>old content</p>",
            PostStatus::Published,
        )];
        idx.rebuild(&v1).unwrap();

        let v2 = vec![make_post(
            "new-post",
            "New Post",
            "<p>new content</p>",
            PostStatus::Published,
        )];
        idx.rebuild(&v2).unwrap();

        let old = idx.search("old content", 10).unwrap();
        assert!(old.is_empty(), "old content should be gone after rebuild");

        let new = idx.search("new content", 10).unwrap();
        assert_eq!(new.len(), 1);
    }
}
