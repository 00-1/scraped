# Handoff

- **Where Jb plays:** in a browser, by downloading `scraped-player-browser.html` from the release https://github.com/00-1/scraped/releases/tag/v0.1.1 and opening it (one self-contained file), or at https://00-1.github.io/scraped/play.html once Pages is switched on (Settings → Pages → Source: GitHub Actions; until then the Pages workflow's `configure-pages` step fails); or on Android with the APK attached to the release https://github.com/00-1/scraped/releases/tag/v0.1.1 (newer test builds: the latest `android-preview-N` prerelease).
- **Version:** v0.1.1 (D10; v0.1.0 was the first release, S03). The player programs for a terminal or an agent are attached to each release (and listed at https://00-1.github.io/scraped/players.html once Pages is on).
- **Sending a bug:** export the save (browser: menu → Export save; Android: long-press the world in the list → Export; terminal: `save`) and send the save file with a line on what looked wrong. It is replayed exactly with `scraped-lang replay SAVE` (add `--build VERSION=PATH` for stretches played on another release).

Where the project stands, for the next agent. Read `CLAUDE.md` first, then this.

## State (2026-10-04)

- **Done:** the first roadmap (M01–M14), the Android app (Compose + Rust; preview APK at the GitHub release `android-preview-1`), depth milestones **D01–D08**, the course corrections **S01** and **S02**, **C01** (shared play and versions), **D09** (the magic, deepened; targets partly met, see LOG), **S03** (before Jb plays) and **D10** (the slow realisation; bots short of their old marks, see LOG).
- **Next:** **D11 — Problems only writing solves** (`docs/milestones/D11-writing-payoff.md`). D10 made the tools ordinary (`crates/sim/src/items.rs`), let cleaning release writing (`clean` in `crates/game/src/writing.rs`), left a third of everyday spells latent, and gave a way out without writing (`world_edge` in `crates/game/src/site.rs`, `go beyond`). Saves are snapshots and releases follow `docs/VERSIONING.md` (`scraped-lang saves check` after a change to rules or generation).
- **Wasm:** usize is 32 bits there. Take a hash modulo as u64 before casting to usize, or picks differ from native (`node tools/smoke/determinism.cjs` catches it).
- **Branch:** `claude/laughing-edison-9brdzg`. Everything is committed and pushed. CI (`.github/workflows/ci.yml`) runs fmt, clippy, tests, the release-mode bot tests, the wasm build, the browser smoke tests and the Android build.
- `content release-check` fails on purpose. It is the release gate, and it waits for Jb's own text.

## How to measure

Every depth milestone is judged by numbers and by reading samples:

- `cargo run --release -p scraped-cli -- depth --seeds 1-10`: the metrics (definitions and the D01/D02 tables in `docs/DEPTH.md`).
- `... -- bots --seeds 1-10 [--bot explorer|scholar]`: how far the bots get.
- `... -- samples D04 [--hours 10]`: explorer transcripts into `docs/samples/D04/`. Write a `NOTES.md` beside them.
- To regenerate snapshots after an intended change: `UPDATE_SNAPSHOTS=1 cargo test --release -p scraped-game --tests`. That covers `crates/game/tests/transcripts.txt` and `crates/world/tests/fingerprints.txt`. Then run `tools/build.sh && node tools/smoke/determinism.cjs`, so wasm matches native.

## How descriptions work now (D02)

**Don't append lines to responses.** Each new thing in the world becomes a candidate `Fact` (`crates/game/src/attention.rs`):

- give it a slot, a stable key, a salience, and rich variables;
- add it in `outdoor_facts`, `room_facts`, `felt_facts` or a sense in `crates/game/src/senses.rs`;
- `attend` decides whether it's said.

Facts whose variables hold rendered names must stay pack-independent in memory. Names and ways are left out of the signature, and `Fact::meaning` stands in for them. Otherwise the hot-reload test fails.

New player-visible text follows `CLAUDE.md`:

- a `SlotDef` in `crates/game/src/quiet_slots.rs` (or `slots.rs`), with a sampler;
- one `example = true` variant in `content/`;
- no season names, region variable names or need levels outside `body.*` (tested).

## How places work now (D03)

The world's places live in `crates/world`:
- `geology.rs` (rock);
- `features.rs` (natural features and old marks, each with its cause);
- `towns.rs` (roles, layouts, districts, streets);
- `structures.rs` (the kinds table, `place_more` for the D03 kinds);
- `underground.rs`;
- `scenes.rs`.

The original buildings are placed first, from their old random stream, so adding kinds never changes them.

The game side is `crates/game/src/places.rs`: facts for districts, features and scenes; district and feature targets. The slots are in `place_slots.rs`.

## Known rough edges

- **Example text prints raw ids** in places ("lake", "birds"). It is for Jb to write over; don't polish the example prose.
- **Group members read alike** ("the intact tomb, to the east" twice).
- **The bots are sensitive** to any change in what's in view or in towns. Check survival on seeds 1–20 after world changes (`bots --seeds 1-20 --bot explorer`; D03: 17 of 20).
- **The scholar bot** reads the deepest text on only 6 of 10 seeds (held doors need rare words; see the D01 findings in `docs/DEPTH.md`). This is expected to improve in D08/D09; its test holds at 5.
- **Open design questions** are collected in `docs/DECISIONS.md`. Build on the defaults there and add new `// DESIGN-Q:` rows as you go.
