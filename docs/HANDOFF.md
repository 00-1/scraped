# Handoff

Where the project stands, for the next agent. Read `CLAUDE.md` first, then this.

## State (2026-10-03)

- **Done:** the first roadmap (M01–M14), the Android app (Compose + Rust; preview APK at the GitHub release `android-preview-1`), and depth milestones **D01 (Instruments)** and **D02 (Quiet text)**.
- **Next:** **D03 — Places with character** (`docs/milestones/D03-places.md`). Nothing of it is started.
- **Branch:** `claude/laughing-edison-9brdzg`. Everything is committed and pushed. CI (`.github/workflows/ci.yml`) runs fmt, clippy, tests, the release-mode bot tests, the wasm build, the browser smoke tests and the Android build.
- `content release-check` fails on purpose. It is the release gate, and it waits for Jb's own text.

## How to measure

Every depth milestone is judged by numbers and by reading samples:

- `cargo run --release -p scraped-cli -- depth --seeds 1-10`: the metrics (definitions and the D01/D02 tables in `docs/DEPTH.md`).
- `... -- bots --seeds 1-10 [--bot explorer|scholar]`: how far the bots get.
- `... -- samples D03`: explorer transcripts into `docs/samples/D03/`. Write a `NOTES.md` beside them.
- To regenerate snapshots after an intended change: `UPDATE_SNAPSHOTS=1 cargo test --release -p scraped-game --tests`. That covers `crates/game/tests/depth.txt` and `transcripts.txt`. Then run `tools/build.sh && node tools/smoke/determinism.cjs`, so wasm matches native.

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

## Known rough edges

- **Example text prints raw ids** in places ("lake", "birds"). It is for Jb to write over; don't polish the example prose.
- **Group members read alike.** D03's building variety should give standouts more to say.
- **The scholar bot** reads the deepest text on only 5 of 10 seeds (held doors need rare words; see the D01 findings in `docs/DEPTH.md`). This is expected to improve in D07/D09; its test holds at 5.
- **Open design questions** are collected in `docs/DECISIONS.md`. Build on the defaults there and add new `// DESIGN-Q:` rows as you go.
