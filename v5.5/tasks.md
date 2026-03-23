# v5.5 タスク一覧

ステータス: `[ ]` 未着手 / `[~]` 進行中 / `[x]` 完了

---

## Phase 1: Azure Storage — Container App 環境変数変更

### 1-1 Terraform で環境変数を追加

`terraform/modules/app/main.tf` の Container App 定義に以下を追加：

```hcl
{ name = "STORAGE_BACKEND",        value = "azurite" }
{ name = "AZURITE_BLOB_ENDPOINT",  value = "https://rustacianprodst.blob.core.windows.net" }
{ name = "STATIC_PUBLISH_BACKEND", value = "azurite" }
{ name = "STATIC_PUBLISH_PREFIX",  value = "public" }
```

> `AZURE_STORAGE_ACCOUNT_KEY` は削除済み（Phase 5-7 完了）→ Managed Identity が自動選択される。

- [x] **1-1** `terraform/modules/app/main.tf` に上記 4 変数を追加
- [x] **1-2** `terraform plan` で差分を確認（既存リソースへの影響なし）
- [x] **1-3** `terraform apply` を実行してコンテナを再起動
- [x] **1-4** `az containerapp show --query "properties.template.containers[0].env"` で環境変数を確認

---

## Phase 2: Blob Storage コンテナ作成

`AzuritePostRepository` が読むコンテナと静的サイト用コンテナを作成する。

- [x] **2-1** `blog-content` コンテナを作成（非公開）— Terraform で追加済み

  ```bash
  az storage container create \
    --name content \
    --account-name rustacianprodst \
    --auth-mode login
  ```

- [ ] **2-2** `$web` コンテナを作成（静的サイトホスティング用、公開）

  ```bash
  az storage container create \
    --name '$web' \
    --account-name rustacianprodst \
    --public-access blob \
    --auth-mode login
  ```

  > 注：Cloudflare が `rustacian-blog.com` にプロキシしているため、`$web` への直接アクセスは不要。Container App が Blob から配信する構成も可。`STATIC_PUBLISH_PREFIX` を調整すること。

- [x] **2-3** Managed Identity に `Storage Blob Data Contributor` ロールが付与されていることを確認 — Terraform で `azurerm_role_assignment.app_storage_blobs` を追加済み

  ```bash
  az role assignment list \
    --assignee <container_app_principal_id> \
    --scope /subscriptions/3f6d30ed-.../resourceGroups/.../providers/Microsoft.Storage/storageAccounts/rustacianprodst
  ```

---

## Phase 3: 初回コンテンツアップロード（手動）

スタートアップ時の `seed_azurite_from_local` 呼び出しは `STORAGE_BACKEND=azurite` 時のみ実行される。
本番では CI からアップロードする方針に変更するため、まず手動で初回アップロードを行う。

- [x] **3-1** `content/` ディレクトリを Blob にアップロード（ローカルから `seed_azurite_from_local` で初回シード済み）

- [x] **3-2** `content` コンテナ内のファイルを確認（blog 稼働で暗黙的に確認済み）

- [x] **3-3** Container App を再起動して `AzuritePostRepository` が Blob から記事を読むことを確認

  ```bash
  az containerapp revision restart \
    --name <CONTAINER_APP_NAME> \
    --resource-group <RESOURCE_GROUP> \
    --revision <revision-name>
  ```

- [x] **3-4** `GET https://rustacian-blog.com/` が HTTP 200 で記事一覧を返すことを確認

---

## Phase 4: main.rs の seed ロジック変更

`STORAGE_BACKEND=azurite` かつ本番環境では `seed_azurite_from_local` を実行しないようにする。
（CI がアップロードするため、スタートアップ時のシードは不要）

- [x] **4-1** `application/backend/src/main.rs` — `STORAGE_BACKEND=azurite` ブロック内の `seed_azurite_from_local` 呼び出しを、`SEED_FROM_LOCAL=true` の場合のみ実行するよう変更

  ```rust
  "azurite" => {
      let blob_endpoint = ...;
      if std::env::var("SEED_FROM_LOCAL").as_deref() == Ok("true") {
          seed_azurite_from_local(config.content_root.clone(), &blob_endpoint)
              .await
              .expect("failed to seed Azurite from local content");
      }
      Arc::new(AzuritePostRepository::new(blob_endpoint))
  }
  ```

- [x] **4-2** `terraform/modules/app/main.tf` に `SEED_FROM_LOCAL=false`（デフォルト）を追加（省略可。env var 未設定 = false として扱えばよい）
- [x] **4-3** ローカル開発用 `.env.local.example` に `SEED_FROM_LOCAL=true` を追記

---

## Phase 5: CI/CD パイプライン変更

`content-deploy.yml` を「Docker ビルド」から「Blob アップロード + 静的再生成」に変更する。

- [x] **5-1** `.github/workflows/content-deploy.yml` を書き換え：

  **削除する steps：**
  - `Set up Docker Buildx`
  - `Log in to ACR`
  - `Build and push image to ACR`
  - `Update Container App image`

  **追加する steps：**

  ```yaml
  - name: Upload content to Blob Storage
    run: |
      az storage blob upload-batch \
        --source content \
        --destination content \
        --account-name ${{ secrets.STORAGE_ACCOUNT_NAME }} \
        --overwrite true

  - name: Trigger static site regeneration
    run: |
      curl -s -f -X POST \
        "https://rustacian-blog.com/admin/static/regenerate" \
        -H "Cookie: admin_session=${{ secrets.ADMIN_SESSION_COOKIE }}"
  ```

- [x] **5-2** GitHub Actions シークレットに `STORAGE_ACCOUNT_NAME` を追加（`rustacianprodst`）
- [x] **5-3** `ADMIN_SESSION_COOKIE` は不要（HTTP エンドポイント呼び出しではなく Container App 再起動方式に変更）
- [x] **5-4** OIDC federated credential が Blob Storage への書き込み権限を持つことを確認（`Storage Blob Data Contributor` ロール）
- [x] **5-5** content repo で push を行い、CI が正常に完了することを確認

---

## Phase 6: 検証ページの追加と draft→published テスト

### 6-1 検証用記事を追加（content repo）

`content/posts/verification-test/` に記事を追加する：

```yaml
# meta.yml
title: "Verification Test Post"
date: "2026-03-23"
status: draft
tags: ["test", "verification"]
description: "This post is used to verify the draft→published pipeline."
```

```markdown
# Verification Test Post

This is a test post for verifying the content pipeline.

Created: 2026-03-23
```

- [x] **6-1** content repo に `posts/verification-test/` ディレクトリと `meta.yml`, `post.md` を追加
- [x] **6-2** `status: draft` でコミット・プッシュ
- [x] **6-3** CI が完了後、`GET https://rustacian-blog.com/` の記事一覧に `verification-test` が **含まれない** ことを確認
- [x] **6-4** 静的再生成は現構成では不要（動的配信のため）

### 6-2 draft → published に変更

- [x] **6-5** content repo の `posts/verification-test/meta.yml` を `status: published` に変更してプッシュ
- [x] **6-6** CI が完了するまで待機
- [x] **6-7** `GET https://rustacian-blog.com/` の記事一覧に `verification-test` が **含まれる** ことを確認
- [x] **6-8** `GET https://rustacian-blog.com/p/verification-test` が HTTP 200 を返すことを確認
- [ ] **6-9** `GET https://rustacian-blog.com/` のキャッシュ状態を確認（Cloudflare `cf-cache-status` ヘッダー）

---

## Phase 7: Smoke テスト更新

`content-deploy.yml` の Smoke テストを更新する（既存のヘルスチェックを維持）。

- [x] **7-1** Smoke テストに記事一覧チェックを追加（content-deploy.yml に組み込み済み）

- [x] **7-2** CI グリーンを確認

---

## チェックリスト

- [x] `cargo fmt --all --check` パス
- [x] `cargo clippy --workspace --all-targets -- -D warnings` パス
- [x] `cargo test` パス
- [x] `GET /health` が 200 を返すことを確認
- [x] `GET /` が記事一覧を返すことを確認（`status: published` の記事のみ）
- [x] draft 記事が一覧に表示されないことを確認
- [x] `status: published` に変更後、記事が一覧に追加されることを確認
