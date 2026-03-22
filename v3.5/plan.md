# v3.5 実装計画 — コンテンツリポジトリ分離

## 背景

v3 でコードと記事コンテンツが同一リポジトリに存在している。
認証情報が漏洩した場合、記事の改ざんが即座に本番反映されるリスクがある。
v3.5 では記事・画像をプライベートリポジトリへ分離し、セキュリティと可搬性を高める。

v4 の本格実装前に独立した検証フェーズとして切り出す。

---

## リポジトリ構成

| リポジトリ | 種別 | 役割 |
|---|---|---|
| `kazuma0606/rustcian_blog` | public | Rust コード + 生成済み HTML（dist/） |
| `kazuma0606/rustcian_blog_content` | private | 記事・画像の正本（content/） |

---

## アーキテクチャ

### ローカル開発

```
rustacian_blog/ (main repo root)
├── application/
├── content/          ← .gitignore で除外
│   ├── posts/        ← rustcian_blog_content を clone して配置
│   └── images/
├── dist/
└── .github/workflows/
```

`content/` のパスは変えず、main リポジトリからは除外した上で
content リポジトリを同パスに clone する。サブモジュールは使わない。

```bash
# 開発者セットアップ（main repo clone 後に一度だけ実行）
git clone https://github.com/kazuma0606/rustcian_blog_content.git content
```

### CI フロー

```
push to rustcian_blog_content (private)
  └→ content repo の workflow が repository_dispatch を発火
       └→ rustcian_blog の static-site.yml が起動
            ├→ code repo を checkout
            ├→ content repo を ./content/ に checkout（PAT 使用）
            ├→ cargo run -- generate-static
            └→ dist/ を成果物としてアップロード / デプロイ
```

---

## 必要な設定

### GitHub Secrets

| Secret | 保存先 | 用途 |
|---|---|---|
| `CONTENT_REPO_PAT` | `rustcian_blog`（main） | CI で content repo を checkout |
| `MAIN_REPO_DISPATCH_TOKEN` | `rustcian_blog_content`（content） | main repo へ repository_dispatch を送信 |

どちらも `repo` スコープを持つ Fine-grained PAT で最小権限に絞る。

### .gitignore（main repo）

```
content/
```

---

## 静的サイト生成ワークフローの変更点

### 変更前（static-site.yml）

```yaml
- uses: actions/checkout@v4   # main repo のみ
- run: cargo run -- generate-static
```

### 変更後

```yaml
- uses: actions/checkout@v4   # main repo (コード)
- uses: actions/checkout@v4   # content repo → ./content/ に配置
  with:
    repository: kazuma0606/rustcian_blog_content
    token: ${{ secrets.CONTENT_REPO_PAT }}
    path: content
- run: cargo run -- generate-static
```

---

## content リポジトリのワークフロー

```yaml
# rustcian_blog_content/.github/workflows/dispatch.yml
on:
  push:
    branches: [main]

jobs:
  dispatch:
    runs-on: ubuntu-latest
    steps:
      - name: Trigger static site build
        uses: actions/github-script@v7
        with:
          github-token: ${{ secrets.MAIN_REPO_DISPATCH_TOKEN }}
          script: |
            await github.rest.repos.createDispatchEvent({
              owner: 'kazuma0606',
              repo: 'rustcian_blog',
              event_type: 'content-updated',
              client_payload: { ref: context.sha }
            })
```

---

## main リポジトリの static-site.yml トリガー追加

```yaml
on:
  push:
    branches: [main, master]
  repository_dispatch:
    types: [content-updated]
  workflow_dispatch:
    inputs:
      site_ref: ...
      base_url: ...
```

---

## ローカル動作への影響

| 操作 | 変更前 | 変更後 |
|---|---|---|
| `cargo run` | そのまま動作 | `content/` に clone 済みであれば動作 |
| `cargo test` | そのまま動作 | 変化なし（テストは content に依存しない） |
| `cargo run -- generate-static` | そのまま動作 | `content/` に clone 済みであれば動作 |
| content 編集 | main repo で git add/commit | content repo で git add/commit |

---

## 移行手順概要

1. `content/` を content リポジトリへ push
2. main リポジトリから `content/` を追跡解除（`git rm --cached`）
3. `.gitignore` に `content/` を追加
4. main リポジトリの `static-site.yml` を更新
5. content リポジトリに `dispatch.yml` を作成
6. 必要な Secrets を両リポジトリに登録
7. content repo へ push → CI が自動起動することを確認

---

## セキュリティ改善効果

| リスク | 変更前 | 変更後 |
|---|---|---|
| main repo 認証情報漏洩 | 記事の改ざんが即本番反映 | dist/ の上書きのみ（再生成で復元可） |
| content repo 認証情報漏洩 | 同上 | 記事の読み書きのみ、コードに影響なし |
| CI secrets 漏洩 | デプロイ可能 | デプロイ可能（変化なし） |

---

## Azure 依存への影響

静的サイト生成が CI 完結になるため、Blob Storage をコンテンツの正本として使う必要がなくなる。
デプロイ先は `dist/` の出力先を変えるだけで Azure / GitHub Pages / Cloudflare Pages 等に切り替え可能。
