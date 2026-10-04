#!/usr/bin/env sh
# Builds the browser tools into tools/dist/: bench.html (the language bench)
# and author.html (the authoring tool). Each is one self-contained page with
# the engine (crates/web) embedded as WebAssembly. Open them in any browser.
set -eu
cd "$(dirname "$0")/.."
cargo build -q -p scraped-web --release --target wasm32-unknown-unknown
wasm="${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/release/scraped_web.wasm"
if [ ! -s "$wasm" ]; then
  echo "error: $wasm is missing or empty" >&2
  exit 1
fi
# The browser player (C01): the same engine without its spoiler and
# authoring calls.
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}/player" cargo build -q -p scraped-web --release --no-default-features --target wasm32-unknown-unknown
player="${CARGO_TARGET_DIR:-target}/player/wasm32-unknown-unknown/release/scraped_web.wasm"
commit=$(git rev-parse --short HEAD 2>/dev/null || echo unknown)
if [ -n "$(git status --porcelain -- crates tools content 2>/dev/null)" ]; then commit="$commit+changes"; fi
mkdir -p tools/dist
cp "$wasm" tools/dist/engine.wasm
# Splice the engine, a build stamp (so a stale copy is obvious) and the
# current content pack into each page template.
python3 - "$wasm" "$commit" "$player" <<'PY'
import base64, datetime, glob, json, os, sys
wasm, commit, player = sys.argv[1], sys.argv[2], sys.argv[3]
engine = base64.b64encode(open(wasm, 'rb').read()).decode()
player_engine = base64.b64encode(open(player, 'rb').read()).decode()
build = json.dumps({"commit": commit, "time": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")})
content = json.dumps([{"path": os.path.basename(f), "text": open(f, encoding="utf-8").read()}
                      for f in sorted(glob.glob("content/*.toml"))], ensure_ascii=False)
# The player's text: templates only, no authoring notes or comments.
def bare(text):
    return "".join(l for l in text.splitlines(True) if not l.lstrip().startswith(("notes =", "#")))
player_content = json.dumps([{"path": os.path.basename(f), "text": bare(open(f, encoding="utf-8").read())}
                             for f in sorted(glob.glob("content/*.toml"))], ensure_ascii=False)
for page in ["bench", "author", "play"]:
    src = open(f"tools/{page}/index.html", encoding="utf-8").read()
    eng, cont = (player_engine, player_content) if page == "play" else (engine, content)
    src = src.replace('"__ENGINE__";', json.dumps(eng) + ";")
    src = src.replace("__BUILD__", build).replace("__CONTENT__", cont.replace("</", "<\\/"))
    out = f"tools/dist/{page}.html"
    open(out, "w", encoding="utf-8").write(src)
    print(f"wrote {out} ({len(src.encode())} bytes)")
PY
