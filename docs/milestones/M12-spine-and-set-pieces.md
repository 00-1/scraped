# M12 — Authored spine and set pieces

**Goal:** give every run a hand-written backbone and let Jb's set pieces appear in generated worlds. The engine provides a storylet system; Jb provides the opening, the frame and the set pieces through the authoring tool.

**Depends on:** M09 (can run in parallel with M10–M11). **Design gate:** the frame story (Jb writes it; the engine only needs hooks).

**Done when:** Jb writes one complete set piece in the authoring tool (with conditions, placement rules and its text), and it appears coherently in several different generated worlds, woven into their local language and history.

## Concepts

- **Storylet:** a self-contained, hand-written event or location with:
  - **conditions** for when and where it can occur (biome, structure type, era, regional state, what the player knows or carries, time, prior storylets);
  - **placement rules** (spawn once per world, near water, inside a temple of the old era, etc.);
  - **content:** text through slots and templates, plus generated-language hooks (`{lang.text …}` for an inscription that belongs to this world);
  - **effects:** add items, mark knowledge, change properties, unlock routes, start a follow-up.
- **Spine:** storylets marked as structural: the opening, the frame story's revelations, and beats tied to tool discoveries and the endings.

## Scope

### In

1. **Storylet engine:** data format (in the content pack, edited by the authoring tool), condition evaluator over game state, placement at world generation time and triggering at play time, effects applied through the normal game systems.
2. **Generated-language bindings:** a storylet can request "an inscription in the potent register about water, in the middle era" and the engine supplies one consistent with the world, so hand-written scenes contain real, decipherable language.
3. **Spine hooks:** the opening (replaces M05's bare wake-up), frame story beats, and hooks at: first scraped-text sighting, finding each tool, first potent release, first player write, reaching a great inscription, finding the deepest text, each ending.
4. **Authoring tool support:** a storylet editor (conditions with dropdowns of known state variables, placement rules, text, effects), and a preview that generates a world, places the storylet and shows where and how it appears.
5. **Lint:** storylets that can never be placed or triggered, missing text, conflicting effects, spine beats left unwritten.

### Out

Further authoring tool features (M13).

## Content slots introduced

Storylets are content; the engine adds only generic slots (for example, "a storylet is available nearby" cues, if Jb wants them).

## Tests

- Condition evaluator unit tests.
- Placement is deterministic and respects rules across seeds.
- A spine-only test run reaches every spine hook on a scripted path.
- Lint fixtures for each storylet problem.

## Checklist

- [x] Storylet data format in the content pack
- [x] Condition evaluator and triggering
- [x] Placement at world generation
- [x] Effects through game systems
- [x] Generated-language requests from storylets
- [x] Spine hooks wired into the game
- [x] Authoring tool storylet editor and placement preview
- [x] Storylet lint
- [x] Tests listed above
- [x] LOG.md entry
