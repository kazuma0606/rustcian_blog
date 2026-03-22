# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Commands

```bash
# Development
docker compose up -d                                    # Start Azurite (required for azurite storage backend)
cargo run -p rustacian_blog_backend                     # Run server (default: auth disabled)
ADMIN_AUTH_MODE=local-dev cargo run -p rustacian_blog_backend  # Run with local-dev auth

# Static site generation
cargo run -p rustacian_blog_backend -- generate-static  # Generate to ./dist (local storage)
cargo run -p rustacian_blog_backend -- publish-static   # Publish to Azurite blob
cargo run -p rustacian_blog_backend -- rebuild-static   # Regenerate without fetch

# Checks (same as CI)
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test

# Single test
cargo test -p rustacian_blog_frontend <test_name>
```

## Architecture

This is a **Rust workspace** with three crates following a clean architecture pattern:

- **`application/core`** — Domain layer. No web framework dependencies. Contains:
  - `domain/` — Entities (`Post`, `PostMetadata`), repository traits (`PostRepository`), service traits (`AdminAuthService`, `AiMetadataGenerator`, `StaticSiteGenerator`)
  - `application/usecase.rs` — Use cases: `ListPostsUseCase`, `GetPostUseCase`, `GenerateAiMetadataUseCase`, `PublishStaticSiteUseCase`

- **`application/backend`** — Actix Web server. Contains:
  - `config.rs` — `AppConfig` loaded from environment variables via `dotenvy`
  - `state.rs` — `AppState` DI container wiring use cases + adapters
  - `presentation.rs` — HTTP route handlers
  - `storage.rs` — `PostRepository` implementations (local filesystem + Azurite Table Storage)
  - `static_site.rs` — Static site generation (HTML, sitemap, RSS, search index)
  - `auth.rs` — Admin auth: `disabled` | `local-dev` | `entra` (Entra ID)
  - `ai.rs` — Azure OpenAI adapter for metadata generation
  - `blob.rs` — Azure Blob Storage adapter for static publishing

- **`application/frontend`** — Leptos SSR rendering. Contains pure render functions (no server state):
  - `render_posts_page()`, `render_post_page()`, `render_tags_page()`, `render_tag_posts_page()`
  - Custom SVG chart rendering (bar, scatter, line) from inline CSV in markdown
  - KaTeX math and Mermaid diagram support via CDN markers in output HTML

## Content Model

Blog content lives in `content/` (Git-managed):
```
content/posts/<slug>/
  post.md       # Markdown body with optional frontmatter
  meta.yml      # PostMetadata: title, date, status, tags, description
content/images/ # Shared images
content/metadata/ # Supplemental JSON metadata
```

`status: published` is required for inclusion in static output. `status: draft` posts are preview-only via authenticated admin.

## Static Output (`dist/`)

Generated artifacts:
- `index.html`, `posts/<slug>/index.html`
- `tags/index.html`, `tags/<tag>/index.html`
- `search.json`, `sitemap.xml`, `rss.xml`
- `_meta/build.json`
- `images/`, `assets/posts/<slug>/`

## Key Environment Variables

| Variable | Values / Notes |
|---|---|
| `STORAGE_BACKEND` | `local` (default) \| `azurite` |
| `ADMIN_AUTH_MODE` | `disabled` \| `local-dev` \| `entra` |
| `CONTENT_ROOT` | Path to content directory (default: `./content`) |
| `APP_HOST` / `APP_PORT` | Server bind address |
| `AZURITE_BLOB_ENDPOINT` | `http://127.0.0.1:10000/devstoreaccount1` |
| `AZURITE_TABLE_ENDPOINT` | `http://127.0.0.1:10002/devstoreaccount1` |
| `STATIC_PUBLISH_PREFIX` | Blob prefix for published static files |
| `BASE_URL` | Used for sitemap/RSS absolute URLs |

See `.env.local.example` for full reference.

## CI

CI runs on GitHub Actions (`ci.yml`): starts Azurite, then runs `fmt --check`, `clippy -D warnings`, `cargo test`. The `static-site.yml` workflow generates the static site and uploads `dist/` as an artifact. Security scanning via `security.yml` (gitleaks + cargo audit).
