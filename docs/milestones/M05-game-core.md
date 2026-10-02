# M05 — Game core and text interface

**Goal:** a playable loop inside a single generated site. The player can look, move between rooms, examine things, pick things up and read text, through a terminal client and a JSON protocol for agents. This is the skeleton every later system plugs into.

**Depends on:** M03, M04.

**Done when:** Jb can start a game on a seed, explore the interior of one site (the starting site), read its inscriptions, save, quit, and resume exactly where he was. An agent can do the same over the JSON protocol.

## Design decisions

| Decision | Proposed default |
|---|---|
| Input style | Short imperative commands with a forgiving parser (`look`, `examine altar`, `read the north wall`, `take lamp`), not free-form natural language |
| Time | Every action costs in-game time. Time is tracked in minutes and shown only through description (light, hunger later), not as a clock |
| Starting point | The player wakes in a generated starting site chosen per seed (M12 adds the authored frame) |
| Save model | Seed + content pack version + command log. Loading replays the log |
| Transcript | Every session can write a plain-text transcript: the raw material for the player's notebook |

## Scope

### In

1. **`game` crate:** game state (player, location, inventory, time, known names), an action pipeline (parse → resolve → simulate → describe), and an event log.
2. **Parser:**
   - Verbs with synonyms (look/l, examine/x/inspect, go/walk, take/get, drop, read, inventory/i, wait, help, save, quit).
   - Noun phrase resolution against what the player can perceive: adjectives, ordinals ("the second door"), "it", disambiguation questions ("Which door: the iron door or the cedar door?").
   - Unknown words and failures handled by content slots, never hard-coded English.
   - Verb list is data, so later milestones add verbs (`scrape`, `write`, `name`) cleanly.
3. **Interiors:** rooms and connections from M04 generators; objects and features in rooms; doors and their states; levels and stairs. Movement inside a site is by exit (`north`, `up`, `through the arch`). Open-world movement comes in M06.
4. **Reading:** reading an unscraped text shows it in the current display mode: glyph descriptions, or the player's own transliteration once they define one (see below). Long texts page.
5. **Player transliteration:** a `transliterate` command (`define glyph 3 as "ka"`) that lets the player assign their own labels to glyphs, after which text can be shown in their labels. This is the bridge between drawing glyphs on paper and working fast, and it never reveals truth: wrong labels stay wrong.
6. **Description rendering:** every description is a slot fed by state: room look, object examine, exits, inventory, reading, time-of-day cues. Register slots with good descriptions and variables (see M03 rules).
7. **Save, load, replay:** deterministic replay from log, with a check that the pack version matches (warn if not).
8. **`play` crate:**
   - **Terminal client:** line-based prompt, word-wrapping, scrollback, `transcript on/off`.
   - **JSON-lines agent protocol** over stdin/stdout: each command in, a response out with `text` (exactly what a human sees) and a `state` summary (location id, visible things, inventory). Ground truth only with `--spoil`.
9. **Browser:** a minimal play page in the bench using the same game core, so the browser build stays honest.

### Out

Outdoor movement and landmarks (M06), physical properties and survival (M07), scraping and effects (M08).

## Content slots introduced

Room look (by room purpose, light, state), object examine, exits, parser errors and disambiguation, inventory, reading frames ("Carved into the lintel, in a cramped hand:"), save/load messages, help text.

## Tests

- Parser tests for each verb, synonym, ordinal, pronoun and ambiguity.
- Replay determinism: random command scripts replayed give identical transcripts.
- Save/load round trip.
- Protocol tests: every response parses as JSON and its `text` equals the terminal output.
- No player-facing string literals in `game` or `play` (a lint test that scans for them, allow-listing debug output).

## Checklist

- [ ] Game state, action pipeline, event log
- [ ] Parser with disambiguation; verbs as data
- [ ] Interiors and in-site movement
- [ ] Reading and paging
- [ ] Player transliteration
- [ ] Description slots registered with examples
- [ ] Save, load, replay
- [ ] Terminal client and transcripts
- [ ] JSON-lines agent protocol
- [ ] Minimal browser play page
- [ ] Tests listed above
- [ ] LOG.md entry
