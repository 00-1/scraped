#!/usr/bin/env bash
# Writes players.html for the Pages site (C01): every released player
# build with its checksum, newest release first, from the repository's
# GitHub releases. Usage: tools/site/players.sh OWNER/REPO OUT.html
# (GITHUB_TOKEN, when set, avoids the API's rate limit).
set -euo pipefail
repo=$1
out=$2
auth=()
[ -n "${GITHUB_TOKEN:-}" ] && auth=(-H "Authorization: Bearer $GITHUB_TOKEN")
releases=$(curl -fsSL "${auth[@]}" "https://api.github.com/repos/$repo/releases?per_page=100" || echo '[]')
{
  cat <<'HEAD'
<!doctype html>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Scraped Again Players</title>
<style>
body { font: 16px/1.5 system-ui, sans-serif; max-width: 48rem; margin: 2rem auto; padding: 0 1rem; }
code { font-size: 0.8em; word-break: break-all; }
td { padding: 0.2rem 0.6rem 0.2rem 0; vertical-align: top; }
</style>
<h1>Player program builds</h1>
<p>Every released build of the player program, with its SHA-256 checksum, so
a saved world can be replayed on the builds it was played on.</p>
HEAD
  [ "$(echo "$releases" | jq '[.[] | select(any(.assets[]; .name | startswith("scraped-player")))] | length')" = 0 ] && echo "<p>No releases with a player program yet.</p>"
  echo "$releases" | jq -r '.[] | select((.draft | not) and any(.assets[]; .name | startswith("scraped-player"))) | .tag_name' | while read -r tag; do
    [ -n "$tag" ] || continue
    sums=$(curl -fsSL "https://github.com/$repo/releases/download/$tag/SHA256SUMS" 2>/dev/null || true)
    echo "<h2>$tag</h2><table>"
    echo "$releases" | jq -r --arg t "$tag" '.[] | select(.tag_name == $t) | .assets[] | select(.name | startswith("scraped-player")) | "\(.name)\t\(.browser_download_url)"' |
      while IFS=$'\t' read -r name url; do
        sum=$(echo "$sums" | awk -v n="$name" '$2 == n { print $1 }')
        echo "<tr><td><a href=\"$url\">$name</a></td><td><code>${sum:-}</code></td></tr>"
      done
    echo "</table>"
  done
} > "$out"
