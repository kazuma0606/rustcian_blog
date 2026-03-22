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

---

### 3.2 Phase 1 — Slack 通知基盤

#### 3.2.1 Core ドメイン
- [ ] `application/core/src/domain/notification.rs` を新規作成する
- [ ] `NotificationSink` トレイトを定義する（`async fn notify(&self, event: NotificationEvent)`）
- [ ] `NotificationEvent` enum を定義する（`PostPublished` / `StaticSiteRebuilt` / `CommentReceived` / `ContactFormSubmitted` / `AiMetadataGenerated`）
- [ ] `domain/mod.rs` に `notification` モジュールを追加する

#### 3.2.2 Backend アダプター
- [ ] `application/backend/src/notification.rs` を新規作成する
- [ ] `NoopNotificationSink` を実装する（テスト・ローカル開発用）
- [ ] `SlackNotificationSink` を実装する（Incoming Webhooks API へ `reqwest` で POST）
- [ ] メッセージフォーマットを定義する（イベント種別ごとの Slack メッセージ文言）
- [ ] `build_notification_sink()` ファクトリ関数を実装する（`SLACK_WEBHOOK_URL` 未設定時は Noop）

#### 3.2.3 Config
- [ ] `config.rs` に `slack_webhook_url: Option<String>` を追加する
- [ ] `.env.local.example` に `SLACK_WEBHOOK_URL=` を追記する

#### 3.2.4 AppState への組み込み
- [ ] `state.rs` に `notification: Arc<dyn NotificationSink>` を追加する
- [ ] `main.rs` でファクトリ呼び出しと DI を行う

#### 3.2.5 既存ルートへのフック
- [ ] `POST /admin/static/regenerate` 完了後に `StaticSiteRebuilt` を emit する
- [ ] `POST /admin/ai/{slug}/metadata` 完了後に `AiMetadataGenerated` を emit する

#### 3.2.6 テスト
- [ ] `NoopNotificationSink` で既存テストが引き続き通ることを確認する
- [ ] `SlackNotificationSink` のメッセージ組み立てのユニットテストを追加する

---

### 3.3 Phase 2 — コメント & お問い合わせフォーム

#### 3.3.1 Core ドメイン
- [ ] `application/core/src/domain/comment.rs` を新規作成する
- [ ] `Comment` エンティティを定義する（`id` / `post_slug` / `author_name` / `content` / `created_at` / `status`）
- [ ] `CommentStatus` enum を定義する（`Pending` / `Approved` / `Rejected`）
- [ ] `ContactMessage` エンティティを定義する（`id` / `from_name` / `from_email` / `body` / `created_at`）
- [ ] `CommentRepository` トレイトを定義する（`create_comment` / `list_comments` / `update_status`）
- [ ] `ContactRepository` トレイトを定義する（`create_contact_message`）
- [ ] `domain/mod.rs` に `comment` モジュールを追加する

#### 3.3.2 サニタイズ
- [ ] `application/backend/Cargo.toml` に `ammonia` を追加する
- [ ] コメント・お問い合わせ投稿時のサニタイズ処理を実装する（HTML タグ除去）

#### 3.3.3 Backend ストレージ実装
- [ ] `application/backend/src/comment_store.rs` を新規作成する
- [ ] `AzuriteCommentRepository` を実装する（Azure Table Storage / Azurite）
- [ ] `AzuriteContactRepository` を実装する（Azure Table Storage / Azurite）
- [ ] `build_comment_repository()` / `build_contact_repository()` ファクトリを実装する

#### 3.3.4 ルート追加
- [ ] `POST /posts/{slug}/comments` を実装する（サニタイズ → 保存 → `CommentReceived` 通知）
- [ ] `GET /posts/{slug}/comments` を実装する（承認済み一覧を SSR 埋め込み）
- [ ] `POST /contact` を実装する（サニタイズ → 保存 → `ContactFormSubmitted` 通知）
- [ ] `GET /admin/comments` を実装する（モデレーションキュー、要認証）
- [ ] `POST /admin/comments/{id}/approve` を実装する（要認証）
- [ ] `POST /admin/comments/{id}/reject` を実装する（要認証）
- [ ] `presentation.rs` のルート設定に追加する

#### 3.3.5 Frontend（SSR）
- [ ] コメント一覧の SSR レンダリング関数をフロントエンドクレートに追加する
- [ ] お問い合わせフォームの SSR レンダリング関数を追加する

#### 3.3.6 テスト
- [ ] XSS ペイロード（`<script>alert(1)</script>` 等）がサニタイズされることを確認するテストを追加する
- [ ] HTML 属性インジェクション（`<img onerror=...>`）のテストを追加する
- [ ] モデレーションステータス遷移（Pending → Approved / Rejected）のテストを追加する
- [ ] 承認済みコメントのみ公開 API に返ることを確認するテストを追加する

---

### 3.4 Phase 3 — Tantivy 全文検索

#### 3.4.1 Core ドメイン
- [ ] `application/core/src/domain/search.rs` を新規作成する
- [ ] `SearchResult` 構造体を定義する（`slug` / `title` / `excerpt` / `tags` / `date`）
- [ ] `SearchQuery` 構造体を定義する
- [ ] `domain/mod.rs` に `search` モジュールを追加する

#### 3.4.2 Backend 実装
- [ ] `application/backend/Cargo.toml` に `tantivy` を追加する
- [ ] `application/backend/src/search.rs` を新規作成する
- [ ] Tantivy インデックススキーマを定義する（`slug` / `title` / `body_text` / `tags` / `date`）
- [ ] `TantivySearchIndex` を実装する（インメモリインデックス構築・クエリ実行）
- [ ] HTML タグ除去してプレーンテキスト化する処理を実装する
- [ ] サーバー起動時に全 published 記事からインデックスを構築する
- [ ] `/admin/static/regenerate` 完了後にインデックスを再構築する

#### 3.4.3 ルート追加
- [ ] `GET /search?q=<query>` を実装する（SSR で結果ページを返す）
- [ ] `presentation.rs` のルート設定に追加する

#### 3.4.4 Frontend（SSR）
- [ ] `render_search_page(query: &str, results: Vec<SearchResult>) -> String` を追加する
- [ ] 検索フォームと結果一覧を warm beige/brown スタイルで実装する（JS なし）

#### 3.4.5 テスト
- [ ] インデックス構築後にクエリが正しい結果を返すことを確認するテストを追加する
- [ ] クエリヒットなし時の挙動テストを追加する
- [ ] draft 記事がインデックスに含まれないことを確認するテストを追加する

---

### 3.5 Phase 4 — Application Insights

#### 3.5.1 Backend 実装
- [ ] `application/backend/src/observability.rs` に `ApplicationInsightsObservabilitySink` を追加する
- [ ] Azure Monitor Track REST API エンドポイントへの POST を実装する（`reqwest`）
- [ ] `AppEvent` の各バリアントを Application Insights の `customEvent` / `request` スキーマにマップする
- [ ] `APPLICATIONINSIGHTS_CONNECTION_STRING` からエンドポイント URL と Instrumentation Key を解析する

#### 3.5.2 Config
- [ ] `config.rs` の `observability_backend` に `appinsights` 値を追加する
- [ ] `build_observability_sink()` ファクトリに `appinsights` ブランチを追加する
- [ ] `.env.local.example` に `OBSERVABILITY_BACKEND=stdout` を明示する

#### 3.5.3 テスト
- [ ] `ApplicationInsightsObservabilitySink` のペイロード組み立てのユニットテストを追加する

---

### 3.6 Phase 5 — Terraform IaC

#### 3.6.1 ディレクトリ・共通設定
- [ ] `terraform/` ディレクトリを作成する
- [ ] `terraform/main.tf` を作成する（`azurerm` provider・`terraform` ブロック）
- [ ] `terraform/variables.tf` を作成する（環境・リージョン・プレフィックス等）
- [ ] `terraform/outputs.tf` を作成する（主要リソースの endpoint / ID 出力）
- [ ] `terraform/.gitignore` を作成する（`*.tfstate` / `.terraform/` 等）

#### 3.6.2 modules/storage
- [ ] `azurerm_storage_account` を定義する
- [ ] Blob コンテナ（静的アセット用）を定義する
- [ ] Table（コメント・お問い合わせ用）を定義する

#### 3.6.3 modules/app
- [ ] `azurerm_service_plan` を定義する
- [ ] `azurerm_linux_web_app` を定義する（backend コンテナ）
- [ ] App Settings（環境変数）を Key Vault 参照で定義する

#### 3.6.4 modules/monitoring
- [ ] `azurerm_log_analytics_workspace` を定義する
- [ ] `azurerm_application_insights` を定義する

#### 3.6.5 modules/keyvault
- [ ] `azurerm_key_vault` を定義する
- [ ] シークレット参照の変数・出力を定義する（`SLACK_WEBHOOK_URL` 等）

#### 3.6.6 modules/openai
- [ ] `azurerm_cognitive_account` を定義する（Azure OpenAI）
- [ ] モデルデプロイメント定義を追加する

#### 3.6.7 modules/comms
- [ ] `azurerm_communication_service` を定義する（将来の Email 通知用、disabled 状態で用意）

#### 3.6.8 ドキュメント
- [ ] `terraform/README.md` に初期化・plan・apply 手順を記載する
- [ ] `v4/azure-boundaries.md` に v4 での Azure リソース境界を記載する

---

### 3.7 Phase 6 — Admin UI（Leptos SSR）

#### 3.7.1 Frontend render 関数
- [ ] `render_admin_dashboard(posts: Vec<PostSummary>, build_info: Option<BuildInfo>) -> String` を実装する
- [ ] `render_admin_post_detail(post: Post, generated_metadata: Option<GeneratedMetadata>) -> String` を実装する
- [ ] `render_admin_comments(pending: Vec<Comment>, approved: Vec<Comment>) -> String` を実装する
- [ ] `render_admin_static_panel(last_build: Option<BuildInfo>) -> String` を実装する
- [ ] Admin UI 共通 CSS を定義する（記事ページの warm beige/brown カラースキーム・フォント流用）

#### 3.7.2 ルート更新
- [ ] `GET /admin` を `render_admin_dashboard()` に切り替える
- [ ] `GET /admin/posts/{slug}` を追加し `render_admin_post_detail()` を呼ぶ
- [ ] `GET /admin/comments` を `render_admin_comments()` に切り替える
- [ ] `GET /admin/static` を追加し `render_admin_static_panel()` を呼ぶ

#### 3.7.3 BuildInfo 連携
- [ ] `_meta/build.json` を読み込む `BuildInfo` 構造体を定義する
- [ ] Admin dashboard / static panel に最終ビルド情報を表示する

#### 3.7.4 テスト
- [ ] 各 render 関数の出力に期待するマークアップが含まれることを確認するテストを追加する
- [ ] Admin ルートが認証なしでアクセスできないことを確認するテストを追加する

---

### 3.8 CI/CD

#### 3.8.1 PR #10 クローズ
- [ ] Dependabot の jsonwebtoken PR #10 をクローズする（v3.0_dev で取り込み済み）

#### 3.8.2 v4 ブランチ戦略
- [ ] `v4.0_dev` ブランチを作成する
- [ ] Phase ごとに feature ブランチを切り、PR → master のフローで進める

---

### 3.9 Docs
- [ ] `v4/spec.md` を整備する（各 Phase の詳細仕様）
- [ ] README に v4 の機能と実行方法を追記する
- [ ] `.env.local.example` を v4 の全環境変数で更新する
