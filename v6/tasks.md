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

- [ ] `scripts/insert_analytics_testdata.sh` を作成（Azurite 向けダミーデータ投入）
- [ ] テストデータを Azurite に投入して確認
- [ ] `GET /admin/analytics` ルートを追加
- [ ] `authenticate_admin` ミドルウェアを適用
- [ ] `analyticspv` テーブルから過去 30 日のデータを取得
- [ ] `analyticsqueries` テーブルから過去 30 日の検索クエリを取得
- [ ] Rust 側でスラグ別 PV・検索クエリを集計
- [ ] Admin UI に Analytics 画面を描画（テキスト棒グラフ）

### ローカル確認 🟢
- [ ] テストデータが投入済みの状態で `/admin/analytics` の表示を確認

---

## マイルストーン 4: OGP / SEO 強化（v6-E）

- [ ] `wrap_document_inner` に OGP パラメータを追加
- [ ] 記事ページに `og:title`, `og:description`, `og:image`, `og:url`, `og:type` を追加
- [ ] トップページに `og:type=website`, デフォルト OGP 画像を追加
- [ ] `twitter:card` メタタグを追加

### 確認 🟢
- [ ] ページソースで OGP タグを確認

---

## マイルストーン 5: ステージング環境（v6-D）

- [ ] `terraform/main.tf` に `module "app_dev"` を追加
- [ ] dev 専用 Storage Account `rustaciandevst` を追加
- [ ] `.github/workflows/deploy-dev.yml` を作成
- [ ] dev 用 GitHub Secrets を設定（`DEV_CONTAINER_APP_NAME`, `DEV_RESOURCE_GROUP`）
- [ ] `terraform plan` で差分を確認してから apply

### 確認 🟢
- [ ] dev Container App の FQDN にアクセスして動作確認
