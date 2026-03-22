# v5 タスク一覧

ステータス: `[ ]` 未着手 / `[~]` 進行中 / `[x]` 完了

---

## Phase 1: サムネイル + AI 要約カード表示（#4）

- [x] **1-1** `application/frontend/src/lib.rs` — `render_posts_page` のカード HTML に `hero_image` の `<img>` を追加
- [x] **1-2** `application/frontend/src/lib.rs` — カード本文に `summary_ai` を優先表示（`None` は `description` にフォールバック）
- [x] **1-3** `application/frontend/src/lib.rs` — `render_tag_posts_page` にも同じ変更を適用
- [x] **1-4** `PostSummary`（core）と `PostSummaryView`（frontend）に `summary_ai` フィールドを追加、CI グリーン確認済み

---

## Phase 2: 画像アップロード UI（#7）

- [x] **2-1** `application/backend/src/blob.rs` — `put_bytes(name, bytes, content_type)` を実装
- [x] **2-2** `application/backend/src/blob.rs` — `list_blobs(prefix)` を実装（Blob Service REST `?comp=list`）
- [x] **2-3** `application/backend/src/presentation.rs` — `POST /admin/images` ルートを追加（multipart 受け取り → Blob 保存）
- [x] **2-4** `application/backend/src/presentation.rs` — `GET /admin/images` ルートを追加（JSON 一覧）
- [x] **2-5** `application/backend/src/presentation.rs` — `DELETE /admin/images/{name}` ルートを追加
- [x] **2-6** `application/backend/src/presentation.rs` — `POST /admin/posts/{slug}/hero` ルートを追加（meta.yml の `hero_image` を更新）
- [x] **2-7** `application/frontend/src/lib.rs` — `render_admin_image_gallery(images: Vec<ImageView>)` を追加
- [x] **2-8** `application/backend/src/state.rs` — `image_blob` が `None` のときに Admin 画像 API が 503 を返すようにする（各ハンドラで実装済み）
- [x] **2-9** ユニットテスト追加（`list_blobs`、`put_bytes`）、CI グリーンを確認

---

## Phase 3: Entra ID PKCE ログインページ（#2）

- [x] **3-1** `application/backend/src/config.rs` — `entra_redirect_uri: Option<String>` を追加
- [x] **3-2** `application/backend/src/auth.rs` — PKCE verifier を `state` に埋め込む `build_auth_redirect_url()` を実装
- [x] **3-3** `application/backend/src/auth.rs` — `exchange_code_for_token(code, state, verifier)` を実装
- [x] **3-4** `application/backend/src/presentation.rs` — `GET /admin/login` ルートを追加（Azure AD 認可 URL にリダイレクト）
- [x] **3-5** `application/backend/src/presentation.rs` — `GET /admin/callback` ルートを追加（code → token 交換 → Cookie 発行 → リダイレクト）
- [x] **3-6** `application/backend/src/presentation.rs` — Cookie から JWT を読む（既存の `authenticate_admin` が `admin_session` Cookie を読む実装済み）
- [x] **3-7** `application/frontend/src/lib.rs` — `render_login_page(error: Option<&str>)` を追加
- [x] **3-8** `terraform/variables.tf` — `entra_redirect_uri` 変数を追加
- [x] **3-9** `.env.local.example` に `ENTRA_REDIRECT_URI` を追記
- [ ] **3-10** ログイン → コールバック → Cookie 認証の E2E テストを追加

---

## Phase 4: CDN + Monitor Alerts（#1 #6）

### Cloudflare キャッシュパージ (#1)
> Azure CDN は不使用。`rustacian-blog.com` は Cloudflare 取得済み。Terraform モジュール不要。
- [ ] **4-1** `application/backend/src/config.rs` — `cloudflare_zone_id`, `cloudflare_api_token` を追加
- [ ] **4-2** `application/backend/src/` — `cloudflare.rs` を新規作成（`CloudflareCacheClient::purge_all()` 実装）
- [ ] **4-3** `application/backend/src/static_site.rs` — publish 完了後に Cloudflare キャッシュパージを呼び出す（設定がある場合のみ）
- [ ] **4-4** `.env.local.example` に `CLOUDFLARE_ZONE_ID`, `CLOUDFLARE_API_TOKEN` を追記

### Monitor Alerts (#6)
- [ ] **4-9** `terraform/modules/monitoring/main.tf` — `azurerm_monitor_action_group`（Slack webhook 宛）を追加
- [ ] **4-10** `terraform/modules/monitoring/main.tf` — `azurerm_monitor_metric_alert` を追加（`ContentError` 閾値超過で通知）
- [ ] **4-11** `terraform/modules/monitoring/variables.tf` — Slack webhook URL 変数を追加
- [ ] **4-12** `terraform/main.tf` — monitoring モジュールに Slack webhook URL を渡す

---

## Phase 5: Key Vault + Managed Identity（#3）

- [ ] **5-1** `application/backend/src/table.rs` — `StorageCredential` enum を追加（`SharedKey` / `ManagedIdentity`）
- [ ] **5-2** `application/backend/src/table.rs` — IMDS トークン取得と 5 分前有効期限キャッシュを実装
- [ ] **5-3** `application/backend/src/table.rs` — `AZURE_STORAGE_ACCOUNT_KEY` の有無で認証方式を自動切り替え
- [ ] **5-4** `application/analytics/src/table.rs` — 同様に Managed Identity 対応
- [ ] **5-5** `application/backend/src/cdn.rs` — IMDS トークンを流用して CDN パージの Bearer 認証に使う（Phase 4 との共通化）
- [ ] **5-6** ローカル（Azurite SharedKey）と本番（Managed Identity）の両方で動作確認
- [ ] **5-7** `terraform/modules/app/main.tf` — `AZURE_STORAGE_ACCOUNT_KEY` の app_setting を削除（Key Vault 参照のみに統一）

---

## Phase 6: Azure AI 拡張（#5）

### Vision — Alt テキスト自動生成 (5a)
- [ ] **6-1** `application/backend/src/config.rs` — `azure_vision_endpoint`, `azure_vision_api_key` を追加
- [ ] **6-2** `application/backend/src/ai.rs` — `VisionAdapter` を追加（Computer Vision Analyze API）
- [ ] **6-3** `application/backend/src/presentation.rs` — `POST /admin/images/{name}/describe` ルートを追加
- [ ] **6-4** Admin 画像ギャラリーに「Alt 生成」ボタン（JS fetch → `render_admin_image_gallery` 更新）を追加

### Translator — 自動翻訳 (5b)
- [ ] **6-5** `application/backend/src/config.rs` — `azure_translator_endpoint`, `azure_translator_api_key` を追加
- [ ] **6-6** `application/backend/src/` — `translator.rs` を新規作成（Azure Translator Text API ラッパー）
- [ ] **6-7** `application/backend/src/presentation.rs` — `GET /en/posts/{slug}` ルートを追加（翻訳 + Blob キャッシュ）
- [ ] **6-8** `application/backend/src/presentation.rs` — `GET /en/` ルートを追加（英語版記事一覧）
- [ ] **6-9** `application/frontend/src/lib.rs` — `render_post_page` に `<link rel="alternate" hreflang>` を追加
- [ ] **6-10** `application/backend/src/static_site.rs` — 静的生成時に翻訳版 HTML を出力
- [ ] **6-11** `.env.local.example` に Vision / Translator 変数を追記

---

## Phase 7: ACS Email 通知（#8）

- [ ] **7-1** `application/backend/src/config.rs` — `acs_endpoint`, `acs_access_key`, `acs_sender_address`, `acs_recipient_address` を追加
- [ ] **7-2** `application/backend/src/notification.rs` — `AcsEmailNotificationSink` を実装（REST API HMAC 署名）
- [ ] **7-3** `application/backend/src/notification.rs` — `MultiNotificationSink` を追加（Slack + ACS を両立）
- [ ] **7-4** `application/backend/src/main.rs` — `build_notification_sink` で ACS 設定があれば Multi に切り替え
- [ ] **7-5** `.env.local.example` に ACS 変数を追記
- [ ] **7-6** ACS 送信のユニットテスト（モックサーバー使用）を追加

---

## Phase 8: Container Apps 移行（#9）

- [ ] **8-1** `terraform/modules/app/main.tf` — `azurerm_linux_web_app` を `azurerm_container_app` に差し替え
- [ ] **8-2** `terraform/modules/app/main.tf` — `azurerm_container_app_environment` を追加（Log Analytics ワークスペースに接続）
- [ ] **8-3** `terraform/modules/app/main.tf` — スケールゼロ設定（`min_replicas = 0`, `max_replicas = 3`）
- [ ] **8-4** `terraform/modules/app/main.tf` — Key Vault 参照を Container Apps secrets 方式に移行（`@Microsoft.KeyVault(...)` は App Service 専用のため）
- [ ] **8-5** `terraform/modules/app/variables.tf` — `sku_name` を削除し `container_cpu`, `container_memory` に変更
- [ ] **8-6** `terraform/modules/app/outputs.tf` — `principal_id` を Container Apps の managed identity から取得するよう更新
- [ ] **8-7** `terraform/variables.tf` — `app_service_sku` を削除し `container_cpu`, `container_memory` に変更
- [ ] **8-8** `terraform/main.tf` — `module "app"` の引数を更新
- [ ] **8-9** Staging 環境で `terraform plan` → `apply` を実行し動作確認
- [ ] **8-10** `terraform/README.md` に Container Apps 移行ノートを追記

---

## Phase 9: 初回デプロイ（Azure 本番環境）

### 前提確認
- [ ] **9-1** Azure サブスクリプション・テナント ID を確認し `terraform/variables.tf` の `entra_tenant_id` 等を設定
- [ ] **9-2** Terraform バックエンド（tfstate 保存先）を設定 — Azure Blob Storage に `terraform { backend "azurerm" {} }` を追加
- [ ] **9-3** Azure Container Registry (ACR) リソースを Terraform に追加（`terraform/modules/registry/` 新規）

### コンテナイメージ CI/CD
- [ ] **9-4** `.github/workflows/ci.yml` に `docker build` ステップを追加（PR では build のみ、`main` マージ時に ACR push）
- [ ] **9-5** GitHub Actions シークレットに `AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, `AZURE_SUBSCRIPTION_ID` を登録（OIDC federated credential 方式、パスワード不要）
- [ ] **9-6** `.github/workflows/deploy.yml` を新規作成 — ACR push 完了後に Container Apps の `--image` を更新してデプロイ

### Terraform apply
- [ ] **9-7** `terraform init` → `terraform plan` でリソース差分を確認
- [ ] **9-8** `terraform apply` を実行してインフラを構築（monitoring → keyvault → storage → openai → comms → cdn → app の順に依存解決される）
- [ ] **9-9** `terraform output` で各エンドポイント URL を記録

### シークレット投入
- [ ] **9-10** Key Vault に Slack webhook URL を設定 (`az keyvault secret set --name slack-webhook-url --value "..."`)
- [ ] **9-11** Key Vault に OpenAI API キーを設定
- [ ] **9-12** Key Vault に ACS アクセスキーを設定（Phase 7 完了後）
- [ ] **9-13** Container Apps が Key Vault シークレットを正常に解決できることを確認（`az containerapp show` でヘルスチェック）

### ドメイン・CDN
- [x] **9-14** `rustacian-blog.com` を Cloudflare で取得済み（2026-03-22）
- [ ] **9-15** Cloudflare ダッシュボードで DNS レコードを設定: `CNAME @ → {container-apps-endpoint}`（Proxy オン）
- [ ] **9-16** Cloudflare の HTTPS 証明書自動発行を確認（Proxy 有効なら自動）
- [ ] **9-17** `BASE_URL=https://rustacian-blog.com` を Container Apps の環境変数に設定し、sitemap / RSS の URL が正しいことを確認

### 動作確認
- [ ] **9-18** `GET /health` が 200 を返すことを確認
- [ ] **9-19** 記事一覧・記事詳細ページが正常表示されることを確認
- [ ] **9-20** 管理画面ログイン（Entra PKCE）が正常に動作することを確認
- [ ] **9-21** `cargo run -p rustacian_blog_backend -- publish-static` で静的サイトが CDN に反映されることを確認
- [ ] **9-22** Application Insights にテレメトリが届いていることを確認

---

## チェックリスト（各 Phase 完了時）

- [ ] `cargo fmt --all --check` パス
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` パス
- [ ] `cargo test` パス
- [ ] CI グリーン確認後にマージ
