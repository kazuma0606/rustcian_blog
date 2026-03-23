# v5.5 Plan — Azure Blob Storage コンテンツ管理への移行

## 背景と経緯

### v5 の設計（Docker イメージ埋め込み）

v5 では `content/` ディレクトリを Docker イメージに `COPY` して配布する設計を採用した。

```
content repo push
  → repository_dispatch
  → GitHub Actions が Docker ビルド（content を COPY）
  → ACR へ push
  → Container Apps の --image を更新
  → 新リビジョンが起動
```

**問題点：**

1. **GHA Docker キャッシュによる古いコンテンツの混入**
   - `cache-from/cache-to: type=gha` を使用していたため、`COPY content/ ./content/` レイヤーが以前のビルドからキャッシュされた。
   - `status: draft` だったコンテンツが `status: published` に変更されても、キャッシュされた旧レイヤーが使われ続けた。
   - `no-cache: true` に変更することで回避可能だが、根本的な問題は別にあった。

2. **Container Apps のリビジョン更新が起きない**
   - `az containerapp update --image IMAGE` で同一タグ（`:latest`）を指定した場合、Multiple revision mode では新リビジョンが作成されない。
   - 旧リビジョンが `ImagePullPolicy: IfNotPresent` でキャッシュされたイメージを使い続ける。
   - `:content-<sha>` タグを使えばリビジョン更新は起きるが、Container Apps が古い revision にトラフィックを維持する場合がある。

3. **新イメージで 500 エラー**
   - `no-cache` ビルドした最新イメージを `-e` フラグ付きで Docker 実行すると 500 エラー。
   - コンテナ内で直接実行すると正常動作（全 3 記事が返る）。
   - Docker 環境変数の伝搬と Dockerfile の `ENV` デフォルト値との相互作用が原因と推測されるが、根本原因は未解明。

4. **デプロイサイクルが長い**
   - `cargo build --release` を含む Docker ビルドに約 8 分かかる。
   - コンテンツ 1 記事の追加・修正に 8 分待つのは非現実的。

### v5.5 の設計（Azure Blob Storage コンテンツ管理）

```
content repo push
  → repository_dispatch or CI
  → az storage blob upload-batch でコンテンツを Blob Storage へアップロード
  → 管理画面から POST /admin/static/regenerate を呼ぶ
     （または CI から curl で呼ぶ）
  → AzuriteBlobStaticSitePublisher が静的 HTML を生成して Blob へ書き込む
  → Cloudflare キャッシュパージ（Phase 4-3 実装済み）
  → <30 秒で公開反映
```

**既存の実装資産：**

| コンポーネント | 状態 |
|---|---|
| `STORAGE_BACKEND=azurite` コードパス | 実装済み（`AzuritePostRepository`） |
| `seed_azurite_from_local` 関数 | 実装済み（ローカル → Blob アップロード） |
| `STATIC_PUBLISH_BACKEND=azurite` コードパス | 実装済み（`AzuriteBlobStaticSitePublisher`） |
| `POST /admin/static/regenerate` エンドポイント | 実装済み |
| Cloudflare キャッシュパージ | 実装済み（Phase 4-3） |
| Azure Storage `rustacianprodst` | 作成済み・Managed Identity アクセス設定済み |
| `AZURITE_BLOB_ENDPOINT` 環境変数 | Container App に未設定（要追加） |

**変更が必要な箇所：**

1. Container App の環境変数を変更
   - `STORAGE_BACKEND=azurite`
   - `AZURITE_BLOB_ENDPOINT=https://rustacianprodst.blob.core.windows.net`
   - `STATIC_PUBLISH_BACKEND=azurite`
   - `STATIC_PUBLISH_PREFIX=public`（静的 HTML の出力先 Blob プレフィックス）

2. `content-deploy.yml` を変更
   - Docker ビルド → `az storage blob upload-batch` + `curl` による静的再生成呼び出し

3. `AzuritePostRepository` の認証
   - ローカル（開発時）: Azurite SharedKey
   - 本番: Azure Storage Managed Identity
   - `StorageCredential` enum は実装済み（Phase 5）

## 検証戦略

### 追加するテスト記事

`content/posts/verification-test/` に記事を追加する：
- 初期状態：`status: draft`
- 目的：draft → published の変更がパイプライン全体に正しく反映されることを確認

### 確認シナリオ

1. **draft 記事は公開一覧に出ない** — `GET /` および静的 HTML の記事一覧に含まれないことを確認
2. **meta.yml を `status: published` に変更して CI/CD を走らせる** — コンテンツが Blob にアップロードされ、静的再生成が呼ばれる
3. **公開一覧に記事が追加される** — `GET /` の記事一覧と静的 HTML の両方で確認

## Blob Storage のコンテナ構成

```
rustacianprodst ストレージアカウント
  ├── content コンテナ（AZURITE_BLOB_ENDPOINT のデフォルトコンテナ）
  │   posts/<slug>/meta.json
  │   posts/<slug>/post.md
  │   posts/index.json
  │   images/<filename>
  └── $web コンテナ（静的サイトホスティング用）
      index.html
      posts/<slug>/index.html
      tags/...
      search.json
      sitemap.xml
      rss.xml
```

> 注意：`AzuritePostRepository` が使用するコンテナ名とプレフィックスは `blob.rs` / `storage.rs` の実装に合わせること。

## 将来の CI/CD フロー

```yaml
# content repo の .github/workflows/notify.yml (cap/notify-main.yml として管理)
on:
  push:
    branches: [main, master]

jobs:
  upload-and-regenerate:
    steps:
      - checkout content repo
      - az login (OIDC)
      - az storage blob upload-batch \
          --source . \
          --destination content \
          --account-name rustacianprodst \
          --overwrite true
      - curl -X POST https://rustacian-blog.com/admin/static/regenerate \
          -H "Cookie: admin_session=<TOKEN>"
```

または現行の `repository_dispatch` 方式を維持し、main repo 側の `content-deploy.yml` でアップロードを行う。

## 注意事項

- `AzuritePostRepository` は Azurite のローカル SharedKey 認証と Azure Storage の Managed Identity 認証の両方をサポート（Phase 5-1〜5-3 実装済み）。
- 本番では `AZURE_STORAGE_ACCOUNT_KEY` を設定しない → Managed Identity が自動選択される。
- `seed_azurite_from_local` はスタートアップ時にローカル content を Blob にアップロードする関数だが、本番では CI がアップロードするため不要になる（`STORAGE_BACKEND=azurite` かつ `seed_on_startup=false` にする、または `seed_azurite_from_local` 呼び出しを条件分岐で外す）。
