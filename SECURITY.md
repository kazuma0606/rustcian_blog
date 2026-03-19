# Security Policy

## Supported Scope
このリポジトリでは、少なくとも以下をセキュリティ対象とする。

- Git に commit されるファイル
- GitHub Actions workflow
- ローカル設定ファイルの取り扱い
- 将来の Azure デプロイ時の認証方式

## Secrets Handling
- `.env.local` は commit しない
- `.env.local.example` のみをサンプルとして commit する
- API key、接続文字列、証明書、秘密鍵は Git に含めない
- ローカル検証用の Azurite 既定キーは公開済み emulator 用の値であり、本番 secret ではない

## GitHub Actions
- push / pull request 時に CI と security workflow を実行する
- secret scan には `gitleaks` を使う
- dependency vulnerability scan には `cargo audit` を使う

## Azure Authentication Policy
- GitHub Actions から Azure へ接続する際は OIDC を優先する
- 長期の client secret を GitHub Secrets に保存する構成は避ける
- 本番 secret は Azure Key Vault を第一候補とする

## Reporting
本番運用前の段階のため、現時点では private な連絡経路を前提とする。
公開運用へ進む際は、報告窓口と response policy を別途整備する。
