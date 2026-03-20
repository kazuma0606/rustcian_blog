# rustacian_blog

Pure Rust blog application playground.

This repository currently includes:
- `v1`: local blog PoC
- `v1.5`: CI and security baseline
- `v2`: content operations, math, charts, AI assist, and admin preview PoC
- `v3`: static public site + dynamic admin preparation for Azure migration

## Features
- Rust workspace layout
- `core / backend / frontend` separation
- Actix Web + Leptos based rendering
- Git-managed content with `post.md` + `meta.yml`
- Static public output generation
- Dynamic `/admin` preview / AI / regenerate routes
- Azurite-backed local Blob workflow
- Markdown features for math, charts, tables, SVG/JPG/PNG images, and Mermaid
- GitHub Actions for CI and static build workflow

## Workspace
```text
application/
|-- core
|-- backend
`-- frontend
```

- `application/core`: domain, use cases, repository traits
- `application/backend`: Actix Web, local/Azurite adapters, admin routes, static generation
- `application/frontend`: render functions and view models

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
- `http://127.0.0.1:8080/p/actix-and-leptos`
- `http://127.0.0.1:8080/admin`
- `http://127.0.0.1:8080/health`

## Static Build
Generate local static output:

```powershell
$env:STORAGE_BACKEND="local"
$env:STATIC_PUBLISH_BACKEND="local"
$env:BASE_URL="https://example.com"
cargo run -p rustacian_blog_backend -- publish-static
```

Useful aliases:

```powershell
cargo run -p rustacian_blog_backend -- generate-static
cargo run -p rustacian_blog_backend -- rebuild-static
```

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

## CI and Static Workflow
GitHub Actions runs:

- `CI`
  - `docker compose config`
  - `cargo fmt --all --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test`
- `Static Site`
  - builds static output on `main` / `master`
  - uploads `dist` as an artifact
  - supports `workflow_dispatch` with `site_ref` for rollback-style rebuilds
- `Security`
  - `gitleaks`
  - `cargo audit`

## Docs
- [Root plan](./plan.md)
- [v2 plan](./v2/plan.md)
- [v2 tasks](./v2/tasks.md)
- [v2 spec](./v2/spec.md)
- [v3 plan](./v3/plan.md)
- [v3 tasks](./v3/tasks.md)
- [v3 spec](./v3/spec.md)
- [v3 Azure boundaries](./v3/azure-boundaries.md)

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
- Public output is intended to be static.
- Admin operations stay under `/admin`.
- `v3` keeps Git as the source of truth and treats Azure services as replaceable adapters.
