---
description: content-deploy ワークフローを手動トリガーし、完了まで監視する
---

以下の手順で content-deploy を手動実行してください。

1. 次のコマンドで repository_dispatch を送信：

```bash
gh api repos/kazuma0606/rustcian_blog/dispatches \
  --input - <<'EOF'
{"event_type":"content-updated","client_payload":{"sha":"manual-trigger","ref":"refs/heads/main"}}
EOF
```

2. 10秒待ってから最新の run ID を取得：

```bash
gh run list --repo kazuma0606/rustcian_blog --limit 1 \
  --json databaseId --jq '.[0].databaseId'
```

3. 取得した run ID で完了まで監視：

```bash
gh run watch --repo kazuma0606/rustcian_blog <RUN_ID>
```

4. 完了後、`https://rustacian-blog.com/` が正常に記事一覧を返すか確認してください。

**注意**: content repo に変更がなくても実行できますが、blob の内容は変わりません。
content repo に変更を push した場合は notify.yml が自動でトリガーするため、手動実行は不要です。
