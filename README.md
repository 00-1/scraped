# Scraped Again

A text-only game about deciphering a lost, procedurally generated language in a world where writing has power — and scraping it away releases that power.

- Design: [`docs/DESIGN.md`](docs/DESIGN.md)
- Current milestone: [`docs/MILESTONE-1.md`](docs/MILESTONE-1.md)
- Agent briefing: [`CLAUDE.md`](CLAUDE.md)
- Work log: [`docs/LOG.md`](docs/LOG.md)

Status: pre-alpha. Building the language engine first.

## Try it

Requires stable Rust.

```sh
# A corpus of 40 inscriptions, romanised only. Decipher this on paper.
cargo run -q -p scraped-cli -- --seed 42 corpus

# Same corpus with types, glosses and translations (spoilers).
cargo run -q -p scraped-cli -- --seed 42 corpus --spoil

# The full generated grammar (spoilers).
cargo run -q -p scraped-cli -- --seed 42 grammar --spoil

# Structured output for agents and tooling; add --spoil for ground truth.
cargo run -q -p scraped-cli -- --seed 42 corpus --count 10 --json
```

A shorter corpus is always the start of a longer one for the same seed, so
`--count 10` and `--count 40` agree on their first ten lines.

## Layout

- `crates/lang` — the language engine (pure library, WebAssembly-compatible).
  - `data/concepts.toml` — the starter concept list.
  - `tests/snapshots/` — pinned output for seeds 1, 42 and 9001. After an
    intended change, regenerate with `UPDATE_SNAPSHOTS=1 cargo test` and review the diff.
- `crates/cli` — the `scraped-lang` terminal tool.

## Checks

```sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build -p scraped-lang --target wasm32-unknown-unknown
```
