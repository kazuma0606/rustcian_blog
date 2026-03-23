# v6 仕様書

> 作成: 2026-03-23

---

## スコープ（確定）

| ID | 機能 | 優先度 |
|---|---|---|
| v6-A | 記事カードリデザイン（1列・Zenn風） | 🔴 高 |
| v6-B | Analytics 管理画面 + テストデータ投入 | 🔴 高 |
| v6-C | ページネーション（10件/ページ） | 🔴 高 |
| v6-D | ステージング環境（dev） | 🟡 中 |
| v6-E | OGP / SEO 強化 | 🟡 中 |
| v6-F | 読了時間表示 | 🟡 中 |

---

## v6-A: 記事カードリデザイン

### デザイン仕様

Zenn 参考画像に基づく1列レイアウト:

```
┌──────────────────────────────────────────────────────────────┐
│  ┌──────────┐  タイトル（h2）                                │
│  │          │  2026-03-23   約 3 分                          │
│  │  image   │                                                │
│  │ or BLOG  │  description / summary_ai / summary の優先順  │
│  │ 100×75px │  で表示するテキスト（2〜3行）                  │
│  └──────────┘  [tag1] [tag2]                                 │
└──────────────────────────────────────────────────────────────┘
```

- **1列**（タイル形式ではない）
- サムネイル: `100×75px`（4:3）, `object-fit: cover`, 角丸 6px
- hero_image なし → グレー背景 + "BLOG" 白文字プレイスホルダー
- 配色は既存サイトに合わせる（`--card-bg`, `--text` 等の CSS 変数を使用）
- モバイル（`max-width: 600px`）: サムネイル `80×60px`、テキスト折り返し

### description フィールドの追加

`meta.yml` に任意フィールド `description` を追加（既存記事は `summary` にフォールバック）。

表示優先順位: `summary_ai` > `description` > `summary`

**影響箇所**:
- `application/core/src/domain/post.rs` — `PostMetadata` に `description: Option<String>`
- `application/core/src/domain/post.rs` — `PostSummary` に `description: Option<String>`
- `application/frontend/src/lib.rs` — `PostSummaryView` に `description: Option<String>`
- `application/backend/src/storage.rs` — メタデータパース部分
- `application/frontend/src/lib.rs` — `PostsPage` コンポーネントのカード描画

---

## v6-B: Analytics 管理画面

### テーブルスキーマ（実装に合わせた型定義）

**`analyticspv`**（ページビュー）:
| フィールド | 型 | 説明 |
|---|---|---|
| PartitionKey | String | 日付 `"YYYY-MM-DD"` |
| RowKey | String | `"{slug}_{timestamp_ms}"` |
| slug | String | 記事スラッグ |
| ip_hash | String | IP + 日付の SHA256 先頭8バイト (base64) |

**`analyticsqueries`**（検索クエリ）:
| フィールド | 型 | 説明 |
|---|---|---|
| PartitionKey | String | 日付 `"YYYY-MM-DD"` |
| RowKey | String | `"{timestamp_ms}"` |
| query | String | 検索語 |
| result_count | String | 結果件数（**文字列**として格納） |

**`analyticssessions`**（閲覧セッション）:
| フィールド | 型 | 説明 |
|---|---|---|
| PartitionKey | String | `"{ip_hash}_{date}"` |
| RowKey | String | `"{timestamp_ms}"` |
| slug | String | 記事スラッグ |

### テストデータ投入

`az storage entity insert` で直接投入（Managed Identity or account key 使用）。
検証用として過去 3 日分のダミーデータを以下の件数で作成:

| 日付 | PV | 検索 | セッション |
|---|---|---|---|
| 2026-03-21 | 10件 | 4件 | 8件 |
| 2026-03-22 | 18件 | 7件 | 14件 |
| 2026-03-23 | 6件 | 2件 | 5件 |

### 管理画面 UI（`GET /admin/analytics`）

```
過去30日サマリー
┌──────────────────────────────────────────────────┐
│  総 PV: 34   ユニーク IP: 9   検索数: 13         │
└──────────────────────────────────────────────────┘

記事別 PV（上位10）
actix-and-leptos        ████████████  20
hello-rustacian-blog    ████████       10
verification-test       ████            4

人気検索クエリ（上位10）
"actix" 5回  "rust" 4回  "leptos" 4回
```

**実装**:
- Table Storage の `analyticspv` を `$filter=PartitionKey ge '2026-02-23'` でクエリ
- Rust 側で集計（外部 BI ツール不使用）
- `AzuriteTableClient` の `query_entities` を使用（読み取り専用）
- Entra ID 認証済み管理者のみアクセス可（既存 `authenticate_admin` ミドルウェアを適用）

---

## v6-C: ページネーション

### 仕様

- `GET /?page=1`（デフォルト page=1）
- 1ページ = 10件（`PER_PAGE = 10`）
- 空リストでもページネーションボタンを表示（検証しやすくするため）
- ページ下部に以下のナビゲーション:

```
                    ← Prev    1 / 3    Next →
```

- page=1 のとき `← Prev` は非活性（リンクなし・グレー）
- 最終ページのとき `Next →` は非活性
- URL: `/?page=N`（タグページは `/tags/<tag>?page=N`）

**実装**:
- `list_posts` の結果をバックエンドでスライス
- `render_posts_page` に `page: usize`, `total_pages: usize` を追加
- 静的サイト生成時は `/page/2/index.html` 等を出力（v6 では対応しない、動的配信のみ）

---

## v6-D: ステージング環境

### Terraform 構成

既存の prod モジュールと同じ構成を `environment = "dev"` で追加:

```hcl
# terraform/main.tf に dev ブロックを追加
module "app_dev" {
  source = "./modules/app"
  prefix = "rustacian-dev"
  container_cpu    = 0.25
  container_memory = "0.5Gi"
  # ADMIN_AUTH_MODE = local-dev（Basic認証）
  # min_replicas = 0, max_replicas = 1
}
```

- dev 専用 Storage Account: `rustaciandevst`
- dev 専用 ACR タグ: `rustacianprodacr.azurecr.io/rustacian-blog:dev-<sha>`
- ドメイン: Container App FQDN をそのまま使用（Cloudflare 不要）

### GitHub Actions

- `ci.yml`: master push → prod ACR push（変更なし）
- `deploy-dev.yml`（新規）: feature branch push → dev Container App 更新
- dev 用 Secrets: `DEV_CONTAINER_APP_NAME`, `DEV_RESOURCE_GROUP`

---

## v6-E: OGP / SEO 強化

各ページの `<head>` に以下を追加:

```html
<!-- 記事ページ -->
<meta property="og:title" content="記事タイトル | Rustacian Blog">
<meta property="og:description" content="description / summary_ai / summary">
<meta property="og:image" content="hero_image URL or デフォルト画像">
<meta property="og:url" content="https://rustacian-blog.com/p/{slug}">
<meta property="og:type" content="article">
<meta name="twitter:card" content="summary_large_image">

<!-- トップページ -->
<meta property="og:type" content="website">
<meta property="og:image" content="https://rustacian-blog.com/images/og-default.png">
```

- デフォルト OGP 画像 (`og-default.png`) を `content/images/` に追加
- `render_post_page` / `render_posts_page` の `<head>` 生成を拡張

---

## v6-F: 読了時間表示

```rust
fn estimate_read_minutes(body: &str) -> usize {
    // 日本語: 400字/分、英語: 200words/分 の混在を考慮
    let char_count = body.chars().count();
    ((char_count as f64 / 400.0).ceil() as usize).max(1)
}
```

- `Post` / `PostSummary` に `read_minutes: usize` を追加
- 記事カード: `約 N 分` をサムネイル下または日付の隣に表示
- 記事ページ: タイトル下に `約 N 分で読めます` を表示

---

## 実装順序（推奨）

1. **v6-A** 記事カード + description フィールド（フロントエンド中心）
2. **v6-F** 読了時間（v6-A と同時に実装できる）
3. **v6-C** ページネーション（バックエンド + フロントエンド）
4. **v6-B** Analytics テストデータ + 管理画面
5. **v6-E** OGP（ヘッド生成の改修）
6. **v6-D** ステージング環境（Terraform・インフラ作業）
