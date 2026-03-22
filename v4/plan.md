# v4 実装計画 — フルスタック化 & Azure デプロイ前仕上げ

## 背景

v3 で静的サイト生成・管理者向け動的ルート・Entra ID 認証・Azure OpenAI 連携・Azurite Blob 統合が完成した。
v4 では Azure デプロイ前にフルスタックアプリケーションとして必要な機能を追加する。

---

## アーキテクチャ上の共通方針

既存の `ObservabilitySink` と同じパターンを全機能で統一する。

```
core に trait + イベント/エンティティ定義
backend に adapter 実装（local / Azure 両対応）
frontend に SSR render 関数追加（JS 不使用）
```

---

## Phase 1 — Slack 通知基盤

### 概要

push 型通知をすべて Slack Incoming Webhooks で実現する。
将来の Azure Communication Services Email への移行を見据え、`NotificationSink` トレイトで抽象化する。

### 新規ファイル

```
application/core/src/domain/notification.rs
application/backend/src/notification.rs
```

### NotificationEvent（core）

```rust
pub enum NotificationEvent {
    PostPublished { slug: String, title: String },
    StaticSiteRebuilt { page_count: usize, outcome: String },
    CommentReceived { slug: String, author_name: String },
    ContactFormSubmitted { from_name: String },
    AiMetadataGenerated { slug: String, outcome: String },
}
```

### アダプター（backend）

- `SlackNotificationSink` — Incoming Webhooks API へ POST（`reqwest`）
- `NoopNotificationSink` — ローカル開発・テスト用

### 設定

| 環境変数 | 説明 |
|---|---|
| `SLACK_WEBHOOK_URL` | 未設定時は Noop にフォールバック |

> Webhook URL はコード・git に含めず、env var / Azure Key Vault に格納すること。

### 通知を追加するルート

- `POST /admin/static/regenerate` → `StaticSiteRebuilt`
- `POST /admin/ai/{slug}/metadata` → `AiMetadataGenerated`
- コメント投稿受信 → `CommentReceived`（Phase 2 で追加）
- お問い合わせ受信 → `ContactFormSubmitted`（Phase 2 で追加）

---

## Phase 2 — コメント & お問い合わせフォーム

### 概要

公開向けコメント投稿・管理者向けモデレーション・お問い合わせフォームを実装する。
XSS / インジェクション対策として `ammonia` クレートによる HTML サニタイズを必須とし、テストをセットで実装する。

### 新規ファイル

```
application/core/src/domain/comment.rs
application/backend/src/comment_store.rs
```

### エンティティ（core）

```rust
pub struct Comment {
    pub id: String,
    pub post_slug: String,
    pub author_name: String,
    pub content: String,          // ammonia でサニタイズ済み
    pub created_at: DateTime<Utc>,
    pub status: CommentStatus,    // Pending | Approved | Rejected
}

pub struct ContactMessage {
    pub id: String,
    pub from_name: String,
    pub from_email: String,
    pub body: String,             // ammonia でサニタイズ済み
    pub created_at: DateTime<Utc>,
}
```

### リポジトリトレイト（core）

```rust
pub trait CommentRepository: Send + Sync {
    async fn create_comment(&self, comment: &Comment) -> Result<(), BlogError>;
    async fn list_comments(&self, slug: &str, include_pending: bool) -> Result<Vec<Comment>, BlogError>;
    async fn update_status(&self, id: &str, status: CommentStatus) -> Result<(), BlogError>;
}

pub trait ContactRepository: Send + Sync {
    async fn create_contact_message(&self, msg: &ContactMessage) -> Result<(), BlogError>;
}
```

### ストレージ実装（backend）

Azure Table Storage / Azurite Table Storage への実装。

### 追加ルート

```
POST /posts/{slug}/comments           ← コメント投稿（ammonia サニタイズ後保存）
GET  /posts/{slug}/comments           ← 承認済みコメント一覧（SSR 埋め込み）
POST /contact                         ← お問い合わせ投稿
GET  /admin/comments                  ← モデレーションキュー
POST /admin/comments/{id}/approve
POST /admin/comments/{id}/reject
```

### テスト方針

- XSS ペイロードがサニタイズされることを確認するユニットテスト
- SQL インジェクション相当のペイロードテスト
- モデレーションステータス遷移テスト

---

## Phase 3 — Tantivy 全文検索（Pure Rust）

### 概要

JS を使わずサーバーサイドで全文検索を実現する。
Tantivy でインデックスをインメモリ構築し、クエリ結果を Leptos SSR でレンダリングする。

### 新規ファイル

```
application/core/src/domain/search.rs
application/backend/src/search.rs
```

### インデックススキーマ

| フィールド | 型 | 備考 |
|---|---|---|
| `slug` | TEXT（stored） | |
| `title` | TEXT（indexed, stored） | |
| `body_text` | TEXT（indexed） | HTML タグ除去済みプレーンテキスト |
| `tags` | TEXT（indexed, stored） | |
| `date` | DATE（stored） | |

### 構築タイミング

- サーバー起動時にインメモリ構築
- `/admin/static/regenerate` 完了後に再構築

### 追加ルート

```
GET /search?q=<query>   ← SSR で検索結果ページ返却
```

### フロントエンド追加

```rust
// application/frontend/src/lib.rs
pub fn render_search_page(query: &str, results: Vec<SearchResult>) -> String { ... }
```

---

## Phase 4 — Application Insights

### 概要

既存の `ObservabilitySink` トレイトに `ApplicationInsightsObservabilitySink` を実装する。
`APPLICATIONINSIGHTS_CONNECTION_STRING`（config に既定義）を使用。

### 実装方針

OpenTelemetry より Track REST API 直接呼び出しの方がクレート依存を最小化できるため採用。

### 設定

| 環境変数 | 説明 |
|---|---|
| `APPLICATIONINSIGHTS_CONNECTION_STRING` | config.rs に定義済み・未使用 |
| `OBSERVABILITY_BACKEND` | `stdout` \| `noop` \| `appinsights` |

---

## Phase 5 — Terraform IaC

### 概要

Azure デプロイに必要なリソースを Terraform で定義する。
まず論理的に必要なリソースを定義し、実際の値は variables + Key Vault 参照で管理する。

### ディレクトリ構成

```
terraform/
├── main.tf
├── variables.tf
├── outputs.tf
└── modules/
    ├── app/          ← App Service Plan + Linux Web App（backend）
    ├── storage/      ← Storage Account + Blob containers + Table
    ├── openai/       ← Azure OpenAI リソース
    ├── monitoring/   ← Application Insights + Log Analytics Workspace
    ├── keyvault/     ← Key Vault + シークレット参照
    └── comms/        ← Communication Services（将来の Email 通知用）
```

### 主要リソース

| リソース | 用途 |
|---|---|
| `azurerm_resource_group` | 全リソースのコンテナ |
| `azurerm_storage_account` | Blob（静的アセット）+ Table（コメント等） |
| `azurerm_linux_web_app` | backend コンテナ実行 |
| `azurerm_static_web_app` | 静的サイト配信 |
| `azurerm_cognitive_account` | Azure OpenAI |
| `azurerm_application_insights` | 監視 |
| `azurerm_log_analytics_workspace` | ログ集約 |
| `azurerm_key_vault` | シークレット管理 |
| `azurerm_communication_service` | 将来の Email 通知 |

---

## Phase 6 — Admin UI（Leptos SSR）

### 概要

現在の `/admin` は簡素な HTML のみ。
Leptos SSR コンポーネントで作り込み、記事ページと同じ warm beige/brown カラースキーム・フォントで統一する。

### 追加 render 関数（frontend）

```rust
pub fn render_admin_dashboard(posts: Vec<PostSummary>, build_info: Option<BuildInfo>) -> String
pub fn render_admin_post_detail(post: Post, generated_metadata: Option<GeneratedMetadata>) -> String
pub fn render_admin_comments(pending: Vec<Comment>, approved: Vec<Comment>) -> String
pub fn render_admin_static_panel(last_build: Option<BuildInfo>) -> String
```

### ページ構成

| URL | 内容 |
|---|---|
| `/admin` | 投稿一覧（published/draft バッジ）+ クイックアクション |
| `/admin/posts/{slug}` | 投稿詳細・AI メタデータ生成ボタン・結果確認 |
| `/admin/comments` | コメントモデレーションキュー |
| `/admin/static` | 再生成ボタン + 最終ビルド情報 |

---

## 実装順序

```
Phase 1 → Phase 2 → Phase 3 → Phase 4 → Phase 5 → Phase 6
（通知基盤）  （対話）    （検索）    （監視）    （IaC）    （Admin UI）
```

Phase 1 を先行する理由: Phase 2 以降のイベントフックが通知基盤に依存するため。

---

## 技術スタック追加分

| 用途 | クレート / サービス |
|---|---|
| Slack 通知 | `reqwest`（既存）|
| HTML サニタイズ | `ammonia` |
| 全文検索 | `tantivy` |
| Application Insights | Azure Monitor Track API（REST 直接）|
| IaC | Terraform + `azurerm` provider |
