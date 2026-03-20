# Repository Guidelines

## Project Structure & Module Organization
This repository is a Rust workspace. Core application code lives under `application/`:
- `application/core`: domain models, use cases, and repository traits
- `application/backend`: Actix Web server, Azurite/content adapters, API routes
- `application/frontend`: Leptos SSR views

Content assets live in `content/`:
- `content/posts/*.md`: Markdown posts with YAML frontmatter
- `content/images/*`: static images served by the backend

Planning and specs are tracked in `v1/`, `v1.5/`, and `v2/`. GitHub workflow files are in `.github/workflows/`.

## Build, Test, and Development Commands
- `docker compose up -d`: start Azurite for local storage tests
- `cargo run -p rustacian_blog_backend`: run the blog locally on `127.0.0.1:8080`
- `cargo fmt --all --check`: verify formatting
- `cargo clippy --workspace --all-targets -- -D warnings`: enforce lint-clean code
- `cargo test`: run unit and integration tests
- `docker compose config`: validate Compose config before pushing

## Coding Style & Naming Conventions
Use Rust 2024 edition defaults. Format with `cargo fmt`; do not hand-format around it. Keep modules small and layered by responsibility. Prefer `snake_case` for files, modules, and functions; `PascalCase` for types; `SCREAMING_SNAKE_CASE` for constants. Keep public APIs explicit and avoid leaking framework concerns into `application/core`.

## Testing Guidelines
Tests are Rust `#[test]` and `#[tokio::test]` tests colocated with source. Name tests by behavior, for example `repository_lists_posts_from_content_directory`. Set `RUN_AZURITE_TESTS=1` when you want Azurite-backed tests to fail hard instead of skipping when Azurite is unavailable.

## Commit & Pull Request Guidelines
Recent commits use short imperative messages, for example `Add README and ignore local captures` and `Run GitHub Actions on master pushes`. Follow that pattern. Prefer feature branches and pull requests over direct pushes to `master`. PRs should include:
- a short summary of what changed
- any config or workflow impact
- validation performed (`fmt`, `clippy`, `test`, screenshots if UI changed)

## Security & Configuration Tips
Do not commit `.env.local`, secrets, certificates, or local logs. Keep only `.env.local.example` in source control. `gitleaks` and `cargo audit` run in GitHub Actions; treat their failures as release blockers.
