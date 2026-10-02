# M03 — Content system and authoring tool v1

**Goal:** make it impossible for the game to contain prose Jb didn't write, and give Jb a tool to write it. The engine declares every piece of text it needs as a **slot**; Jb fills slots with **templates** in a **content pack**; a browser **authoring tool** shows what's missing and previews his writing against real generated data.

**Depends on:** M01 (M02 slots are registered here too if M02 is done).

**Done when:** Jb opens the authoring tool in a browser, sees the list of slots with their status, writes glyph-description templates, previews them against several seeds, and saves them into the repo's `content/` folder.

## Concepts

- **Slot:** a named place where the engine needs text, such as `glyph.describe`, `inscription.surface.stone`, or later `place.look.forest`. Declared in engine code with:
  - an id;
  - a description of when it fires, written for Jb;
  - the **variables** available (name, type, description), such as `{surface.material}`, `{light.level}` or `{landmark.bearing}`;
  - requirements: minimum number of variants, whether it's required for release, and maximum length.
- **Template:** Jb's text for a slot. Several variants per slot, optionally with **conditions** (`when: light.level == "dark"`) and **weights**.
- **Content pack:** all templates, in plain, diff-friendly files under `content/` in the repo, versioned with a hash. A run records the pack version it used.
- **Placeholder:** what the engine shows when a slot is empty in a dev build. It is loud and obvious, like `⟦slot: place.look.forest⟧`, never fake prose. Release builds refuse to start (and CI fails) if any required slot is empty.

## Template language

Small and readable; Jb should never need to read code to use it.

- Variables: `{surface.material}`.
- Choices inline: `{a|an|the}`-style alternatives, chosen deterministically from seed and context.
- Conditions on whole variants, and on inline sections: `[if light.level == "dim"]…[end]`.
- Calls to other slots: `{>weather.brief}` so text can be composed from smaller authored pieces.
- English helpers: articles (`{a surface.material}`), plurals by count, capitalising sentence starts, lists ("a, b and c"), number words, bearings ("north-east"), rough distances and durations.
- **Language hooks:** `{lang.word concept}` and `{lang.text inscription}` insert generated language in the current display mode; `{lang.glyphs …}` inserts glyph descriptions. This is how authored prose and procedural language mix.
- **Damage markup** for palimpsest rendering, reserved now and implemented in M08: text passed through a damage filter shows only part of itself.
- Selection is deterministic: the same seed, context and pack always choose the same variant. It avoids repeating the last variant used for the same slot where possible.

## Scope

### In

1. **`content` crate**
   - Slot registry: a macro or builder that engine code uses to declare slots. The full registry can be exported as JSON for the tool.
   - Pack format (TOML with multi-line strings, or a similar documented, human-friendly format; one file per slot family, such as `content/glyphs.toml`). Include a short `content/README.md` for Jb explaining the format with examples.
   - Parser, validator and renderer for the template language. WebAssembly-compatible.
   - **Lint:** unknown variables, wrong types in conditions, unreachable conditions, too few variants, over-long text, templates for slots that no longer exist, near-duplicate variants.
   - **Coverage:** which slots are empty, under-filled or only partially covered by conditions (for example, no variant for `light.level == "dark"`).
2. **Migrate existing text.** Glyph descriptions (if M02 is done) and anything else player-facing become slots. `english.rs` stays as a spoiler-only placeholder translator, which is allowed.
3. **CLI:** `scraped content lint`, `scraped content coverage`, `scraped content preview <slot> --seed N --count K`, `scraped content registry --json`.
4. **`web` crate:** WebAssembly bindings exposing the language engine, slot registry, lint and renderer to JavaScript. The existing bench moves onto this.
5. **Authoring tool v1** (`tools/author`, a static web app; deploy to GitHub Pages from CI):
   - **Slot browser:** every slot with status (empty / under-filled / OK / lint errors), grouped by family, searchable.
   - **Slot page:** the slot's description, its variables with descriptions and example values, requirements, and the current variants.
   - **Editor:** add, edit, reorder and delete variants and conditions, with syntax highlighting and inline lint as you type.
   - **Live preview:** render the slot against a configurable number of sample contexts drawn from real seeds. Show each sample's variable values alongside the output, and allow pinning a seed.
   - **Saving:** open the repo's `content/` folder directly with the browser's File System Access API (Chrome/Edge) so edits save straight to disk for Jb to commit. Fallback for other browsers: import and export the pack as a zip.
   - **Spoiler toggle:** previews can show ground truth (meanings, glosses) because the author needs it.
   - Works offline after first load; no server.
6. **Release gate:** a `--release-check` mode (CLI and CI job, allowed to fail until M14) that fails if any required slot is empty or any lint error exists.

### Out

Reachability-based coverage, playtesting inside the tool, world and language inspectors (M13). Storylets (M12).

## Rules for every later milestone

Add these to `CLAUDE.md`:

- Any new player-visible text must be a declared slot with a clear description and well-named variables. Writing prose in Rust is not allowed.
- Each new slot comes with **one** short example variant marked `example = true`, so Jb can see the intended shape. Examples count as placeholders for the release gate.
- Slot descriptions are written for Jb, not for programmers: when the text appears, what the player is doing, what it must convey and what it must not reveal.

## Tests

- Template parser and renderer unit tests, including every helper.
- Determinism: same inputs give the same variant choice.
- Lint catches each listed problem (one fixture per lint).
- Registry export is stable and snapshot-tested.
- The web bindings build for `wasm32-unknown-unknown` in CI.
- A headless smoke test of the authoring tool (load, list slots, render a preview).

## Checklist

- [x] Slot registry and JSON export
- [x] Pack format, loader and `content/README.md`
- [x] Template language: parser, renderer, helpers, language hooks
- [x] Lint and coverage
- [x] Existing player-facing text migrated to slots
- [x] CLI commands
- [x] `web` crate bindings; bench moved onto them
- [x] Authoring tool: browser, slot page, editor, preview, save via File System Access, zip fallback
- [x] GitHub Pages deployment from CI
- [x] Release-check mode
- [x] CLAUDE.md updated with the content rules
- [x] LOG.md entry
