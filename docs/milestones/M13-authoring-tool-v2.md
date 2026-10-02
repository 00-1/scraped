# M13 — Authoring tool v2

**Goal:** make finishing the content pack fast and confident. v1 (M03) shows which slots are empty; v2 shows which slots and conditions players will actually hit, how often, and in what situations, and lets Jb play the game inside the tool and fix text as he goes.

**Depends on:** M12 (can start after M09).

**Done when:** Jb can see a ranked list of the content gaps players will meet most, play from any point in a seed inside the tool, click on any line of output to jump to the template that produced it, edit it, and see the change immediately.

## Scope

### In

1. **Reachability coverage:** run many simulated playthroughs (bots from M06–M09 tests) across many seeds and record every slot render with its variable values. Report:
   - how often each slot and each condition branch is hit;
   - variable combinations that are common but fall back to generic variants;
   - slots that are never reached (candidates for deletion).
   Ranked by "player-hours affected", so Jb writes the most valuable text first.
2. **Repetition analysis:** how often a player would see the same variant in a typical run; flag slots that need more variants.
3. **Playtest in the tool:** the full game running in the browser beside the editor, from a fresh start or from any saved state or replay point. Every line of output links back to the slot, the variant and the variable values that produced it ("why did I see this?").
4. **Hot reload:** edits apply to the running playtest without restarting.
5. **Inspectors (spoiler panels):**
   - **Language:** grammar sheet, eras and sound changes, glyph SVGs, lexicon, with search.
   - **World:** map, history timeline, sites, surface stacks, active claims, regional variables over time.
   - **Run:** understanding tracking, run record, what the player has and hasn't encountered.
6. **Voice tools:** a glossary of Jb's own recurring words and phrases; warnings when a variant repeats a distinctive phrase used elsewhere; length and rhythm stats per slot family.
7. **Review mode:** step through every slot family in a sensible order (opening first, then early-game, then late-game) with progress tracking, for systematic writing sessions.
8. **Pack versioning:** show the diff of the content pack since the last commit, and warn when a change will invalidate existing saves.

### Out

Any AI-generated text suggestions. The tool never writes prose for Jb.

## Tests

- Coverage runs are deterministic for a given set of seeds and bot scripts.
- Output-to-template linking is correct for every slot type (fixture runs).
- Hot reload never changes game state, only text.
- Tool builds and passes a headless smoke test in CI.

## Checklist

- [x] Coverage runner over bots and seeds
- [x] Ranked gaps by player-hours affected
- [x] Repetition analysis
- [x] Playtest panel with output-to-template links
- [x] Hot reload
- [x] Language, world and run inspectors
- [x] Voice tools
- [x] Review mode
- [x] Pack diff and save-compatibility warnings
- [x] Tests listed above
- [x] LOG.md entry
