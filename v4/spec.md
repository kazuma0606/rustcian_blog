# v4 仕様書: フルスタック化 & Azure デプロイ前仕上げ

## 概要

v4 は v3.5 で確立した静的サイト生成パイプラインの上に、以下の機能を追加する。

| フェーズ | 機能 |
|---------|------|
| Phase 1 | Slack 通知基盤 |
| Phase 2 | コメント & お問い合わせフォーム |
| Phase 3 | Tantivy 全文検索 |
| Phase 4 | Application Insights 監視 |
| Phase 5 | Terraform IaC |
| Phase 6 | Admin UI Leptos SSR |

---

## Phase 1 — Slack 通知基盤

### ドメイン (`application/core/src/domain/notification.rs`)

```rust
pub trait NotificationSink: Send + Sync {
    async fn notify(&self, event: NotificationEvent);
}

pub enum NotificationEvent {
    StaticSiteRebuilt { pages: usize, assets: usize },
    CommentReceived { post_slug: String, author: String },
    ContactFormSubmitted { from_name: String },
    AiMetadataGenerated { slug: String },
}
```

### アダプター (`application/backend/src/notification.rs`)

- `NoopNotificationSink` — テスト・ローカル開発用（副作用なし）
- `SlackNotificationSink` — `SLACK_WEBHOOK_URL` へ `reqwest` で POST
- `build_notification_sink()` — `SLACK_WEBHOOK_URL` 未設定時は Noop を返す

### フック済みルート

| ルート | イベント |
|--------|---------|
| `POST /admin/static/regenerate` | `StaticSiteRebuilt` |
| `POST /admin/ai/{slug}/metadata` | `AiMetadataGenerated` |
| `POST /posts/{slug}/comments` | `CommentReceived` |
| `POST /contact` | `ContactFormSubmitted` |

---

## Phase 2 — コメント & お問い合わせフォーム

### エンティティ (`application/core/src/domain/comment.rs`)

- `Comment` — `id / post_slug / author_name / content / created_at / status`
- `CommentStatus` — `Pending | Approved | Rejected`
- `ContactMessage` — `id / from_name / from_email / body / created_at`

### リポジトリトレイト

- `CommentRepository` — `create_comment / list_comments / list_all_pending / update_status`
- `ContactRepository` — `create_contact_message`

### ストレージ実装 (`application/backend/src/comment_store.rs`)

- `InMemoryCommentRepository` / `InMemoryContactRepository` — テスト用
- `AzuriteCommentRepository` / `AzuriteContactRepository` — Azure Table Storage (REST)

### XSS 対策

`ammonia` クレートを使用し、コメント・お問い合わせ投稿時にすべての HTML タグを除去。

### 公開ルート

| メソッド | パス | 説明 |
|---------|------|------|
| GET | `/posts/{slug}/comments` | 承認済みコメント一覧 (SSR) |
| POST | `/posts/{slug}/comments` | コメント投稿 (サニタイズ → Pending) |
| GET | `/contact` | お問い合わせフォーム (SSR) |
| POST | `/contact` | お問い合わせ送信 |

### Admin ルート (要認証)

| メソッド | パス | 説明 |
|---------|------|------|
| GET | `/admin/comments` | 承認待ちコメント一覧 |
| POST | `/admin/comments/{id}/approve` | 承認 |
| POST | `/admin/comments/{id}/reject` | 却下 |

---

## Phase 3 — Tantivy 全文検索

### 実装 (`application/backend/src/search.rs`)

`TantivySearchIndex` (インメモリ、JS なし):

- **スキーマ**: `slug`(STRING|STORED), `title`(TEXT|STORED), `body_text`(TEXT|STORED), `tags`(TEXT|STORED), `date`(STRING|STORED)
- **`rebuild(&[Post])`**: Published 記事のみインデックス。`ammonia` で HTML タグ除去後にテキスト化。`RwLock<Option<SearchInner>>` でアトミック切り替え。
- **`search(query_str, limit)`**: `QueryParser`（title + body_text + tags、AND デフォルト）

### タイミング

- サーバー起動時: `CONTENT_ROOT` から全 published 記事をインデックス
- `POST /admin/static/regenerate` 完了後: 再インデックス

### ルート

| メソッド | パス | 説明 |
|---------|------|------|
| GET | `/search?q=<query>` | 全文検索結果ページ (SSR) |

`q` パラメータ省略時は空結果を返す（400 にならない）。

---

## Phase 4 — Application Insights

### 実装 (`application/backend/src/observability.rs`)

```rust
pub trait ObservabilitySink: Send + Sync {
    fn emit(&self, event: AppEvent);
}
```

- `NoopObservabilitySink` — 副作用なし
- `StdoutObservabilitySink` — JSON を stdout に出力（ローカルデフォルト）
- `ApplicationInsightsObservabilitySink` — Azure Monitor Track API (`/v2/track`) へ fire-and-forget POST

### Connection String 解析

`InstrumentationKey=...;IngestionEndpoint=https://...;` 形式を `;` 分割でパース。`IngestionEndpoint` 省略時は `https://dc.services.visualstudio.com/` を使用。

### イベントマッピング (AppEvent → EventData)

| AppEvent バリアント | Application Insights name |
|--------------------|--------------------------|
| `PublicRequestServed` | `public_request_served` |
| `AdminAuthChecked` | `admin_auth_checked` |
| `AiMetadataGenerated` | `ai_metadata_generated` |
| `StaticSitePublished` | `static_site_published` |
| `ContentError` | `content_error` |

### 環境変数

| 変数 | 値 |
|-----|----|
| `OBSERVABILITY_BACKEND` | `stdout` (デフォルト) \| `appinsights` \| `noop` |
| `APPLICATIONINSIGHTS_CONNECTION_STRING` | `InstrumentationKey=...;IngestionEndpoint=...;` |

---

## Phase 5 — Terraform IaC

### ディレクトリ構成

```
terraform/
  main.tf           # provider, resource group, module calls, RBAC assignments
  variables.tf      # prefix, location, env, sku, container_image, base_url, Entra vars
  outputs.tf        # hostname, principal_id, table_endpoint, kv_uri, ai_cs, openai
  modules/
    storage/        # Storage Account (LRS), comments + contacts Table
    app/            # Service Plan (Linux) + Linux Web App (Docker, SystemAssigned MSI)
    monitoring/     # Log Analytics Workspace + Application Insights
    keyvault/       # Key Vault (RBAC), 4 secrets
    openai/         # Cognitive Account (OpenAI S0) + gpt-4o-mini deployment
    comms/          # Azure Communication Services (将来の Email 用)
```

### 主要設計ポイント

- **RBAC 認可**: Key Vault は `enable_rbac_authorization = true`。App MSI に `Key Vault Secrets User` ロールを root `main.tf` で付与。
- **App Settings の Key Vault 参照**: `@Microsoft.KeyVault(SecretUri=...)` 形式。
- **循環依存回避**: `azurerm_role_assignment` を root `main.tf` に配置することで `module.keyvault` と `module.app` の相互依存を排除。
- **手動シークレット**: `SLACK_WEBHOOK_URL` / `AZURE_OPENAI_API_KEY` / `ADMIN_PASSWORD` は `lifecycle { ignore_changes = [value] }` でポータル設定を保持。

詳細: `v4/azure-boundaries.md`

---

## Phase 6 — Admin UI Leptos SSR

### フロントエンド関数 (`application/frontend/src/lib.rs`)

| 関数 | 説明 |
|------|------|
| `render_admin_dashboard(posts)` | 記事一覧テーブル (status 列含む) |
| `render_admin_post_detail(post, metadata)` | 記事詳細 + AI メタデータ表示 |
| `render_admin_comments(pending)` | 承認待ちコメント一覧 + 操作ボタン |
| `render_admin_static_panel()` | 静的サイト再生成ボタン |

スタイル: `ADMIN_CSS` 定数 (warm beige/brown スキーム)。`admin_document()` / `admin_nav()` ヘルパーで共通レイアウト。

### Admin ルート (更新)

| メソッド | パス | 説明 |
|---------|------|------|
| GET | `/admin` | `render_admin_dashboard()` |
| GET | `/admin/posts/{slug}` | `render_admin_post_detail()` + metadata JSON 読込 |
| GET | `/admin/comments` | `render_admin_comments()` |
| GET | `/admin/static` | `render_admin_static_panel()` |

### `load_generated_metadata()` ヘルパー

`content/metadata/<slug>.json` を `std::fs::read` で同期読み込みし、`GeneratedMetadataView` にデシリアライズ。ファイル不在時は `None`。
