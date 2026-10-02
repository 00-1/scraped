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
# Convert to plain JavaScript: some hosts (e.g. claude.ai artifacts) forbid
# compiling WebAssembly, but run ordinary scripts.
js=$(mktemp)
npx --yes -p binaryen@125 wasm2js --enable-bulk-memory --enable-sign-ext \
  --enable-nontrapping-float-to-int --enable-mutable-globals -O2 "$wasm" -o "$js"
# Turn the module's exports into one global, ENGINE.
sed -i -e '/^export var /d' "$js"
echo 'var ENGINE = { memory: retasmFunc.memory, generate: retasmFunc.generate, output_len: retasmFunc.output_len };' >> "$js"
# The template holds the engine on a line of its own: __ENGINE__
{
  sed '/^__ENGINE__$/,$d' index.html
  cat "$js"
  sed '1,/^__ENGINE__$/d' index.html
} > dist/scraped-bench.html
rm -f "$js"
echo "wrote tools/bench/dist/scraped-bench.html ($(wc -c < dist/scraped-bench.html) bytes)"
