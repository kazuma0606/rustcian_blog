#!/usr/bin/env bash
# Runs cargo fmt --check and cargo clippy before any git push.
# Blocks the push if either check fails.

input=$(cat)
cmd=$(node -e "const d=JSON.parse(process.argv[1]); console.log(d.tool_input&&d.tool_input.command||'')" "$input" 2>/dev/null)

if ! echo "$cmd" | grep -q 'git push'; then
  exit 0
fi

echo "Running pre-push checks (fmt + clippy)..." >&2

if ! cargo fmt --all --check 2>&1; then
  echo '{"continue":false,"stopReason":"cargo fmt --all --check failed. Run: cargo fmt --all"}'
  exit 1
fi

if ! cargo clippy --workspace --all-targets -- -D warnings 2>&1; then
  echo '{"continue":false,"stopReason":"cargo clippy failed. Fix warnings before pushing."}'
  exit 1
fi

echo "Pre-push checks passed." >&2
exit 0
