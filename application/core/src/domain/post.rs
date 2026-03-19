use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::BlogError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostFrontmatter {
    pub title: String,
    pub slug: String,
    pub published_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub summary: String,
    #[serde(default)]
    pub hero_image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Post {
    pub title: String,
    pub slug: String,
    pub published_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub summary: String,
    pub hero_image: Option<String>,
    pub body_markdown: String,
    pub body_html: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostSummary {
    pub title: String,
    pub slug: String,
    pub published_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub summary: String,
    pub hero_image: Option<String>,
}

impl Post {
    pub fn new(
        frontmatter: PostFrontmatter,
        body_markdown: String,
        body_html: String,
    ) -> Result<Self, BlogError> {
        validate_slug(&frontmatter.slug)?;

        if frontmatter.title.trim().is_empty() {
            return Err(BlogError::Validation("title is required".to_owned()));
        }

        if frontmatter.summary.trim().is_empty() {
            return Err(BlogError::Validation("summary is required".to_owned()));
        }

        Ok(Self {
            title: frontmatter.title,
            slug: frontmatter.slug,
            published_at: frontmatter.published_at,
            tags: frontmatter.tags,
            summary: frontmatter.summary,
            hero_image: frontmatter.hero_image,
            body_markdown,
            body_html,
        })
    }

    pub fn summary(&self) -> PostSummary {
        PostSummary {
            title: self.title.clone(),
            slug: self.slug.clone(),
            published_at: self.published_at,
            tags: self.tags.clone(),
            summary: self.summary.clone(),
            hero_image: self.hero_image.clone(),
        }
    }
}

fn validate_slug(slug: &str) -> Result<(), BlogError> {
    if slug.trim().is_empty() {
        return Err(BlogError::Validation("slug is required".to_owned()));
    }

    if !slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(BlogError::Validation(format!(
            "slug must use lowercase ascii, digits, and hyphen: {slug}"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use super::*;

    #[test]
    fn post_accepts_valid_frontmatter() {
        let frontmatter = PostFrontmatter {
            title: "Valid".to_owned(),
            slug: "valid-post".to_owned(),
            published_at: DateTime::parse_from_rfc3339("2026-03-19T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            tags: vec!["rust".to_owned()],
            summary: "summary".to_owned(),
            hero_image: None,
        };

        let post = Post::new(frontmatter, "body".to_owned(), "<p>body</p>".to_owned()).unwrap();

        assert_eq!(post.slug, "valid-post");
    }

    #[test]
    fn post_rejects_invalid_slug() {
        let frontmatter = PostFrontmatter {
            title: "Invalid".to_owned(),
            slug: "Invalid Slug".to_owned(),
            published_at: DateTime::parse_from_rfc3339("2026-03-19T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            tags: vec!["rust".to_owned()],
            summary: "summary".to_owned(),
            hero_image: None,
        };

        let error =
            Post::new(frontmatter, "body".to_owned(), "<p>body</p>".to_owned()).unwrap_err();

        assert!(matches!(error, BlogError::Validation(_)));
    }
}
