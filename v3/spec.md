# Rustacian Blog 仕様書 (v3)

## 1. 概要
- 公開面は Git 管理コンテンツから静的出力として生成する。
- 管理機能は動的かつ認証付きのまま維持する。
- 検証ルールと生成境界は `application/core` が責務を持つ。
- `v3` は Azure 移行準備フェーズであり、Azure 固有実装を完成させる段階ではない。

## 2. 入力コンテンツ
- `content/posts/<slug>/post.md`
- `content/posts/<slug>/meta.yml`
- `content/posts/<slug>/*` 記事アセット
- `content/images/*`
- `content/metadata/<slug>.json`

## 3. 対応している記事機能
- 見出しからの TOC 生成とアンカー付与
- KaTeX によるインライン数式とブロック数式の描画
- CSV を元にした chart の SVG 描画と table 表示
- Mermaid fenced code block の図表描画
- `meta.yml` による `draft / published` の状態管理

## 4. 静的出力物
- `index.html`
- `posts/<slug>/index.html`
- `tags/index.html`
- `tags/<tag>/index.html`
- `search.json`
- `sitemap.xml`
- `rss.xml`
- `_meta/build.json`
- `images/*`
- `assets/posts/<slug>/*`

## 5. 生成ルール
- 公開出力には `published` の記事だけを含める。
- validation error があれば build を失敗させる。
- Markdown、TOC、数式、chart SVG、chart table、Mermaid 変換は build 時に解決する。
- `draft` 記事は認証済みの管理 preview でのみ参照可能とする。

## 6. 公開面
- 読み取り専用の静的 HTML とする。
- 公開出力には管理 route や preview 導線を含めない。
- 画像と記事アセットは静的 path で参照する。
- Mermaid ブロックは Mermaid 用コンテナとして出力し、その記事で必要な場合だけ client-side で描画する。
- 出力最適化方針:
  - HTML: 構造を壊さない形で保持する
  - JSON / XML: 1 行の compact 出力にする
  - 画像 / 記事アセット / CSV: 無変換でコピーする
  - `_meta/build.json` に build 時の最適化方針を記録する

## 7. 管理面
- `/admin` は動的 route として維持する。
- draft preview は認証済みユーザーのみ利用可能とする。
- AI 補助メタ生成は管理側操作として扱う。
- 静的 regenerate は管理画面とローカル / CI で同じ publish pipeline を使う。

## 8. Azure との対応関係
- 公開出力: Azure static hosting
- 管理アプリ: App Service または Container Apps
- 認証: Entra ID
- AI 補助: Azure OpenAI
- アセット / 生成物保存: Blob Storage
- 観測: Application Insights

## 9. Blob 配置ルール
- 静的 publish は `STATIC_PUBLISH_PREFIX` で prefix を切り替えられる。
- Blob 上の出力は local `dist/` と同じ相対構造を持つ。
- 配置例:
  - `<prefix>/index.html`
  - `<prefix>/posts/<slug>/index.html`
  - `<prefix>/tags/index.html`
  - `<prefix>/tags/<tag>/index.html`
  - `<prefix>/search.json`
  - `<prefix>/sitemap.xml`
  - `<prefix>/rss.xml`
  - `<prefix>/images/*`
  - `<prefix>/assets/posts/<slug>/*`
  - `<prefix>/_meta/build.json`
- `_meta/build.json` は生成された page / asset の機械可読 inventory を持つ。
- AI 補助メタは `metadata/<slug>.json` として別管理する。

## 10. アセット配信ルール
- 公開の `/images/*` は backend 管理の image delivery で返す。
- local mode では `content/images` から読む。
- azurite mode では blob の `images/*` から読む。
- 記事ローカルアセットは `assets/posts/<slug>/*` で配信する。

## 11. 観測対象
- `OBSERVABILITY_BACKEND=stdout|noop` で現在の sink を切り替える。
- 将来の Application Insights adapter でも同じ event taxonomy を維持する。
- 現在の主な event category:
  - public request served
  - admin auth checked
  - AI metadata generated
  - static site published
  - content error
- `APPLICATIONINSIGHTS_CONNECTION_STRING` は Azure adapter 用の設定値として予約する。

## 12. ワークフロー
- ローカル publish コマンド:
  - `cargo run -p rustacian_blog_backend -- publish-static`
  - `cargo run -p rustacian_blog_backend -- rebuild-static`
- `generate-static`, `publish-static`, `rebuild-static` は現時点では同じ static publish flow の alias である。
- CI publish flow は `.github/workflows/static-site.yml` に定義する。
- `main` / `master` への push で静的出力を生成し、`dist` を artifact として保存する。
- `workflow_dispatch` では次を受け取る:
  - `site_ref`: branch, tag, commit SHA
  - `base_url`: sitemap / RSS 用の base URL
- rollback 方針:
  - 以前の `site_ref` を指定して static-site workflow を再実行する
  - その ref から生成された artifact を再配置する
