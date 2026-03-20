# Phase 3 Tasks: Static Public Site and Dynamic Admin

## 1. Goal
- `v2` の記事運用と表示基盤を維持したまま、公開面を静的配信へ寄せる
- 管理 UI と preview を動的アプリとして分離する
- Azure 移行前提で、AI 連携、認証、配信の差し替え境界を整える

## 2. Scope
- 対象: 静的サイト生成、管理 UI 分離、認証基盤、Azure 接続準備
- 対象: Git を正本にした publish / rebuild フロー
- 対象: 公開面と管理面の責務分離
- 非対象: DB を正本にする CMS、本格的な管理画面 AI 自動運用

## 3. Tasks

### 3.1 Planning
- [x] `v3/plan.md` を作成する
- [x] `v3/tasks.md` を作成する
- [x] `v3/spec.md` の初版を用意する

### 3.2 Static Site Generation
- [x] `StaticSiteGenerator` の trait を `application/core` に追加する
- [x] 記事一覧、記事詳細、タグ一覧、タグ別一覧の出力仕様を決める
- [x] 出力ディレクトリ構成を決める
- [x] `published` のみを公開対象にする build ルールを実装する
- [x] validation error 時に build failure にする
- [x] sitemap / RSS / 検索用 JSON の扱いを決める

### 3.3 Public / Admin Split
- [x] 公開 route と管理 route の責務を分離する
- [x] `/admin` 向けの entrypoint を用意する
- [x] draft preview を管理側専用に移す
- [x] 公開側から管理 UI 導線を外す

### 3.4 Infrastructure Abstractions
- [x] `AssetStore` を導入する
- [x] `GeneratedMetadataStore` の local / Azure 差し替え境界を整理する
- [x] `AdminAuthService` を PoC から正式抽象化へ寄せる
- [x] `StaticSitePublisher` を導入する
- [x] local filesystem adapter と Azure adapter の責務を分離する

### 3.5 Azure Preparation
- [x] Azure static hosting 向け publish 方法を決める
- [x] Entra ID の正式な JWT / OIDC 検証方法を設計する
- [x] Azure OpenAI 連携を管理導線で再利用できるようにする
- [x] Blob Storage への生成物配置ルールを決める
- [x] Application Insights の観測対象を決める

### 3.6 UI and Rendering
- [x] 静的出力時に使える renderer 境界を整理する
- [x] 管理 UI から preview / AI 補助 / regenerate を扱えるようにする
- [x] 公開面の HTML / asset 最適化方針を決める

### 3.7 Workflow
- [x] Git push から静的生成、配置までの流れを設計する
- [x] publish / rebuild / rollback の運用導線を用意する
- [x] ローカル確認フローと CI/CD フローを揃える

### 3.8 Tests
- [x] 静的生成 output の snapshot テストを追加する
- [x] `draft` が静的出力されないことをテストする
- [x] 管理 preview だけが draft を返すことをテストする
- [x] adapter 差し替えテストを追加する

### 3.9 Docs
- [x] `v3/spec.md` を整備する
- [x] README に `v3` 方針を反映する
- [x] Azure 移行前の責務分離を文書化する
