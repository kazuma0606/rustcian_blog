# v6 仕様書

> 作成: 2026-03-23

---

## 確定機能（ユーザー要件）

### v6-A: 記事カードのリデザイン

**現状**: 縦並び（ヒーロー画像フルwidth → タイトル → 要約 → タグ）

**目標**: Zenn 風の横並びカード

```
┌──────────────────────────────────────────────────────┐
│  ┌─────────┐  タイトル                               │
│  │         │  2026-03-23                             │
│  │  hero   │  要約テキスト（description）...         │
│  │ or BLOG │                                         │
│  │  128px  │  [tag1] [tag2]                          │
│  └─────────┘                                         │
└──────────────────────────────────────────────────────┘
```

**実装内容**:
- サムネイル: `hero_image` があれば表示、なければ `"BLOG"` テキストのプレイスホルダー（グレー背景 + 白文字）
- サムネイルサイズ: 128×96px 固定（4:3）、`object-fit: cover`
- 右側: タイトル、公開日、`description`（後述）、タグ
- レスポンシブ: スマホでは縦並びにフォールバック
- `summary_ai` があれば description の代わりに使用（既存挙動を維持）

**`description` フィールドの追加**:
- `PostMetadata` / `PostSummaryView` に `description: Option<String>` を追加
- `meta.yml` で任意指定。未指定時は `summary` にフォールバック
- 優先順位: `summary_ai` > `description` > `summary`

---

### v6-B: Analytics 管理画面

**現状**: `AnalyticsWriter` が Table Storage に PV / 検索 / 読了ステップを書き込むのみ。閲覧 UI なし。

**実装内容**:

`GET /admin/analytics` に以下を表示:

```
┌── 過去 30 日 ─────────────────────────────────────────┐
│  総 PV: 1,234  ユニークIP: 456  検索数: 89            │
└────────────────────────────────────────────────────────┘

記事別 PV ランキング（上位 10）
┌─────────────────────────────────────┬──────┐
│ 記事タイトル                        │  PV  │
├─────────────────────────────────────┼──────┤
│ Building with Actix Web and Leptos  │  312 │
│ Hello Rustacian Blog                │  201 │
└─────────────────────────────────────┴──────┘

人気検索クエリ（上位 10）
検索語: "actix" (23回), "leptos" (18回), ...
```

- 集計は Table Storage の `analyticspv` / `analyticsqueries` テーブルを直接クエリ
- 当月のみの集計（複雑な時系列グラフは v7 以降）
- Entra ID 認証済みの管理者のみアクセス可

---

### v6-C: ステージング環境（dev）

**現状**: prod 環境のみ（`rustacian-prod-*`）。

**目標**: dev 環境を Terraform で追加し、master push → prod、feature branch → dev に自動デプロイ。

**実装内容**:

Terraform:
- `environment = "dev"` で同じモジュール構成を追加（prefix: `rustacian-dev-*`）
- スペック縮小: `container_cpu = 0.25`, `container_memory = "0.5Gi"`, `min_replicas = 0`, `max_replicas = 1`
- `ADMIN_AUTH_MODE = local-dev`（dev 環境は Basic 認証で簡略化）
- dev 専用ストレージアカウント（`rustaciandevst`）

GitHub Actions:
- `ci.yml`: feature branch でも Docker build → dev ACR push
- `deploy-dev.yml`: dev Container App のイメージ更新（PR マージ or 手動）
- dev 用 GitHub Secrets: `DEV_CONTAINER_APP_NAME`, `DEV_RESOURCE_GROUP` など

ドメイン:
- `dev.rustacian-blog.com` または Container App の FQDN を直接使用（Cloudflare は prod のみ）

---

## 提案機能（優先度順）

### v6-D: ページネーション（記事一覧）

現状は全記事が1ページに表示。記事が増えると UX 低下。

- `GET /?page=1&per=10` 形式
- 「← 前のページ」「次のページ →」ナビゲーション
- 静的サイト生成時も page ディレクトリに対応

---

### v6-E: OGP / SEO 強化

現状: OGP タグなし → SNS シェア時に画像なし。

- `<meta property="og:title">`, `og:description`, `og:image` を各ページに追加
- `og:image`: hero_image があれば使用、なければデフォルト画像
- `<meta name="twitter:card" content="summary_large_image">` も追加

---

### v6-F: 読了時間の表示

- 本文の文字数から概算（日本語: 400字/分、英語: 200words/分）
- 記事カードと記事ページの両方に「約 X 分で読めます」を表示

---

### v6-G: 関連記事

- 記事詳細ページの末尾に、同タグの記事を最大3件表示
- `list_posts` の結果をタグでフィルタするだけで実装可能

---

### v6-H: コメント管理 UI（admin）

現状: コメントは Table Storage に保存されるが、管理画面から確認・削除する UI がない。

- `GET /admin/comments`: コメント一覧（記事・日時・内容）
- `DELETE /admin/comments/{id}`: コメント削除

---

## 実装優先度（提案）

| 優先度 | 機能 | 理由 |
|---|---|---|
| 🔴 高 | v6-A 記事カード | UX に直結、コンテンツが増える前に対応すべき |
| 🔴 高 | v6-B Analytics | データはすでに蓄積中。早めに見れるようにしたい |
| 🟡 中 | v6-C ステージング | 今後の開発で必須になる |
| 🟡 中 | v6-E OGP | SNS からの流入に直結 |
| 🟡 中 | v6-F 読了時間 | 実装コスト低・UX 向上 |
| 🟢 低 | v6-D ページネーション | 記事10件以下のうちは不要 |
| 🟢 低 | v6-G 関連記事 | 記事が増えてから |
| 🟢 低 | v6-H コメント管理 | コメント機能の利用状況次第 |
