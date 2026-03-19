# Phase 1.5 Tasks: CI/CD and Security Baseline

## 1. ゴール
- push / pull request 時に自動検査が走ること
- secrets を誤ってコミットしにくい構成になっていること
- dependency / security の基本検査が自動化されていること
- Azure デプロイ前提の安全な認証方針が整理されていること

## 2. スコープ
- 対象: GitHub Actions, `.gitignore`, security policy, secret scan, dependency scan
- 対象: Rust の自動検査
- 非対象: Azure 本番デプロイ workflow の本実装

## 3. 実装タスク

### 3.1 計画整理
- [x] `v1.5/plan.md` を作成する
- [x] `v1.5/tasks.md` を作成する

### 3.2 ignore / secrets 運用
- [x] `.gitignore` を見直す
- [x] `.env.local.example` を残したまま `.env.*` の取り扱いを整理する
- [x] ログや証明書系ファイルを除外する

### 3.3 CI
- [x] `.github/workflows/ci.yml` を追加する
- [x] `cargo fmt --all --check` を自動実行する
- [x] `cargo clippy --workspace --all-targets -- -D warnings` を自動実行する
- [x] `cargo test` を自動実行する
- [x] `docker compose config` を自動実行する
- [x] Azurite を使う結合テストを CI で実行できるようにする

### 3.4 Security
- [x] `.github/workflows/security.yml` を追加する
- [x] `gitleaks` を導入する
- [x] `cargo audit` を導入する
- [x] `gitleaks` 用の設定ファイルを追加する

### 3.5 ドキュメント
- [x] `SECURITY.md` を追加する
- [x] secrets の扱い方針を文書化する
- [x] Azure OIDC 方針を文書化する

### 3.6 GitHub 運用補助
- [x] Dependabot 設定を追加する

## 4. 完了条件
1. push / PR で Rust の自動検査が走る
2. secret scan が push / PR で走る
3. dependency scan が定期または push 時に走る
4. secret を Git に含めない運用ルールが明文化されている
5. Azurite を使うテストが CI で再現可能である
