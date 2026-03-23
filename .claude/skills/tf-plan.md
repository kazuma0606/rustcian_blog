---
description: terraform plan を実行して差分を確認する。apply が必要な場合は確認を求める
---

以下の手順で Terraform の差分を確認してください。

1. terraform ディレクトリに移動して plan を実行：

```bash
cd /c/Users/yoshi/rustacian_blog/terraform && terraform plan 2>&1
```

2. plan の結果を確認し、変更内容をわかりやすく要約してください：
   - 追加されるリソース（`+`）
   - 変更されるリソース（`~`）
   - 削除されるリソース（`-`）

3. 削除や意図しない変更が含まれていないか確認し、問題なければユーザーに `terraform apply` の実行可否を確認してください。

**注意事項**:
- `terraform.tfvars` の `container_image` が `latest` になっている場合、ACR に `latest` タグが存在しないためエラーになります。現在の SHA タグを確認して修正してください。
- apply を実行する前に必ずユーザーの承認を得てください。
