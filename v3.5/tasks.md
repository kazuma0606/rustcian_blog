# v3.5 Tasks: コンテンツリポジトリ分離

## 1. Goal
- 記事・画像の正本を private リポジトリ（`rustcian_blog_content`）へ分離する
- main リポジトリはコードのみを管理する
- content push → CI 自動起動 → 静的サイト生成 のフローを確立する
- ローカル開発のセットアップを最小限の手順に保つ

## 2. Scope
- 対象: content/ の分離、CI ワークフローの更新
- 対象: ローカル開発手順の更新（README）
- 非対象: Azure デプロイ先の変更（v4 で対応）
- 非対象: サブモジュール化（v3.5 では採用しない）
- 非対象: GitHub Secrets（rustcian_blog が public のため不要と判明）

---

## 3. Tasks

### 3.1 計画・ドキュメント
- [x] `v3.5/plan.md` を作成する
- [x] `v3.5/tasks.md` を作成する
- [x] `v3.5/content_workflow.md` を作成する

---

### 3.2 content リポジトリへのコンテンツ移行

- [x] `rustcian_blog_content` リポジトリの default branch を確認する（main）
- [x] `content/` ディレクトリを content リポジトリへ push する
- [x] push 後に GitHub 上で記事・画像が正しく反映されていることを確認する

---

### 3.3 main リポジトリからの content 除外

- [x] `git rm -r --cached content/` で追跡を解除する
- [x] `.gitignore` に `content/` を追記する
- [x] main リポジトリに commit する

---

### 3.4 GitHub Secrets の設定

- [x] **不要と判明** — `rustcian_blog` が public リポジトリのため、
      content repo の workflow から token なしで checkout 可能。
      Secret は一切不要。

---

### 3.5 main リポジトリのワークフロー更新

- [x] `static-site.yml` の push トリガーを削除する（content なしでは実行不可のため）
- [x] `workflow_dispatch` のみ残す（手動実行用）
- [x] `repository_dispatch` トリガーを削除する（content repo が自己完結するため不要）

---

### 3.6 content リポジトリのワークフロー作成

- [x] `rustcian_blog_content/.github/workflows/build.yml` を作成する
  - トリガー: `push` to `main`
  - content repo (記事) + rustcian_blog (コード, public) を両方 checkout
  - `CONTENT_ROOT` を指定して `publish-static` を実行
  - `dist/` を Artifact としてアップロード

---

### 3.7 ローカル開発手順の整備

- [x] README に content リポジトリの clone 手順を追記する
- [ ] `.env.local.example` の `CONTENT_ROOT` が `./content` を指していることを確認する

---

### 3.8 動作検証

- [x] content repo に push して CI が自動起動することを確認する
- [x] CI の build ワークフローが成功し `dist/` 成果物（12 pages, 5 assets）が生成されることを確認する
- [ ] ローカルで `content/` clone 後に `cargo run` が起動することを確認する
- [ ] ローカルで `cargo run -- generate-static` が `dist/` を生成することを確認する

---

### 3.9 CI の既存チェック確認

- [ ] `ci.yml`（cargo test）が content なしで通ることを確認する
- [ ] `security.yml` が引き続き通ることを確認する
