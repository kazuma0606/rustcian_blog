# Azure Migration Boundaries (v3)

## Goal
Keep Git-managed content and Rust domain logic intact while making Azure services swappable infrastructure.

## Source of Truth
- `content/posts/<slug>/post.md`
- `content/posts/<slug>/meta.yml`
- `content/posts/<slug>/*`
- `content/images/*`
- `content/metadata/<slug>.json`

Git remains the source of truth. Azure is used for delivery, auth, AI, and observability.

## Public vs Admin

### Public
- Generated static HTML
- Static assets and article assets
- Read-only delivery
- `published` only

### Admin
- Entra ID protected routes
- Draft preview
- AI metadata generation
- Static regenerate / publish actions

## Adapter Boundaries
- `PostRepository`
  - local filesystem or Blob-backed content reads
- `AssetStore`
  - local assets or Azure-backed asset access
- `GeneratedMetadataStore`
  - local `content/metadata` or Blob-backed generated metadata
- `AdminAuthService`
  - PoC claims mode or Entra OIDC verification
- `AiMetadataGenerator`
  - Azure OpenAI adapter
- `StaticSitePublisher`
  - local `dist/` output or Blob prefix publish
- `ObservabilitySink`
  - stdout today, Application Insights later

## Azure Service Mapping
- Static hosting
  - Azure Static Web Apps or Blob static website
- Auth
  - Microsoft Entra ID
- AI
  - Azure OpenAI
- Artifact storage
  - Blob Storage
- Observability
  - Application Insights

## What Stays Out of Azure Logic
- Domain validation
- Post metadata rules
- Draft / published visibility rules
- TOC / math / chart parsing
- Static output shape

These remain in `application/core` or deterministic build logic so Azure migration does not change behavior.

## Operational Rule
- Local commands and CI use the same backend static publish entrypoint.
- Rollback is done by rebuilding a previous git ref and republishing that artifact.
