# Phase 1 Tasks: Local PoC

## 1. ゴール
- ローカル環境でブログの最小動作を確認できること。
- Azurite を使って、Azure Storage 互換の保存先に接続できること。
- 記事一覧と記事詳細が Leptos / Actix Web 経由で表示できること。
- 今後 Azure 実装へ差し替えやすい境界が定義されていること。

## 2. スコープ
- 対象: Core / Rust実装 / Backend / Frontend の最小構成
- 対象: Docker Compose、`.env.local`、サンプルコンテンツ
- 非対象: Azure 本番デプロイ、GitHub Actions、自動同期、監視の本番接続

## 3. 構造方針
- `sample/rusted-ca` を参考に、`domain / application / infrastructure / presentation / shared` の責務分割を採用する。
- ただし `rusted-ca` は単一 crate 構成なので、本プロジェクトではそれを Cargo Workspace に再配置する。
- `core` はドメインとユースケースに限定し、外部 I/O に依存させない。
- ルートの `infra/` は IaC 用ディレクトリとし、Rust 実装は `application/` 配下に置く。
- Azurite / Azure Storage などの具象実装は Rust アプリケーション側に持つ。
- `backend` と `frontend` は Adapter として `core` を利用する。

## 4. 実装タスク

### 3.1 ワークスペース初期化
- [x] Cargo Workspace を作成する
- [x] ルートに `application/` と `infra/` を分ける
- [x] `application/core`, `application/backend`, `application/frontend` を作成する
- [x] `rusted-ca` の責務分割に合わせて、各 crate 内の module 構成を決める
  例: `core/domain`, `core/application`, `backend/presentation`
- [x] 共通の Rust edition / lint / dependency 方針を定める

### 3.2 コンテンツ準備
- [x] `content/posts/` を作成する
- [x] `content/images/` を作成する
- [x] サンプル記事 Markdown を 1-2 本作成する
- [x] 記事参照用のサンプル画像を配置する
- [x] Frontmatter の最小仕様を決める
  例: `title`, `slug`, `published_at`, `tags`, `summary`

### 3.3 ローカル実行環境
- [x] `docker-compose.yml` を作成する
- [x] Azurite Blob/Table を起動できるようにする
- [x] `.env.local.example` を作成する
- [x] アプリ側の設定読み込み方法を決める
  例: `APP_ENV=local`, `STORAGE_BACKEND=azurite`

### 3.4 Core 層
- [x] `Post` エンティティを定義する
- [x] `PostSummary` など一覧表示用の型を定義する
- [x] `PostRepository` Trait を定義する
- [x] 記事一覧取得 Usecase を定義する
- [x] 記事詳細取得 Usecase を定義する
- [x] エラー型とバリデーション方針を定義する

### 3.5 Rust 実装層
- [x] Azurite を使う `PostRepository` 実装を作成する
- [x] Markdown 読み込み処理を実装する
- [x] Frontmatter 解析処理を実装する
- [x] 画像パス解決ルールを実装する
- [x] 将来の Azure 実装と差し替え可能な構成にする

### 3.6 Backend 層
- [x] Actix Web の最小サーバーを作成する
- [x] Repository と Usecase を DI する
- [x] 記事一覧 API を実装する
- [x] 記事詳細 API を実装する
- [x] ヘルスチェック API を実装する

### 3.7 Frontend 層
- [x] Leptos アプリを初期化する
- [x] 記事一覧画面を作成する
- [x] 記事詳細画面を作成する
- [x] Markdown を HTML に変換して表示する
- [x] 画像が表示されることを確認する

### 3.8 テスト
- [x] `Post` の単体テストを作成する
- [x] Usecase のモックテストを作成する
- [x] Frontmatter 解析の単体テストを作成する
- [x] Azurite を使った結合テストを作成する
- [x] 主要 API の疎通確認を行う

## 5. 完了条件
1. `docker compose up` で PoC 環境が起動する。
2. `.env.local` を用いてローカル設定でアプリが起動する。
3. サンプル Markdown の記事一覧と記事詳細が表示される。
4. Core とストレージ実装の境界が `Trait` ベースで分離されている。
5. 単体テストと最低限の結合テストが通る。

## 6. Phase 2 への引き継ぎ条件
- Storage 実装の切り替えポイントが明確であること。
- コンテンツ構造と Frontmatter 仕様が固定されていること。
- Docker ベースでアプリを起動できること。
- Azure 実装追加時に Core を変更しなくて済むこと。
