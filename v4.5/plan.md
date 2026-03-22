# v4.5 Plan: application/analytics マイクロサービス

## 目標
ブログ固有の解析機能を独立したサービスとして実装する。
JS不要・Cookie不要のプライバシーフレンドリー設計。
デプロイ時は Azure Container App として backend と独立して動かせる。

## アーキテクチャ

```
┌─────────────────────┐         ┌──────────────────────────┐
│  rustacian_blog     │  write  │   Azure Table Storage    │
│  backend (:8080)    │────────▶│  analyticspv             │
│                     │         │  analyticsqueries        │
└─────────────────────┘         │  analyticssessions       │
                                └───────────┬──────────────┘
┌─────────────────────┐              read   │
│  rustacian_blog     │◀────────────────────┘
│  analytics (:8081)  │
│  /api/popular       │
│  /api/gaps          │
│  /api/summary       │
│  /api/coread/{slug} │
└─────────────────────┘
```

HTTP 結合なし。共有ストレージのみ。

## Table スキーマ

| テーブル | PartitionKey | RowKey | フィールド |
|---------|-------------|--------|-----------|
| `analyticspv` | `YYYY-MM-DD` | `{slug}_{timestamp_ms}` | slug, ip_hash |
| `analyticsqueries` | `YYYY-MM-DD` | `{timestamp_ms}` | query, result_count |
| `analyticssessions` | `{ip_hash}_{YYYY-MM-DD}` | `{timestamp_ms}` | slug |

## フェーズ構成

| Phase | 機能 | 担当クレート |
|-------|------|------------|
| 1 | クレートスケルトン・テーブル初期化 | analytics |
| 2 | PV 集計（IP ハッシュ化・プライバシー設計） | backend + analytics |
| 3 | 検索クエリログ・ゼロヒット分析 | backend + analytics |
| 4 | 一緒に読まれているグラフ | backend + analytics |
| 5 | OpenAI コンテンツギャップ分析 | analytics |
| 6 | Admin UI 統合 | analytics |

## 分離デプロイ方法

ローカル: `cargo run -p rustacian_blog_analytics`（ポート 8081）
Azure: backend と analytics を別々の Container App としてデプロイ。
同じ Azure Storage Account（Table Storage）を参照する。
Terraform: `modules/analytics/` を追加して別 `azurerm_linux_web_app` を定義。
