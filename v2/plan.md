# Rustacian Blog Project Plan (v2)

## 1. このドキュメントの位置づけ
この文書は v1 Local PoC の次段階として、ブログの完成度を高めるための設計方針を整理する。
主目的は Azure 本番デプロイそのものではなく、公開運用に耐えるコンテンツ管理と表示機能を整えることにある。

## 2. v2 の目的
- 公開ブログを読み取り専用の安全な構成として維持する
- Git ベースの更新フローを前提に、投稿・下書き・公開の運用を整理する
- Markdown 表示品質を上げ、LaTeX 数式を安定して表示できるようにする
- CSV などのデータファイルから図表を生成できるようにする
- Azure のサービスを使い、管理導線と AI 補助機能の拡張余地を作る
- 記事本文と運用メタデータを分離し、将来のタグ管理や補助生成に耐える構成にする

## 3. 前提方針

### 3.1 公開アプリの責務
- 公開アプリは原則として読み取り専用とする
- 匿名ユーザー向けに投稿 API を公開しない
- 記事表示、検索、要約表示、図表表示などの閲覧機能に集中する

### 3.2 投稿・更新フロー
- 記事更新は Git ベースで行う
- Markdown を主たるコンテンツソースとする
- 記事は Markdown 単体ではなく、記事ディレクトリ単位で管理する
- 記事ごとの設定は `meta.yml` に分離し、本文は `post.md` に集中させる
- 公開用アプリに直接投稿 UI を持たせることは v2 の必須要件にしない

### 3.3 管理系の考え方
- 将来的な管理画面や管理 API は Azure 認証を必須にする
- 認証基盤は Microsoft Entra ID を第一候補とする
- 一般公開面と管理面は明確に分離する

## 4. v2 で強化したい領域

### 4.1 コンテンツ管理
- 下書きと公開状態を持てるようにする
- 記事ごとの `meta.yml` 仕様を定義して運用しやすくする
- slug 重複や `meta.yml` 不備を事前検証できるようにする

### 4.2 表示品質
- LaTeX 数式を安定してレンダリングする
- コードブロックや埋め込み表現を改善する
- 記事内画像、図表、メタ情報の表現力を高める

### 4.3 データ駆動記事
- CSV を記事アセットとして扱えるようにする
- CSV から図表を自動生成できるようにする
- 記事本文、メタデータ、データファイルを分離し、保守しやすい構成にする

### 4.4 AI 補助
- Azure OpenAI を使った記事要約の生成を検討する
- タグ候補、冒頭リード、要約などの補助生成を視野に入れる
- 推論は表示時ではなく、記事更新時または事前生成を基本とする

### 4.5 セキュリティ
- 投稿機能を一般公開しない
- 管理機能には Azure 認証を前提にする
- 将来の管理 API を作る場合でも、公開 API とは分離する

## 5. 投稿・公開モデル案

### 5.1 基本モデル
- 記事ソースは `content/posts/` 配下の記事ディレクトリとする
- 各記事ディレクトリは少なくとも `post.md` と `meta.yml` を持つ
- Git push によって記事を追加・更新する
- 公開アプリは Git 管理されたコンテンツを読む

例:

```text
content/posts/example-post/
├── post.md
├── meta.yml
├── hero.png
└── chart.csv
```

### 5.2 下書き管理案
- 記事状態は `meta.yml` の `status: draft | published` で管理する
- 公開側は `published` のみ表示する
- `draft` は管理者プレビューまたはローカル確認の対象とする
- ディレクトリ分離ではなく、単一の `content/posts/` 配下で状態管理する

### 5.3 管理者プレビュー
- 管理者向けプレビューは将来導入可能な形にする
- 公開アプリからは `draft` を見せない
- 管理者プレビューを実装する場合は Entra ID 認証後のみ閲覧可能とする

## 6. 記事メタデータ設計案
本文から分離した `meta.yml` に、記事の運用メタデータを持たせる。
v2 では項目を「必須」「推奨」「将来用」に分けて管理する。

### 6.1 必須項目
- `title`
- `slug`
- `published_at`
- `status`
- `tags`
- `summary`

これらは公開制御、記事一覧、記事詳細、URL 解決に必要な最小セットとする。

### 6.2 推奨項目
- `updated_at`
- `hero_image`
- `series`
- `toc`
- `math`

これらは表示品質や記事整理のために有効だが、全記事必須にはしない。

### 6.3 将来用項目
- `charts`
- `summary_ai`
- `canonical_url`
- `description`
- `draft_note`
- `aliases`

これらは v2 時点では必須にせず、将来の表示改善や管理機能で利用する。

例:

```yaml
title: Example Post
slug: example-post
published_at: 2026-03-20T00:00:00Z
updated_at: 2026-03-21T12:00:00Z
status: published
tags:
  - rust
  - math
summary: サンプル記事
hero_image: ./hero.png
series: rustacian-blog-build
toc: true
math: true
charts:
  - type: line
    source: ./example.csv
    x: date
    y: value
```

- `post.md` は本文専用とし、運用メタは持たせない
- 画像や CSV は記事ディレクトリ内で相対参照する
- AI 生成物は `generated.json` など別ファイルで扱う

### 6.4 フィールド運用ルール
- `status` は `draft | published` のみを許可する
- `slug` は英小文字、数字、ハイフンのみを許可する
- `tags` は文字列配列とし、空配列は許可する
- `summary` は記事一覧や概要表示に使うため必須とする
- `updated_at` は更新時のみ設定し、未設定を許可する
- `hero_image` は記事代表画像がある場合のみ設定する
- `toc` と `math` は既定値を `false` とする

### 6.5 タグ管理方針
- 記事ごとの `meta.yml` に `tags` を持たせる
- 将来的な表記ゆれ対策のため、全体辞書ファイルの導入余地を残す
- 例: `content/tags.yml` でタグ ID や表示名を管理する

## 7. LaTeX / 数式表示方針

### 7.1 背景
- Markdown だけでは数式表示が不安定になりやすい
- 記事品質を上げるには、数式レンダリング基盤を明確に入れる必要がある

### 7.2 方針案
- `KaTeX` を第一候補とする
- 表示速度と運用の軽さを重視する
- 記法はインライン数式とブロック数式の両方を扱えるようにする

### 7.3 取り扱い
- `math: true` を `meta.yml` に持たせるか、常時有効にするかを検討する
- 記事中の数式記法とエスケープルールを仕様化する
- レンダリング失敗時のフォールバック表示を決める

### 7.4 実装済み仕様
- `meta.yml` の `math: true` があれば KaTeX 用 asset を読み込む
- `math: true` がなくても、本文中の `$...$`, `$$...$$`, `\(...\)`, `\[...\]` を検出した場合は自動で数式表示を有効化する
- インライン数式は `$...$` または `\(...\)` を使う
- ブロック数式は `$$` で囲むか `\[...\]` を使う
- 価格表記などの `\$` はエスケープ済みドルとして扱い、数式開始記号にしない
- Markdown 前処理でインライン数式は `<span class="math-inline">`, ブロック数式は `<div class="math-display">` に包んでから描画する
- KaTeX の読み込みや auto-render が失敗した場合は、raw 数式記法をそのまま読めるフォールバック表示に切り替える

## 8. CSV / 図表機能

### 8.1 方針
- データは Markdown 本文に直接埋め込まず、別ファイルとして管理する
- 第一候補は記事ディレクトリ配下の相対パスで持つ
- 例: `content/posts/example-post/example.csv`
- 記事は `meta.yml` から図表定義を参照する

### 8.2 期待する利点
- 本文とデータを分離できる
- 記事単位でアセットを閉じ込められる
- 差分管理しやすい
- CSV 差し替えだけで図表更新ができる
- 将来的に JSON や Parquet などへ拡張しやすい

### 8.3 図表仕様案
- `type`: line, bar, scatter など
- `source`: CSV パス
- `x`: X 軸列名
- `y`: Y 軸列名
- `title`: 図表タイトル
- `caption`: 補足説明

### 8.4 実装済み仕様
- 図表定義は `meta.yml` の `charts` 配列に持つ
- `source` は記事ディレクトリ基準の相対パスを使う
- CSV はヘッダ行を必須とし、`x`, `y` で指定された列名が存在しなければ検証エラーにする
- `y` 列は数値変換できる必要がある
- データ行が 0 件の CSV は検証エラーにする
- backend で CSV を読み、`chart_data` として数値化してから frontend に渡す
- frontend では `line`, `bar`, `scatter` を簡易 SVG として描画する
- 図表タイトル、caption、軸ラベルは記事詳細画面に表示する
- 記事ディレクトリ配下の CSV や画像は `/assets/posts/<slug>/...` として配信する

## 9. AI 補助機能

### 9.1 対象候補
- 記事要約
- タグ候補生成
- 導入文の候補生成
- 記事全体のメタデータ補助

### 9.2 Azure 利用方針
- Azure OpenAI を利用する
- 表示時に毎回推論せず、事前生成を原則とする
- 生成結果は本文とは別メタデータとして保存する案を優先する

### 9.3 保存先候補
1. `meta.yml` に書き戻す
2. 記事ごとの補助 JSON を別保存する

v2 時点では「別 JSON 保存」を第一候補とする。

### 9.4 実装済み方針
- Core では `AiMetadataGenerator` と `GeneratedMetadataStore` を分離し、外部推論と保存先を切り替え可能にする
- 記事読込は `summary_ai` を `meta.yml` より優先しないが、未設定時は `content/metadata/<slug>.json` から補完できる
- 生成結果は本文と分離し、`content/metadata/<slug>.json` に保存する
- 保存 JSON の主な項目は `summary_ai`, `suggested_tags`, `intro_candidates`, `generated_at`, `source_model`
- 生成対象は記事要約だけでなく、タグ候補と導入文候補までを含める
- 表示時推論は行わず、記事更新時や管理操作時に事前生成する
- Azure OpenAI adapter は backend 側に閉じ込め、Core には依存を持ち込まない

## 10. Azure 認証方針

### 10.1 方針
- 管理機能は Microsoft Entra ID を使う
- 一般公開の閲覧には認証を要求しない
- 管理者ユーザーまたは管理者グループのみアクセス可能にする

### 10.2 適用対象候補
- 下書きプレビュー
- 将来の投稿 UI
- 記事メタ生成トリガー
- 管理 API

### 10.3 実装済み PoC 方針
- 管理 preview route は `/admin/preview/{slug}` に分離する
- 公開 route は `/`, `/p/{slug}`, `/posts`, `/posts/{slug}` に限定し、`published` のみ返す
- 管理 preview は `PostVisibility::IncludeDrafts` で取得するが、認証通過後にのみ許可する
- `ADMIN_AUTH_MODE=entra-poc` のとき、Bearer token の JWT claim を見て tenant / audience / group または user oid を検証する
- 設定値は `ENTRA_TENANT_ID`, `ENTRA_CLIENT_ID`, `ENTRA_ADMIN_GROUP_ID` または `ENTRA_ADMIN_USER_OID` を使う
- 現段階は PoC のため、署名検証や OIDC metadata 取得までは未実装で、claim ベースの保護に留める
- 本番化時はここを Microsoft Entra ID の正式な OIDC / JWT 検証へ差し替える

## 11. 想定ディレクトリ拡張

```text
rustacian_blog/
├── content/
│   ├── posts/
│   │   └── example-post/
│   │       ├── post.md
│   │       ├── meta.yml
│   │       ├── hero.png
│   │       └── example.csv
│   ├── images/
│   ├── metadata/
│   └── tags.yml
├── application/
│   ├── core/
│   ├── backend/
│   └── frontend/
├── v1/
└── v2/
    └── plan.md
```

## 12. v2 の実装候補タスク

### 12.1 コンテンツ仕様
- 記事ディレクトリと `meta.yml` の仕様を定義する
- `meta.yml` に `status` を追加する
- `draft` を公開一覧から除外する
- slug 重複検出を実装する
- `meta.yml` バリデーションを強化する
- `content/tags.yml` のようなタグ辞書の要否を整理する

### 12.2 レンダリング
- KaTeX 連携を追加する
- Markdown 処理系を数式対応に拡張する
- コードブロック表示を改善する

### 12.3 データアセット
- 記事ディレクトリ配下の CSV 参照仕様を定義する
- 図表コンポーネントを実装する

### 12.4 AI 補助
- Azure OpenAI 呼び出しのアダプタ境界を設ける
- 記事要約生成の PoC を作る
- 生成メタの保存形式を決める

### 12.5 認証
- Entra ID を使う管理用認証の PoC を作る
- 管理ルートの保護方式を決める
- 公開面と管理面のルーティング分離を設計する

## 13. 優先順位案
実装順は次を推奨する。

1. `draft/published` の導入
2. `post.md` + `meta.yml` の記事構造導入
3. LaTeX 対応
4. CSV / 図表仕様の導入
5. AI 要約の事前生成
6. Entra ID による管理導線

## 14. v2 完了イメージ
- 公開側は安全な読み取り専用ブログとして運用できる
- Git 更新だけで下書きと公開を管理できる
- 数式入り記事を安定表示できる
- CSV ベースの図表を記事に載せられる
- AI による要約や補助メタを事前生成できる
- 管理機能を Azure 認証で安全に拡張できる
