# v6 実装プラン

> 作成: 2026-03-23

---

## 実装順序

1. **v6-A + v6-F**（記事カードリデザイン + 読了時間）— ドメイン層から始めてフロントエンドで完結
2. **v6-C**（ページネーション）— バックエンド + フロントエンド
3. **v6-B**（Analytics 管理画面 + テストデータ）— Table Storage クエリ + Admin UI
4. **v6-E**（OGP / SEO 強化）— head 生成の改修
5. **v6-D**（ステージング環境）— Terraform + GitHub Actions

---

## v6-A + v6-F: 記事カード + 読了時間

### 変更ファイル

**`application/core/src/domain/post.rs`**
- `PostMetadata` に `description: Option<String>` を追加（`#[serde(default)]`）
- `PostSummary` に `description: Option<String>`, `read_minutes: usize` を追加
- `Post` に `description: Option<String>`, `read_minutes: usize` を追加
- `Post::new()` で `estimate_read_minutes(&body_markdown)` を呼び出して設定
- `Post::summary()` で `description` と `read_minutes` を伝播
- `estimate_read_minutes(body: &str) -> usize` 関数を追加（400字/分）

**`application/frontend/src/lib.rs`**
- `PostSummaryView` に `description: Option<String>`, `read_minutes: usize` を追加（`#[serde(default)]`）
- CSS `.card` を `display: flex; flex-direction: row; align-items: flex-start; gap: 16px;` に変更
- CSS `.card img` の全幅スタイルを削除し、`.card-thumb` クラスを追加
- `.card-thumb-placeholder`（"BLOG" プレースホルダー）クラスを追加
- `PostsPage` コンポーネントのカード HTML を書き換え

**カード構造（新）:**
```
<a class="card">
  <div class="card-thumb"> <!-- 100×75px, 角丸6px -->
    <img> または <div class="card-thumb-placeholder">BLOG</div>
  </div>
  <div class="card-body"> <!-- flex:1 -->
    <h2 class="card-title">タイトル</h2>
    <div class="meta">日付 · 約N分</div>
    <p class="card-desc">description / summary_ai / summary</p>
    <div class="tags">...</div>
  </div>
</a>
```

**バックエンド (`application/backend/src/storage.rs` 付近)**
- `PostSummary` → `PostSummaryView` 変換時に `description`, `read_minutes` を追加

---

## v6-C: ページネーション

### 変更ファイル

**`application/frontend/src/lib.rs`**
- `render_posts_page` に `page: usize, total_pages: usize` パラメータを追加
- `PostsPage` コンポーネントにページネーション UI を追加
- ページネーション HTML:
  ```
  ← Prev   {page} / {total_pages}   Next →
  ```
  page=1 → Prev 非活性（グレー）、最終ページ → Next 非活性

**`application/backend/src/presentation.rs`**
- `GET /` ハンドラでクエリパラメータ `page` を受け取る（デフォルト: 1）
- `list_posts` 結果を `PER_PAGE=10` でスライス
- `render_posts_page(posts, page, total_pages)` を呼び出す

**`GET /tags/<tag>` も同様に対応**

---

## v6-B: Analytics 管理画面

### 変更ファイル

**`application/backend/src/presentation.rs`**
- `GET /admin/analytics` ルートを追加
- `authenticate_admin` ミドルウェアを適用
- `AzuriteTableClient::query_entities` で `analyticspv` をクエリ（過去30日）
- Rust 側で集計（スラグ別 PV、検索クエリ）
- HTML レスポンス（棒グラフは ASCII 風テキストで簡略実装）

**テストデータ投入スクリプト**
- `scripts/insert_analytics_testdata.sh` を作成
- `az storage entity insert` で 3 日分のダミーデータを投入（Azurite）

---

## v6-E: OGP / SEO 強化

**`application/frontend/src/lib.rs`**
- `wrap_document_inner` に `og_title`, `og_description`, `og_image`, `og_url` パラメータを追加
- 各 render 関数（`render_post_page`, `render_posts_page`）から適切な値を渡す
- `<meta property="og:*">` タグを `<head>` に挿入

---

## v6-D: ステージング環境（Terraform）

**`terraform/main.tf`**
- `module "app_dev"` ブロックを追加（`prefix = "rustacian-dev"`, CPU 0.25, メモリ 0.5Gi）
- dev 専用 Storage Account を追加

**`.github/workflows/deploy-dev.yml`**
- feature ブランチへの push で dev Container App を更新

---

## ローカル開発フロー

```bash
# Azurite 起動（必須）
docker compose up -d

# サーバー起動
cargo run -p rustacian_blog_backend

# ブラウザで確認
open http://localhost:8080
```

各マイルストーンでローカル確認してから次フェーズへ進む。
