# Scraped Again

A text-only game about deciphering a lost, procedurally generated language in a world where writing has power — and scraping it away releases that power.

- Design: [`docs/DESIGN.md`](docs/DESIGN.md)
- Roadmap and milestones: [`docs/ROADMAP.md`](docs/ROADMAP.md)
- Agent briefing: [`CLAUDE.md`](CLAUDE.md)
- Work log: [`docs/LOG.md`](docs/LOG.md)

Status: pre-alpha. Building the language engine first.

## Try it

Requires stable Rust.

```sh
# 40 inscriptions, romanised only. Decipher this on paper.
cargo run -q -p scraped-cli -- --seed 42 corpus

# The same texts two eras later, and as numbered glyphs.
cargo run -q -p scraped-cli -- --seed 42 corpus --era 2
cargo run -q -p scraped-cli -- --seed 42 corpus --glyphs

# Spoilers: glosses, the grammar, the script table, the sound changes.
cargo run -q -p scraped-cli -- --seed 42 corpus --spoil
cargo run -q -p scraped-cli -- --seed 42 grammar --spoil --era 1
cargo run -q -p scraped-cli -- --seed 42 script --spoil
cargo run -q -p scraped-cli -- --seed 42 eras --spoil

# Difficulty dials, and JSON for agents and tools.
cargo run -q -p scraped-cli -- --seed 42 corpus --separation none --mark-names --script-kind abjad
cargo run -q -p scraped-cli -- --seed 42 corpus --count 10 --json
```

A shorter corpus is always the start of a longer one for the same seed.

### Play

```sh
# Wake in a generated town and explore it: look, go temple, read stele, help.
cargo run -q -p scraped-play -- --seed 42

# For agents: one JSON command per line in, one JSON response per line out.
echo '{"cmd": "look"}' | cargo run -q -p scraped-play -- --seed 42 --json
```

In play, `save`, `load`, `transcript on|off` and `quit` handle the session;
`define 3 as ka` gives a glyph your own label. Outdoors: `head north`,
`go to the tower` (anything in view), `follow the river downstream`,
`go back`, `name this place the gap` and later `go to the gap`. In fog or at
night you drift without knowing it; the bench's Play tab shows where you
really are when spoilers are on.

Staying alive: `drink` (from a well, river or lake), `fill waterskin`, `eat`,
`sleep`, `forage`, `gather wood`, `make fire` (firesteel and wood), `make
torch`, `light torch`, `wear cloak`, `status`. Mechanisms: `open sluice`,
`pull lever`, `pry door`. `shout` scares some creatures off and brings down
loose stone; `cross` tries ice, wading or swimming.

### Worlds

```sh
# Spoilers: the map (ASCII, or PNG), the history, one settlement's buildings and writing.
cargo run -q -p scraped-cli -- world --seed 42 map --spoil
cargo run -q -p scraped-cli -- world --seed 42 map --spoil --png world.png
cargo run -q -p scraped-cli -- world --seed 42 history --spoil
cargo run -q -p scraped-cli -- world --seed 42 site 0 --spoil
```

### Browser tools

`tools/build.sh` builds two self-contained pages into `tools/dist/`, each with
the engine (`crates/web`) compiled to WebAssembly:

- `author.html`, the **authoring tool**: every content slot with its status,
  what it is for and its variables; an editor that lints as you type; live
  previews against real generated worlds; saving straight into `content/`
  (Chrome/Edge), or via zip export and copy-to-clipboard.
- `bench.html`, the **bench**: the game (Play tab; spoilers show your true position and body), eras side by side, glyphs, the script
  table, every difficulty dial, and the world (map, history, sites).

The Pages workflow publishes both. It needs the `wasm32-unknown-unknown`
target (`rustup target add wasm32-unknown-unknown`) and `python3`.

### Content

Every English word the player reads is written by Jb in `content/` (see
`content/README.md`). Check it with `scraped-lang content lint`,
`content coverage`, `content preview SLOT` and `content release-check`.

## Layout

- `crates/lang` — the language engine (pure library, WebAssembly-compatible).
  - `data/concepts.toml` — the starter concept list.
  - `tests/snapshots/` — pinned output for seeds 1, 42 and 9001, every era. After an
    intended change, regenerate with `UPDATE_SNAPSHOTS=1 cargo test` and review the diff.
- `crates/cli` — the `scraped-lang` terminal tool.
- `crates/content` — content slots, the template language, the pack, lint and coverage.
- `crates/world` — world generation: terrain, rivers, history, structures, texts, decay.
- `crates/sim` — the physical world: sight, landmarks, travel costs, local properties,
  the rule table (`data/rules.toml`), mechanisms, items, the body, creatures.
- `crates/game` — game state, the parser (`data/verbs.toml`), commands and travel.
- `crates/play` — the `scraped` terminal client and JSON-lines agent protocol.
- `crates/web` — the engine for browsers: one WebAssembly module with a JSON interface.
- `tools/bench`, `tools/author` — page templates for the bench and the authoring tool.
- `content/` — Jb's content pack.

## Checks

```sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build -p scraped-lang --target wasm32-unknown-unknown
```
