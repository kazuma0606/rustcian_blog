# v6.5 タスクリスト

> 作成: 2026-03-28

---

## 作業ブランチ・デプロイフロー

```
1. v6.5_dev ブランチを切る（master から）
2. 各マイルストーンを実装・テスト
3. v6.5_dev を push → deploy-dev.yml が自動実行
4. DEV 環境（rustacian-dev-ca）でページ表示を目視確認
5. 確認後に master へマージ → CI + deploy.yml が自動実行
6. prod 環境（rustacian-blog.com）で最終確認
```

### ブランチ準備
- [x] `git checkout master && git pull origin master`
- [x] `git checkout -b v6.5_dev`

### 各マイルストーン共通の push 手順
- [x] `git push origin v6.5_dev` → Deploy Dev 完了を確認
- [x] DEV 環境で対象ページを目視確認（下記「DEV 確認」参照）
- [x] master マージ → prod で最終確認（下記「prod 確認」参照）

---

## M4: 記事ページへのコメント埋め込み

### バックエンド層
- [x] `post_page` ハンドラで `comment_repo.list_comments(&slug, false)` を呼び承認済みコメントを取得
- [x] `render_post_page` の引数に `comments: Vec<CommentView>` を追加
- [x] `post_comment` ハンドラのリダイレクト先を `/p/{slug}#comments` に変更

### フロントエンド層
- [x] `render_post_page` のシグネチャに `comments: Vec<CommentView>` を追加
- [x] `PostPage` コンポーネント末尾に `<section id="comments">` を追加
  - [x] 承認済みコメント一覧（名前・日時・本文）
  - [x] 投稿フォーム（名前・本文・投稿ボタン）
  - [x] コメント 0 件時のメッセージ表示

### 単体テスト（`application/frontend/`）
- [x] `render_post_page` にコメントありで渡したとき `<section id="comments">` が HTML に含まれる
- [x] `render_post_page` にコメントありで渡したとき著者名・本文が HTML に含まれる
- [x] `render_post_page` にコメント空リストで渡したとき「まだコメントはありません」が HTML に含まれる
- [x] `render_post_page` の HTML に `<form` および投稿ボタンが含まれる

### 結合テスト（`application/backend/` — `#[actix_web::test]`）
- [x] `GET /p/{slug}` が 200 を返し、HTML に `id="comments"` が含まれる
- [x] `GET /p/{slug}` で承認済みコメントがある場合、著者名が HTML に含まれる
- [x] `GET /p/{slug}` で承認済みコメントがない場合、「まだコメントはありません」が HTML に含まれる
- [x] `POST /posts/{slug}/comments` が成功すると `/p/{slug}#comments` にリダイレクトされる
- [x] `POST /posts/{slug}/comments` で必須フィールド欠損時に 400 を返す

### ローカル確認 🔲
- [x] `cargo fmt --all --check` が通る
- [x] `cargo clippy --workspace --all-targets -- -D warnings` が通る
- [x] `cargo test` が通る
- [x] ローカルサーバーで記事ページにコメントセクションが表示される
- [ ] ローカルでコメント投稿 → 管理画面で承認 → 記事ページに反映される

### DEV 確認 🔲
- [x] `git push origin v6.5_dev` → Deploy Dev 成功
- [x] DEV の記事ページ（`/p/{slug}`）にコメントセクションが表示される
- [x] DEV でコメント投稿フォームが表示される
- [x] DEV でコメント投稿 → 管理画面で承認 → 記事ページに反映される

### prod 確認 🔲（master マージ後）
- [x] master マージ → CI + Deploy 成功
- [x] prod の記事ページ（`/p/{slug}`）にコメントセクションが表示される
- [x] prod でコメント投稿フォームが表示される

---

## M1: `application/search/` クレート分離

### クレート作成
- [x] `application/search/Cargo.toml` を作成（依存: `tantivy` のみ）
- [x] ルート `Cargo.toml` のワークスペースメンバーに追加
- [x] `SearchHit`、`SearchQuery`、`SearchPage` 構造体を定義
- [x] `IndexStorage` trait を定義
- [x] `SearchEngine::new()` を実装
- [x] `SearchEngine::rebuild(posts: &[PostDoc])` を実装
- [x] `SearchEngine::search(query: &SearchQuery) -> Result<SearchPage>` を実装
- [x] `SearchEngine::save_to(storage: &dyn IndexStorage)` を実装
- [x] `SearchEngine::load_from(storage: &dyn IndexStorage)` を実装

### backend 側の差し替え
- [x] `application/backend/Cargo.toml` に `rustacian_blog_search` を追加
- [x] `LocalIndexStorage` を `backend/src/search_storage.rs` に実装
- [x] `BlobIndexStorage` を `backend/src/search_storage.rs` に実装
- [x] `backend/src/state.rs` の `TantivySearchIndex` を `SearchEngine` に差し替え
- [x] `backend/src/search.rs` を削除
- [x] `backend/src/presentation.rs` の import を更新

### 単体テスト（`application/search/`）
- [x] 既存 `backend/src/search.rs` のテストを移植（5件）
  - [x] インデックスにマッチする記事が返る
  - [x] マッチしない場合は空リストが返る
  - [x] draft 記事はインデックスされない
  - [x] 空クエリは空リストを返す
  - [x] `rebuild` で古いインデックスが置き換えられる
- [x] `SearchQuery.page` / `per_page` によるページネーションが正しく動作する
  - [x] `total`・`total_pages` が正確に計算される
  - [x] `page=2` で正しいオフセットのヒットが返る
- [x] AND 検索（`"rust AND azure"`）で両方含む記事のみヒットする
- [x] OR 検索（`"rust OR leptos"`）でいずれかを含む記事がヒットする
- [x] `tags:rust` クエリでタグ一致記事のみヒットする
- [x] `save_to` → `load_from` でインデックスが再現され同じ結果が返る（`MockIndexStorage` を用意）

### ローカル確認 🔲
- [x] `cargo fmt --all --check` が通る
- [x] `cargo clippy --workspace --all-targets -- -D warnings` が通る
- [x] `cargo test` が通る（既存テストがすべて通ること）
- [ ] ローカルサーバーで検索機能が既存と同様に動作する（リグレッションなし）

### DEV 確認 🔲
- [x] `git push origin v6.5_dev` → Deploy Dev 成功
- [x] DEV の `/search?q=rust` が正常に動作する（クレート分離後もリグレッションなし）

### prod 確認（M2 完了・master マージ後にまとめて実施）
- [x] M2 の prod 確認と合わせて行う

---

## M2: index.html 検索フォーム + 結果表示

### バックエンド層
- [x] `PageQuery` に `q: Option<String>` を追加
- [x] `index_page` ハンドラに検索分岐を追加
  - [x] `q` が空 → 既存の記事一覧（ページネーション維持）
  - [x] `q` がある → `SearchEngine::search()` を呼び結果を返す
  - [x] `q` がある場合も `?q=...&page=N` でページネーション
  - [x] `analyticsqueries` への記録を追加（既存 `/search` と同じ）
- [x] `render_posts_page` の引数に `query: &str`, `search_results: Option<Vec<SearchResultView>>` を追加

### フロントエンド層
- [x] `PostsPage` コンポーネントに `query: String`, `search_results: Option<Vec<SearchResultView>>` を追加
- [x] hero と記事一覧の間に検索フォームを追加
  - [x] `value="{query}"` で入力値を維持
  - [x] placeholder に AND/OR 構文のヒントを表示
- [x] `search_results` が `Some` の場合
  - [x] 件数表示「N 件の検索結果」
  - [x] 検索結果を `.card` スタイルで描画（タイトル・抜粋・タグ・日付）
  - [x] 0 件時のメッセージ表示
  - [x] ページネーションを検索結果用に表示（`?q=...&page=N`）
- [x] `render_tag_posts_page` の呼び出し側も新しいシグネチャに対応

### 単体テスト（`application/frontend/`）
- [x] `render_posts_page` の HTML に `<form` と検索入力欄が含まれる
- [x] `render_posts_page` に `query="rust"` を渡すと input の `value="rust"` が HTML に含まれる
- [x] `render_posts_page` に `search_results=None` を渡すと通常の記事カードが表示される
- [x] `render_posts_page` に `search_results=Some([...])` を渡すと「N 件の検索結果」が HTML に含まれる
- [x] `render_posts_page` に空の `search_results=Some([])` を渡すと「見つかりませんでした」が HTML に含まれる

### 結合テスト（`application/backend/` — `#[actix_web::test]`）
- [x] `GET /?q=`（空）が 200 を返し、通常の記事一覧 HTML が返る
- [x] `GET /?q=rust` が 200 を返し、HTML に検索フォームが含まれる
- [x] `GET /?q=rust` で検索フォームの `value` に `rust` が含まれる
- [x] `GET /?q=nonexistent_xyz` で「見つかりませんでした」相当のメッセージが HTML に含まれる
- [x] `GET /?q=rust&page=1` が 200 を返す
- [x] `GET /?q=rust&page=99` が 200 を返す（範囲外ページでクラッシュしない）

### ローカル確認 🔲
- [x] `cargo fmt --all --check` が通る
- [x] `cargo clippy --workspace --all-targets -- -D warnings` が通る
- [x] `cargo test` が通る
- [ ] ローカルサーバーでキーワード検索が動作する
- [ ] ローカルで `rust AND azure` の AND 検索が動作する
- [ ] ローカルで `tags:rust` のタグ検索が動作する
- [ ] ローカルで検索結果のページネーションが動作する
- [ ] ローカルで検索後に入力値がフォームに残る

### DEV 確認 🔲
- [x] `git push origin v6.5_dev` → Deploy Dev 成功
- [x] DEV の `index.html` に検索フォームが表示される
- [x] DEV でキーワード検索 → 検索結果カードが表示される
- [x] DEV で AND/OR 検索が動作する（例: `rust AND azure`）
- [x] DEV で `tags:rust` タグ検索が動作する
- [x] DEV で検索結果のページネーションが動作する
- [x] DEV で検索後にフォームの入力値が維持されている
- [x] DEV で 0 件検索時のメッセージが表示される

### prod 確認（master マージ後）
- [x] master マージ → CI + Deploy 成功
- [x] prod の `index.html` に検索フォームが表示される
- [x] prod でキーワード検索 → 検索結果カードが表示される
- [x] prod で `/search?q=rust` も引き続き動作する（既存エンドポイント維持）

---

## M3: バッチインデクサ

### `application/search-indexer/` 作成
- [x] `application/search-indexer/Cargo.toml` を作成
- [x] ルート `Cargo.toml` のワークスペースメンバーに追加
- [x] 環境変数から Blob 接続情報を取得
- [x] Blob から `posts/index.json` を読み込んで記事一覧を構築
- [x] `SearchEngine::rebuild()` でインデックス構築
- [x] `BlobIndexStorage::save()` でインデックスを Blob にアップロード
- [x] 完了ログを標準出力に出力

### バックエンド起動時のロード
- [x] `main.rs` 起動時に `BlobIndexStorage::load()` → `SearchEngine::load_from()` を呼ぶ
- [x] Blob に index がない場合はインメモリ再構築にフォールバック

### CI/CD
- [x] `.github/workflows/search-index.yml` を作成（`schedule: cron: '0 * * * *'`）
- [x] ワークフローで `search-indexer` バイナリをビルド・実行

### 単体テスト（`application/search/`）
- [x] `MockIndexStorage` を使い `save_to` → `load_from` でインデックスが復元できる（M1 で実装済みであれば流用）
- [x] `BlobIndexStorage` の `save` / `load` が Azurite（ローカル）で動作する

### 結合テスト（`application/backend/` — `#[actix_web::test]`）
- [x] バックエンド起動時に Blob に index があれば検索が即座に動作する
- [x] Blob に index がない場合でも `GET /` が 200 を返す（フォールバック確認）

### ローカル確認 🔲
- [x] `cargo fmt --all --check` が通る
- [x] `cargo clippy --workspace --all-targets -- -D warnings` が通る
- [x] `cargo test` が通る
- [ ] `search-indexer` を手動実行してインデックスが Azurite Blob に保存される
- [ ] ローカルでバックエンド再起動後もインデックスが復元され検索できる

### DEV 確認 🔲
- [x] `git push origin v6.5_dev` → Deploy Dev 成功
- [ ] GitHub Actions の `search-index.yml` を手動実行（`workflow_dispatch`）してインデックスが Blob に保存される
- [ ] DEV のバックエンドを再起動後も検索が動作する（Blob からロード確認）
- [ ] DEV で cron が動作することを次の定期実行タイミングで確認

### prod 確認（master マージ後）
- [x] master マージ → CI + Deploy 成功
- [x] `search-index.yml` を手動実行して prod Blob にインデックスが保存される
- [x] prod で検索が動作する
- [ ] prod で次回 cron 実行後も検索が維持される（次の毎時 cron で自動確認）

---

## 強化案（M5 以降・レビュー後に着手判断）

### v6.5-A: タグクラウド
- [ ] `index_page` で全タグ一覧を取得して `render_posts_page` に渡す
- [ ] 検索フォーム下にタグピルを表示（クリックで `/?q=tags:{tag}` へ遷移）
- [ ] 単体テスト: `render_posts_page` にタグ一覧を渡すと `/?q=tags:rust` リンクが HTML に含まれる

### v6.5-B: 人気検索ワード
- [ ] `analytics_reader` から上位クエリを取得して `render_posts_page` に渡す
- [ ] 検索フォーム下に人気キーワードをリンク表示
- [ ] 単体テスト: `render_posts_page` に人気クエリを渡すとリンクが HTML に含まれる

### v6.5-C: 関連記事
- [ ] `post_page` ハンドラでタグ + タイトル語をクエリにして `SearchEngine::search()` を呼ぶ
- [ ] `PostPage` 末尾（コメントセクションの前）に関連記事カードを 2〜3 件表示
- [ ] 単体テスト: `render_post_page` に関連記事リストを渡すとカードが HTML に含まれる
- [ ] 結合テスト: `GET /p/{slug}` の HTML に関連記事セクションが含まれる

### v6.5-D: co-read グラフ
- [ ] `analyticssessions` からセッション連鎖データを集計
- [ ] SVG グラフとして記事間のリンクを可視化
