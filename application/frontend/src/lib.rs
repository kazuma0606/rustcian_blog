use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocItemView {
    pub level: u8,
    pub title: String,
    pub anchor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChartPointView {
    pub x: String,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenderedChartView {
    pub chart_type: String,
    pub source: String,
    pub x: String,
    pub y: String,
    pub title: Option<String>,
    pub caption: Option<String>,
    pub points: Vec<ChartPointView>,
    pub table_headers: Vec<String>,
    pub table_rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostSummaryView {
    pub title: String,
    pub slug: String,
    pub published_at: String,
    pub updated_at: Option<String>,
    pub tags: Vec<String>,
    pub summary: String,
    pub hero_image: Option<String>,
    pub toc: bool,
    pub math: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagLinkView {
    pub tag: String,
    pub href: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostView {
    pub title: String,
    pub slug: String,
    pub published_at: String,
    pub updated_at: Option<String>,
    pub tags: Vec<String>,
    pub summary: String,
    pub hero_image: Option<String>,
    pub toc: bool,
    pub math: bool,
    pub summary_ai: Option<String>,
    pub charts: Vec<RenderedChartView>,
    pub toc_items: Vec<TocItemView>,
    pub body_html: String,
}

pub fn render_posts_page(posts: Vec<PostSummaryView>) -> String {
    render_posts_shell(
        "Rustacian Blog",
        "Rustacian Blog PoC",
        "Markdown, Actix Web, and Leptos",
        "Local-first で記事を管理しつつ、Core と adapter を分離した Rust ブログの実験場です。",
        posts,
    )
}

pub fn render_tag_posts_page(tag: &str, posts: Vec<PostSummaryView>) -> String {
    render_posts_shell(
        &format!("Posts tagged {tag}"),
        "Tag Archive",
        &format!("Posts tagged {tag}"),
        &format!("`{tag}` を付けた公開記事の一覧です。"),
        posts,
    )
}

pub fn render_tags_page(tags: Vec<TagLinkView>) -> String {
    let body = view! { <TagsPage tags=tags/> }.to_html();
    wrap_document("Tags", &body, false, false)
}

pub fn render_post_page(post: PostView) -> String {
    let title = post.title.clone();
    let enable_math = post.math;
    let enable_mermaid = post.body_html.contains("class=\"mermaid\"");
    let body = view! { <PostPage post=post/> }.to_html();
    wrap_document(&title, &body, enable_math, enable_mermaid)
}

fn render_posts_shell(
    title: &str,
    eyebrow: &str,
    headline: &str,
    summary: &str,
    posts: Vec<PostSummaryView>,
) -> String {
    let body = view! {
        <PostsPage
            posts=posts
            eyebrow=eyebrow.to_owned()
            headline=headline.to_owned()
            summary=summary.to_owned()
        />
    }
    .to_html();
    wrap_document(title, &body, false, false)
}

fn wrap_document(title: &str, body: &str, enable_math: bool, enable_mermaid: bool) -> String {
    let math_head = if enable_math {
        r#"
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.css">
    <script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.js"></script>
    <script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/contrib/auto-render.min.js"></script>
    <script>
      document.addEventListener("DOMContentLoaded", function () {
        function showMathFallback(message) {
          document.body.classList.add("math-fallback");
          var note = document.querySelector("[data-math-fallback]");
          if (note) {
            note.hidden = false;
            note.textContent = message;
          }
        }

        if (window.renderMathInElement) {
          try {
            window.renderMathInElement(document.body, {
              delimiters: [
                { left: "$$", right: "$$", display: true },
                { left: "\\[", right: "\\]", display: true },
                { left: "$", right: "$", display: false },
                { left: "\\(", right: "\\)", display: false }
              ],
              throwOnError: false
            });
          } catch (error) {
            showMathFallback("Math rendering is unavailable. Raw formulas are shown instead.");
          }
        } else {
          showMathFallback("Math rendering is unavailable. Raw formulas are shown instead.");
        }
      });
    </script>"#
    } else {
        ""
    };
    let mermaid_head = if enable_mermaid {
        r#"
    <script type="module">
      import mermaid from "https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs";
      mermaid.initialize({
        startOnLoad: false,
        securityLevel: "loose",
        theme: "neutral"
      });
      document.addEventListener("DOMContentLoaded", async function () {
        const blocks = document.querySelectorAll("pre.mermaid");
        if (!blocks.length) return;
        await mermaid.run({ nodes: blocks });
      });
    </script>"#
    } else {
        ""
    };

    format!(
        r#"<!doctype html>
<html lang="ja">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{title}</title>
    {math_head}
    {mermaid_head}
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
      .card img[src$=".svg"], .post img[src$=".svg"] {{
        object-fit: contain;
        background: transparent;
      }}
      .meta {{
        color: var(--muted);
        font-size: 14px;
      }}
      .meta-stack {{
        display: grid;
        gap: 6px;
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
      .tag.is-info {{
        background: #f2eadc;
        color: var(--muted);
      }}
      .post {{
        margin-top: 28px;
        padding: 28px;
        border-radius: 28px;
        border: 1px solid var(--line);
        background: rgba(255, 252, 246, 0.94);
      }}
      .toc {{
        margin-top: 24px;
        padding: 18px 20px;
        border: 1px solid var(--line);
        border-radius: 20px;
        background: rgba(247, 241, 231, 0.8);
      }}
      .toc h2 {{
        margin: 0 0 12px;
        font-size: 18px;
      }}
      .toc ul {{
        margin: 0;
        padding-left: 18px;
        display: grid;
        gap: 8px;
      }}
      .toc a {{
        text-decoration: none;
        color: var(--accent);
      }}
      .post-body {{
        line-height: 1.7;
      }}
      .post-body pre {{
        margin: 20px 0;
        padding: 18px 20px;
        overflow-x: auto;
        border-radius: 18px;
        border: 1px solid #d8c5ad;
        background: #2a211c;
        color: #f8efe4;
      }}
      .post-body pre.mermaid {{
        padding: 12px;
        border: 1px solid #d8c5ad;
        background: rgba(255, 252, 246, 0.98);
        color: var(--text);
      }}
      .post-body pre code {{
        display: block;
        padding: 0;
        background: transparent;
        color: inherit;
        border: 0;
      }}
      .post-body code {{
        padding: 0.15em 0.45em;
        border-radius: 8px;
        background: rgba(239, 216, 185, 0.45);
        border: 1px solid rgba(179, 84, 30, 0.12);
        color: #713f12;
        font-family: "Courier New", monospace;
        font-size: 0.95em;
      }}
      .post-body .katex-display {{
        overflow-x: auto;
        overflow-y: hidden;
        padding: 8px 0;
      }}
      .math-fallback-note {{
        margin-top: 24px;
        padding: 14px 16px;
        border: 1px solid #d3b689;
        border-radius: 16px;
        background: rgba(239, 216, 185, 0.55);
        color: #7c4a1d;
      }}
      body.math-fallback .post-body .math-inline,
      body.math-fallback .post-body .math-display {{
        display: inline-block;
        padding: 2px 6px;
        border-radius: 8px;
        background: rgba(247, 241, 231, 0.95);
        border: 1px dashed var(--line);
        font-family: "Courier New", monospace;
        white-space: pre-wrap;
      }}
      body.math-fallback .post-body .math-display {{
        display: block;
        padding: 12px 14px;
        margin: 16px 0;
        overflow-x: auto;
      }}
      .ai-summary {{
        margin-top: 24px;
        padding: 18px 20px;
        border-left: 4px solid var(--accent);
        background: rgba(239, 216, 185, 0.4);
        border-radius: 12px;
      }}
      .chart-list {{
        margin-top: 24px;
        padding: 18px 20px;
        border: 1px solid var(--line);
        border-radius: 20px;
        background: rgba(247, 241, 231, 0.7);
      }}
      .chart-card {{
        display: grid;
        gap: 12px;
        padding: 18px 0;
        border-top: 1px solid rgba(217, 194, 163, 0.8);
      }}
      .chart-card:first-child {{
        padding-top: 0;
        border-top: 0;
      }}
      .chart-svg {{
        width: 100%;
        height: auto;
        border-radius: 16px;
        border: 1px solid rgba(217, 194, 163, 0.8);
        background: linear-gradient(180deg, rgba(255, 252, 246, 0.96), rgba(247, 241, 231, 0.82));
      }}
      .chart-caption {{
        color: var(--muted);
        font-size: 14px;
      }}
      .chart-list h2 {{
        margin: 0 0 12px;
        font-size: 18px;
      }}
      .chart-list code {{
        font-size: 13px;
      }}
      .chart-table-wrap {{
        overflow-x: auto;
      }}
      .chart-table {{
        width: 100%;
        border-collapse: collapse;
        border: 1px solid rgba(217, 194, 163, 0.9);
        background: rgba(255, 252, 246, 0.9);
        font-size: 14px;
      }}
      .chart-table th,
      .chart-table td {{
        padding: 10px 12px;
        border: 1px solid rgba(217, 194, 163, 0.7);
        text-align: left;
        white-space: nowrap;
      }}
      .chart-table th {{
        background: rgba(239, 216, 185, 0.55);
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

fn render_chart_svg(chart: &RenderedChartView) -> String {
    if chart.points.is_empty() {
        return String::new();
    }

    let width = 640.0;
    let height = 280.0;
    let left = 54.0;
    let right = 22.0;
    let top = 20.0;
    let bottom = 54.0;
    let plot_width = width - left - right;
    let plot_height = height - top - bottom;
    let max_y = chart
        .points
        .iter()
        .map(|point| point.y)
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let step_x = if chart.points.len() > 1 {
        plot_width / (chart.points.len() - 1) as f64
    } else {
        plot_width / 2.0
    };

    let coordinates = chart
        .points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let x = if chart.points.len() > 1 {
                left + step_x * index as f64
            } else {
                left + plot_width / 2.0
            };
            let y = top + plot_height - (point.y / max_y) * plot_height;
            (x, y, point)
        })
        .collect::<Vec<_>>();

    let grid = (0..=4)
        .map(|index| {
            let ratio = index as f64 / 4.0;
            let y = top + plot_height - ratio * plot_height;
            let value = max_y * ratio;
            format!(
                "<g><line x1=\"{left}\" y1=\"{y:.1}\" x2=\"{}\" y2=\"{y:.1}\" stroke=\"rgba(82, 96, 109, 0.18)\" stroke-width=\"1\" /><text x=\"10\" y=\"{:.1}\" fill=\"#52606d\" font-size=\"12\">{value:.0}</text></g>",
                width - right,
                y + 4.0
            )
        })
        .collect::<String>();

    let labels = coordinates
        .iter()
        .map(|(x, _, point)| {
            format!(
                "<text x=\"{x:.1}\" y=\"{}\" text-anchor=\"middle\" fill=\"#52606d\" font-size=\"12\">{}</text>",
                height - 18.0,
                escape_html(&point.x)
            )
        })
        .collect::<String>();

    let series = match chart.chart_type.as_str() {
        "bar" => {
            let bar_width = (plot_width / chart.points.len().max(1) as f64 * 0.55).max(18.0);
            coordinates
                .iter()
                .map(|(x, y, point)| {
                    let bar_height = top + plot_height - y;
                    format!(
                        "<g><rect x=\"{:.1}\" y=\"{y:.1}\" width=\"{bar_width:.1}\" height=\"{bar_height:.1}\" rx=\"8\" fill=\"#b3541e\" opacity=\"0.82\" /><text x=\"{x:.1}\" y=\"{:.1}\" text-anchor=\"middle\" fill=\"#7c4a1d\" font-size=\"12\">{:.0}</text></g>",
                        x - bar_width / 2.0,
                        y - 8.0,
                        point.y
                    )
                })
                .collect::<String>()
        }
        "scatter" => coordinates
            .iter()
            .map(|(x, y, point)| {
                format!(
                    "<g><circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"6\" fill=\"#b3541e\" /><text x=\"{x:.1}\" y=\"{:.1}\" text-anchor=\"middle\" fill=\"#7c4a1d\" font-size=\"12\">{:.0}</text></g>",
                    y - 12.0,
                    point.y
                )
            })
            .collect::<String>(),
        _ => {
            let path = coordinates
                .iter()
                .enumerate()
                .map(|(index, (x, y, _))| {
                    if index == 0 {
                        format!("M {x:.1} {y:.1}")
                    } else {
                        format!(" L {x:.1} {y:.1}")
                    }
                })
                .collect::<String>();
            let markers = coordinates
                .iter()
                .map(|(x, y, point)| {
                    format!(
                        "<g><circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"5\" fill=\"#b3541e\" /><text x=\"{x:.1}\" y=\"{:.1}\" text-anchor=\"middle\" fill=\"#7c4a1d\" font-size=\"12\">{:.0}</text></g>",
                        y - 12.0,
                        point.y
                    )
                })
                .collect::<String>();
            format!(
                "<path d=\"{path}\" fill=\"none\" stroke=\"#b3541e\" stroke-width=\"3\" stroke-linecap=\"round\" stroke-linejoin=\"round\" />{markers}"
            )
        }
    };

    format!(
        "<svg class=\"chart-svg\" viewBox=\"0 0 {width:.0} {height:.0}\" role=\"img\" aria-label=\"{}\"><line x1=\"{left}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#8d6e52\" stroke-width=\"1.5\" /><line x1=\"{left}\" y1=\"{top}\" x2=\"{left}\" y2=\"{}\" stroke=\"#8d6e52\" stroke-width=\"1.5\" />{grid}{series}{labels}<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" fill=\"#52606d\" font-size=\"12\">{}</text><text x=\"18\" y=\"{}\" text-anchor=\"middle\" fill=\"#52606d\" font-size=\"12\" transform=\"rotate(-90 18,{})\">{}</text></svg>",
        escape_html(chart.title.as_deref().unwrap_or("Chart")),
        top + plot_height,
        width - right,
        top + plot_height,
        top + plot_height,
        left + plot_width / 2.0,
        height - 2.0,
        escape_html(&chart.x),
        top + plot_height / 2.0,
        top + plot_height / 2.0,
        escape_html(&chart.y)
    )
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[component]
fn PostsPage(
    posts: Vec<PostSummaryView>,
    eyebrow: String,
    headline: String,
    summary: String,
) -> impl IntoView {
    let post_cards = posts
        .into_iter()
        .map(|post| {
            let hero_view = post
                .hero_image
                .clone()
                .map(|src| view! { <img src=src alt=post.title.clone()/> }.into_any())
                .unwrap_or_else(|| ().into_any());
            let updated_view = post
                .updated_at
                .clone()
                .map(|value| {
                    view! { <div class="meta">{format!("Updated {value}")}</div> }.into_any()
                })
                .unwrap_or_else(|| ().into_any());
            let toc_tag = if post.toc {
                view! { <span class="tag is-info">"TOC"</span> }.into_any()
            } else {
                ().into_any()
            };
            let math_tag = if post.math {
                view! { <span class="tag is-info">"Math"</span> }.into_any()
            } else {
                ().into_any()
            };
            let tags = post
                .tags
                .into_iter()
                .map(|tag| view! { <span class="tag">{tag}</span> })
                .collect_view();

            view! {
                <a class="card" href=format!("/p/{}", post.slug)>
                    {hero_view}
                    <div class="meta-stack">
                        <div class="meta">{post.published_at}</div>
                        {updated_view}
                    </div>
                    <h2>{post.title}</h2>
                    <p>{post.summary}</p>
                    <div class="tags">{tags}{toc_tag}{math_tag}</div>
                </a>
            }
        })
        .collect_view();

    view! {
        <main class="shell">
            <section class="hero">
                <div class="eyebrow">{eyebrow}</div>
                <h1>{headline}</h1>
                <p>{summary}</p>
            </section>
            <section class="posts">{post_cards}</section>
        </main>
    }
}

#[component]
fn TagsPage(tags: Vec<TagLinkView>) -> impl IntoView {
    let tag_cards = tags
        .into_iter()
        .map(|item| {
            view! {
                <a class="card" href=item.href>
                    <div class="eyebrow">"Tag"</div>
                    <h2>{item.tag}</h2>
                    <p>{format!("{} post(s)", item.count)}</p>
                </a>
            }
        })
        .collect_view();

    view! {
        <main class="shell">
            <section class="hero">
                <div class="eyebrow">"Tag Archive"</div>
                <h1>"Browse by tag"</h1>
                <p>"公開記事から集計したタグ一覧です。"</p>
            </section>
            <section class="posts">{tag_cards}</section>
        </main>
    }
}

#[component]
fn PostPage(post: PostView) -> impl IntoView {
    let hero_view = post
        .hero_image
        .clone()
        .map(|src| view! { <img src=src alt=post.title.clone()/> }.into_any())
        .unwrap_or_else(|| ().into_any());
    let updated_view = post
        .updated_at
        .clone()
        .map(|value| view! { <div class="meta">{format!("Updated {value}")}</div> }.into_any())
        .unwrap_or_else(|| ().into_any());
    let toc_tag = if post.toc {
        view! { <span class="tag is-info">"TOC"</span> }.into_any()
    } else {
        ().into_any()
    };
    let math_tag = if post.math {
        view! { <span class="tag is-info">"Math"</span> }.into_any()
    } else {
        ().into_any()
    };
    let tags = post
        .tags
        .into_iter()
        .map(|tag| view! { <span class="tag">{tag}</span> })
        .collect_view();
    let math_fallback_view = if post.math {
        view! {
            <div class="math-fallback-note" data-math-fallback hidden=true>
                "Math rendering is unavailable. Raw formulas are shown instead."
            </div>
        }
        .into_any()
    } else {
        ().into_any()
    };
    let toc_view = if post.toc && !post.toc_items.is_empty() {
        let items = post
            .toc_items
            .into_iter()
            .map(|item| {
                view! {
                    <li>
                        <a href=format!("#{}", item.anchor)>{item.title}</a>
                    </li>
                }
            })
            .collect_view();
        view! {
            <nav class="toc" aria-label="Table of contents">
                <h2>"Contents"</h2>
                <ul>{items}</ul>
            </nav>
        }
        .into_any()
    } else {
        ().into_any()
    };
    let summary_ai_view = post
        .summary_ai
        .clone()
        .map(|value| {
            view! {
                <section class="ai-summary">
                    <div class="eyebrow">"AI Summary"</div>
                    <p>{value}</p>
                </section>
            }
            .into_any()
        })
        .unwrap_or_else(|| ().into_any());
    let charts_view = if !post.charts.is_empty() {
        let items = post
            .charts
            .into_iter()
            .map(|chart| {
                let title = chart
                    .title
                    .clone()
                    .unwrap_or_else(|| format!("{} chart", chart.chart_type));
                let caption = chart
                    .caption
                    .clone()
                    .map(|value| view! { <div class="chart-caption">{value}</div> }.into_any())
                    .unwrap_or_else(|| ().into_any());
                let chart_svg = render_chart_svg(&chart);
                let table_view = if !chart.table_headers.is_empty() && !chart.table_rows.is_empty()
                {
                    let headers = chart
                        .table_headers
                        .iter()
                        .map(|header| view! { <th>{header.clone()}</th> })
                        .collect_view();
                    let rows = chart
                        .table_rows
                        .iter()
                        .map(|row| {
                            let cells = row
                                .iter()
                                .map(|cell| view! { <td>{cell.clone()}</td> })
                                .collect_view();
                            view! { <tr>{cells}</tr> }
                        })
                        .collect_view();
                    view! {
                        <div class="chart-table-wrap">
                            <table class="chart-table">
                                <thead><tr>{headers}</tr></thead>
                                <tbody>{rows}</tbody>
                            </table>
                        </div>
                    }
                    .into_any()
                } else {
                    ().into_any()
                };

                view! {
                    <section class="chart-card">
                        <strong>{title}</strong>
                        <div class="meta">
                            <code>{chart.source.clone()}</code>
                            {" / "}
                            {format!("x: {}, y: {}", chart.x, chart.y)}
                        </div>
                        <div inner_html=chart_svg></div>
                        {table_view}
                        {caption}
                    </section>
                }
            })
            .collect_view();
        view! {
            <section class="chart-list">
                <h2>"Charts"</h2>
                <div>{items}</div>
            </section>
        }
        .into_any()
    } else {
        ().into_any()
    };

    view! {
        <main class="shell">
            <section class="hero">
                <div class="eyebrow">"Post Detail"</div>
                <h1>{post.title.clone()}</h1>
                <p>{post.summary.clone()}</p>
                <div class="meta-stack">
                    <div class="meta">{post.published_at.clone()}</div>
                    {updated_view}
                </div>
                <div class="tags">{tags}{toc_tag}{math_tag}</div>
            </section>
            <article class="post">
                {hero_view}
                {toc_view}
                {math_fallback_view}
                {summary_ai_view}
                {charts_view}
                <div class="post-body" inner_html=post.body_html.clone()></div>
                <a class="nav" href="/">"Back to posts"</a>
            </article>
        </main>
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ChartPointView, PostSummaryView, PostView, RenderedChartView, TagLinkView,
        render_post_page, render_tag_posts_page, render_tags_page,
    };

    fn sample_post_view() -> PostView {
        PostView {
            title: "Sample".to_owned(),
            slug: "sample".to_owned(),
            published_at: "2026-03-20".to_owned(),
            updated_at: None,
            tags: vec!["rust".to_owned()],
            summary: "summary".to_owned(),
            hero_image: None,
            toc: false,
            math: true,
            summary_ai: None,
            charts: vec![RenderedChartView {
                chart_type: "line".to_owned(),
                source: "/assets/posts/sample/metrics.csv".to_owned(),
                x: "step".to_owned(),
                y: "ms".to_owned(),
                title: Some("Latency".to_owned()),
                caption: Some("caption".to_owned()),
                points: vec![
                    ChartPointView {
                        x: "bootstrap".to_owned(),
                        y: 38.0,
                    },
                    ChartPointView {
                        x: "api".to_owned(),
                        y: 24.0,
                    },
                ],
                table_headers: vec!["step".to_owned(), "ms".to_owned()],
                table_rows: vec![
                    vec!["bootstrap".to_owned(), "38".to_owned()],
                    vec!["api".to_owned(), "24".to_owned()],
                ],
            }],
            toc_items: Vec::new(),
            body_html:
                "<p><span class=\"math-inline\">\\(x^2\\)</span></p><div class=\"math-display\">\\[x+y\\]</div><pre class=\"mermaid\">flowchart LR\nA --&gt; B</pre>"
                    .to_owned(),
        }
    }

    #[test]
    fn rendered_post_page_includes_math_assets_and_markers() {
        let html = render_post_page(sample_post_view());

        assert!(html.contains("katex.min.css"));
        assert!(html.contains("mermaid.esm.min.mjs"));
        assert!(html.contains("math-inline"));
        assert!(html.contains("math-display"));
        assert!(html.contains("pre.mermaid"));
        assert!(html.contains("renderMathInElement"));
        assert!(html.contains("<table class=\"chart-table\">"));
    }

    #[test]
    fn rendered_tag_pages_include_expected_links() {
        let tag_html = render_tags_page(vec![TagLinkView {
            tag: "rust".to_owned(),
            href: "/tags/rust/".to_owned(),
            count: 2,
        }]);
        let posts_html = render_tag_posts_page(
            "rust",
            vec![PostSummaryView {
                title: "Sample".to_owned(),
                slug: "sample".to_owned(),
                published_at: "2026-03-20".to_owned(),
                updated_at: None,
                tags: vec!["rust".to_owned()],
                summary: "summary".to_owned(),
                hero_image: None,
                toc: false,
                math: false,
            }],
        );

        assert!(tag_html.contains("/tags/rust/"));
        assert!(posts_html.contains("Posts tagged rust"));
        assert!(posts_html.contains("/p/sample"));
    }
}
