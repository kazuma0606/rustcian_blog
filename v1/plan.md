# Rustacian Blog Project Plan (v1)

## 1. このドキュメントの位置づけ
この文書は Phase 1 の Local PoC 用設計メモである。
全体方針はルートの `plan.md` を参照し、本書では「ローカルで何をどう成立させるか」に絞る。

## 2. Phase 1 の目的
- ローカル環境でブログアプリの最小機能を動かす。
- Azurite を使い、Azure Storage 互換の保存先を前提に PoC を行う。
- 将来 Azure に移行しても Core の設計を崩さない構造にする。
- Markdown ベースの記事管理を Rust フルスタックで成立させる。

## 3. PoC の対象範囲

### 対象
- Cargo Workspace の初期化
- Markdown 記事と画像のサンプル作成
- Azurite を使ったローカルストレージ検証
- Actix Web による記事一覧・記事詳細 API
- Leptos による記事一覧・記事詳細画面
- `.env.local` による設定切り替え
- Unit Test / 最低限の Integration Test

### 対象外
- Azure 本番デプロイ
- GitHub Actions による自動同期
- Application Insights への本番接続
- 本格的なアクセス解析、通知、管理画面
- WASM 埋め込み機能の実装

## 4. 構造設計方針

### 4.1 参考にする構造
`sample/rusted-ca` を参照元とする。
このリポジトリは単一 crate の中で以下を分離している。

- `shared`
- `domain`
- `application`
- `infrastructure`
- `presentation`
- `state`

本プロジェクトでは、この責務分割の考え方を踏襲する。
ただしブログでは Frontend と Backend を分けて進めた方が自然なため、単一 crate ではなく Cargo Workspace として再構成する。

### 4.2 本プロジェクトでの再構成
- `application/core`
  - `domain`
  - `application`
- `application/backend`
  - `presentation`
  - `router`
  - `controller`
- `application/frontend`
  - Leptos UI
- `infra`
  - IaC, Azure 関連定義, デプロイ設定

必要に応じて、`shared` 相当の責務は `core` に寄せる。
後から共通化の必要が明確になった場合のみ、別 crate 化を検討する。

### 4.3 Clean Architecture の境界
- `core` はビジネスルールとユースケースのみを持つ。
- `core` は外部 I/O、SDK、フレームワークに依存しない。
- Azurite / Azure Storage の具象実装は Rust アプリケーション側に持たせる。
- ルートの `infra` は Rust 実装ではなく IaC の置き場とする。
- `backend` と `frontend` は adapter として `core` を利用する。
- 実装の切り替えは Trait と DI で行う。

## 5. 技術方針

### 5.1 コンテンツ管理
- 記事本文は Markdown を採用する。
- ローカルではサンプル記事を `content/posts/` に配置する。
- 画像は `content/images/` に配置する。
- Frontmatter に最低限のメタデータを持たせる。

想定する最小 Frontmatter:
- `title`
- `slug`
- `published_at`
- `tags`
- `summary`

### 5.2 ストレージ
- PoC では Azurite を利用する。
- Blob Storage 相当で Markdown 本文と画像を扱う。
- Table Storage 相当は将来のメタデータ管理を見据えるが、Phase 1 では無理に広げない。
- 初期段階では「記事一覧」と「記事本文取得」に必要な最小モデルに絞る。

### 5.3 バックエンド
- Actix Web を採用する。
- 最低限の API は以下とする。
  - `GET /health`
  - `GET /posts`
  - `GET /posts/{slug}`
- 起動時に設定を読み込み、Repository 実装を DI する。

### 5.4 フロントエンド
- Leptos を採用する。
- Phase 1 では記事一覧画面と記事詳細画面のみ実装する。
- Markdown は HTML に変換して表示する。
- 画像パスは表示時に解決できる状態にする。

### 5.5 テスト
- `core` のユニットテストを優先する。
- Frontmatter 解析と Markdown 読み込みのテストを行う。
- Azurite を使った最低限の結合テストを行う。
- E2E は Phase 1 では必須にしない。

## 6. 想定ディレクトリ構成

```plaintext
rustacian_blog/
├── plan.md
├── v1/
│   ├── plan.md
│   └── tasks.md
├── content/
│   ├── posts/
│   └── images/
├── application/
│   ├── core/
│   ├── backend/
│   └── frontend/
├── infra/
├── docker-compose.yml
└── .env.local.example
```

## 7. PoC の完了条件
- `docker compose up` でローカル検証環境が起動する。
- `.env.local` を使って設定を切り替えられる。
- サンプル記事の一覧と詳細が画面に表示される。
- `PostRepository` の境界が Trait として定義されている。
- Azurite を使った最低限の結合確認ができる。

## 8. Phase 2 への引き継ぎ前提
- Core を変更せずに Azure 実装を追加できること。
- コンテンツ構造と Frontmatter 仕様が固まっていること。
- ローカル構成が Docker ベースで再現可能であること。
- 監視や CI/CD を後から追加できるだけの境界が整理されていること。
