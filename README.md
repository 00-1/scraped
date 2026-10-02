# Scraped Again

A text-only game about deciphering a lost, procedurally generated language in a world where writing has power — and scraping it away releases that power.

- Design: [`docs/DESIGN.md`](docs/DESIGN.md)
- Roadmap and milestones: [`docs/ROADMAP.md`](docs/ROADMAP.md)
- Agent briefing: [`CLAUDE.md`](CLAUDE.md)
- Work log: [`docs/LOG.md`](docs/LOG.md)

Status: feature-complete (milestones M01–M14); waiting on Jb's text. The release gate passes once every required slot has his own writing.

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

In a browser: `tools/build.sh`, then open `tools/dist/play.html` (also
published to GitHub Pages). Saves, transcripts and notebooks stay in the
browser or download as files.

In a terminal (release binaries are attached to each GitHub release):

```sh
# Wake in a generated town and explore it: look, go temple, read stele, help.
cargo run -q -p scraped-play -- --seed 42
cargo run -q -p scraped-play -- --difficulty gentle      # gentle | standard | archaeologist
cargo run -q -p scraped-play -- --code K5G0-9ZQ1          # a world someone shared

# For agents: one JSON command per line in, one JSON response per line out.
echo '{"cmd": "look"}' | cargo run -q -p scraped-play -- --seed 42 --json
```

In play, `save`, `load`, `transcript on|off`, `export` (the notebook:
transcript, named places and run record, once the run is over) and `quit`
handle the session; `--legacy [FILE]` carries the last run's final
inscription into the next world as a faint, very old layer;
`code` shows the world's shareable code and `manual` the player's
manual. A seed whose world fails the fairness check is quietly replaced by
a fair world derived from it (`--raw` turns that off). `define 3 as ka`
gives a glyph your own label. Outdoors: `head north`,
`go to the tower` (anything in view), `follow the river downstream`,
`go back`, `name this place the gap` and later `go to the gap`. In fog or at
night you drift without knowing it; the bench's Play tab shows where you
really are when spoilers are on.

Staying alive: `drink` (from a well, river or lake), `fill waterskin`, `eat`,
`sleep`, `forage`, `gather wood`, `make fire` (firesteel and wood), `make
torch`, `light torch`, `wear cloak`, `status`. Mechanisms: `open sluice`,
`pull lever`, `pry door`. `shout` scares some creatures off and brings down
loose stone; `cross` tries ice, wading or swimming. With the scraper,
`scrape <thing>` scrapes its top fresh writing away, and potent writing acts.
With the stylus, `write 4 12 7 / 3 9 on wall` writes glyphs (script numbers,
or your own labels from `define`) once you have met each word in two texts;
the lens shows the layer beneath when you read. Time: `wait 2 weeks`; the
world drifts by region (spoilers: `world --seed N regions --spoil`), and
stronger scrapers reach farther, up to the great inscriptions.

Agents can play too: `--json` (see [`docs/PROTOCOL.md`](docs/PROTOCOL.md)),
or the MCP server `scraped-mcp`, with house rules for playing alongside a
person in [`docs/coop/CLAUDE.md`](docs/coop/CLAUDE.md).

### Worlds

```sh
# Spoilers: the map (ASCII, or PNG), the history, one settlement's buildings and writing.
cargo run -q -p scraped-cli -- world --seed 42 map --spoil
cargo run -q -p scraped-cli -- world --seed 42 map --spoil --png world.png
cargo run -q -p scraped-cli -- world --seed 42 history --spoil
cargo run -q -p scraped-cli -- world --seed 42 site 0 --spoil
cargo run -q -p scraped-cli -- world --seed 42 writing --spoil   # live claims, surface stacks
cargo run -q -p scraped-cli -- fair --seeds 1-50 --all            # solvability batch
```

### Browser tools

`tools/build.sh` builds three self-contained pages into `tools/dist/`, each
with the engine (`crates/web`) compiled to WebAssembly: `play.html` (the
game, above) and:

- `author.html`, the **authoring tool**: every content slot with its status,
  what it is for and its variables; an editor that lints as you type; live
  previews against real generated worlds; storylets with a placement
  preview; a playtest beside the editor where every line links to the text
  that wrote it and edits show at once; gaps ranked by how often players
  meet them; voice tools; inspectors; the diff since the last commit; and
  saving straight into `content/` (Chrome/Edge), or via zip export and
  copy-to-clipboard.
- `bench.html`, the **bench**: the game (Play tab; spoilers show your true position and body), eras side by side, glyphs, the script
  table, every difficulty dial, and the world (map, history, sites).

The Pages workflow publishes both. It needs the `wasm32-unknown-unknown`
target (`rustup target add wasm32-unknown-unknown`) and `python3`.

### Content

Every English word the player reads is written by Jb in `content/` (see
`content/README.md`). Check it with `scraped-lang content lint`,
`content coverage`, `content preview SLOT` and `content release-check`.

## Layout

- `crates/lang` — the language engine (pure library, WebAssembly-compatible), including
  the parser that reads text back into meaning.
  - `data/concepts.toml` — the starter concept list.
  - `tests/snapshots/` — pinned output for seeds 1, 42 and 9001, every era. After an
    intended change, regenerate with `UPDATE_SNAPSHOTS=1 cargo test` and review the diff.
- `crates/cli` — the `scraped-lang` terminal tool.
- `crates/content` — content slots, the template language, the pack, lint and coverage.
- `crates/world` — world generation: terrain, rivers, history, structures, texts, decay.
- `crates/sim` — the physical world: sight, landmarks, travel costs, local properties,
  the rule table (`data/rules.toml`), mechanisms, items, the body, creatures, and
  writing that acts (`data/claims.toml`, surfaces, layers, claims).
- `crates/game` — game state, the parser (`data/verbs.toml`), commands and travel.
- `crates/play` — the `scraped` terminal client, the JSON-lines agent protocol and
  the `scraped-mcp` MCP server.
- `crates/web` — the engine for browsers: one WebAssembly module with a JSON interface.
- `tools/play`, `tools/bench`, `tools/author` — page templates for the player, the
  bench and the authoring tool; `tools/smoke` — their headless tests.
- `content/` — Jb's content pack.

## Checks

```sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build -p scraped-lang --target wasm32-unknown-unknown
```
