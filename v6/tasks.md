# v6 タスクリスト

> 作成: 2026-03-23

---

## マイルストーン 1: 記事カードリデザイン + 読了時間（v6-A + v6-F）

### ドメイン層
- [x] `PostMetadata` に `description: Option<String>` を追加
- [x] `Post` に `description: Option<String>`, `read_minutes: usize` を追加
- [x] `PostSummary` に `description: Option<String>`, `read_minutes: usize` を追加
- [x] `estimate_read_minutes(body: &str) -> usize` 関数を実装
- [x] `Post::new()` で `read_minutes` を計算して設定
- [x] `Post::summary()` で `description`, `read_minutes` を伝播

### フロントエンド層
- [x] `PostSummaryView` に `description: Option<String>`, `read_minutes: usize` を追加
- [x] CSS `.card` をフレックスレイアウト（横並び）に変更
- [x] `.card-thumb`, `.card-thumb-placeholder`, `.card-body` CSS クラスを追加
- [x] `PostsPage` カード HTML を書き換え（サムネイル左、タイトル+説明右）

### バックエンド層
- [x] `PostSummary` → `PostSummaryView` 変換に `description`, `read_minutes` を追加

### ローカル確認 🟢
- [x] `cargo fmt --all --check` が通る
- [x] `cargo clippy --workspace --all-targets -- -D warnings` が通る
- [x] `cargo test` が通る
- [x] ローカルサーバーでカードデザインを確認

---

## マイルストーン 2: ページネーション（v6-C）

- [x] `render_posts_page` に `page: usize, total_pages: usize` パラメータを追加
- [x] `PostsPage` コンポーネントにページネーション UI（← Prev / N / Total / Next →）を追加
- [x] page=1 のとき Prev を非活性にする
- [x] 最終ページのとき Next を非活性にする
- [x] 空リストでもページネーションボタンを表示する
- [x] バックエンド `GET /` に `?page=N` クエリパラメータを追加
- [x] `PER_PAGE = 10` で posts をスライス
- [ ] `GET /tags/<tag>` にもページネーションを追加（動的配信にタグページ未実装のため後回し）

### ローカル確認 🟢
- [x] `cargo fmt --all --check` が通る
- [x] `cargo clippy --workspace --all-targets -- -D warnings` が通る
- [x] `cargo test` が通る
- [ ] ローカルサーバーでページネーション動作を確認（page=1 Prev 非活性など）

---

## マイルストーン 3: Analytics 管理画面 + テストデータ（v6-B）

- [x] `content/analytics/pv.csv`, `queries.csv` を作成（CSVテストデータ）
- [x] `analytics_reader.rs` を作成（CSV → Table Storage → NoData の優先順位で読み込み）
- [x] `GET /admin/analytics` ルートを追加
- [x] `authenticate_admin` ミドルウェアを適用
- [x] `analyticspv` / `analyticsqueries` テーブルからデータを取得（Table Storage fallback）
- [x] Rust 側でスラグ別 PV・検索クエリを集計
- [x] Admin UI に Analytics 画面を描画（CSS 棒グラフ + 統計カード）
- [x] `fmt`, `clippy`, `test` 通過

### ローカル確認 🟢
- [ ] `/admin/analytics` の表示を確認（CSVデータ表示確認）

---

## マイルストーン 4: OGP / SEO 強化（v6-E）

- [x] `build_ogp_meta` ヘルパーを追加（`og:type`, `og:title`, `og:description`, `og:image`, `og:url`, `og:site_name`, `twitter:card`, `name=description`）
- [x] `render_post_page` に `base_url: &str` を追加し記事ページに OGP タグを注入
- [x] `render_posts_page` / `render_tag_posts_page` / `render_tags_page` にも同様に追加
- [x] トップページに `og:type=website`, デフォルト OGP 画像（`/images/OGP.svg`）を追加
- [x] `twitter:card=summary_large_image` を追加
- [x] `fmt`, `clippy`, `test`（129 件）通過
- [ ] OGP.svg → PNG 変換（SNS クローラー対応）

### 確認 🟢
- [ ] ページソースで OGP タグを確認

---

## マイルストーン 5: ステージング環境（v6-D）

- [x] `terraform/main.tf` に `module "app_dev"` + `module "storage_dev"` + `azurerm_resource_group.dev` を追加
- [x] dev 専用 Storage Account `rustaciandevst`（monitoring/keyvault/openai/comms/registry は prod 共有）
- [x] `terraform/variables.tf` に `container_image_dev`, `base_url_dev` を追加
- [x] `terraform/terraform.tfvars` に dev セクション追加（初回デプロイ後に FQDN を記入）
- [x] `terraform/outputs.tf` に `dev_container_app_hostname`, `dev_storage_account_name` を追加
- [x] `.github/workflows/deploy-dev.yml` を作成（`v*_dev` / `develop` ブランチで自動起動、build + push + deploy）
- [ ] GitHub Secrets に `DEV_CONTAINER_APP_NAME`, `DEV_RESOURCE_GROUP` を設定（手動）
- [ ] `terraform plan` で差分を確認してから `terraform apply`（手動）
- [ ] 初回デプロイ後に `container_image_dev`, `base_url_dev` を tfvars に記入

### 確認 🟢
- [ ] dev Container App の FQDN にアクセスして動作確認
