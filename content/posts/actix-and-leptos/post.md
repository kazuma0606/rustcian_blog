# Actix Web と Leptos を分けて使う

バックエンドは API と静的配信を担当し、フロントエンドは Leptos コンポーネントに責務を寄せます。

## 最小ルーティング

- `/health`
- `/posts`
- `/posts/{slug}`
- `/`
- `/p/{slug}`

SSR を使うことで、WASM ビルドを待たずに一覧画面と詳細画面を確認できます。
