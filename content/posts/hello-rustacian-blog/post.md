# Hello Rustacian Blog

このブログは Rust を中心にしたフルスタック構成の PoC です。`application/core` にドメインを寄せ、Web とストレージは adapter として差し替えられる形を目指しています。

![Ferris notes](/images/ferris-notes.svg)

## この PoC で確認したいこと

- Core と Web 層の責務分離
- Markdown と `meta.yml` ベースの記事運用
- Azurite を使った Blob 風の配信フロー

Repository 境界を先に決めておけば、ローカル運用から Azure 構成へ段階的に移行できます。

## 画像表示の確認

JPEG と PNG の表示確認用に、既存の静的画像も本文から参照します。

![Workspace photo](/images/134099572612174367.jpg)

![Gradient norms chart](/images/exp1_grad_norms.png)

## 数式表示の確認

インライン数式として $e^{i\pi} + 1 = 0$ を表示します。

ブロック数式:

$$
\int_0^1 x^2 \, dx = \frac{1}{3}
$$
