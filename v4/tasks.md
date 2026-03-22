# Phase 4 Tasks: フルスタック化 & Azure デプロイ前仕上げ

## 1. Goal
- Slack push 通知による運用イベント可視化
- コメント・お問い合わせによる読者インタラクション実現
- Pure Rust (Tantivy) による JS レス全文検索
- Application Insights による本番監視基盤
- Terraform による Azure リソース定義
- Admin UI の作り込みと記事ページとのデザイン統一

## 2. Scope
- 対象: 通知基盤、インタラクション機能、検索、監視、IaC、Admin UI
- 対象: XSS / インジェクション対策とセキュリティテスト
- 非対象: Azure Communication Services Email（将来フェーズ）
- 非対象: マルチユーザー・CMS 機能

---

## 3. Tasks

### 3.1 Planning
- [x] `v4/plan.md` を作成する
- [x] `v4/tasks.md` を作成する
- [x] v3.5 との整合性チェックを実施し plan/tasks を修正する

### 3.2 事前整理（v3.5 対応）

- [ ] `dist/` を main repo から削除して `.gitignore` に追加する
  - v3.5 以降 main repo では生成しないため不要
- [ ] Dependabot PR #10（jsonwebtoken）をクローズする（v3.0_dev で取り込み済み）

---

### 3.3 Phase 1 — Slack 通知基盤

#### 3.3.1 Core ドメイン
- [x] `application/core/src/domain/notification.rs` を新規作成する
- [x] `NotificationSink` トレイトを定義する（`async fn notify(&self, event: NotificationEvent)`）
- [x] `NotificationEvent` enum を定義する
  - `StaticSiteRebuilt` / `CommentReceived` / `ContactFormSubmitted` / `AiMetadataGenerated`
  - ※ `PostPublished` は backend 対象外（content repo の CI で担当）
- [x] `domain/mod.rs` に `notification` モジュールを追加する

#### 3.3.2 Backend アダプター
- [x] `application/backend/src/notification.rs` を新規作成する
- [x] `NoopNotificationSink` を実装する（テスト・ローカル開発用）
- [x] `SlackNotificationSink` を実装する（Incoming Webhooks API へ `reqwest` で POST）
- [x] メッセージフォーマットを定義する（イベント種別ごとの Slack メッセージ文言）
- [x] `build_notification_sink()` ファクトリ関数を実装する（`SLACK_WEBHOOK_URL` 未設定時は Noop）

#### 3.3.3 Config
- [x] `config.rs` に `slack_webhook_url: Option<String>` を追加する
- [x] `.env.local.example` に `SLACK_WEBHOOK_URL=` を追記する

#### 3.3.4 AppState への組み込み
- [x] `state.rs` に `notification: Arc<dyn NotificationSink>` を追加する
- [x] `main.rs` でファクトリ呼び出しと DI を行う

#### 3.3.5 既存ルートへのフック
- [x] `POST /admin/static/regenerate` 完了後に `StaticSiteRebuilt` を emit する
- [x] `POST /admin/ai/{slug}/metadata` 完了後に `AiMetadataGenerated` を emit する

#### 3.3.6 content repo への PostPublished 通知追加
- [x] `rustcian_blog_content/.github/workflows/build.yml` に Slack 通知ステップを追加する
  - ビルド成功時に `SLACK_WEBHOOK_URL` へ curl で POST
- [x] `SLACK_WEBHOOK_URL` を content repo の Secrets に登録する

#### 3.3.7 テスト
- [x] `NoopNotificationSink` で既存テストが引き続き通ることを確認する（61 tests pass）
- [x] `SlackNotificationSink` のメッセージ組み立てのユニットテストを追加する

---

### 3.4 Phase 2 — コメント & お問い合わせフォーム

#### 3.4.1 Core ドメイン
- [x] `application/core/src/domain/comment.rs` を新規作成する
- [x] `Comment` エンティティを定義する（`id` / `post_slug` / `author_name` / `content` / `created_at` / `status`）
- [x] `CommentStatus` enum を定義する（`Pending` / `Approved` / `Rejected`）
- [x] `ContactMessage` エンティティを定義する（`id` / `from_name` / `from_email` / `body` / `created_at`）
- [x] `CommentRepository` トレイトを定義する（`create_comment` / `list_comments` / `list_all_pending` / `update_status`）
- [x] `ContactRepository` トレイトを定義する（`create_contact_message`）
- [x] `domain/mod.rs` に `comment` モジュールを追加する

#### 3.4.2 サニタイズ
- [x] `application/backend/Cargo.toml` に `ammonia` を追加する
- [x] コメント・お問い合わせ投稿時のサニタイズ処理を実装する（HTML タグ除去）

#### 3.4.3 Backend ストレージ実装
- [x] `application/backend/src/comment_store.rs` を新規作成する
- [x] `InMemoryCommentRepository` / `AzuriteCommentRepository` を実装する（Table Storage / Azurite）
- [x] `InMemoryContactRepository` / `AzuriteContactRepository` を実装する
- [x] `build_comment_repository()` / `build_contact_repository()` ファクトリを実装する
- [x] `application/backend/src/table.rs` を新規作成する（Azurite Table Storage REST クライアント）

#### 3.4.4 ルート追加
- [x] `POST /posts/{slug}/comments` を実装する（サニタイズ → 保存 → `CommentReceived` 通知）
- [x] `GET /posts/{slug}/comments` を実装する（承認済み一覧を SSR 埋め込み）
- [x] `GET /contact` を実装する（お問い合わせフォームページ）
- [x] `POST /contact` を実装する（サニタイズ → 保存 → `ContactFormSubmitted` 通知）
- [x] `GET /admin/comments` を実装する（モデレーションキュー、要認証）
- [x] `POST /admin/comments/{id}/approve` を実装する（要認証）
- [x] `POST /admin/comments/{id}/reject` を実装する（要認証）
- [x] `presentation.rs` のルート設定に追加する

#### 3.4.5 Frontend（SSR）
- [x] コメント一覧の SSR レンダリング関数をフロントエンドクレートに追加する（`render_comment_list`）
- [x] お問い合わせフォームの SSR レンダリング関数を追加する（`render_contact_page`）

#### 3.4.6 テスト
- [x] XSS ペイロード（`<script>alert(1)</script>` 等）がサニタイズされることを確認するテストを追加する
- [x] HTML 属性インジェクション（`<img onerror=...>`）のテストを追加する
- [x] モデレーションステータス遷移（Pending → Approved / Rejected）のテストを追加する（comment_store.rs）
- [x] 承認済みコメントのみ公開 API に返ることを確認するテストを追加する

---

### 3.5 Phase 3 — Tantivy 全文検索

#### 3.5.1 Core ドメイン
- [x] `application/core/src/domain/search.rs` を新規作成する
- [x] `SearchResult` 構造体を定義する（`slug` / `title` / `excerpt` / `tags` / `date`）
- [x] `SearchQuery` 構造体を定義する
- [x] `domain/mod.rs` に `search` モジュールを追加する

#### 3.5.2 Backend 実装
- [x] `application/backend/Cargo.toml` に `tantivy` を追加する
- [x] `application/backend/src/search.rs` を新規作成する
- [x] Tantivy インデックススキーマを定義する（`slug` / `title` / `body_text` / `tags` / `date`）
- [x] `TantivySearchIndex` を実装する（インメモリインデックス構築・クエリ実行）
- [x] HTML タグ除去してプレーンテキスト化する処理を実装する
- [x] サーバー起動時に `CONTENT_ROOT` から全 published 記事のインデックスを構築する
- [x] `/admin/static/regenerate` 完了後にインデックスを再構築する

#### 3.5.3 ルート追加
- [x] `GET /search?q=<query>` を実装する（SSR で結果ページを返す）
- [x] `presentation.rs` のルート設定に追加する

#### 3.5.4 Frontend（SSR）
- [x] `render_search_page(query: &str, results: Vec<SearchResultView>) -> String` を追加する
- [x] 検索フォームと結果一覧を warm beige/brown スタイルで実装する（JS なし）

#### 3.5.5 テスト
- [x] インデックス構築後にクエリが正しい結果を返すことを確認するテストを追加する
- [x] クエリヒットなし時の挙動テストを追加する
- [x] draft 記事がインデックスに含まれないことを確認するテストを追加する

---

### 3.6 Phase 4 — Application Insights

#### 3.6.1 Backend 実装
- [x] `application/backend/src/observability.rs` に `ApplicationInsightsObservabilitySink` を追加する
- [x] Azure Monitor Track REST API エンドポイントへの POST を実装する（`reqwest`、fire-and-forget `tokio::spawn`）
- [x] `AppEvent` の各バリアントを Application Insights の `customEvent`（`EventData`）スキーマにマップする
- [x] `APPLICATIONINSIGHTS_CONNECTION_STRING` からエンドポイント URL と Instrumentation Key を解析する

#### 3.6.2 Config
- [x] `config.rs` の `observability_backend` に `appinsights` 値を追加する
- [x] `build_observability_sink()` ファクトリに `appinsights` ブランチを追加する（未設定時は stdout にフォールバック）
- [x] `.env.local.example` に `OBSERVABILITY_BACKEND=stdout` と `appinsights` 設定例を追記する

#### 3.6.3 テスト
- [x] `ApplicationInsightsObservabilitySink` のペイロード組み立てのユニットテストを追加する（9 tests）

---

### 3.7 Phase 5 — Terraform IaC

#### 3.7.1 ディレクトリ・共通設定
- [x] `terraform/` ディレクトリを作成する
- [x] `terraform/main.tf` を作成する（`azurerm` provider・`terraform` ブロック・モジュール呼び出し・RBAC role assignments）
- [x] `terraform/variables.tf` を作成する（環境・リージョン・プレフィックス・コンテナイメージ等）
- [x] `terraform/outputs.tf` を作成する（主要リソースの endpoint / ID 出力）
- [x] `terraform/.gitignore` を作成する（`*.tfstate` / `.terraform/` 等）

#### 3.7.2 modules/storage
- [x] `azurerm_storage_account` を定義する（Standard LRS）
- [x] Table（コメント・お問い合わせ用）を定義する（`comments` / `contacts`）
  - ※ 静的アセット用 Blob コンテナは v3.5 により不要

#### 3.7.3 modules/app
- [x] `azurerm_service_plan` を定義する（Linux、B1/P1v3 変数）
- [x] `azurerm_linux_web_app` を定義する（Docker コンテナ、SystemAssigned managed identity）
- [x] App Settings（環境変数）を Key Vault 参照で定義する（@Microsoft.KeyVault(...)）

#### 3.7.4 modules/monitoring
- [x] `azurerm_log_analytics_workspace` を定義する
- [x] `azurerm_application_insights` を定義する

#### 3.7.5 modules/keyvault
- [x] `azurerm_key_vault` を定義する（RBAC 認可、soft-delete 7日）
- [x] シークレット参照の変数・出力を定義する（AppInsights CS / Slack / OpenAI API key / storage key）
  - AppInsights CS は Terraform が自動投入、他は lifecycle ignore_changes でポータル設定保持

#### 3.7.6 modules/openai
- [x] `azurerm_cognitive_account` を定義する（Azure OpenAI）
- [x] モデルデプロイメント定義を追加する（gpt-4o-mini Standard 10K TPM）

#### 3.7.7 modules/comms
- [x] `azurerm_communication_service` を定義する（将来の Email 通知用、作成のみ）

#### 3.7.8 ドキュメント
- [x] `terraform/README.md` に初期化・plan・apply 手順を記載する
- [x] `v4/azure-boundaries.md` に v4 での Azure リソース境界・既知制限を記載する

---

### 3.8 Phase 6 — Admin UI（Leptos SSR）

#### 3.8.1 Frontend render 関数
- [x] `render_admin_dashboard(posts: Vec<PostSummaryView>) -> String` を実装する（PostSummaryView に `status` フィールド追加）
- [x] `render_admin_post_detail(post: PostView, metadata: Option<GeneratedMetadataView>) -> String` を実装する
- [x] `render_admin_comments(pending: Vec<CommentView>) -> String` を実装する（承認待ちのみ）
- [x] `render_admin_static_panel() -> String` を実装する（再生成ボタンのみ）
- [x] Admin UI 共通 CSS を定義する（warm beige/brown スキーム、`ADMIN_CSS` 定数）

#### 3.8.2 ルート更新
- [x] `GET /admin` を `render_admin_dashboard()` に切り替える（旧 render_admin_home 削除）
- [x] `GET /admin/posts/{slug}` を追加し `render_admin_post_detail()` を呼ぶ（metadata JSON ファイルを読み込んで表示）
- [x] `GET /admin/comments` を `render_admin_comments()` に切り替える
- [x] `GET /admin/static` を追加し `render_admin_static_panel()` を呼ぶ

#### 3.8.3 テスト
- [x] `admin_dashboard_contains_post_table` テストを追加する
- [x] `admin_post_detail_returns_html_for_published_post` / `admin_post_detail_requires_auth` を追加する
- [x] `admin_static_panel_returns_html` / `admin_static_panel_requires_auth` を追加する

---

### 3.9 CI/CD

- [ ] `v4.0_dev` ブランチを作成する
- [ ] Phase ごとに feature ブランチを切り、PR → master のフローで進める

---

### 3.10 Docs
- [ ] `v4/spec.md` を整備する（各 Phase の詳細仕様）
- [ ] README に v4 の機能と実行方法を追記する
- [ ] `.env.local.example` を v4 の全環境変数で更新する
