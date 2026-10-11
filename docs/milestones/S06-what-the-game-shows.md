# S06 — What the game shows

**When:** now. **Size:** small to medium. No new systems, except the cue follow-ups in section 4. Release when done.

**Goal:** Jb's first playtest with an AI partner (2026-10-11, v0.5.0, about 75 commands in the start town's temple) showed that the game and its framing point at the writing from the first turn, and that an agent sees more than a person. Fix what the game shows and says, following the new sections of `docs/DESIGN.md` ("How the script is perceived", revised; "What the game says about itself"; "Measures as a body knows them"; "Following up strangeness"). D13 builds the rest of "Noticing and knowing".

## 1. Nothing beyond the text

The playtest's MCP replies carried a `state` object that the agent leaned on throughout. It held:

- `things` with objects the text never mentioned (the stele and basin in the forecourt, the brazier and a second torch in the hall);
- every building in town with its condition (`exits`);
- `landmarks` with bearings and distances no text had given;
- internal ids (`"place":"structure 15 room 0"`);
- the exact clock (`minutes`);
- `load` figures;
- body fields;
- a `wrote` field that revealed `write` exists.

- **The player program's JSON and MCP replies carry the text and nothing more** (plus the protocol version and, where a client needs it, a flag that the run has ended). Remove `state` from the player build, or reduce it to fields whose every value is said in that same response's text. Developer and spoiler builds keep the full state.
- **Test** against the built player binary and wasm: no reply contains a field beyond the allowed set; no id, number of minutes or object name appears that isn't in the text.
- Check the Android app and the browser player for the same: nothing on screen that the text didn't say (status chips, exits lists, a clock).

## 2. No statement of what the game is about

- Remove "a text game about deciphering a lost language" (and any other description) from the MCP server's instructions, the first JSON reply, `--help`, the app store text, the Pages site and the README's player-facing parts. The MCP instructions keep only mechanics: how to start, how to send a command, saving, table talk, and the fair-play note.
- Any text the game or its clients show about the game becomes a slot (`app.about`), empty by default, for Jb.
- **Agents get what humans get.** Retire `docs/coop/CLAUDE.md`'s house rules and advice (deciphering, sign tables, "above all what to scrape"). Keep `docs/coop/FAIR-PLAY.md` as Jb's text to edit, shown to agents. `docs/coop/PLAYING.md` keeps only the technical steps, with neutral example commands (`look`, `go north`, `help`).
- The intro (`say.intro`) and every example variant shown in the first hour say nothing about writing or reading. Today's example is "Writing is everywhere, and you cannot read it."

## 3. Help and manual

- **`help` lists the basics** (looking, the senses, moving, going back, taking and dropping, eating, drinking, sleeping, waiting) **plus every verb the player has used successfully**. Every verb stays understood from the start. (D13 adds suggestions; this step only stops the list from naming verbs the player hasn't found.)
- **The manual is how to play:** one page (`manual.page`, Jb's text) on how to talk to the game, going places, going back, saving. Remove the sections (playing, reading, notebook, writing, survival, endings). The playtest agent read the placeholders as "the game isn't finished; someone called Jb is building it".
- App chips and hints (`notebook_hint` "Glyphs, words, guesses…") say nothing about writing.

## 4. Marks, in three steps

Per "How the script is perceived" (revised):

- **Marks, not writing.** Every first-hour slot and example that says "writing", "signs" or "glyphs" says "marks" instead, until the character has studied marks on several surfaces (proposed: three, with recurring shapes; DESIGN-Q). `read.whole` and `read.glyph` get a variable saying which word applies.
- **Three steps:** the marks as a whole (today's `read`); each shape in turn (today's `read closely`); one mark exactly (`trace`). `examine sign N` and `trace sign N` both lead to tracing. Remove the separate "fuller impression" (`glyph.closer`), folding its one distinguishing detail into the list where it tells shapes apart.
- **Tracing needs a tool:** charcoal (from the player's own fires, or found) with cloth, bark or hide; or wax or soft clay pressed into cut marks, which needs no light. No tool: "You've nothing to take the marks with" (slot). The fairness checker covers it: a tracing tool can be reached.
- **Tracings aren't kept.** Shown once, as now.
- **`read` comes late.** `read`, `study`, `look at the marks` all work from the start, but responses speak of looking and studying, not reading, until the player has recognised a word or heard a mark's sound. "You could read it" goes.
- **Tracing matches the impression.** The playtest's sign 1 was "a slight, curved sign like an egg" and traced as "A ring, right.". Check that the traced form and the impression describe the same shape (an egg-like ring, say); tune whichever is off.

## 5. No modern units

- No hours, minutes, metres or kilometres in any player-facing text: travel, stairs, waiting, sleeping, tracing, cleaning, the app. Time by sun, light, sleeps and how long a torch has burned; distance by steps, a stone's throw, a morning's walk; through slots with coarse variables (`when`, `how_long`, `how_far`).
- Remove the day counter from the app and any client.
- Test: no rendered response in the bots' runs contains a modern unit or a clock time.

## 6. Cues can be followed up

The playtest's "Something about the door is not as it should be" led nowhere:

- `examine door` → "a door north";
- `touch door` → "The air is cool; the ground is dry.";
- `look closer at door` → the room again;
- `open door` → "already open".

- **Everything a cue names is examinable**, including doors, exits and features named only inside other descriptions (the seal "pressed at the foot").
- **Following a cue up always gives more:** examining, touching, looking closer at, listening at and smelling the thing a cue names each give a response about that thing, through a slot family per kind of strangeness (`cue.examine`, `cue.touch`, `cue.closer`…). Even when there's nothing physical to find, it says so in the character's terms: something wrong that can't be named (Jb's words).
- `touch X` touches X, never the room's air.
- **Test:** for every cue in the explorer's and hand player's runs on seeds 1, 42 and 9001, each follow-up command gets a response about the named thing.

## 7. Bugs from the playtest

- `read stele closely` → "You see no stele closely here." Accept trailing adverbs: `read X closely`, `look closely at X`, `look closely X`.
- `look closely door`, `look closer at door` → a plain room look. These should look closer at the door.
- `examine door` → "a door north" (a raw exit name). Give doors and exits a proper description.
- `examine seal` → "You see no seal here.", though the wall's description names it.
- "There is no writing on a stone inscription." Don't call a thing an inscription if it bears no marks.
- The old map: "a town to the northwest, a town to the northwest… a town to the north… a town to the north". Give distances or something to tell them apart ("two towns to the northwest, one far").
- Doubled "like": "like a tall, mixed sign like a tree, with a loop beside". The related form should read as one phrase ("like the tree sign, with a loop beside").
- Add every command above to the hand player's test cases, and the playtest transcript (`docs/playtests/2026-10-11/`, text only, without any personal details) as a fixture.

## 8. Fairness covers what play leaves waiting

Jb is happy with a slow build, as long as nothing is left waiting forever. The fairness checker confirms, from the start of every world:

- a flame source is reachable (a firesteel, embers, a lit hearth);
- a tracing tool is reachable;
- every cue's cause can be reached.

## Done when

- Sections 1–8 tested as described, on the player build, on seeds 1, 42 and 9001.
- The first hour of the explorer's and hand player's transcripts says nothing about writing, reading, signs or glyphs, and contains no modern unit.
- Release; HANDOFF; LOG; DECISIONS for the DESIGN-Qs. Then D13.

## Checklist

- [ ] JSON and MCP carry only the text; clients show nothing more
- [ ] No "about" anywhere; agent docs reduced to mechanics and fair play
- [ ] `help` lists basics plus verbs used; manual is how to play
- [ ] Marks not writing; three steps; tracing needs a tool; `read` late
- [ ] No modern units; no day counter
- [ ] Cues followed up; everything a cue names is examinable
- [ ] Playtest bugs; transcript fixture
- [ ] Fairness: flame, tracing tool, cue causes
- [ ] Release; HANDOFF; LOG; then D13
