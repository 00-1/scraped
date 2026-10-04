# CLAUDE.md — Scraped Again

Briefing for coding agents working in this repo. Read this first, then `docs/ROADMAP.md` (order of work), `docs/DESIGN.md` (intent) and the current milestone spec (scope).

## What this is

A text-only, procedurally generated exploration/survival game about deciphering a lost language in a world where writing has power. Output is text only; input is simple typed prompts. The full design lives in `docs/DESIGN.md`. **Do not invent design.** If a decision isn't covered there or in the milestone doc, pick the simplest option that keeps doors open, leave a `// DESIGN-Q:` comment, and list it in your summary for Jb to decide.

## Current milestone

**C01 — Shared play and versions** (`docs/milestones/C01-shared-play-and-versions.md`), between D08 and **D09 — The magic, deepened** (`docs/milestones/D09-magic.md`) in the depth roadmap in `docs/DEPTH.md`. D01–D08, S01 and S02 are done (instruments, quiet text, places with character, course corrections, great interiors, things and mechanisms, a living world, a language for long texts, sign impressions as whole shapes, history and what the writing says). The first roadmap (M01–M14, `docs/ROADMAP.md`) is complete. See `docs/HANDOFF.md` for where things stand.

C01 is a hardened player build, snapshot saves that survive minor updates, a versioning policy enforced in CI, and shared worlds with table talk. If Jb starts a separate session for C01, that session works only on C01.

Before starting, read `docs/DEPTH.md` and the two principles at the top of `docs/DESIGN.md`: "Writing is background, at first" and "Say little; let the player dig". They apply to every depth milestone and override anything older that conflicts with them. A depth milestone is done when its numeric targets are met and its sample transcripts read well, not when its checklist is ticked.

- Work through milestones in order, each within its own spec. Jb has asked for unattended work: when a milestone is complete, carry straight on to the next one rather than stopping to report. Only stop to ask when a decision genuinely blocks progress.
- Keep the browser bench (`tools/bench`, later the `web` crate) up to date with each milestone; once there is a game, it goes in the bench too.
- Before building, check the spec's design gates. If Jb hasn't settled one, build on the proposed default, mark it `// DESIGN-Q:`, and list it in your report.
- When a milestone is complete, tick its checklist, write the LOG entry, and update this section to point at the next milestone.

## Tech stack

- **Rust**, stable toolchain, edition 2021.
- Cargo workspace:
  - `crates/lang` — the language engine (library). Pure logic, no I/O.
  - `crates/cli` — a small terminal binary for inspecting generated output.
  - `crates/content` — content slots, template language, pack, lint (library).
  - `crates/world` — world generation: terrain, water, history, structures, texts, decay (library).
  - `crates/sim` — the physical world: perception, local properties, rule table, mechanisms, items, body, creatures (library).
  - `crates/game` — game state, parser, commands, travel; `crates/play` — terminal client and JSON protocol.
  - `crates/web` — WebAssembly entry point (JSON in, JSON out) for the browser tools.
  - `tools/` — the bench and authoring tool page templates; `tools/build.sh` builds them.
  - Later crates (`sim`, `game`, `play`) are listed in `docs/ROADMAP.md`. Create each only in the milestone that introduces it.
- **Content slots:** declare new player-visible text with `SlotDef` (see `crates/lang/src/slots.rs`), add it to the registry, give it a sampler, and add one `example = true` variant to the family's file in `content/`. Do not edit Jb's own (non-example) variants.
- Keep the core crates **WebAssembly-compatible**: no threads, filesystem or OS-specific calls inside library crates. I/O belongs only in `cli` (and later `web`).
- Floating point in generation: only `+ - * /` and `sqrt` (never `sin`, `exp`, `powf`…), whose results are identical on every platform.
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
- Check in proportion. On every commit: `fmt`, `clippy`, `cargo test --release` (fast). Before a milestone's last commit, or when a change touches what they cover: the slow bot tests (`cargo test --release -p scraped-game --test depth -- --ignored`), `tools/build.sh` and the browser smokes. Iterate on bots with 3 seeds; run 10 once at the end.
- Don't chase a numeric target past two or three focused attempts: report it as not met, with why, and move on.
- Write each fact once: the LOG says what was done and what's open; `docs/DEPTH.md` holds the numbers; sample NOTES say only what the transcripts show.
- When finishing a session, update the checklist in the milestone spec and add a short entry to `docs/LOG.md`: date, what was done, open questions, new content slots added, and approximate usage cost if known.

## Working with Jb

Jb is the designer and decides game direction. Bring design questions back rather than resolving them silently. Keep summaries short: what changed, what to try, what needs a decision.
