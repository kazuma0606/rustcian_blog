use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostSummaryView {
    pub title: String,
    pub slug: String,
    pub published_at: String,
    pub tags: Vec<String>,
    pub summary: String,
    pub hero_image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostView {
    pub title: String,
    pub slug: String,
    pub published_at: String,
    pub tags: Vec<String>,
    pub summary: String,
    pub hero_image: Option<String>,
    pub body_html: String,
}

pub fn render_posts_page(posts: Vec<PostSummaryView>) -> String {
    let body = view! { <PostsPage posts=posts/> }.to_html();
    wrap_document("Rustacian Blog", &body)
}

pub fn render_post_page(post: PostView) -> String {
    let title = post.title.clone();
    let body = view! { <PostPage post=post/> }.to_html();
    wrap_document(&title, &body)
}

fn wrap_document(title: &str, body: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="ja">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{title}</title>
    <style>
      :root {{
        --bg: #f7f1e7;
        --surface: rgba(255, 252, 246, 0.88);
        --line: #d9c2a3;
        --text: #1f2933;
        --muted: #52606d;
        --accent: #b3541e;
        --accent-soft: #efd8b9;
      }}
      * {{ box-sizing: border-box; }}
      body {{
        margin: 0;
        color: var(--text);
        background:
          radial-gradient(circle at top left, rgba(239, 216, 185, 0.9), transparent 32%),
          linear-gradient(160deg, #f7f1e7 0%, #efe5d4 100%);
        font-family: Georgia, "Times New Roman", serif;
      }}
      a {{ color: inherit; }}
      .shell {{
        max-width: 960px;
        margin: 0 auto;
        padding: 32px 20px 72px;
      }}
      .hero {{
        padding: 28px;
        border: 1px solid var(--line);
        border-radius: 28px;
        background: var(--surface);
        box-shadow: 0 18px 48px rgba(98, 72, 50, 0.12);
      }}
      .eyebrow {{
        letter-spacing: 0.12em;
        text-transform: uppercase;
        font-size: 12px;
        color: var(--accent);
      }}
      h1, h2, h3 {{ line-height: 1.1; }}
      .posts {{
        display: grid;
        gap: 18px;
        margin-top: 28px;
      }}
      .card {{
        display: grid;
        gap: 14px;
        border-radius: 24px;
        padding: 22px;
        border: 1px solid var(--line);
        background: rgba(255, 249, 242, 0.94);
        text-decoration: none;
        box-shadow: 0 12px 28px rgba(98, 72, 50, 0.08);
      }}
      .card img, .post img {{
        width: 100%;
        max-height: 280px;
        object-fit: cover;
        border-radius: 18px;
        border: 1px solid var(--line);
        background: white;
      }}
      .meta {{
        color: var(--muted);
        font-size: 14px;
      }}
      .tags {{
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
      }}
      .tag {{
        display: inline-flex;
        align-items: center;
        padding: 6px 12px;
        border-radius: 999px;
        background: var(--accent-soft);
        color: #713f12;
        font-size: 13px;
      }}
      .post {{
        margin-top: 28px;
        padding: 28px;
        border-radius: 28px;
        border: 1px solid var(--line);
        background: rgba(255, 252, 246, 0.94);
      }}
      .post-body {{
        line-height: 1.7;
      }}
      .post-body h1, .post-body h2, .post-body h3 {{
        margin-top: 1.8em;
      }}
      .nav {{
        display: inline-flex;
        margin-top: 20px;
        color: var(--accent);
        text-decoration: none;
      }}
      @media (max-width: 640px) {{
        .shell {{ padding: 20px 16px 48px; }}
        .hero, .post, .card {{ border-radius: 22px; }}
      }}
    </style>
  </head>
  <body>{body}</body>
</html>"#
    )
}

#[component]
fn PostsPage(posts: Vec<PostSummaryView>) -> impl IntoView {
    let post_cards = posts
        .into_iter()
        .map(|post| {
            let hero = post.hero_image.clone();
            let slug = post.slug;
            let title = post.title;
            let published_at = post.published_at;
            let summary = post.summary;
            let hero_view = if let Some(src) = hero {
                view! { <img src=src alt=title.clone()/> }.into_any()
            } else {
                ().into_any()
            };
            let tags = post
                .tags
                .into_iter()
                .map(|tag| view! { <span class="tag">{tag}</span> })
                .collect_view();

            view! {
                <a class="card" href=format!("/p/{slug}")>
                    {hero_view}
                    <div class="meta">{published_at}</div>
                    <h2>{title}</h2>
                    <p>{summary}</p>
                    <div class="tags">{tags}</div>
                </a>
            }
        })
        .collect_view();

    view! {
        <main class="shell">
            <section class="hero">
                <div class="eyebrow">"Rustacian Blog PoC"</div>
                <h1>"Markdown, Actix Web, and Leptos"</h1>
                <p>
                    "Local-first で記事を読み込みつつ、Core と adapter を分離した最小ブログ構成です。"
                </p>
            </section>
            <section class="posts">{post_cards}</section>
        </main>
    }
}

#[component]
fn PostPage(post: PostView) -> impl IntoView {
    let hero = post.hero_image.clone();
    let hero_view = if let Some(src) = hero {
        view! { <img src=src alt=post.title.clone()/> }.into_any()
    } else {
        ().into_any()
    };
    let tags = post
        .tags
        .into_iter()
        .map(|tag| view! { <span class="tag">{tag}</span> })
        .collect_view();

    view! {
        <main class="shell">
            <section class="hero">
                <div class="eyebrow">"Post Detail"</div>
                <h1>{post.title.clone()}</h1>
                <p>{post.summary.clone()}</p>
                <div class="meta">{post.published_at.clone()}</div>
                <div class="tags">{tags}</div>
            </section>
            <article class="post">
                {hero_view}
                <div class="post-body" inner_html=post.body_html.clone()></div>
                <a class="nav" href="/">"← Back to posts"</a>
            </article>
        </main>
    }
}
