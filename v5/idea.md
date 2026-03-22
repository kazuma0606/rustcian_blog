# v5 拡張アイデア: Azure クラウド機能の活用

Azure にデプロイするからこそ活きる機能の候補。

---

## 高優先度（デプロイ直後に効果大）

### 1. Azure CDN + カスタムドメイン + HTTPS
静的サイトを Blob から Azure CDN 経由で配信。Terraform に `azurerm_cdn_profile` / `azurerm_cdn_endpoint` を追加。
`publish-static` コマンドが本番 CDN キャッシュを自動パージ。

### 2. Azure Entra ID SSO（管理画面ログイン）
`ADMIN_AUTH_MODE=entra` は実装済みだが、フロントエンドの OAuth2 PKCE ログインページが未実装。
管理者が組織アカウントでシングルサインオンできるようにする。

### 3. Key Vault + Managed Identity による秘密情報管理
Terraform 定義済み。API キーをコンテナ起動時に自動注入し、環境変数のハードコードを排除。

---

## 中優先度（UX・運用改善）

### 4. 記事一覧のサムネイル + AI 要約表示
- `PostSummaryView` の `summary_ai` / `hero_image` を記事一覧カードに表示
- Azure OpenAI で `summary_ai` を自動生成（既存の AI メタデータ生成 USE Case を流用）
- モックデータで先行実装し、後から OpenAI 連携に差し替え

### 5. Azure AI サービス連携の拡張
- **画像 Alt テキスト自動生成**（Azure AI Vision → 記事内画像に alt 属性を自動付与）
- **記事自動翻訳**（Azure Translator → 日本語記事を英語でも公開、`/en/posts/{slug}` ルート追加）

### 6. Azure Monitor Alerts
Terraform に `azurerm_monitor_metric_alert` を追加。`ContentError` イベントが閾値を超えたら Slack / PagerDuty に自動通知。

### 7. 画像アップロード UI（Admin）
- `POST /admin/images` で Blob SAS URL を発行しクライアントが直接アップロード
- `GET /admin/images` でアップロード済み画像一覧を表示
- 記事の `hero_image` を Admin から選択できるようにする

---

## 将来フェーズ

### 8. Azure Communication Services Email
Terraform に定義済み（`modules/comms`）。コメント承認・問い合わせ受信を Email で通知。

### 9. Azure Container Apps への移行
App Service → Container Apps に差し替えるとスケールゼロでコスト削減。Terraform モジュールの差し替えのみ。

### 10. 読者向けフィード購読（Azure Event Grid）
新記事公開時に Event Grid トピックへイベント発行 → Logic Apps 経由で読者メール配信。

---

## 実装優先順位（案）

| 順位 | 機能 | 理由 |
|------|------|------|
| 1 | サムネイル + AI 要約表示 | UX 改善、ローカル完結で実装可能 |
| 2 | Entra ID SSO | セキュリティ、デプロイ必須 |
| 3 | CDN + カスタムドメイン | 本番公開に必要 |
| 4 | 画像アップロード UI | コンテンツ管理の利便性 |
| 5 | 自動翻訳 | リーチ拡大 |
