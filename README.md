# rustcian_blog

Pure Rust blog application playground.

This repository currently includes:
- `v1`: local blog PoC
- `v1.5`: CI and security baseline
- `v2`: content operations, math, charts, AI assist, and admin preview PoC

## Features
- Rust workspace layout
- `core / backend / frontend` separation
- Actix Web + Leptos SSR
- Markdown + Frontmatter based posts
- Azurite-backed local Blob workflow
- GitHub Actions for CI and security checks

## Workspace
```text
application/
|-- core
|-- backend
`-- frontend
```

- `application/core`: domain, use cases, repository trait
- `application/backend`: Actix Web, content/Azurite adapters, APIs
- `application/frontend`: Leptos SSR views

## Local Run
Start Azurite:

```powershell
docker compose up -d
```

Run the backend:

```powershell
cargo run -p rustacian_blog_backend
```

Open:
- `http://127.0.0.1:8080/`
- `http://127.0.0.1:8080/p/hello-rustacian-blog`
- `http://127.0.0.1:8080/health`

## Local Checks
```powershell
docker compose config
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test
```

## Content
- Posts: `content/posts/<slug>/post.md`
- Metadata: `content/posts/<slug>/meta.yml`
- Supplemental metadata: `content/metadata/<slug>.json`
- Images: `content/images/*`
- Article assets: `content/posts/<slug>/*`

Minimal `meta.yml` example:

```yaml
title: Hello Rustacian Blog
slug: hello-rustacian-blog
published_at: 2026-03-18T09:00:00Z
status: published
tags:
  - rust
summary: sample post
hero_image: /images/ferris-notes.svg
toc: true
math: true
```

## CI and Security
GitHub Actions runs:

- `CI`
  - `docker compose config`
  - `cargo fmt --all --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test`
- `Security`
  - `gitleaks`
  - `cargo audit`

## Docs
- [Root plan](./plan.md)
- [v2 plan](./v2/plan.md)
- [v2 tasks](./v2/tasks.md)
- [v2 spec](./v2/spec.md)

<details>
<summary>Previous Phase Docs</summary>

- [v1 plan](./v1/plan.md)
- [v1 tasks](./v1/tasks.md)
- [v1 spec](./v1/spec.md)
- [v1.5 plan](./v1.5/plan.md)
- [v1.5 tasks](./v1.5/tasks.md)
- [v1.5 spec](./v1.5/spec.md)

</details>

## Notes
- The public app is currently read-only.
- Post updates are Git/Markdown based.
- `v2` uses article directories with `post.md` + `meta.yml`.
- Admin preview is split under `/admin/preview/{slug}` and uses Entra ID PoC mode when enabled.
