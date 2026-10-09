#!/usr/bin/env sh
# Checks the player build's real text (S05): the hand player's commands on
# seeds 1, 42 and 9001, and the seed 42 fixture from a hand replay, fed to
# the released player program; any slip (a hole, an empty item, a line cut
# off on a word that needs another, choices joined with "and") fails.
#   tools/smoke/slips.sh [PLAYER] [SCRAPED-LANG]
set -eu
bin="${1:-target/player/scraped-player}"
lang="${2:-target/release/scraped-lang}"
here=$(dirname "$0")/../..
for seed in 1 42 9001; do
  "$bin" --seed "$seed" < "$here/docs/samples/S04/commands-$seed.txt" | "$lang" slips
done
"$bin" --seed 42 < "$here/tests/fixtures/s05-seed42.txt" | "$lang" slips
echo "slips smoke: ok"
