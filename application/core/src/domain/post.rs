use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::BlogError;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum PostStatus {
    Draft,
    #[default]
    Published,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TocItem {
    pub level: u8,
    pub title: String,
    pub anchor: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PostVisibility {
    PublishedOnly,
    IncludeDrafts,
}

impl PostVisibility {
    pub fn allows(self, status: PostStatus) -> bool {
        match self {
            Self::PublishedOnly => matches!(status, PostStatus::Published),
            Self::IncludeDrafts => true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChartDefinition {
    pub r#type: String,
    pub source: String,
    pub x: String,
    pub y: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub caption: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChartPoint {
    pub x: String,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenderedChart {
    pub chart_type: String,
    pub source: String,
    pub x: String,
    pub y: String,
    pub title: Option<String>,
    pub caption: Option<String>,
    #[serde(default)]
    pub points: Vec<ChartPoint>,
    #[serde(default)]
    pub table_headers: Vec<String>,
    #[serde(default)]
    pub table_rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostMetadata {
    pub title: String,
    pub slug: String,
    pub published_at: DateTime<Utc>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub summary: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub hero_image: Option<String>,
    #[serde(default)]
    pub status: PostStatus,
    #[serde(default)]
    pub toc: bool,
    #[serde(default)]
    pub math: bool,
    #[serde(default)]
    pub charts: Vec<ChartDefinition>,
    #[serde(default)]
    pub summary_ai: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Post {
    pub title: String,
    pub slug: String,
    pub published_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
    pub summary: String,
    #[serde(default)]
    pub description: Option<String>,
    pub hero_image: Option<String>,
    pub status: PostStatus,
    pub toc: bool,
    pub math: bool,
    pub summary_ai: Option<String>,
    pub read_minutes: usize,
    #[serde(default)]
    pub charts: Vec<ChartDefinition>,
    #[serde(default)]
    pub chart_data: Vec<RenderedChart>,
    #[serde(default)]
    pub toc_items: Vec<TocItem>,
    pub body_markdown: String,
    pub body_html: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostSummary {
    pub title: String,
    pub slug: String,
    pub published_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
    pub summary: String,
    #[serde(default)]
    pub description: Option<String>,
    pub hero_image: Option<String>,
    pub status: PostStatus,
    pub toc: bool,
    pub math: bool,
    #[serde(default)]
    pub summary_ai: Option<String>,
    pub read_minutes: usize,
}

impl Post {
    pub fn new(
        metadata: PostMetadata,
        toc_items: Vec<TocItem>,
        body_markdown: String,
        body_html: String,
    ) -> Result<Self, BlogError> {
        validate_slug(&metadata.slug)?;

        if metadata.title.trim().is_empty() {
            return Err(BlogError::Validation("title is required".to_owned()));
        }

        if metadata.summary.trim().is_empty() {
            return Err(BlogError::Validation("summary is required".to_owned()));
        }

        validate_metadata_rules(&metadata)?;

        if body_markdown.trim().is_empty() {
            return Err(BlogError::Validation(
                "markdown body is required".to_owned(),
            ));
        }

        let read_minutes = estimate_read_minutes(&body_markdown);

        Ok(Self {
            title: metadata.title,
            slug: metadata.slug,
            published_at: metadata.published_at,
            updated_at: metadata.updated_at,
            tags: metadata.tags,
            summary: metadata.summary,
            description: metadata.description,
            hero_image: metadata.hero_image,
            status: metadata.status,
            toc: metadata.toc,
            math: metadata.math,
            summary_ai: metadata.summary_ai,
            read_minutes,
            charts: metadata.charts,
            chart_data: Vec::new(),
            toc_items,
            body_markdown,
            body_html,
        })
    }

    pub fn is_published(&self) -> bool {
        matches!(self.status, PostStatus::Published)
    }

    pub fn summary(&self) -> PostSummary {
        PostSummary {
            title: self.title.clone(),
            slug: self.slug.clone(),
            published_at: self.published_at,
            updated_at: self.updated_at,
            tags: self.tags.clone(),
            summary: self.summary.clone(),
            description: self.description.clone(),
            hero_image: self.hero_image.clone(),
            status: self.status,
            toc: self.toc,
            math: self.math,
            summary_ai: self.summary_ai.clone(),
            read_minutes: self.read_minutes,
        }
    }
}

fn estimate_read_minutes(body: &str) -> usize {
    let char_count = body.chars().count();
    ((char_count as f64 / 400.0).ceil() as usize).max(1)
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

fn validate_metadata_rules(metadata: &PostMetadata) -> Result<(), BlogError> {
    if let Some(updated_at) = metadata.updated_at
        && updated_at < metadata.published_at
    {
        return Err(BlogError::Validation(
            "updated_at must be greater than or equal to published_at".to_owned(),
        ));
    }

    if let Some(summary_ai) = &metadata.summary_ai
        && summary_ai.trim().is_empty()
    {
        return Err(BlogError::Validation(
            "summary_ai must not be empty when present".to_owned(),
        ));
    }

    if !metadata.charts.is_empty() && !metadata.math && metadata.summary_ai.is_none() {
        // Charts and AI metadata are allowed independently, but chart usage should be explicit in metadata.
    }

    for chart in &metadata.charts {
        validate_chart_definition(chart)?;
    }

    Ok(())
}

fn validate_chart_definition(chart: &ChartDefinition) -> Result<(), BlogError> {
    match chart.r#type.as_str() {
        "line" | "bar" | "scatter" => {}
        other => {
            return Err(BlogError::Validation(format!(
                "unsupported chart type: {other}"
            )));
        }
    }

    if chart.source.trim().is_empty() {
        return Err(BlogError::Validation("chart source is required".to_owned()));
    }
    if chart.x.trim().is_empty() {
        return Err(BlogError::Validation("chart x is required".to_owned()));
    }
    if chart.y.trim().is_empty() {
        return Err(BlogError::Validation("chart y is required".to_owned()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use super::*;

    fn sample_metadata() -> PostMetadata {
        PostMetadata {
            title: "Valid".to_owned(),
            slug: "valid-post".to_owned(),
            published_at: DateTime::parse_from_rfc3339("2026-03-19T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            updated_at: Some(
                DateTime::parse_from_rfc3339("2026-03-20T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
            ),
            tags: vec!["rust".to_owned()],
            summary: "summary".to_owned(),
            description: None,
            hero_image: None,
            status: PostStatus::Published,
            toc: true,
            math: false,
            charts: vec![ChartDefinition {
                r#type: "line".to_owned(),
                source: "./metrics.csv".to_owned(),
                x: "date".to_owned(),
                y: "value".to_owned(),
                title: Some("Metrics".to_owned()),
                caption: None,
            }],
            summary_ai: Some("AI generated summary".to_owned()),
        }
    }

    #[test]
    fn post_accepts_valid_metadata() {
        let post = Post::new(
            sample_metadata(),
            vec![TocItem {
                level: 2,
                title: "Section".to_owned(),
                anchor: "section".to_owned(),
            }],
            "body".to_owned(),
            "<p>body</p>".to_owned(),
        )
        .unwrap();

        assert_eq!(post.slug, "valid-post");
        assert!(post.is_published());
        assert!(post.toc);
        assert_eq!(post.toc_items.len(), 1);
        assert_eq!(post.charts.len(), 1);
        assert_eq!(post.summary_ai.as_deref(), Some("AI generated summary"));
        assert_eq!(
            post.updated_at,
            Some(
                DateTime::parse_from_rfc3339("2026-03-20T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc)
            )
        );
    }

    #[test]
    fn post_rejects_invalid_slug() {
        let mut metadata = sample_metadata();
        metadata.slug = "Invalid Slug".to_owned();

        let error = Post::new(
            metadata,
            Vec::new(),
            "body".to_owned(),
            "<p>body</p>".to_owned(),
        )
        .unwrap_err();

        assert!(matches!(error, BlogError::Validation(_)));
    }

    #[test]
    fn draft_posts_are_not_published() {
        let mut metadata = sample_metadata();
        metadata.status = PostStatus::Draft;

        let post = Post::new(
            metadata,
            Vec::new(),
            "body".to_owned(),
            "<p>body</p>".to_owned(),
        )
        .unwrap();

        assert!(!post.is_published());
    }

    #[test]
    fn published_only_visibility_excludes_drafts() {
        assert!(PostVisibility::PublishedOnly.allows(PostStatus::Published));
        assert!(!PostVisibility::PublishedOnly.allows(PostStatus::Draft));
    }

    #[test]
    fn post_rejects_invalid_chart_type() {
        let mut metadata = sample_metadata();
        metadata.charts[0].r#type = "pie".to_owned();

        let error = Post::new(
            metadata,
            Vec::new(),
            "body".to_owned(),
            "<p>body</p>".to_owned(),
        )
        .unwrap_err();

        assert!(matches!(error, BlogError::Validation(_)));
    }

    #[test]
    fn post_rejects_updated_at_before_published_at() {
        let mut metadata = sample_metadata();
        metadata.updated_at = Some(
            DateTime::parse_from_rfc3339("2026-03-18T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
        );

        let error = Post::new(
            metadata,
            Vec::new(),
            "body".to_owned(),
            "<p>body</p>".to_owned(),
        )
        .unwrap_err();

        assert!(matches!(error, BlogError::Validation(_)));
    }

    #[test]
    fn post_rejects_blank_summary_ai_when_present() {
        let mut metadata = sample_metadata();
        metadata.summary_ai = Some("   ".to_owned());

        let error = Post::new(
            metadata,
            Vec::new(),
            "body".to_owned(),
            "<p>body</p>".to_owned(),
        )
        .unwrap_err();

        assert!(matches!(error, BlogError::Validation(_)));
    }
}
