# Future Task: 開発者体験向上のためのDiscord通知連携

## 概要
- Cargo buildやCargo check、テスト実行などのCI/CDやローカル開発の結果をDiscordに自動通知することで、開発者体験（DX）を向上させる。
- チーム全体でビルド・テスト状況を即座に共有でき、リモート開発や複数人開発でも安心して進められる。

## メリット
- CI/CDの失敗や成功を即座にチームで共有できる
- “壊れたまま気づかない”を防げる
- Slackやメールよりも気軽に通知を受け取れる
- PRレビューやデプロイ通知にも拡張可能

## 実現方法案

### 1. CI/CD（GitHub Actions, GitLab CI, etc.）でDiscord Webhook通知
- 例: GitHub Actionsのstepsでcurlやdiscord.shを使い、ビルド・テスト結果をDiscordにPOST

```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build
        run: cargo build
      - name: Notify Discord on Success
        if: success()
        run: |
          curl -H "Content-Type: application/json" \
            -X POST \
            -d '{"content":":white_check_mark: Cargo build succeeded!"}' \
            ${{ secrets.DISCORD_WEBHOOK_URL }}
      - name: Notify Discord on Failure
        if: failure()
        run: |
          curl -H "Content-Type: application/json" \
            -X POST \
            -d '{"content":":x: Cargo build failed!"}' \
            ${{ secrets.DISCORD_WEBHOOK_URL }}
```

### 2. ローカル開発でも通知
- cargo buildやcargo checkのラッパースクリプト（bash/powershell）を作り、成功・失敗時にDiscord WebhookへPOST

```bash
#!/bin/bash
if cargo build; then
  curl -H "Content-Type: application/json" -X POST -d '{"content":"✅ Cargo build succeeded!"}' "$DISCORD_WEBHOOK_URL"
else
  curl -H "Content-Type: application/json" -X POST -d '{"content":"❌ Cargo build failed!"}' "$DISCORD_WEBHOOK_URL"
fi
```

### 3. Rustのビルドスクリプトやカスタムツールで通知
- build.rsや独自のRust CLIツールでビルド結果を検知し、reqwestでDiscord通知

## 今後の拡張例
- PRレビューやデプロイ通知、テストカバレッジ通知などにも応用可能
- 通知内容のカスタマイズや、通知先チャンネルの切り替えも検討

---

「こういう通知が欲しい」「このタイミングでアラートしたい」など、要望があれば具体的な実装例も検討する。 