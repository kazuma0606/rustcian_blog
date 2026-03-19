# Rustacian Blog Project Plan (v1.5)

## 1. このドキュメントの位置づけ
この文書は v1 Local PoC と v2 機能拡張の間に置く「品質・安全性向上フェーズ」の計画書である。
主目的は、将来 Azure にデプロイする前に、CI/CD と secrets 保護の土台を先に固めることにある。

## 2. v1.5 の目的
- push / pull request 時に自動テストと静的検査を実行できるようにする
- 機密情報を誤ってリポジトリに含める事故を防ぎやすくする
- 依存関係やサプライチェーンの基本的な検査を追加する
- 将来の Azure デプロイに向けて、安全な認証方針を先に定める

## 3. v1.5 で最重要とするもの

### 3.1 secrets 漏洩防止
- `.env.local` のようなローカル秘密情報を commit しない
- 誤って API key や接続文字列を push した場合に検知できる
- 将来の Azure デプロイでも長期 secret を極力増やさない

### 3.2 push 時の自動検証
- 手元で通っていても push 後に再検証される
- main に壊れた状態を入れにくくする
- フォーマット、lint、テスト、secret scan を最低限自動化する

## 4. 基本方針

### 4.1 secrets の取り扱い方針
- 機密情報は Git に含めない
- `.env.local.example` のようなサンプルのみを commit する
- 本番 secret は GitHub Secrets または Azure Key Vault に置く
- GitHub Actions から Azure へは、可能な限り OIDC を使う

### 4.2 CI 方針
- GitHub Actions を使う
- `push` と `pull_request` をトリガーにする
- Rust の品質検査を workflow として定義する
- secret scan と dependency scan を組み込む

### 4.3 CD 方針
- v1.5 では本番デプロイ自体は必須にしない
- ただし、将来の Azure デプロイに耐える認証・secret 管理方針を先に決める
- デプロイ workflow を作る場合でも、long-lived secret を前提にしない

## 5. 必要な CI 項目

### 5.1 Rust の基本検査
最低限、以下を自動実行する。

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test`

### 5.2 補助検査
追加で以下を検討する。

- `cargo audit`
- `docker compose config`
- Azurite を使う integration test

### 5.3 secret scan
最低限 1 つの secret scanning ツールを入れる。

候補:
- `gitleaks`
- `trufflehog`

v1.5 の第一候補は `gitleaks` とする。

### 5.4 workflow 設計
最初は 1 workflow にまとめても良いが、将来的には次の分割を想定する。

- `ci.yml`
  - fmt
  - clippy
  - test
  - docker compose validation
- `security.yml`
  - gitleaks
  - cargo audit

## 6. secrets 漏洩防止の具体策

### 6.1 `.gitignore` 見直し
最低限、次の対象を除外候補として見直す。

- `.env`
- `.env.*`
- `*.pem`
- `*.key`
- `*.pfx`
- `*.crt`
- `*.cer`
- `secrets/`

ただし、サンプルファイルは除外しないようにルール設計する。

例:
- `.env.local` は除外
- `.env.local.example` は commit 対象

### 6.2 ドキュメント整備
最低限、以下を整備する。

- secrets の扱いルール
- ローカル設定ファイルの作り方
- Azure へ渡す設定値の保管先

### 6.3 secret scan 導入
- push / PR で自動実行する
- 必要に応じて allowlist を持つ
- 誤検知を減らしつつ、見逃しを優先しない

## 7. サプライチェーン対策

### 7.1 依存脆弱性検査
- `cargo audit` を CI に組み込む
- GitHub の Dependabot alerts を有効にする

### 7.2 dependency 更新方針
候補:
- Dependabot
- Renovate

v1.5 では Dependabot を第一候補とする。

### 7.3 GitHub Actions の安全性
- action は信頼できるものだけを使う
- 可能なら commit SHA pin を使う
- 少なくともメジャーバージョン固定にする

## 8. Azure デプロイ前提の認証方針

### 8.1 方針
- GitHub Actions から Azure へ接続する際は OIDC を優先する
- Client secret を GitHub Secrets に長期保存する方式は極力避ける
- Azure 側に federated credential を設定する

### 8.2 将来の保存先
- アプリの機密設定は Azure Key Vault を第一候補とする
- Azure Container Apps / App Service 側の設定は Azure 側に持つ
- GitHub Actions から直接秘密値を大量に配布しない

## 9. GitHub 運用ルール案

### 9.1 Branch Protection
main には以下を設定する想定とする。

- CI success 必須
- 直接 push 制限
- PR 経由でのみ merge

### 9.2 レビュー運用
- 少なくとも CI が通っていない変更は merge しない
- 将来的に `CODEOWNERS` を追加できるようにする

## 10. v1.5 の実装候補タスク

### 10.1 CI
- `.github/workflows/ci.yml` を追加する
- Rust toolchain のセットアップを行う
- `fmt`, `clippy`, `test` を workflow に追加する
- `docker compose config` を workflow に追加する

### 10.2 Security
- `.github/workflows/security.yml` を追加する
- `gitleaks` を導入する
- `cargo audit` を導入する
- `.gitignore` を見直す

### 10.3 ドキュメント
- `SECURITY.md` を追加する
- secret の扱いルールを明文化する
- ローカル設定と本番設定の分離方針を記述する

### 10.4 Azure 前提整理
- OIDC を使う Azure ログイン方針を整理する
- 将来の deploy workflow の前提を文書化する

## 11. 完了条件
- push / PR で `fmt`, `clippy`, `test` が自動実行される
- secret scan が push / PR で自動実行される
- dependency 脆弱性検査が自動実行される
- secrets を Git に入れない運用ルールが文書化されている
- 将来の Azure デプロイで OIDC を使う方針が明確になっている

## 12. 優先順位
実装順は以下を推奨する。

1. `.gitignore` と secrets 取り扱い整理
2. `ci.yml` の追加
3. `gitleaks` の追加
4. `cargo audit` の追加
5. `SECURITY.md` の追加
6. Azure OIDC 方針の明文化

## 13. v2 への引き継ぎ
- v2 の機能拡張は、v1.5 の CI / security 基盤の上で進める
- 下書き、LaTeX、CSV、AI 要約、管理認証などの実装は、CI に守られた状態で進める
- Azure 本番デプロイは、この v1.5 の方針を満たした上で着手する
