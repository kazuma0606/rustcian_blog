# Rustacian Blog v1 仕様書

## 1. 位置づけ
この文書は Phase 1 Local PoC の実装仕様書である。
`v1/plan.md` と `v1/tasks.md` を実装した結果として、現在の起動方法、設定、コンテンツ配置、確認方法をまとめる。

## 2. 前提環境
- Rust / Cargo が利用できること
- Docker / Docker Compose が利用できること
- リポジトリのルートは `rustacian_blog/` であること

## 3. ディレクトリ構成
```text
rustacian_blog/
├── application/
│   ├── core/
│   ├── backend/
│   └── frontend/
├── content/
│   ├── posts/
│   └── images/
├── v1/
│   ├── plan.md
│   ├── tasks.md
│   └── spec.md
├── docker-compose.yml
└── .env.local.example
```

## 4. 起動方式

### 4.1 Azurite を起動する
リポジトリルートで以下を実行する。

```powershell
docker compose up -d
```

停止する場合は以下を実行する。

```powershell
docker compose down
```

### 4.2 ローカル設定ファイルを作成する
`.env.local.example` を元に `.env.local` を作成する。

```powershell
Copy-Item .env.local.example .env.local
```

`.env.local` がなくても、v1 実装では Azurite 用の既定値を使って起動できる。
ただし設定を明示したい場合は `.env.local` を作成すること。

### 4.3 バックエンドを起動する
リポジトリルートで以下を実行する。

```powershell
cargo run -p rustacian_blog_backend
```

既定では `http://127.0.0.1:8080` で待ち受ける。

## 5. 設定値
`.env.local` では以下を使用する。

```env
APP_ENV=local
APP_HOST=127.0.0.1
APP_PORT=8080
STORAGE_BACKEND=azurite
CONTENT_ROOT=./content
AZURITE_BLOB_ENDPOINT=http://127.0.0.1:10000/devstoreaccount1
AZURITE_TABLE_ENDPOINT=http://127.0.0.1:10002/devstoreaccount1
```

### 5.1 各設定の意味
- `APP_ENV`: 実行環境名
- `APP_HOST`: バックエンドの bind host
- `APP_PORT`: バックエンドの bind port
- `STORAGE_BACKEND`: `azurite` の場合は Azurite Blob を使う。その他はローカルファイル読みにフォールバックする
- `CONTENT_ROOT`: 記事と画像を配置するルート
- `AZURITE_BLOB_ENDPOINT`: Azurite Blob endpoint
- `AZURITE_TABLE_ENDPOINT`: 将来拡張用。v1 では health response に含めるのみ

## 6. コンテンツ配置仕様

### 6.1 記事ファイル
記事は `content/posts/` 配下に `.md` ファイルとして配置する。

例:
- `content/posts/hello-rustacian-blog.md`
- `content/posts/actix-and-leptos.md`

### 6.2 画像ファイル
画像は `content/images/` 配下に配置する。

例:
- `content/images/ferris-notes.svg`
- `content/images/stack-flow.svg`

画像は Actix Web により `/images/...` で静的配信される。

例:
- `content/images/ferris-notes.svg` -> `/images/ferris-notes.svg`

### 6.3 Markdown Frontmatter 仕様
記事ファイルは YAML frontmatter を先頭に持つ。

必須項目:
- `title`
- `slug`
- `published_at`
- `tags`
- `summary`

任意項目:
- `hero_image`

例:

```md
---
title: Hello Rustacian Blog
slug: hello-rustacian-blog
published_at: 2026-03-18T09:00:00Z
tags:
  - rust
  - architecture
summary: 最初のサンプル記事。Rust フルスタック構成の PoC の狙いをまとめる。
hero_image: /images/ferris-notes.svg
---

# Hello Rustacian Blog
```

### 6.4 `slug` の制約
- 小文字 ASCII
- 数字
- ハイフン `-`

空文字や大文字、空白を含む `slug` は不正とする。

## 7. ストレージ仕様

### 7.1 `STORAGE_BACKEND=azurite`
- 起動時に `content/posts/` の Markdown を Azurite Blob に seed する
- `posts/index.json` を manifest として作成する
- 記事本文は Blob から取得する
- 画像は Azurite ではなくローカル `content/images/` から静的配信する

### 7.2 `STORAGE_BACKEND` が `azurite` 以外
- `content/posts/` から直接 Markdown を読み込む
- これはローカル開発用のフォールバック実装である

## 8. API 仕様

### 8.1 Health Check
- Method: `GET`
- Path: `/health`

レスポンス例:

```json
{
  "status": "ok",
  "environment": "local",
  "storage_backend": "azurite",
  "azurite_blob_endpoint": "http://127.0.0.1:10000/devstoreaccount1",
  "azurite_table_endpoint": "http://127.0.0.1:10002/devstoreaccount1"
}
```

### 8.2 記事一覧 API
- Method: `GET`
- Path: `/posts`

レスポンスは `PostSummary` の配列で、`published_at` の降順とする。

### 8.3 記事詳細 API
- Method: `GET`
- Path: `/posts/{slug}`

レスポンスは `Post` を返す。
`body_markdown` と `body_html` の両方を含む。

## 9. 画面仕様

### 9.1 一覧画面
- Path: `/`
- Leptos SSR により HTML を生成する
- 記事カードの一覧を表示する
- 画像、公開日、タイトル、summary、tags を表示する

### 9.2 詳細画面
- Path: `/p/{slug}`
- Leptos SSR により HTML を生成する
- 記事タイトル、summary、公開日、tags、hero image を表示する
- Markdown は HTML に変換した結果を表示する

## 10. 動作確認手順

### 10.1 テスト実行
```powershell
cargo test
```

### 10.2 フォーマット確認
```powershell
cargo fmt --all --check
```

### 10.3 手動疎通確認
バックエンド起動後に以下を確認する。

```powershell
Invoke-RestMethod http://127.0.0.1:8080/health
Invoke-RestMethod http://127.0.0.1:8080/posts
Invoke-RestMethod http://127.0.0.1:8080/posts/hello-rustacian-blog
```

ブラウザ確認先:
- `http://127.0.0.1:8080/`
- `http://127.0.0.1:8080/p/hello-rustacian-blog`

## 11. v1 の制約
- Table Storage は未使用
- 画像は Blob 配信ではなくローカル静的配信
- Leptos は SSR のみで、WASM hydrate は未実装
- 管理画面、投稿更新 API、Git 同期は未実装

## 12. 次フェーズへの引き継ぎ
- `PostRepository` 境界は維持したまま Azure Blob 実装へ拡張可能
- 画像配信も Azure Blob に寄せる場合は `content/images/` の扱いを再設計する
- metadata を Table Storage へ分離する場合は manifest の責務を見直す
