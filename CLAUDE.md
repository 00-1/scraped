# CLAUDE.md — Scraped Again

Briefing for coding agents working in this repo. Read this first, then `docs/DESIGN.md` (intent) and the current milestone doc (scope).

## What this is

A text-only, procedurally generated exploration/survival game about deciphering a lost language in a world where writing has power. Output is text only; input is simple typed prompts. The full design lives in `docs/DESIGN.md`. **Do not invent design.** If a decision isn't covered there or in the milestone doc, pick the simplest option that keeps doors open, leave a `// DESIGN-Q:` comment, and list it in your summary for Jb to decide.

## Current milestone

`docs/MILESTONE-1.md` — the language engine slice. Work only within its scope. Do not start world generation, movement, survival or UI work yet.

## Tech stack

- **Rust**, stable toolchain, edition 2021.
- Cargo workspace:
  - `crates/lang` — the language engine (library). Pure logic, no I/O.
  - `crates/cli` — a small terminal binary for inspecting generated output.
  - Later: `crates/world`, `crates/game`, `crates/web` (WebAssembly front end). Don't create these yet.
- Keep the core crates **WebAssembly-compatible**: no threads, filesystem or OS-specific calls inside library crates. I/O belongs only in `cli` (and later `web`).
- Randomness: a seeded RNG (`rand_chacha::ChaCha8Rng` or similar) passed explicitly. **Never** use thread-local or OS randomness in library code.
- Serialisation: `serde` + `serde_json` for any structured output.
- Keep dependencies few and well-known.

## Non-negotiable principles

1. **Determinism.** Same seed → identical output, byte for byte, on every platform. Test this.
2. **Meaning first, then form.** Text is generated from a structured meaning representation and rendered through the grammar. The engine must always be able to report the true meaning of anything it generated.
3. **Real rules, not random strings.** Phonology, morphology and grammar follow consistent, discoverable rules. If a patient human with a notebook couldn't work a rule out from enough examples, it's wrong.
4. **Two outputs.** Anything player-facing should be producible as (a) human text and (b) structured JSON for agents/debugging. The JSON may include ground truth (meanings, glosses) behind a debug/spoiler flag; the human text never does.
5. **No runtime AI prose.** All prose is hand-authored templates or procedural language output. Placeholder English templates are fine for now; mark them `// PLACEHOLDER-PROSE`.

## Conventions

- `cargo fmt` and `cargo clippy -- -D warnings` must pass.
- `cargo test` must pass. Prefer many small unit tests plus a few seed-snapshot tests.
- Public items get short doc comments explaining *why*, not just *what*.
- Commit in small, coherent steps with clear messages.
- When finishing a session, update the checklist in the milestone doc and add a short entry to `docs/LOG.md` (create it if missing): date, what was done, open questions.

## Working with Jb

Jb is the designer and decides game direction. Bring design questions back rather than resolving them silently. Keep summaries short: what changed, what to try, what needs a decision.
