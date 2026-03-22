# v5 実装計画

スコープ: idea.md の 1〜9 番

---

## フェーズ構成

| Phase | 対象 | 理由 |
|-------|------|------|
| 1 | #4 サムネイル + AI 要約カード | ローカル完結、リリース前でも効果大 |
| 2 | #7 画像アップロード UI | Admin コンテンツ管理の前提 |
| 3 | #2 Entra ID PKCE ログイン | Admin 画面のセキュリティ確立 |
| 4 | #1 CDN + #6 Monitor Alerts | Terraform インフラ整備 |
| 5 | #3 Key Vault + Managed Identity | 認証方式の本番切り替え |
| 6 | #5 Azure AI 拡張（Vision + Translator） | 既存 AI 基盤を活用 |
| 7 | #8 ACS Email 通知 | 通知チャネル追加 |
| 8 | #9 Container Apps 移行 | 最後にデプロイ方式を切り替え |

---

## Phase 1 — サムネイル + AI 要約カード表示（#4）

### 現状
`PostSummaryView` に `hero_image: Option<String>` と `summary_ai: Option<String>` フィールドは定義済みだが、
`render_posts_page` / `render_tag_posts_page` のカード HTML には反映されていない。

### 実装方針
- `application/frontend/src/lib.rs` のカード HTML を修正
  - `hero_image` が `Some` → `<img>` タグをカード上部に表示（`/images/{path}` 経由）
  - `summary_ai` が `Some` → description の代わりに表示、`None` は従来の `description` にフォールバック
- CSS 変更なし（既存の `<style>` ブロックを活用）
- `render_post_page` のサイドバー等には影響しない

---

## Phase 2 — 画像アップロード UI（#7）

### 現状
- `AzuriteBlobAdapter::put_bytes` は未実装（`get_bytes` のみある）
- `/images/{path}` GET はある
- Admin API に画像関連ルートなし

### 実装方針

**Backend**
```
POST /admin/images          multipart/form-data → Blob に保存 → {url, name} を返す
GET  /admin/images          Blob 一覧 → JSON [{name, url, uploaded_at}]
DELETE /admin/images/{name} Blob 削除
POST /admin/posts/{slug}/hero  body: {image: "filename"} → meta.yml の hero_image を更新
```
- `AzuriteBlobAdapter::put_bytes(name, bytes, content_type)` を追加
- `AzuriteBlobAdapter::list_blobs(prefix)` を追加（`?comp=list&prefix=images/` API）
- 画像は `images/{filename}` パスで保存（既存の GET と一致）
- 認証: 既存の `extract_admin_identity` ミドルウェアを流用

**Frontend**
- `render_admin_image_gallery()` を追加 — `GET /admin/images` の JSON をもとに一覧表示
- 記事編集画面の hero_image 選択 UI は既存の admin post detail に追加

---

## Phase 3 — Entra ID PKCE ログインページ（#2）

### 現状
- `auth.rs` に `entra-oidc` (JWT 検証) は実装済み
- フロントエンドに OAuth2 ログインフロー（認可コードリダイレクト、コールバック）が未実装
- 現在 Admin は Bearer トークンを手動で付与する必要がある

### 実装方針

**新規 config 変数**
```
ENTRA_REDIRECT_URI    = "https://{host}/admin/callback"
```

**新規ルート**
```
GET /admin/login
  → Azure AD 認可エンドポイントへリダイレクト
    (response_type=code, scope=openid profile, code_challenge=S256, state=nonce)

GET /admin/callback?code=...&state=...
  → token_endpoint に code + code_verifier を POST
  → id_token を取得、verify
  → Set-Cookie: admin_token=<JWT>; HttpOnly; Secure; SameSite=Strict
  → /admin にリダイレクト
```

**セッション方式**
- PKCE verifier は `actix-session` + in-memory store でなく、`state` パラメータに暗号化して埋め込む（外部ストレージ不要）
- Cookie に JWT を格納し、既存の `extract_admin_identity` で検証

**Frontend**
- `render_login_page(error: Option<&str>)` をフロントエンド crate に追加

---

## Phase 4 — Cloudflare 設定 + Monitor Alerts（#1 #6）

### #1: Cloudflare DNS + CDN

> **決定**: Azure CDN の代わりに Cloudflare を使用。
> `rustacian-blog.com` は Cloudflare で取得済み（2026-03-22）。
> Cloudflare が CDN・HTTPS・DDoS 保護を無料で提供するため、Azure CDN Terraform モジュールは不要。

**Cloudflare DNS 設定（手動）**
- Container Apps のエンドポイント URL が確定したら（Phase 8 完了後）、Cloudflare ダッシュボードで以下を設定:
  - `CNAME @ → {name}.{region}.azurecontainerapps.io`（Proxy 有効でオレンジ雲マーク）
- HTTPS 証明書は Cloudflare が自動発行

**Backend: publish 後のキャッシュパージ**
- `static_site.rs` の `publish` 完了後、Cloudflare Cache Purge API を呼び出す
- `POST https://api.cloudflare.com/client/v4/zones/{zone_id}/purge_cache`
- config: `CLOUDFLARE_ZONE_ID`, `CLOUDFLARE_API_TOKEN`

### #6: Monitor Alerts

**Terraform** (`terraform/modules/monitoring/main.tf` に追記)
```hcl
resource "azurerm_monitor_metric_alert" "content_error" {
  name     = "${var.prefix}-content-error-alert"
  ...
  criteria { metric_name = "customEvents/count" ... }
  action { action_group_id = azurerm_monitor_action_group.slack.id }
}
```
- `azurerm_monitor_action_group` で Slack webhook を通知先に設定

---

## Phase 5 — Key Vault + Managed Identity（#3）

### 現状
- `table.rs` は SharedKey 認証（Azurite 用キーをハードコード）
- App Service / Container Apps は `StorageTableDataContributor` ロールが割り当て済み（Terraform 済み）
- `AZURE_STORAGE_ACCOUNT_KEY` が Key Vault 参照でインジェクト済み

### 実装方針

**`table.rs` の認証切り替え**
```rust
enum StorageCredential {
    SharedKey { account: String, key: String },
    ManagedIdentity { account: String, token_cache: RwLock<Option<CachedToken>> },
}
```
- `AZURE_STORAGE_ACCOUNT_KEY` が設定されている → SharedKey（Azurite / 旧互換）
- キーがない + `AZURE_STORAGE_ACCOUNT_NAME` だけある → Managed Identity
  - IMDS エンドポイント: `http://169.254.169.254/metadata/identity/oauth2/token?resource=https://storage.azure.com/`
  - トークンをキャッシュ（`expires_in` の 5 分前に再取得）
  - `Authorization: Bearer <token>` でリクエスト

**`analytics/src/table.rs` も同様に対応**

---

## Phase 6 — Azure AI 拡張（#5）

### 5a: 画像 Alt テキスト自動生成（Azure AI Vision）

**新規 config 変数**
```
AZURE_VISION_ENDPOINT
AZURE_VISION_API_KEY   (or Managed Identity)
```

**新規ルート**
```
POST /admin/images/{name}/describe
  → Azure Computer Vision Analyze API (features=description)
  → {alt_text: "..."} を返す
```
- `ai.rs` に `VisionAdapter` を追加
- Admin 画像ギャラリーから「Alt 生成」ボタンで呼び出し

### 5b: 記事自動翻訳（Azure Translator）

**新規 config 変数**
```
AZURE_TRANSLATOR_ENDPOINT
AZURE_TRANSLATOR_API_KEY
```

**新規ルート**
```
GET /en/posts/{slug}   → 日本語本文を Translator API に送り英語 HTML を返す（キャッシュ付き）
GET /en/               → 英語版記事一覧
```
- 翻訳結果は Blob に `translated/{slug}/en.html` としてキャッシュ
- `<link rel="alternate" hreflang="ja" href="/posts/{slug}">` を双方のページに追加
- 静的サイト生成時も翻訳版を出力（`/en/posts/{slug}/index.html`）

---

## Phase 7 — ACS Email 通知（#8）

### 現状
- `notification.rs` に `SlackNotificationSink` と `NoopNotificationSink` がある
- `modules/comms` Terraform モジュールで ACS リソース定義済み

### 実装方針
- `AcsEmailNotificationSink` を `notification.rs` に追加（`NotificationSink` trait を実装）
- ACS Email REST API: `POST {endpoint}/emails:send?api-version=2023-03-31`
- 認証: ACS connection string から HMAC 署名（Blob SharedKey と同方式）
- config: `ACS_ENDPOINT`, `ACS_ACCESS_KEY`, `ACS_SENDER_ADDRESS`, `ACS_RECIPIENT_ADDRESS`
- Slack と ACS を両方有効にできるよう `MultiNotificationSink` でラップ

---

## Phase 8 — Container Apps 移行（#9）

### 現状
- `modules/app/` は `azurerm_linux_web_app`（App Service）

### 実装方針
- `modules/app/main.tf` を差し替え:
  - `azurerm_container_app_environment`（Log Analytics に接続）
  - `azurerm_container_app`（スケールゼロ設定: `min_replicas = 0`）
  - Managed Identity は Container Apps でも `identity { type = "SystemAssigned" }` で同様
- Key Vault 参照: App Service と同じ `@Microsoft.KeyVault(...)` 記法は Container Apps では使えないため、起動スクリプトで `az keyvault secret show` を呼び出すか、Container Apps secrets 機能を使う
- `variables.tf` から `app_service_sku` を削除し `container_app_cpu` / `container_app_memory` に変更
- `outputs.tf` の `principal_id` は Container Apps でも同様に取得可能

---

## Phase 9 — 初回デプロイ（#CDN ドメイン含む）

### コンテナイメージ CI/CD
- `main` ブランチへのマージをトリガーに `docker build → ACR push → Container Apps 更新` を実行
- GitHub Actions の OIDC federated credential を使い、サービスプリンシパルのパスワードを持たない

### Terraform
- tfstate は Azure Blob Storage に保存（`backend "azurerm"`）
- `terraform apply` は手動実行（CD に組み込むのは v6 以降でよい）

### ドメイン方針

**決定: `rustacian-blog.com` を Cloudflare で取得済み（2026-03-22）**

> Azure App Service Domain は MCA 個人契約では購入不可（`SubscriptionExceededMaxDomainLimit`）だったため Cloudflare を使用。

| 用途 | URL |
|------|-----|
| 本番ブログ | `https://rustacian-blog.com` |
| API / Admin | `https://rustacian-blog.com/api/...`（同一ドメイン、パスで分離） |

- Cloudflare DNS で CNAME を Container Apps エンドポイントに向ける（Phase 8 完了後）
- Cloudflare Proxy（オレンジ雲）を有効にすることで HTTPS・CDN・DDoS 保護が無料で得られる
- Azure CDN Terraform モジュールは**不要**

---

## 依存関係図

```
Phase 1 (サムネイル)
    ↓
Phase 2 (画像アップロード) ← Phase 3 (Entra SSO) で認証を確立してから本番利用可
    ↓
Phase 4 (CDN + Alerts) ← Terraform 先行
    ↓
Phase 5 (Managed Identity) ← Phase 4 の CDN パージで IMDS 共通化
    ↓
Phase 6 (AI 拡張) ← Phase 5 の認証基盤が前提
    ↓
Phase 7 (ACS Email) ← Phase 5 の認証基盤が前提
    ↓
Phase 8 (Container Apps) ← 全機能が完成してから移行
    ↓
Phase 9 (初回デプロイ) ← terraform apply + CI/CD + ドメイン設定
```

---

## 追加する環境変数一覧

| 変数 | Phase | 用途 |
|------|-------|------|
| `ENTRA_REDIRECT_URI` | 3 | OAuth2 コールバック URL |
| `CLOUDFLARE_ZONE_ID` | 4 | Cloudflare ゾーン ID（キャッシュパージ用） |
| `CLOUDFLARE_API_TOKEN` | 4 | Cloudflare API トークン（Cache Purge 権限） |
| `AZURE_VISION_ENDPOINT` | 6 | Computer Vision エンドポイント |
| `AZURE_VISION_API_KEY` | 6 | Computer Vision API キー |
| `AZURE_TRANSLATOR_ENDPOINT` | 6 | Translator エンドポイント |
| `AZURE_TRANSLATOR_API_KEY` | 6 | Translator API キー |
| `ACS_ENDPOINT` | 7 | ACS Email エンドポイント |
| `ACS_ACCESS_KEY` | 7 | ACS アクセスキー |
| `ACS_SENDER_ADDRESS` | 7 | 送信元メールアドレス |
| `ACS_RECIPIENT_ADDRESS` | 7 | 通知先メールアドレス |
