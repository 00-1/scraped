# M14 — Fairness, clients and release

**Goal:** prove every world is solvable with a notebook, tune the difficulty, polish the ways people play (terminal, browser, agent), and lock down the release gate so nothing un-authored ships.

**Depends on:** M11, M12, M13.

**Done when:** the release check passes on a complete content pack, the solvability check passes on a large batch of seeds, and the game is playable in a browser from a public link, in a terminal from a downloadable binary, and by an agent through a documented protocol.

## Scope

### In

1. **Solvability checker:** for a seed, trace what a player needs to understand to reach each key goal (the scraping tool, the first potent release, a great inscription, the deepest text, leaving), and verify the world contains enough evidence for each: every needed root appears in at least N readable contexts before it's needed; every needed grammatical construction is attested; every needed route has an ordinary or reachable written solution. Seeds that fail are rejected or repaired at generation time.
2. **Decipherment difficulty metrics:** per seed, measure evidence density, ambiguity and anchors (numerals, formulae, repeated names), and use the M02 difficulty dials to keep seeds within target bands. Offer difficulty presets (gentle, standard, archaeologist).
3. **Balance pass:** survival pace, threat frequency, regional simulation speed, tool placement distances, informed by bot runs and Jb's playtests.
4. **Browser player:** a polished, accessible play page (screen-reader friendly, keyboard-first, adjustable text size, high contrast, dark/light), with save slots in browser storage plus save export/import, and transcript download for notebooks. Hosted on GitHub Pages.
5. **Terminal player:** release binaries for Windows, macOS and Linux from CI; screen-reader friendly output (no box drawing in play), configurable wrap width.
6. **Agent harness:** documented JSON-lines protocol (versioned), plus an optional MCP server wrapper so an agent like Claude can play alongside a human, with a sample `CLAUDE.md`-style house-rules file for co-op play (consult the human on decisions, keep a journal file).
7. **Seed sharing:** short shareable seed codes (including difficulty and pack version), so players can compare notebooks for the same world.
8. **Release gate:** CI job that must pass to tag a release: release check (no empty required slots, no example variants, no lint errors), determinism across platforms, solvability on a seed batch, browser and binary builds.
9. **Player documentation:** a short in-game help and a manual written by Jb (slots or a static page), including a "how to keep a notebook" guide that doesn't spoil anything.

### Out

New mechanics. Anything new goes in a post-release roadmap.

## Tests

- Solvability checker unit fixtures (solvable and unsolvable crafted worlds).
- Batch run: N seeds per difficulty pass solvability.
- Cross-platform determinism: the same replay on Linux, macOS, Windows and WebAssembly gives identical transcripts.
- Accessibility checks on the browser player (automated audit plus keyboard-only test).
- Protocol compatibility tests for the agent harness.

## Checklist

- [x] Solvability checker and seed repair/rejection
- [x] Difficulty metrics and presets
- [x] Balance pass
- [x] Browser player: accessible, saves, transcripts, hosted
- [x] Terminal release binaries
- [x] Agent protocol docs and MCP wrapper with co-op house rules
- [x] Seed sharing codes
- [x] Release gate in CI
- [x] Player help and manual (content): slots and the `manual` command are in; the text is Jb's to write
- [x] Tests listed above
- [x] LOG.md entry
