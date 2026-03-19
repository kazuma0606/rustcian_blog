## はじめに

現在、個人開発として **Rust 製のブログアプリ** を開発しています。本記事では、その技術スタックと設計方針について紹介します。
実装理念->Rust製ブログアプリの技術スタック: 軽量 & 高速 & 拡張性を兼ね備えた構成

**目的:**
- **高速 & 軽量** で、**依存関係を最小限に抑えた構成**
- **開発効率を向上** させ、**デプロイの手間を最小限に**
- **スケーラブルな設計** で、アクセス増加時にも対応可能

この構成は、**MVP開発 → 本番運用へのスムーズな移行** を意識したものになっています。

---

## 技術スタック

| カテゴリ | 選定技術 | 理由 |
| ---- | ---- | ---- |
| フロントエンド | Leptos (Rust) | SSR & CSR対応、Rustネイティブ |
| バックエンド | Actix Web (Rust) | フルマネージドDB、無料枠あり |
| データベース | Supabase (PostgreSQL) | フルマネージドDB、無料枠あり |
| ORM |  SeaORM (Rust) | 型安全 & 非同期対応 |
| 認証 |  Clerk / Supabase Auth | シンプルな認証管理 |
| API |  async-graphql (Rust) | 柔軟なデータ取得 |
| デプロイ（FE） | Vercel | Leptos SSR対応、無料枠あり |
| デプロイ（BE） | Shuttle.rs  | Rust特化 & 簡単デプロイ |

---

## アーキテクチャ設計

**Rust製ブログの全体構成:**

```plaintext
+----------------------+         +-----------------------+
| Frontend (Leptos)    |         | Backend (Shuttle)     |
| - SSR & Hydration    | <--->   | - Actix Web (API)     |
| - Vercel Hosting     |         | - async-graphql       |
+----------------------+         | - Supabase (DB)       |
                                 | - SeaORM (ORM)        |
                                 +-----------------------+
```

この構成のメリット:
1. **フロントエンドは Leptos で Rust ネイティブ**
   - SSR（Server-Side Rendering）で高速な初期描画
   - クライアント側の Hydration により動的UIを実現

2. **バックエンドは Shuttle で Rust 特化の簡単デプロイ**
   - Actix Web を使用し、高速な非同期APIを提供
   - Supabase（PostgreSQL）をデータベースとして採用
   - ORMには SeaORM を使用し、型安全なクエリを実現

3. **MVPフェーズからスケールしやすい設計**
   - Supabase を利用することで、データ管理の手間を軽減
   - 必要に応じて AWS/GCP へ移行可能
   - GraphQL により、クライアントが必要なデータのみ取得可能

---

## **開発ロードマップ**

### **1️⃣ MVP開発 & デプロイ**
✅ Shuttle で Actix Web + Supabase の API を実装
✅ `shuttle run` で API をデプロイ & 動作確認
✅ Leptos のフロントエンドを開発 & Vercel にデプロイ
✅ GraphQL API とフロントを接続

### **2️⃣ 本番運用への移行**
✅ Supabase をフルマネージドDBへ移行（AWS RDS など）
✅ API のキャッシュ & CDN 最適化
✅ 認証（Clerk / Supabase Auth）を導入

### **3️⃣ 機能追加 & 収益化**
✅ **検索機能:** `tantivy` で全文検索を追加
✅ **リアルタイム機能:** WebSocket or GraphQL Subscriptions
✅ **通知機能:** WebPush or メール通知（Rustで実装）
✅ **収益化:** Stripe API 連携で有料ブログ機能

---

## **結論: なぜこの構成を選んだのか？**

- **Rustフルスタック開発** により、型安全 & 高速なアプリケーションを実現
- **MVPを最速で開発 & デプロイ可能**（Shuttle + Vercel の活用）
- **BaaS依存を最小限に抑え、スケール可能な設計**（Supabase をDBのみに利用）
- **GraphQL を活用し、柔軟なデータ取得 & パフォーマンス最適化**

この技術スタックは **「個人開発からスケールまでを見据えたRust製ブログ」** の理想形に近いと考えています。

今後、開発を進めながらアップデート情報も公開していきます！

---

**💬 もし興味がある方は、ぜひコメントやフィードバックをお願いします！**

