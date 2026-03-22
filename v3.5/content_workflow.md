# 記事更新ワークフロー（v3.5）

## 概要

記事・画像の正本は `rustcian_blog_content`（private）に置く。
push するだけで GitHub Actions が静的サイトを自動生成する。

---

## リポジトリ構成

| リポジトリ | 種別 | 内容 |
|---|---|---|
| `kazuma0606/rustcian_blog` | public | Rust コード（変更不要） |
| `kazuma0606/rustcian_blog_content` | private | 記事・画像の正本 |

---

## ローカル初回セットアップ

```bash
# 1. コードリポジトリを clone（初回のみ）
git clone https://github.com/kazuma0606/rustcian_blog.git
cd rustacian_blog

# 2. content リポジトリを content/ に clone（初回のみ）
git clone https://github.com/kazuma0606/rustcian_blog_content.git content
```

以降は `rustacian_blog/` ルートで作業する。`content/` は `.gitignore` に登録済みなので main repo に混入しない。

---

## 記事を新規作成する

```bash
cd content

# 1. 記事ディレクトリを作成
mkdir -p posts/<slug>

# 2. メタデータを作成
cat > posts/<slug>/meta.yml <<'EOF'
title: "記事タイトル"
date: "2026-03-22"
status: draft
tags: ["rust", "web"]
description: "記事の概要"
hero_image: ""
EOF

# 3. 本文を作成
touch posts/<slug>/post.md
```

---

## 記事を編集・公開する

```bash
cd content

# 編集
$EDITOR posts/<slug>/post.md

# draft → published に変更して公開
# meta.yml の status: "draft" → "published"

# 変更を push（これで CI が自動起動）
git add posts/<slug>/
git commit -m "Publish: <slug>"
git push origin main
```

---

## push 後の自動フロー

```
git push origin main（content repo）
  │
  └→ .github/workflows/build.yml が起動
       │
       ├─ Checkout content（この private repo）
       ├─ Checkout rustcian_blog コード（public, token 不要）
       ├─ cargo run -p rustacian_blog_backend -- publish-static
       │    CONTENT_ROOT=./content
       │
       └─ dist/ を Artifact としてアップロード
            └─ GitHub Actions の該当 run から zip でダウンロード可能
```

> **Note:** v3.5 時点では生成した HTML は Artifact として保存される。
> v4 で Azure Static Web Apps / GitHub Pages へのデプロイステップを追加予定。

---

## 画像を追加する

```bash
cd content

# 記事固有の画像（記事内から ./image.png で参照）
cp ~/Desktop/image.png posts/<slug>/image.png

# サイト共通の画像（記事内から /images/image.png で参照）
cp ~/Desktop/image.png images/image.png

git add .
git commit -m "Add image for <slug>"
git push origin main
```

---

## AI メタデータを生成する（admin UI）

静的サイト生成とは別に、動的バックエンドの admin 機能で AI メタデータを生成できる。

```bash
# ローカルでバックエンドを起動
cd /path/to/rustacian_blog
cargo run -p rustacian_blog_backend

# ブラウザで管理画面を開く
# http://localhost:8080/admin

# 記事を選択 → "Generate AI Metadata" ボタンを押す
# → content/metadata/<slug>.json が生成される
# → content repo に commit して push
```

---

## ドラフトをプレビューする

```bash
# バックエンドを起動
cargo run -p rustacian_blog_backend

# ブラウザで admin preview を開く
# http://localhost:8080/admin/preview/<slug>
# → status: draft の記事も表示される（認証モード: local-dev）
```

---

## ローカルで静的サイトを生成する

```bash
cd rustacian_blog

# content/ が clone 済みであること
cargo run -p rustacian_blog_backend -- generate-static

# dist/ に HTML が生成される
ls dist/
```

---

## content リポジトリのブランチ運用（推奨）

| ブランチ | 用途 |
|---|---|
| `main` | push で自動ビルドされる。公開記事はここに merge |
| `draft/<slug>` | 執筆中の記事。main に merge されるまでビルドされない |

```bash
# 新記事は draft ブランチで執筆
git checkout -b draft/my-new-post
# ... 執筆 ...
git push origin draft/my-new-post

# 公開時に main へ merge
git checkout main
git merge draft/my-new-post
git push origin main   # ← ここで自動ビルドが起動
```

---

## セキュリティ特性

| 操作 | 必要な権限 |
|---|---|
| 記事の読み取り（CI） | content repo への read アクセス（ワークフローが自動取得） |
| 記事の公開 | content repo への push 権限 |
| コードの変更 | main repo への push 権限 |
| 静的サイトの改ざん | **content repo と main repo の両方**への権限が必要 |

main repo の認証情報が漏洩しても、content repo への書き込みはできない。
