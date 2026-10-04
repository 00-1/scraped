#!/usr/bin/env sh
# Checks a built player program (C01): no spoiler options, no ground truth
# in its replies, the fair-play note shown, and no spoiler internals or
# authoring notes inside the binary.
#   tools/smoke/player.sh target/player/scraped-player
set -eu
bin="${1:-target/player/scraped-player}"
fail() { echo "player smoke: $1" >&2; exit 1; }
"$bin" --version | grep -q "scraped-player" || fail "no version"
if "$bin" --spoil </dev/null >/dev/null 2>&1; then fail "--spoil accepted"; fi
out=$(printf 'look\nread\nhelp\n' | "$bin" --json --seed 42)
echo "$out" | head -1 | grep -q '"fair_play"' || fail "no fair-play note in the first reply"
if echo "$out" | grep -q '"truth"'; then fail "ground truth in a reply"; fi
# Spoiler internals (the facts the attention model weighed) and authoring
# notes must not be in the binary at all.
if strings "$bin" | grep -q 'salience'; then fail "spoiler fields compiled in"; fi
if strings "$bin" | grep -q 'notes = "'; then fail "authoring notes baked in"; fi
echo "player smoke: ok"
