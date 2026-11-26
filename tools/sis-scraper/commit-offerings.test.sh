#!/usr/bin/env bash
# Exercises commit-offerings.sh against local git repos. No network.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
script="$root/tools/sis-scraper/commit-offerings.sh"
workflow="$root/.github/workflows/offerings.yml"

if grep -nE '^[[:space:]]*cache:' "$workflow"; then
  echo "offerings.yml must not enable actions/setup-node cache on the self-hosted runner" >&2
  exit 1
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

git_identity() {
  git -C "$1" config user.email "test@example.com"
  git -C "$1" config user.name "test"
}

init_origin() {
  local origin="$1" seed="$2"
  git init --bare -b main "$origin" >/dev/null
  git clone "file://$origin" "$seed" >/dev/null
  git_identity "$seed"
  mkdir -p "$seed/data/offerings"
  printf '%s\n' '{"v":0}' >"$seed/data/offerings/index.json"
  printf '%s\n' base >"$seed/README"
  git -C "$seed" add .
  git -C "$seed" commit -m init >/dev/null
  git -C "$seed" push origin main >/dev/null
}

advance_main() {
  local seed="$1" name="$2" body="$3"
  printf '%s\n' "$body" >"$seed/$name"
  git -C "$seed" add "$name"
  git -C "$seed" commit -m "$name" >/dev/null
  git -C "$seed" push origin main >/dev/null
}

shallow_runner() {
  local origin="$1" runner="$2"
  git clone --depth 1 "file://$origin" "$runner" >/dev/null
  if [ "$(git -C "$runner" rev-parse --is-shallow-repository)" != true ]; then
    echo "expected a shallow clone" >&2
    exit 1
  fi
}

# Remote main moves during the scrape. The offerings commit lands on top.
{
  origin="$tmp/move.git"
  seed="$tmp/move-seed"
  runner="$tmp/move-runner"
  init_origin "$origin" "$seed"
  shallow_runner "$origin" "$runner"
  advance_main "$seed" other.txt "other work"
  printf '%s\n' '{"v":1}' >"$runner/data/offerings/index.json"
  (cd "$runner" && bash "$script" main origin)
  git -C "$seed" fetch origin >/dev/null
  tip=$(git -C "$seed" log -1 --format='%s' origin/main)
  case "$tip" in
    "Refresh SIS offerings ("*) ;;
    *) echo "unexpected tip: $tip" >&2; exit 1 ;;
  esac
  git -C "$seed" merge-base --is-ancestor origin/main~1 origin/main
  test "$(git -C "$seed" show origin/main:data/offerings/index.json)" = '{"v":1}'
  test "$(git -C "$seed" show origin/main:other.txt)" = "other work"
  echo "ok: rebase onto moved main"
}

# No offering diff: do not commit or push.
{
  origin="$tmp/noop.git"
  seed="$tmp/noop-seed"
  runner="$tmp/noop-runner"
  init_origin "$origin" "$seed"
  shallow_runner "$origin" "$runner"
  before=$(git -C "$origin" rev-parse main)
  out=$(cd "$runner" && bash "$script" main origin)
  after=$(git -C "$origin" rev-parse main)
  test "$before" = "$after"
  case "$out" in
    *"No offering changes."*) ;;
    *) echo "missing no-change message: $out" >&2; exit 1 ;;
  esac
  echo "ok: no changes"
}

# First push is rejected, the retry publishes the offerings commit.
{
  origin="$tmp/retry.git"
  seed="$tmp/retry-seed"
  runner="$tmp/retry-runner"
  marker="$tmp/retry-rejected"
  init_origin "$origin" "$seed"
  shallow_runner "$origin" "$runner"
  advance_main "$seed" other.txt "other work"
  cat >"$origin/hooks/pre-receive" <<EOF
#!/bin/sh
if [ ! -f "$marker" ]; then
  touch "$marker"
  echo "rejected once" >&2
  exit 1
fi
EOF
  chmod +x "$origin/hooks/pre-receive"
  printf '%s\n' '{"v":1}' >"$runner/data/offerings/index.json"
  (cd "$runner" && bash "$script" main origin)
  test -f "$marker"
  git -C "$seed" fetch origin >/dev/null
  test "$(git -C "$seed" show origin/main:data/offerings/index.json)" = '{"v":1}'
  echo "ok: retry after rejected push"
}

# Same lines edited on both sides: fail, leave no rebase in progress, do not push.
{
  origin="$tmp/conflict.git"
  seed="$tmp/conflict-seed"
  runner="$tmp/conflict-runner"
  init_origin "$origin" "$seed"
  shallow_runner "$origin" "$runner"
  printf '%s\n' '{"v":2}' >"$seed/data/offerings/index.json"
  git -C "$seed" add data/offerings/index.json
  git -C "$seed" commit -m remote-edit >/dev/null
  git -C "$seed" push origin main >/dev/null
  remote_tip=$(git -C "$origin" rev-parse main)
  printf '%s\n' '{"v":1}' >"$runner/data/offerings/index.json"
  set +e
  (cd "$runner" && bash "$script" main origin)
  status=$?
  set -e
  test "$status" -ne 0
  test ! -d "$runner/.git/rebase-merge"
  test ! -d "$runner/.git/rebase-apply"
  test "$(git -C "$origin" rev-parse main)" = "$remote_tip"
  echo "ok: conflict does not push"
}

echo "commit-offerings tests passed"
