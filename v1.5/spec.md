# Rustacian Blog v1.5 仕様書

## 1. 位置づけ
この文書は v1.5 の CI/CD・security baseline の運用仕様書である。
主に GitHub 側で必要になる設定内容と、現在リポジトリに追加済みの workflow / policy の意味を整理する。

## 2. 追加済みファイル

### 2.1 CI / Security
- `.github/workflows/ci.yml`
- `.github/workflows/security.yml`
- `.github/dependabot.yml`
- `.gitleaks.toml`
- `SECURITY.md`

### 2.2 計画
- `v1.5/plan.md`
- `v1.5/tasks.md`
- `v1.5/spec.md`

## 3. GitHub Actions 仕様

### 3.1 `ci.yml`
目的:
- push / pull request 時に Rust の基本品質を検証する

実行内容:
- `docker compose config`
- `docker compose up -d`
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test`
- 終了時に `docker compose down -v`

備考:
- Azurite を起動した状態で integration test を含めて実行する
- `RUN_AZURITE_TESTS=1` を workflow で有効化している

### 3.2 `security.yml`
目的:
- secret 漏洩と依存脆弱性の基本検査を行う

実行内容:
- `gitleaks`
- `cargo audit`

トリガー:
- `push`
- `pull_request`
- 週次 schedule

## 4. `.gitignore` 仕様
機密情報やローカル生成物を commit しないため、以下を除外している。

- `.env`
- `.env.*`
- `.env.local`
- `*.pem`
- `*.key`
- `*.pfx`
- `*.crt`
- `*.cer`
- `secrets/`
- `*.log`
- `target/`

例外:
- `.env.local.example` は commit 対象

## 5. `gitleaks` 仕様

### 5.1 目的
- push / PR 時に secret らしき文字列を検出する

### 5.2 allowlist
`.gitleaks.toml` では、Azurite の公開 emulator key を allowlist に入れている。
これは local development 用の公開済み値であり、本番 secret ではない。

## 6. GitHub 側で必要な設定

### 6.1 Branch Protection
対象ブランチ:
- `main`

推奨設定:
- Require a pull request before merging: `ON`
- Require status checks to pass before merging: `ON`
- Required status checks:
  - `rust`
  - `gitleaks`
  - `cargo-audit`
- Require branches to be up to date before merging: `ON`
- Restrict who can push to matching branches: 必要に応じて `ON`

### 6.2 Dependabot Alerts
GitHub リポジトリ設定で以下を有効にする。

- Dependabot alerts
- Dependabot security updates

`dependabot.yml` は weekly 更新を前提としている。

### 6.3 Code Scanning / Secret Scanning
GitHub Advanced Security を使える場合は、追加で有効化を検討する。

候補:
- Secret scanning
- Push protection
- Code scanning

v1.5 時点では `gitleaks` workflow を基本とするが、GitHub ネイティブ機能が使えるなら併用してよい。

## 7. Azure OIDC 方針

### 7.1 目的
- GitHub Actions から Azure へ安全にログインする
- 長期 secret を GitHub Secrets に持ち込まない

### 7.2 基本方針
- Azure service principal を作成する
- Federated credential を設定する
- GitHub Actions は OIDC で Azure login する

### 7.3 将来必要になる GitHub 側情報
- GitHub organization / user 名
- repository 名
- branch 名
- workflow 対象 environment

### 7.4 将来必要になる Azure 側情報
- `AZURE_CLIENT_ID`
- `AZURE_TENANT_ID`
- `AZURE_SUBSCRIPTION_ID`

注意:
- これらは secret として扱う場合もあるが、client secret 自体は作らない方針を優先する
- 本番アプリの接続文字列や API key は Azure Key Vault を第一候補とする

## 8. GitHub Secrets / Variables の扱い

### 8.1 v1.5 時点
必須ではない。
現在の CI / security workflow は GitHub 標準の `GITHUB_TOKEN` だけで動く前提である。

### 8.2 Azure デプロイ時に追加候補となる値
- `AZURE_CLIENT_ID`
- `AZURE_TENANT_ID`
- `AZURE_SUBSCRIPTION_ID`

アプリ固有の secret 候補:
- Azure OpenAI endpoint / key
- Storage 接続情報
- App Insights connection string

これらは GitHub に直接多く持たず、可能な限り Azure Key Vault 経由へ寄せる。

## 9. ローカル確認手順

### 9.1 CI 相当の確認
```powershell
docker compose config
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test
```

### 9.2 Azurite を使う test を明示的に確認したい場合
```powershell
$env:RUN_AZURITE_TESTS="1"
docker compose up -d
cargo test
docker compose down -v
```

## 10. 推奨運用
- main へ直接 push しない
- PR 作成時に CI success を確認する
- secret scan が失敗したら最優先で原因を確認する
- `.env.local` は作れても commit しない
- Azure 用情報はリポジトリではなく Azure 側へ寄せる

## 11. v1.5 完了の判断基準
- GitHub 上で `ci.yml` が成功する
- GitHub 上で `security.yml` が成功する
- Branch protection が有効になっている
- Dependabot alerts が有効になっている
- Azure OIDC を使う方針がチーム内で共有されている
