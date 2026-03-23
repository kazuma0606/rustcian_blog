---
description: ブログの稼働状態を確認する（HTTP ステータス・アクティブ revision・最新ワークフロー結果）
---

以下のコマンドを順番に実行して、ブログの現在の稼働状態を確認してください。

1. `curl -s -o /dev/null -w "%{http_code}" https://rustacian-blog.com/health` でヘルスチェック
2. `curl -s -o /dev/null -w "%{http_code}" https://rustacian-blog.com/` でトップページ確認
3. `az containerapp show --name rustacian-prod-ca --resource-group rustacian-prod-rg --query "properties.latestRevisionName" -o tsv` でアクティブ revision 確認
4. `gh run list --repo kazuma0606/rustcian_blog --limit 5 --json name,status,conclusion,createdAt --jq '.[] | "\(.name) | \(.status) | \(.conclusion) | \(.createdAt)"'` で最新ワークフロー結果確認

結果を表形式でまとめ、異常があれば原因と対処を提案してください。
