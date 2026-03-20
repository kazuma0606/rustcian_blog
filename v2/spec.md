# Rustacian Blog v2 仕様書

## 1. 位置づけ
この文書は v2 の実装仕様書である。
`v2/plan.md` と `v2/tasks.md` を実装した結果として、現在のコンテンツ構成、表示仕様、補助機能、管理導線の振る舞いをまとめる。

## 2. 前提環境
- Rust / Cargo が利用できること
- Docker / Docker Compose が利用できること
- リポジトリルートが `rustacian_blog/` であること

## 3. ディレクトリ構成
```text
rustacian_blog/
├── application/
│   ├── core/
│   ├── backend/
│   └── frontend/
├── content/
│   ├── posts/
│   │   └── <slug>/
│   │       ├── post.md
│   │       ├── meta.yml
│   │       └── <assets>
│   ├── images/
│   ├── metadata/
│   └── tags.yml
├── v2/
│   ├── plan.md
│   ├── tasks.md
│   └── spec.md
└── .env.local.example
```

## 4. 起動方式

### 4.1 Azurite を起動する
```powershell
docker compose up -d
```

停止:

```powershell
docker compose down
```

### 4.2 バックエンドを起動する
```powershell
cargo run -p rustacian_blog_backend
```

既定の待受:
- `http://127.0.0.1:8080`

## 5. 設定値

### 5.1 基本設定
```env
APP_ENV=local
APP_HOST=127.0.0.1
APP_PORT=8080
STORAGE_BACKEND=azurite
CONTENT_ROOT=./content
AZURITE_BLOB_ENDPOINT=http://127.0.0.1:10000/devstoreaccount1
AZURITE_TABLE_ENDPOINT=http://127.0.0.1:10002/devstoreaccount1
```

### 5.2 AI 補助設定
```env
AZURE_OPENAI_ENDPOINT=
AZURE_OPENAI_DEPLOYMENT=
AZURE_OPENAI_API_KEY=
AZURE_OPENAI_API_VERSION=2024-10-21
AZURE_OPENAI_MODEL_NAME=
```

### 5.3 管理 preview 設定
```env
ADMIN_AUTH_MODE=disabled
ENTRA_TENANT_ID=
ENTRA_CLIENT_ID=
ENTRA_ADMIN_GROUP_ID=
ENTRA_ADMIN_USER_OID=
```

備考:
- `ADMIN_AUTH_MODE=entra-poc` の場合のみ管理 preview 認証を有効化する
- 現段階は PoC のため、Entra ID の署名検証ではなく JWT claim の確認に留める

## 6. コンテンツ仕様

### 6.1 記事構成
記事は `content/posts/<slug>/` 単位で管理する。

必須ファイル:
- `post.md`
- `meta.yml`

任意ファイル:
- `hero.png` などの画像
- `*.csv` などの図表データ

### 6.2 `meta.yml` 仕様
必須項目:
- `title`
- `slug`
- `published_at`
- `status`
- `tags`
- `summary`

推奨項目:
- `updated_at`
- `hero_image`
- `toc`
- `math`

拡張項目:
- `summary_ai`
- `charts`

例:

```yaml
title: Example Post
slug: example-post
published_at: 2026-03-20T00:00:00Z
updated_at: 2026-03-21T12:00:00Z
status: published
tags:
  - rust
summary: サンプル記事
hero_image: ./hero.png
toc: true
math: true
charts:
  - type: line
    source: ./metrics.csv
    x: step
    y: ms
    title: Response latency
    caption: Sample chart
```

### 6.3 `meta.yml` バリデーション
- `slug` は英小文字、数字、ハイフンのみを許可する
- `status` は `draft | published` のみを許可する
- `updated_at` は `published_at` 以上である必要がある
- `summary_ai` は存在する場合、空文字を許可しない
- `charts[*].type` は `line | bar | scatter` のみを許可する
- `charts[*].source`, `x`, `y` は必須とする
- `hero_image` と `charts[*].source` は参照先ファイルの存在を検証する
- `content/tags.yml` が存在する場合、記事タグは辞書内定義と一致する必要がある
- slug 重複は読込時に検証エラーとする

### 6.4 補助メタデータ
AI 補助メタは `content/metadata/<slug>.json` に保存する。

保存項目:
- `summary_ai`
- `suggested_tags`
- `intro_candidates`
- `generated_at`
- `source_model`

`meta.yml` に `summary_ai` が無く、補助 JSON に値がある場合は読込時に補完する。

## 7. 公開 / 下書き仕様

### 7.1 公開判定
- `status: published` の記事のみ公開面で表示する
- `status: draft` は公開一覧、公開詳細、公開 API には出さない

### 7.2 管理 preview
- `/admin/preview/{slug}` は管理 preview 用 route とする
- 認証通過後のみ `draft` を含めて取得する
- 認証未設定時は `501 Not Implemented`
- Bearer token 不在または不正時は `401 Unauthorized`
- group / user 条件不一致時は `403 Forbidden`

## 8. API / 画面仕様

### 8.1 公開 API
- `GET /health`
- `GET /posts`
- `GET /posts/{slug}`

公開 API は `published` のみ返す。

### 8.2 公開画面
- `GET /`
- `GET /p/{slug}`

一覧画面:
- 公開日
- 更新日
- title
- summary
- tags
- TOC / Math バッジ
- hero image

詳細画面:
- title
- summary
- 公開日
- 更新日
- tags
- TOC
- hero image
- AI summary
- charts
- rendered markdown

### 8.3 管理画面相当の route
- `GET /admin/preview/{slug}`

## 9. アセット配信仕様
- `content/images/` は `/images/...` として配信する
- 記事ディレクトリ配下のアセットは `/assets/posts/<slug>/...` として配信する
- 本文中の `src="./..."`, `href="./..."` は詳細画面描画時に `/assets/posts/<slug>/...` へ解決する
- `hero_image` と chart source も記事 slug 基準で URL 解決する

## 10. 数式表示仕様

### 10.1 有効化条件
- `meta.yml` の `math: true`
- または本文中に数式記法を検出した場合

### 10.2 記法
インライン数式:
- `$...$`
- `\(...\)`

ブロック数式:
- `$$ ... $$`
- `\[...\]`

### 10.3 エスケープ
- `\$` は価格表記などの通常文字として扱う

### 10.4 描画方式
- frontend は KaTeX と auto-render を読み込む
- backend は Markdown 前処理で数式記法を保護してから HTML 化する
- KaTeX 失敗時は raw 記法をそのまま読めるフォールバック表示を行う

## 11. CSV / 図表仕様

### 11.1 図表定義
`meta.yml` の `charts` で指定する。

項目:
- `type`
- `source`
- `x`
- `y`
- `title`
- `caption`

### 11.2 CSV 仕様
- 先頭行はヘッダ必須
- `x` と `y` に指定した列名が存在する必要がある
- `y` 列は数値変換できる必要がある
- データ行 0 件はエラー

### 11.3 表示方式
- backend で CSV を読み、`chart_data` に正規化する
- frontend で SVG により `line`, `bar`, `scatter` を描画する
- title, caption, 軸ラベルを詳細画面に表示する

## 12. AI 補助仕様

### 12.1 境界
Core:
- `AiMetadataGenerator`
- `GeneratedMetadataStore`
- `GenerateAiMetadataUseCase`

Backend:
- Azure OpenAI adapter
- local JSON store

### 12.2 生成対象
- 記事要約
- タグ候補
- 導入文候補

### 12.3 生成フロー
- 記事本体を `IncludeDrafts` で取得する
- generator が補助メタを生成する
- store が `content/metadata/<slug>.json` に保存する
- 表示時に毎回推論は行わない

## 13. 管理認証仕様

### 13.1 `entra-poc`
`ADMIN_AUTH_MODE=entra-poc` の場合、Bearer token の payload claim を確認する。

検証項目:
- `tid` が `ENTRA_TENANT_ID` と一致すること
- `aud` が `ENTRA_CLIENT_ID` と一致すること
- `groups` に `ENTRA_ADMIN_GROUP_ID` が含まれること
  または `oid` が `ENTRA_ADMIN_USER_OID` と一致すること

### 13.2 制約
- JWT 署名検証は未実装
- OIDC metadata 取得は未実装
- 本番利用前に正式な Entra ID / OIDC 検証へ置き換えること

## 14. 確認手順

### 14.1 品質確認
```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test
```

### 14.2 手動確認
```powershell
http://127.0.0.1:8080/
http://127.0.0.1:8080/p/hello-rustacian-blog
http://127.0.0.1:8080/p/actix-and-leptos
```

確認項目:
- `draft` が公開面に出ないこと
- `hello-rustacian-blog` でインライン数式とブロック数式が表示されること
- `actix-and-leptos` で chart が表示されること
- hero image や記事内相対アセットが解決されること

## 15. 制約
- Entra ID 認証は PoC であり、本番用の署名検証は未実装
- Azure OpenAI 呼び出しは adapter 境界までで、管理 UI からの起動導線は未実装
- chart 描画は簡易 SVG であり、高機能 chart library ではない
- 投稿 UI や一般公開向け更新 API は未実装
