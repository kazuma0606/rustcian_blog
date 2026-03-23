# Architecture — rustacian-blog.com

> 2026-03-23 時点のシステム全体構成

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
  ├─ Azure Key Vault（シークレット取得）
  ├─ Azure Storage（Table Storage: コメント/お問い合わせ）
  └─ Azure Application Insights（テレメトリ）

UptimeRobot
  └─ /health を5分ごとにping（コールドスタート抑制）

GitHub Actions
  ├─ CI: fmt / clippy / test / docker build
  ├─ Deploy: ACR push → Container App イメージ更新
  └─ Static Site: dist/ を artifact 出力
```

---

## 各サービスの役割

### Cloudflare（無料プラン）
| 設定項目 | 値 |
|---|---|
| ドメイン取得 | rustacian-blog.com（2026-03-22） |
| DNS レコード | CNAME @ → `rustacian-prod-ca.victoriouspebble-82e399d5.japaneast.azurecontainerapps.io`（プロキシ済み） |
| SSL/TLS モード | **Full**（Cloudflare → オリジン間も HTTPS） |
| CDN | Cloudflare のエッジキャッシュ（cf-cache-status: DYNAMIC） |
| キャッシュパージ | `POST /admin/publish` 完了後に自動パージ（`CLOUDFLARE_ZONE_ID` + `CLOUDFLARE_API_TOKEN` 設定時） |

> **注**: Azure CDN は不使用。Cloudflare が CDN + DNS + TLS を一括担当。

---

### Azure Container Apps（メインアプリ）
| 項目 | 値 |
|---|---|
| リソース名 | `rustacian-prod-ca` |
| リソースグループ | `rustacian-prod-rg`（japaneast） |
| イメージ | `rustacianprodacr.azurecr.io/rustacian-blog:latest` |
| CPU / メモリ | 0.5 vCPU / 1 Gi |
| スケール | min_replicas=0, max_replicas=3（スケール・トゥ・ゼロ） |
| 認証 | System-Assigned Managed Identity |
| カスタムドメイン | rustacian-blog.com（Azure Managed Certificate: `mc-rustacian-prod-rustacian-blog-c-2010`） |
| ヘルスチェック | `GET /health`（30秒間隔） |
| イングレス | external, port 8080, HTTPS のみ |

**コールドスタートについて**: スケール・トゥ・ゼロのためアイドル後の初回アクセスは30〜60秒かかる。UptimeRobot の定期 ping により実運用上はほぼウォーム状態を維持。

---

### Azure Container Registry（ACR）
| 項目 | 値 |
|---|---|
| レジストリ名 | `rustacianprodacr` |
| SKU | Basic |
| 認証 | Admin 無効。Container App は Managed Identity（AcrPull）、GitHub Actions は OIDC（AcrPush） |

---

### Azure Key Vault
| 項目 | 値 |
|---|---|
| Vault 名 | `rustacian-prod-kv` |
| 認証方式 | RBAC（Key Vault Secrets User） |
| 格納シークレット | `appinsights-connection-string`, `slack-webhook-url`, `azure-openai-api-key`, `acs-access-key` |

Container App は Managed Identity 経由でシークレットを取得し、環境変数として注入。

---

### Azure Storage Account
| 項目 | 値 |
|---|---|
| アカウント名 | `rustacianprodst` |
| 種別 | Standard LRS（japaneast） |
| 用途 | Table Storage（comments テーブル / contacts テーブル） |
| 認証 | Managed Identity（Storage Table Data Contributor） |

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
| `ci.yml` | push / PR | `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`、master/main では Docker build + ACR push |
| `deploy.yml` | CI 成功後 / 手動 | Container App のイメージを更新 → ヘルスチェック |
| `static-site.yml` | 手動 | `publish-static` 実行 → `dist/` を artifact 出力 |
| `security.yml` | push / PR | gitleaks + cargo audit |

**OIDC 認証**: GitHub Actions から Azure へのアクセスはパスワードレス（フェデレーテッド ID クレデンシャル）。

GitHub Secrets に設定済み:
- `AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, `AZURE_SUBSCRIPTION_ID`
- `ACR_NAME`, `ACR_LOGIN_SERVER`
- `CONTAINER_APP_NAME`, `RESOURCE_GROUP`

---

### Terraform（インフラ IaC）
| 項目 | 値 |
|---|---|
| バージョン | >= 1.7、AzureRM ~> 4.0 |
| 状態ファイル | Azure Blob Storage（`rustaciantfstate` ストレージアカウント / `tfstate` コンテナ） |
| モジュール | `app`, `registry`, `keyvault`, `storage`, `monitoring`, `comms`, `openai` |

---

## データフロー（記事公開）

```
管理者
  │ POST /admin/publish（Cookie 認証: Entra ID PKCE）
  ▼
Container App
  ├─ 静的 HTML 生成（Leptos SSR）
  ├─ Blob Storage へアップロード（省略可）
  └─ Cloudflare キャッシュパージ（zone_id 設定時）
```

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
| Azure Storage | 最初の 5 GiB 無料 | Table Storage 使用量は微量 |
| Azure Key Vault | 10,000 操作/月 無料 | |
| Azure Application Insights | 5 GB/月 無料 | |
| Azure Communication Services | 無料（メール送信は別途課金） | 未稼働 |
| Azure OpenAI | クォータ 0（未使用） | 承認後は従量課金 |
| Cloudflare | 無料プラン | |
| UptimeRobot | 無料プラン（5分間隔まで） | |
