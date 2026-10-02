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

### Browser bench

`tools/bench/build.sh` builds `tools/bench/dist/scraped-bench.html`: one page with
the engine compiled to WebAssembly. It shows eras side by side, glyphs and the
script table, and has every difficulty dial. It needs the
`wasm32-unknown-unknown` target (`rustup target add wasm32-unknown-unknown`).

## Layout

- `crates/lang` — the language engine (pure library, WebAssembly-compatible).
  - `data/concepts.toml` — the starter concept list.
  - `tests/snapshots/` — pinned output for seeds 1, 42 and 9001, every era. After an
    intended change, regenerate with `UPDATE_SNAPSHOTS=1 cargo test` and review the diff.
- `crates/cli` — the `scraped-lang` terminal tool.
- `tools/bench` — the browser bench (outside the workspace; replaced by the `web` crate in M03).

## Checks

```sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build -p scraped-lang --target wasm32-unknown-unknown
```
