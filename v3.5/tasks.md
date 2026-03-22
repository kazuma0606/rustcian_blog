# v3.5 Tasks: コンテンツリポジトリ分離

## 1. Goal
- 記事・画像の正本を private リポジトリ（`rustcian_blog_content`）へ分離する
- main リポジトリはコードと生成済み HTML のみを管理する
- content push → CI 自動起動 → 静的サイト生成 のフローを確立する
- ローカル開発のセットアップを最小限の手順に保つ

## 2. Scope
- 対象: content/ の分離、CI ワークフローの更新、Secrets 設定
- 対象: ローカル開発手順の更新（README）
- 非対象: Azure デプロイ先の変更（v4 で対応）
- 非対象: サブモジュール化（v3.5 では採用しない）

---

## 3. Tasks

### 3.1 計画・ドキュメント
- [x] `v3.5/plan.md` を作成する
- [x] `v3.5/tasks.md` を作成する

---

### 3.2 content リポジトリへのコンテンツ移行

- [ ] `rustcian_blog_content` リポジトリの default branch を確認する
- [ ] `content/` ディレクトリを content リポジトリへ push する
  - `content/` 内で git init → remote 追加 → push
- [ ] push 後に GitHub 上で記事・画像が正しく反映されていることを確認する

---

### 3.3 main リポジトリからの content 除外

- [ ] `git rm -r --cached content/` で追跡を解除する
- [ ] `.gitignore` に `content/` を追記する
- [ ] 除外後に `git status` でクリーンなことを確認する
- [ ] main リポジトリに commit する

---

### 3.4 GitHub Secrets の設定

- [ ] Fine-grained PAT（`CONTENT_REPO_PAT`）を作成する
  - 対象リポジトリ: `rustcian_blog_content`
  - 権限: `Contents: Read`
- [ ] `CONTENT_REPO_PAT` を `rustcian_blog`（main）の Secrets に登録する
- [ ] Fine-grained PAT（`MAIN_REPO_DISPATCH_TOKEN`）を作成する
  - 対象リポジトリ: `rustcian_blog`
  - 権限: `Actions: Write`（repository_dispatch の送信に必要）
- [ ] `MAIN_REPO_DISPATCH_TOKEN` を `rustcian_blog_content` の Secrets に登録する

---

### 3.5 main リポジトリのワークフロー更新

- [ ] `.github/workflows/static-site.yml` に `repository_dispatch` トリガーを追加する
  - `types: [content-updated]`
- [ ] content repo を `./content/` に checkout するステップを追加する
  ```yaml
  - uses: actions/checkout@v4
    with:
      repository: kazuma0606/rustcian_blog_content
      token: ${{ secrets.CONTENT_REPO_PAT }}
      path: content
  ```
- [ ] ワークフローが `CONTENT_REPO_PAT` を参照していることを確認する

---

### 3.6 content リポジトリのワークフロー作成

- [ ] `rustcian_blog_content/.github/workflows/dispatch.yml` を作成する
  - トリガー: `push` to `main`
  - ステップ: `actions/github-script` で `repository_dispatch` を送信
- [ ] content repo の default branch 名に合わせてトリガーを設定する

---

### 3.7 ローカル開発手順の整備

- [ ] README に content リポジトリの clone 手順を追記する
  ```bash
  git clone https://github.com/kazuma0606/rustcian_blog_content.git content
  ```
- [ ] `.env.local.example` の `CONTENT_ROOT` が `./content` を指していることを確認する

---

### 3.8 動作検証

- [ ] ローカルで `content/` clone 後に `cargo run` が起動することを確認する
- [ ] ローカルで `cargo run -- generate-static` が `dist/` を生成することを確認する
- [ ] content repo に test commit を push して CI が自動起動することを確認する
- [ ] CI の static-site ワークフローが成功し `dist/` 成果物が生成されることを確認する
- [ ] main repo の認証情報なしでは content repo に触れないことを確認する

---

### 3.9 CI の既存チェック確認

- [ ] `ci.yml`（cargo test）が content なしで通ることを確認する
  - テストは content に依存しないため変更不要のはず
- [ ] `security.yml` が引き続き通ることを確認する
