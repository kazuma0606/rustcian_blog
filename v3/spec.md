# Rustacian Blog Specification (v3)

## 1. Overview
- Public site is generated as static output from Git-managed content.
- Admin features remain dynamic and authenticated.
- `application/core` owns validation and generation boundaries.
- `v3` is the Azure migration preparation phase, not the final Azure-specific implementation.

## 2. Content Inputs
- `content/posts/<slug>/post.md`
- `content/posts/<slug>/meta.yml`
- `content/posts/<slug>/*` article assets
- `content/images/*`
- `content/metadata/<slug>.json`

## 3. Supported Content Features
- Markdown headings with generated TOC anchors
- Math blocks and inline math rendered with KaTeX when required
- CSV-backed charts rendered as SVG plus HTML tables
- Mermaid fenced code blocks rendered as diagrams
- Draft and published visibility control via `meta.yml`

## 4. Static Output
- `index.html`
- `posts/<slug>/index.html`
- `tags/index.html`
- `tags/<tag>/index.html`
- `search.json`
- `sitemap.xml`
- `rss.xml`
- `_meta/build.json`
- `images/*`
- `assets/posts/<slug>/*`

## 5. Generation Rules
- Only `published` posts are included in public output.
- Validation errors fail the build.
- Markdown, TOC, math, chart SVG, chart table, and Mermaid block transforms are resolved during build.
- `draft` content remains available only through authenticated admin preview.

## 6. Public Site
- Read-only static HTML.
- No admin routes or preview behavior in public output.
- Images and article assets are referenced with static paths.
- Mermaid blocks are emitted as Mermaid containers and rendered client-side only for pages that contain them.
- Output optimization policy:
  - HTML: preserve valid rendered HTML without structural minification
  - JSON / XML: emit compact single-line payloads
  - Images / article assets / CSV: copy as-is without transformation
  - Build inventory records the active optimization strategy in `_meta/build.json`

## 7. Admin Site
- `/admin` remains dynamic.
- Draft preview uses authenticated access.
- AI metadata generation remains an admin-side operation.
- Static regenerate is triggered from admin and uses the same publish pipeline as local / CI commands.

## 8. Azure Mapping
- Public output: Azure static hosting
- Admin app: App Service or Container Apps
- Auth: Entra ID
- AI assist: Azure OpenAI
- Assets / generated artifacts: Blob Storage
- Observability: Application Insights

## 9. Blob Layout
- Static publish uses a configurable prefix via `STATIC_PUBLISH_PREFIX`.
- Blob output keeps the same relative paths as local `dist/`.
- Example layout:
  - `<prefix>/index.html`
  - `<prefix>/posts/<slug>/index.html`
  - `<prefix>/tags/index.html`
  - `<prefix>/tags/<tag>/index.html`
  - `<prefix>/search.json`
  - `<prefix>/sitemap.xml`
  - `<prefix>/rss.xml`
  - `<prefix>/images/*`
  - `<prefix>/assets/posts/<slug>/*`
  - `<prefix>/_meta/build.json`
- `_meta/build.json` contains a machine-readable inventory of generated pages and assets.
- AI supplemental metadata is stored separately as `metadata/<slug>.json`.

## 10. Asset Delivery Rules
- Public `/images/*` requests are served by backend-controlled image delivery.
- Local mode reads from `content/images`.
- Azurite mode reads from blob `images/*`.
- Article-local assets are served from `assets/posts/<slug>/*`.

## 11. Observability Targets
- `OBSERVABILITY_BACKEND=stdout|noop` controls the active sink today.
- The future Application Insights adapter should preserve the same event taxonomy.
- Current event categories:
  - public request served
  - admin auth checked
  - AI metadata generated
  - static site published
  - content error
- `APPLICATIONINSIGHTS_CONNECTION_STRING` is reserved for the Azure adapter.

## 12. Workflow
- Local publish commands:
  - `cargo run -p rustacian_blog_backend -- publish-static`
  - `cargo run -p rustacian_blog_backend -- rebuild-static`
- `generate-static`, `publish-static`, `rebuild-static` are currently aliases of the same static publish flow.
- CI publish flow lives in `.github/workflows/static-site.yml`.
- Push to `main` / `master` builds static output and uploads `dist` as an artifact.
- Manual `workflow_dispatch` accepts:
  - `site_ref`: target branch, tag, or commit SHA
  - `base_url`: sitemap / RSS base URL
- Rollback policy:
  - rerun the static-site workflow with a previous `site_ref`
  - publish the artifact generated from that older ref
