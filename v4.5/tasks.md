# v4.5 Tasks: application/analytics マイクロサービス

## Phase 1 — クレートスケルトン

- [x] `v4.5/plan.md` を作成する
- [x] `v4.5/tasks.md` を作成する
- [x] `application/analytics/` ディレクトリを作成する
- [x] `application/analytics/Cargo.toml` を作成する
- [x] `application/analytics/src/config.rs` を作成する（`AnalyticsConfig`）
- [x] `application/analytics/src/table.rs` を作成する（Azurite Table Storage クライアント）
- [x] `application/analytics/src/store.rs` を作成する（Table Storage 読み込み）
- [x] `application/analytics/src/presentation.rs` を作成する（HTTP ルート）
- [x] `application/analytics/src/lib.rs` を作成する
- [x] `application/analytics/src/main.rs` を作成する
- [x] workspace `Cargo.toml` に `application/analytics` を追加する
- [x] サーバー起動時に 3 テーブルを初期化する（`analyticspv` / `analyticsqueries` / `analyticssessions`）
- [x] `GET /health` を実装する

## Phase 2 — PV 集計（backend → analytics）

- [x] `application/backend/src/analytics_writer.rs` を作成する（`AnalyticsWriter` 構造体）
- [x] IP を SHA-256 でハッシュ化する（daily salt 付き、Cookie 不要）
- [x] `AppState` に `analytics: Option<Arc<AnalyticsWriter>>` を追加する
- [x] `main.rs` で `AnalyticsWriter` を初期化する（`AZURITE_TABLE_ENDPOINT` 設定時のみ）
- [x] 公開記事ページ（`GET /posts/{slug}`）でページビューを fire-and-forget 書き込みする
- [x] analytics service: `GET /api/popular?days=N&limit=N` を実装する
- [x] analytics service: `GET /api/summary?days=N` を実装する

## Phase 3 — 検索クエリログ

- [x] `search_handler` で検索クエリと結果件数を `analyticsqueries` テーブルに書き込む
- [x] analytics service: `GET /api/gaps?days=N` を実装する（ゼロヒットクエリ一覧）

## Phase 4 — 一緒に読まれているグラフ

- [ ] `analyticssessions` テーブルにページ遷移を記録する（セッション = IP hash + date）
- [ ] analytics service: `GET /api/coread/{slug}` を実装する（共起スラッグ一覧）
- [ ] `POST /admin/static/regenerate` 時に各記事の `related.json` を生成して静的ファイルに含める

## Phase 5 — OpenAI コンテンツギャップ分析

- [ ] `GET /api/gaps` の結果を Azure OpenAI に送り「不足コンテンツトピック案」を生成する
- [ ] analytics service に `AZURE_OPENAI_*` 環境変数を追加する
- [ ] `GET /api/suggestions` を実装する（OpenAI 生成のコンテンツ提案）

## Phase 6 — Admin UI 統合

- [ ] analytics service: `GET /dashboard` を実装する（SSR HTML ダッシュボード）
- [ ] backend admin: `/admin` ダッシュボードに analytics サービスへのリンクを追加する
- [ ] `.env.local.example` に `ANALYTICS_HOST` / `ANALYTICS_PORT` を追記する
