#!/usr/bin/env sh
# Builds tools/bench/dist/scraped-bench.html: one self-contained page with the
# language engine embedded as WebAssembly. Open it in any browser.
set -eu
cd "$(dirname "$0")"
cargo build --release --target wasm32-unknown-unknown
mkdir -p dist
wasm="${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/release/scraped_bench.wasm"
if [ ! -s "$wasm" ]; then
  echo "error: $wasm is missing or empty" >&2
  exit 1
fi
b64=$(base64 < "$wasm" | tr -d '\n')
# The template holds the engine on a line of its own: "__ENGINE__";
{
  sed '/^"__ENGINE__";$/,$d' index.html
  printf '"%s";\n' "$b64"
  sed '1,/^"__ENGINE__";$/d' index.html
} > dist/scraped-bench.html
echo "wrote tools/bench/dist/scraped-bench.html ($(wc -c < dist/scraped-bench.html) bytes)"
