#!/usr/bin/env bash
# Commit data/offerings and push it onto the remote branch.
# The scrape runs for about an hour. A plain `git push` is rejected when main
# moves during that time (run 45004068037). Rebase, then push, and retry when
# the remote moves again between rebase and push.
set -euo pipefail

branch="${1:-main}"
remote="${2:-origin}"

git config user.name "github-actions[bot]"
git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
git add data/offerings
if git diff --cached --quiet; then
  echo "No offering changes."
  exit 0
fi

git commit -m "Refresh SIS offerings ($(date -u +%F))"

for attempt in 1 2 3; do
  # actions/checkout fetches with depth 1, which cannot rebase until the
  # remote tip shares history with HEAD.
  git fetch --depth=50 "$remote" "$branch"
  if ! git rebase "$remote/$branch"; then
    git rebase --abort >/dev/null 2>&1 || true
    echo "::error::Offerings commit conflicts with ${remote}/${branch}"
    exit 1
  fi
  if git push "$remote" "HEAD:${branch}"; then
    exit 0
  fi
  echo "Push to ${remote}/${branch} rejected (attempt ${attempt}); retrying."
done

echo "::error::Push of SIS offerings to ${remote}/${branch} was rejected after 3 attempts"
exit 1
