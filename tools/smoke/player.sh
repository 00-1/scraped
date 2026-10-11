#!/usr/bin/env sh
# Checks a built player program (C01): no spoiler options, no ground truth
# in its replies, the fair-play note shown, and no spoiler internals or
# authoring notes inside the binary.
#   tools/smoke/player.sh [PLAYER] [PLAYER-WASM]
set -eu
bin="${1:-target/player/scraped-player}"
wasm="${2:-target/player/wasm32-unknown-unknown/release/scraped_web.wasm}"
fail() { echo "player smoke: $1" >&2; exit 1; }
"$bin" --version | grep -q "scraped-player" || fail "no version"
if "$bin" --spoil </dev/null >/dev/null 2>&1; then fail "--spoil accepted"; fi
out=$(printf 'look\nread\nhelp\n' | "$bin" --json --seed 42)
echo "$out" | head -1 | grep -q '"fair_play"' || fail "no fair-play note in the first reply"
if echo "$out" | grep -q '"truth"'; then fail "ground truth in a reply"; fi
# The text and nothing more (S06): every reply's keys are among the allowed
# few, and nothing in it but the text names the world.
long=$(printf 'look\nlook around\nlisten\ngo in\nlook\ntake all\ninventory\nstatus\nhelp\nout\nwait\n' | "$bin" --json --seed 42)
printf "%s\n" "$long" | python3 -c '
import json, sys
allowed = {"protocol", "text", "ended", "fair_play", "code", "file"}
for n, line in enumerate(sys.stdin):
    v = json.loads(line)
    extra = set(v) - allowed
    if extra:
        sys.exit("reply %d carries %s" % (n, sorted(extra)))
' || fail "a reply carries more than the text"
# Spoiler internals (the facts the attention model weighed) and authoring
# notes must not be in the binary at all.
if strings "$bin" | grep -q 'salience'; then fail "spoiler fields compiled in"; fi
if strings "$bin" | grep -q 'notes = "'; then fail "authoring notes baked in"; fi
# The browser player's engine, built by tools/build.sh: the same, and no
# bench or authoring calls.
if [ -f "$wasm" ]; then
  if grep -a -q 'salience' "$wasm"; then fail "spoiler fields in the browser player"; fi
  if grep -a -q 'lint_variant' "$wasm"; then fail "authoring calls in the browser player"; fi
fi
echo "player smoke: ok"
