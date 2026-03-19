---
title: Actix Web と Leptos を分けて使う
slug: actix-and-leptos
published_at: 2026-03-19T00:30:00Z
tags:
  - actix-web
  - leptos
summary: API と UI を分離しつつ、Leptos SSR で最小構成を組む方針を整理する。
hero_image: /images/stack-flow.svg
---

# Actix Web と Leptos を分けて使う

バックエンドは API と静的配信を担当し、フロントエンドは Leptos コンポーネントに責務を寄せます。

## 最小ルーティング

- `/health`
- `/posts`
- `/posts/{slug}`
- `/`
- `/p/{slug}`

SSR を使うことで、WASM ビルドを待たずに一覧画面と詳細画面を確認できます。
