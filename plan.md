# Rustacian Blog Master Plan

## 1. プロジェクト方針
- Rust フルスタックで個人技術ブログを構築する。
- 開発の主目的は、ブログ運営そのものに加えて、Rust によるフルスタック実装と Azure への移行可能な設計を実践すること。
- 開発は段階的に進め、まずローカルで PoC を成立させ、その後 Azure に移行する。

## 2. 開発原則
- **Local First:** 先にローカルで検証可能な最小構成を完成させる。
- **Azure Ready:** ローカル実装の延長で Azure に載せ替えられる構造にする。
- **Clean Architecture:** Core を中心に、Rust 実装と IaC を明確に分離する。
- **Git-Driven CMS:** Markdown を Git に push する運用を基本とする。
- **Observability:** ログ、メトリクス、アクセス解析を後付けではなく設計に含める。

## 3. 全体ロードマップ

### Phase 1: Local PoC
- Cargo Workspace を構築する。
- Core 層に `Post` と `PostRepository` などのドメイン境界を定義する。
- ローカル開発用に Azurite を導入し、Blob/Table 互換の検証環境を整える。
- Actix Web と Leptos で最小限の記事一覧・記事詳細表示を動かす。
- `.env.local` による設定切り替えを整備する。
- Markdown と画像のサンプルコンテンツを用意する。

### Phase 2: Azure Integration
- Azure Blob Storage / Table Storage 向けの Infra 実装を追加する。
- ローカル実装と Azure 実装を環境変数で切り替えられるようにする。
- `tracing` と OpenTelemetry を導入し、Application Insights 連携の基盤を作る。
- Dockerfile とコンテナ実行構成を整える。

### Phase 3: Delivery / Deployment
- GitHub Actions により、コンテンツ同期とアプリのビルドを自動化する。
- Azure Container Apps へのデプロイフローを構築する。
- IaC として Bicep を導入し、主要リソースをコード管理する。

### Phase 4: Production Features
- Git push による Markdown 自動反映を完成させる。
- 画像 URL の自動埋め込みや変換ルールを整備する。
- アクセス解析とエラー通知を実装する。
- 将来的な Rust/WASM 埋め込み機能を検討・追加する。

## 4. 目標アーキテクチャ
- **Core:** ドメインモデル、ユースケース、Repository Trait
- **Application:** Local/Azurite 実装、Azure Storage 実装、Backend、Frontend
- **Infra:** IaC、Azure リソース定義、デプロイ設定
- **Backend:** Actix Web による API と SSR 支援
- **Frontend:** Leptos による UI と必要に応じた Hydration

## 5. 構造設計の参照元
- 以前に作成した `rusted-ca` のワークスペース構造を参考にする。
- `sample/rusted-ca` の実体は、単一 crate の中で `domain / application / infrastructure / presentation / shared / state` を分離する構成だった。
- 本プロジェクトではその責務分割を踏襲しつつ、ブログでは Frontend と Backend の分離が明確なため、Cargo Workspace で crate を分ける。
- 基本方針は「Core を中心に境界を定義し、Infra / Backend / Frontend をアダプタとして分離する」形とする。
- つまり、層の考え方は `rusted-ca` に寄せ、パッケージ分割は本プロジェクト向けに再編する。

## 6. 環境ごとの役割

### ローカル
- Docker Compose で Azurite を起動する。
- サンプル Markdown / 画像を用いて動作確認する。
- `.env.local` に接続先や実行モードを定義する。

### Azure
- Azure Container Apps をアプリ実行基盤とする。
- Azure Blob Storage に本文・画像を格納する。
- Azure Table Storage にメタデータやアクセス情報を格納する。
- Azure Monitor / Application Insights を監視基盤とする。

## 7. ドキュメント配置
- ルートの `plan.md`: 全体のマスタープラン
- `v1/plan.md`: Phase 1 PoC の設計メモ
- `v1/tasks.md`: Phase 1 実装タスク
