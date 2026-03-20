# Rustacian Blog Project Plan (v3)

## 1. Positioning
- v3 is the pre-Azure migration phase.
- Keep the validated v2 content model, but separate public static delivery from dynamic admin operations.
- Add adapter boundaries so Azure services can be adopted by swapping infrastructure implementations.

## 2. Direction
- Public site: generated static HTML and assets
- Admin site: authenticated dynamic app for preview and content operations
- Source of truth: Git-managed files under `content/`
- Shared business rules: `application/core`

## 3. Architecture

### 3.1 Public
- Build static HTML from `post.md`, `meta.yml`, charts, and assets.
- Publish only `published` posts.
- Fail the build on validation errors.

### 3.2 Admin
- Protect `/admin` with Entra ID.
- Keep draft preview and AI metadata generation inside admin routes only.
- Prepare future publish / rebuild / rollback operations here.

### 3.3 Shared Core
- Keep `Post`, `PostMetadata`, `PostVisibility`, validation, and generation use cases framework-independent.
- Use traits for storage, auth, AI, and static publishing boundaries.

## 4. Azure Mapping
- Static public output: Azure Static Web Apps or Blob static hosting
- Admin app: App Service or Container Apps
- Auth: Entra ID
- AI assist: Azure OpenAI
- Assets and generated artifacts: Blob Storage
- Observability: Application Insights

## 5. Blob Placement Rules
- Static output is published under a configurable prefix via `STATIC_PUBLISH_PREFIX`.
- The prefix root contains the same relative paths as local `dist/`.
- Required examples:
  - `<prefix>/index.html`
  - `<prefix>/posts/<slug>/index.html`
  - `<prefix>/tags/index.html`
  - `<prefix>/tags/<tag>/index.html`
  - `<prefix>/search.json`
  - `<prefix>/sitemap.xml`
  - `<prefix>/rss.xml`
  - `<prefix>/images/*`
  - `<prefix>/assets/posts/<slug>/*`
- Build inventory is written to `<prefix>/_meta/build.json`.
- AI supplemental metadata remains separate from static output and lives under `metadata/<slug>.json`.

## 6. Abstraction Boundaries
- `PostRepository`
- `AssetStore`
- `GeneratedMetadataStore`
- `AiMetadataGenerator`
- `AdminAuthService`
- `StaticSiteGenerator`
- `StaticSitePublisher`

## 7. Current v3 Targets
- Complete static build and publish boundaries
- Complete Entra ID OIDC verification path
- Reuse Azure OpenAI from authenticated admin routes
- Define Blob layout and publish inventory
- Add Azure-oriented observability and workflow documentation

## 8. Application Insights Targets
- Public request events for index, post detail, and posts API
- Admin auth outcome events for preview and AI operations
- AI metadata generation success / failure events
- Static publish completion events including target, page count, and asset count
- Content loading error events for validation / parse / storage failures

These are emitted through a backend observability sink now and can later be forwarded to Application Insights without changing route logic.

## 9. Public Optimization Policy
- Minify generated HTML by collapsing inter-tag whitespace during static build.
- Emit compact JSON and XML for feed, sitemap, search, and build inventory payloads.
- Keep images, article assets, and CSV files byte-identical to the Git-managed source.
- Record the applied optimization policy in `_meta/build.json`.

## 10. Workflow
- Local static publish uses `cargo run -p rustacian_blog_backend -- publish-static`.
- `rebuild-static` is an alias of the same publish path and is used when regenerating the current ref.
- GitHub Actions `static-site.yml` builds static output on `main` / `master` pushes and uploads `dist` as an artifact.
- Manual `workflow_dispatch` accepts `site_ref` so a previous commit SHA or tag can be republished as rollback.
- Local and CI both use the same backend static publish entrypoint.

## 11. Out of Scope
- DB as source of truth
- Browser CMS as the main authoring workflow
- Real-time AI generation on public requests
