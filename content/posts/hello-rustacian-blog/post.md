# Hello Rustacian Blog

このブログは Rust を中心にしたフルスタック構成の検証用 PoC です。

![Ferris notes](/images/ferris-notes.svg)

## この PoC で確認したいこと

- Core と Web 層の分離
- Markdown 記事の読み込み
- Azurite を前提にした設定切り替え

今はローカルコンテンツを直接読んでいますが、Repository 境界は Azure 実装へ差し替え可能です。

## 数式表示の確認

インライン数式の例として $e^{i\pi} + 1 = 0$ を置いておきます。

ブロック数式:

$$
\int_0^1 x^2 \, dx = \frac{1}{3}
$$
