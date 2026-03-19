---
title: Hello Rustacian Blog
slug: hello-rustacian-blog
published_at: 2026-03-18T09:00:00Z
tags:
  - rust
  - architecture
summary: 最初のサンプル記事。Rust フルスタック構成の PoC の狙いをまとめる。
hero_image: /images/ferris-notes.svg
---

# Hello Rustacian Blog

このブログは Rust を中心にしたフルスタック構成の検証用 PoC です。

![Ferris notes](/images/ferris-notes.svg)

## この PoC で確認したいこと

- Core と Web 層の分離
- Markdown 記事の読み込み
- Azurite を前提にした設定切り替え

今はローカルコンテンツを直接読んでいますが、Repository 境界は Azure 実装へ差し替え可能です。
