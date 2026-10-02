# CLAUDE.md — Scraped Again

Briefing for coding agents working in this repo. Read this first, then `docs/ROADMAP.md` (order of work), `docs/DESIGN.md` (intent) and the current milestone spec (scope).

## What this is

A text-only, procedurally generated exploration/survival game about deciphering a lost language in a world where writing has power. Output is text only; input is simple typed prompts. The full design lives in `docs/DESIGN.md`. **Do not invent design.** If a decision isn't covered there or in the milestone doc, pick the simplest option that keeps doors open, leave a `// DESIGN-Q:` comment, and list it in your summary for Jb to decide.

## Current milestone

**M03 — Content system and authoring tool v1** (`docs/milestones/M03-content-and-authoring.md`). M01 and M02 are done.

- Work on one milestone per session, only within its spec. Do not start the next milestone, even if there is time left; stop and report instead.
- Before building, check the spec's design gates. If Jb hasn't settled one, build on the proposed default, mark it `// DESIGN-Q:`, and list it in your report.
- When a milestone is complete, tick its checklist, write the LOG entry, and update this section to point at the next milestone.

## Tech stack

- **Rust**, stable toolchain, edition 2021.
- Cargo workspace:
  - `crates/lang` — the language engine (library). Pure logic, no I/O.
  - `crates/cli` — a small terminal binary for inspecting generated output.
  - Later crates (`content`, `world`, `sim`, `game`, `play`, `web`) and the authoring tool are listed in `docs/ROADMAP.md`. Create each only in the milestone that introduces it.
- Keep the core crates **WebAssembly-compatible**: no threads, filesystem or OS-specific calls inside library crates. I/O belongs only in `cli` (and later `web`).
- Randomness: a seeded RNG (`rand_chacha::ChaCha8Rng` or similar) passed explicitly. **Never** use thread-local or OS randomness in library code.
- Serialisation: `serde` + `serde_json` for any structured output.
- Keep dependencies few and well-known.

## Non-negotiable principles

1. **Determinism.** Same seed → identical output, byte for byte, on every platform. Test this.
2. **Meaning first, then form.** Text is generated from a structured meaning representation and rendered through the grammar. The engine must always be able to report the true meaning of anything it generated.
3. **Real rules, not random strings.** Phonology, morphology and grammar follow consistent, discoverable rules. If a patient human with a notebook couldn't work a rule out from enough examples, it's wrong.
4. **Two outputs.** Anything player-facing should be producible as (a) human text and (b) structured JSON for agents/debugging. The JSON may include ground truth (meanings, glosses) behind a debug/spoiler flag; the human text never does.
5. **No runtime AI prose, and no prose in code.** All player-visible text is either Jb's hand-authored templates or generated language. Before M03 lands, placeholder English is allowed if marked `// PLACEHOLDER-PROSE`. From M03 onwards, every new piece of player-visible text must be a declared content slot (see `docs/milestones/M03-content-and-authoring.md`): a clear description written for Jb, well-named variables, and one short example variant marked `example = true`. Never write the real prose yourself; that is Jb's job. Debug and spoiler output are exempt.

## Conventions

- `cargo fmt` and `cargo clippy -- -D warnings` must pass.
- `cargo test` must pass. Prefer many small unit tests plus a few seed-snapshot tests.
- Public items get short doc comments explaining *why*, not just *what*.
- Commit in small, coherent steps with clear messages.
- When finishing a session, update the checklist in the milestone spec and add a short entry to `docs/LOG.md`: date, what was done, open questions, new content slots added, and approximate usage cost if known.

## Working with Jb

Jb is the designer and decides game direction. Bring design questions back rather than resolving them silently. Keep summaries short: what changed, what to try, what needs a decision.
