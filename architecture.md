# Architecture — rustacian-blog.com

> 2026-03-23 時点のシステム全体構成（v5.5 以降）

---

## 全体像

```
ユーザー
  │ HTTPS
  ▼
Cloudflare（DNS + CDN プロキシ）
  │ HTTPS（Full モード）
  ▼
Azure Container Apps（rustacian-prod-ca）
  ├─ Azure Blob Storage（blog-content: 記事データ読み込み）
  ├─ Azure Key Vault（シークレット取得）
  ├─ Azure Storage Table Storage（コメント/お問い合わせ）
  └─ Azure Application Insights（テレメトリ）

UptimeRobot
  └─ /health を5分ごとにping（コールドスタート抑制）

GitHub Actions（content repo push）
  ├─ content-deploy.yml: Blob Upload → Container App 再起動
  ├─ CI: fmt / clippy / test / docker build + ACR push
  ├─ Deploy: Container App イメージ更新
  └─ Static Site: dist/ を artifact 出力（手動）
```

---

## 記事コンテンツのフロー

```
kazuma0606/rustcian_blog_content（private repo）
  │ push to main
  ▼
notify.yml → repository_dispatch (content-updated)
  ▼
rustcian_blog: content-deploy.yml
  1. content repo を clone
  2. posts/index.json を meta.yml から生成（jq）
  3. az storage blob upload-batch → blog-content コンテナ
  4. az containerapp revision restart（検索インデックス再構築）
  5. Smoke test（/health + 記事件数）
  ▼
rustacian-blog.com に反映（published 記事のみ掲載）
```

**コンテンツルール**
- `status: published` → 一覧・詳細ページに掲載
- `status: draft` → 非掲載（管理者 preview のみ）

---

## 各サービスの役割

### Cloudflare（無料プラン）
| 設定項目 | 値 |
|---|---|
| ドメイン取得 | rustacian-blog.com（2026-03-22） |
| DNS レコード | CNAME @ → `rustacian-prod-ca.victoriouspebble-82e399d5.japaneast.azurecontainerapps.io`（プロキシ済み） |
| SSL/TLS モード | **Full**（Cloudflare → オリジン間も HTTPS） |
| CDN | Cloudflare のエッジキャッシュ（cf-cache-status: DYNAMIC） |
| キャッシュパージ | `CLOUDFLARE_ZONE_ID` + `CLOUDFLARE_API_TOKEN` 設定時に自動パージ |

> **注**: Azure CDN は不使用。Cloudflare が CDN + DNS + TLS を一括担当。

---

### Azure Container Apps（メインアプリ）
| 項目 | 値 |
|---|---|
| リソース名 | `rustacian-prod-ca` |
| リソースグループ | `rustacian-prod-rg`（japaneast） |
| イメージ | `rustacianprodacr.azurecr.io/rustacian-blog:<sha>` |
| CPU / メモリ | 0.5 vCPU / 1 Gi |
| スケール | min_replicas=0, max_replicas=3（スケール・トゥ・ゼロ） |
| 認証 | System-Assigned Managed Identity |
| カスタムドメイン | rustacian-blog.com（Azure Managed Certificate） |
| ヘルスチェック | `GET /health`（30秒間隔） |
| イングレス | external, port 8080, HTTPS のみ |

**コールドスタートについて**: スケール・トゥ・ゼロのためアイドル後の初回アクセスは30〜60秒かかる。UptimeRobot の定期 ping により実運用上はほぼウォーム状態を維持。

**コンテンツ配信**: 記事データは Docker イメージに含まれない。起動時に Azure Blob Storage (`blog-content` コンテナ) から `posts/index.json` を参照し、各リクエストで動的にブログ記事を配信する。

---

### Azure Container Registry（ACR）
| 項目 | 値 |
|---|---|
| レジストリ名 | `rustacianprodacr` |
| SKU | Basic |
| 認証 | Admin 無効。Container App は Managed Identity（AcrPull）、GitHub Actions は OIDC（AcrPush） |

> **注**: コンテンツ更新時は Docker イメージを再ビルドしない。アプリコードの変更時のみ CI でビルド＆プッシュ。

---

### Azure Key Vault
| 項目 | 値 |
|---|---|
| Vault 名 | `rustacian-prod-kv` |
| 認証方式 | RBAC（Key Vault Secrets User） |
| 格納シークレット | `appinsights-connection-string`, `slack-webhook-url`, `azure-openai-api-key`, `acs-access-key`, `storage-account-key` |

Container App は Managed Identity 経由でシークレットを取得し、環境変数として注入。

> **備考**: Container App 環境では IMDS（`169.254.169.254`）へのアクセスが不可のため、Blob Storage への認証は Managed Identity ではなく `AZURE_STORAGE_ACCOUNT_KEY`（Key Vault 経由）による SharedKey 認証を使用。

---

### Azure Storage Account
| 項目 | 値 |
|---|---|
| アカウント名 | `rustacianprodst` |
| 種別 | Standard LRS（japaneast） |

**Blob Storage**
| コンテナ名 | 用途 | アクセス |
|---|---|---|
| `blog-content` | 記事データ（`posts/index.json` + `posts/*/meta.yml` + `posts/*/post.md`） | 非公開（Managed Identity / SharedKey） |

**Table Storage**
| テーブル名 | 用途 | 認証 |
|---|---|---|
| `comments` | ブログコメント | Managed Identity（Storage Table Data Contributor） |
| `contacts` | お問い合わせ | Managed Identity（Storage Table Data Contributor） |

---

### Azure Application Insights + Log Analytics
| 項目 | 値 |
|---|---|
| AI リソース名 | `rustacian-prod-ai` |
| Log Analytics | `rustacian-prod-law`（保持期間30日、PerGB2018） |
| アラート | `ContentError` カスタムメトリクスが15分間に5件超で Slack 通知 |

接続文字列は Key Vault 経由で Container App に注入。

---

### Azure Communication Services（ACS）
| 項目 | 値 |
|---|---|
| リソース名 | `rustacian-prod-acs` |
| データリージョン | Asia Pacific |
| 用途 | メール通知（実装済み・送信ドメイン未設定のため未稼働） |

ACS ドメイン検証後に `ACS_SENDER_ADDRESS` を設定することで有効化。

---

### Azure OpenAI
| 項目 | 値 |
|---|---|
| リージョン | eastus（japaneast はクォータ 0） |
| モデル | gpt-4o-mini GlobalStandard |
| 現在の状態 | クォータ未承認のため capacity=0（デプロイなし） |

クォータ承認後: `terraform apply -var='openai_model_capacity=10'` で有効化。

---

### UptimeRobot（無料プラン）
| 項目 | 値 |
|---|---|
| 監視 URL | `https://rustacian-blog.com/health` |
| 間隔 | 5分ごと |
| 目的 | Container App のコールドスタート抑制（コンテナをウォーム状態に維持） |
| タイムアウト | 60秒（初回起動の遅延を吸収） |

---

### GitHub Actions

| ワークフロー | トリガー | 内容 |
|---|---|---|
| `ci.yml` | push / PR | `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`、master では Docker build + ACR push |
| `deploy.yml` | CI 成功後 / 手動 | Container App のイメージを更新 → ヘルスチェック |
| `content-deploy.yml` | `repository_dispatch: content-updated` | Blob アップロード → Container App 再起動 → Smoke test |
| `static-site.yml` | 手動 | `publish-static` 実行 → `dist/` を artifact 出力 |
| `security.yml` | push / PR | gitleaks + cargo audit |

**OIDC 認証**: GitHub Actions から Azure へのアクセスはパスワードレス（フェデレーテッド ID クレデンシャル）。

GitHub Secrets に設定済み:
- `AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, `AZURE_SUBSCRIPTION_ID`
- `ACR_NAME`, `ACR_LOGIN_SERVER`
- `CONTAINER_APP_NAME`, `RESOURCE_GROUP`
- `STORAGE_ACCOUNT_NAME`（`rustacianprodst`）
- `CONTENT_REPO_TOKEN`（content repo clone 用 PAT）

---

### Terraform（インフラ IaC）
| 項目 | 値 |
|---|---|
| バージョン | >= 1.7、AzureRM ~> 4.0 |
| 状態ファイル | Azure Blob Storage（`rustaciantfstate` ストレージアカウント / `tfstate` コンテナ） |
| モジュール | `app`, `registry`, `keyvault`, `storage`, `monitoring`, `comms`, `openai` |

---

## 管理者認証（Entra ID PKCE）

```
管理者ブラウザ
  │ GET /admin/login
  ▼
Container App → Entra ID 認可エンドポイント（リダイレクト）
  │ code + state
  ▼
Container App（/admin/callback）→ token 交換 → admin_session Cookie 発行
```

- テナント: `03b0b372-9099-4316-ba6a-a40a33ba2644`
- クライアント: `c2f581b6-fc04-4341-a6c0-1d7bd9782e54`
- 管理者グループ: `c9ed55dc-7739-4bee-89da-8163c1267ebd`

---

## コスト構成（概算）

| サービス | 無料枠 / プラン | 補足 |
|---|---|---|
| Azure Container Apps | 180,000 vCPU秒/月、360,000 GiB秒/月 無料 | スケール・トゥ・ゼロで無料枠内に収まる見込み |
| Azure Container Registry | Basic ~$5/月 | |
| Azure Storage | 最初の 5 GiB 無料 | Blob + Table Storage 使用量は微量 |
| Azure Key Vault | 10,000 操作/月 無料 | |
| Azure Application Insights | 5 GB/月 無料 | |
| Azure Communication Services | 無料（メール送信は別途課金） | 未稼働 |
| Azure OpenAI | クォータ 0（未使用） | 承認後は従量課金 |
| Cloudflare | 無料プラン | |
| UptimeRobot | 無料プラン（5分間隔まで） | |
