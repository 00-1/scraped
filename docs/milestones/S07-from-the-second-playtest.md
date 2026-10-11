# S07 — From the second playtest

**When:** after D13. **Size:** one milestone. Release when done.

**Goal:** build what Jb decided while playing v0.6.0 (`docs/DESIGN.md`: "Noticing once, itemising at will", "Doing several things at once", "Pieces of writing on one thing"), and fix what the playtest showed. The playtest is in `docs/playtests/2026-10-11-v0.6.0/`: about 370 commands, with transcript, save and field book. Use its transcript as a fixture, as S06 did.

## 1. Looking as a progression

In the playtest, rooms needed 5–10 `look`s to find what was there. Key things showed up only on later looks (the temple's torch, the forecourt's map). Repeated looks then returned "Ways out: …" again and again. Outdoors, looks shuffled and repeated "The intact temple stands alone of its kind here." three times running.

- The first `look` in a place is a first glance: the whole and its standout, with a variable saying it's the first look (Jb's words, e.g. "At first glance…").
- Each later look gives the next most noticeable facts, in a fixed order set by salience and keenness (D13), never repeating one already given in this visit.
- When nothing is left, one line says so (`look.done`). It can point to closer looking or the senses. It never loops back.
- The progression starts afresh when the place changes: light, weather, something moved or new, or a return after a long absence.
- Loose things in plain view still come as one group, early (S04).
- **Test:** in the bots' and the playtest's runs, no fact is repeated by `look` within a visit, and every loose thing in view is told by the second look.

## 2. Say it once; recognise on return

- **Passive noticing says each fact once** per visit: arrivals, looks, ambient lines ("Here and there: bare earth", "Ways out: …" after the first time), and status lines. They come again only if something changed, or the player asks.
- **Itemising repeats on request:**
  - `examine X` again gives its description again;
  - studying marks again shows them again;
  - a new `take stock` (also `what's here`) lists what the player knows is in plain view here, briefly: things, ways out, people or creatures.
- **Recognition.** Every slot that describes a place or thing gets `seen_before` and `absence` (short, long) variables. Jb writes first-sight and recognition variants. After a short absence: recognition and changes only. After a long one: the place may be described again, in recognition wording. New or changed things are always worded as first sight.
- **Known places are known from afar.** The start village was "the plain town" seen from outside and "an intact farming village" on arrival; the partner found it confusing for a long time. Once visited, a place is named the way the player met it.
- **The cue line said once.** "Something about the door is not as it should be." was printed in every room of the temple. A cue is told where its thing is, once per visit.
- **Test:** no fact repeated unasked within a visit (bots, fixture); returns use recognition variables.

## 3. Groups, chains and the sweep

- **`all` and plurals for every verb:** `examine all`, `touch the walls`, `study all` (every marked thing in view).
- **Chains** with `and`, `then` and commas, and `it` for the last thing named. Each step costs its own time. The reply is one message per step, each headed by its command. In JSON, a list of replies; in the app, separate bubbles.
- **A chain stops** at a sound, danger, failure or anything newly noticed, and says where it stopped.
- **The sweep:** `search` (also `search the room`, `go over the room`) runs the obvious inspection: the next looks, examining each thing not yet examined, the senses not yet tried here. It skips what's known, stops at the first new thing, and carries on if repeated. When nothing is left it says so. Keenness orders it.
- **Studying a long text in one go:** `study X fully` (or `more all`) gives every remaining page, one message per page. The playtest's walls ran to 22 and 37 pages of `more`.
- **Test:** chains, sweeps and `all` in the hand player's cases; a chain stops on a scripted sound.

## 4. Pieces of writing on one thing

The "—" in a mark list separates pieces of writing on one thing (usually layers). The playtest met it on the tablet and three walls, and nothing explained it.

- Remove the separator. Each piece is its own thing to study: "the marks", "the fainter marks", "the marks beneath".
- The first glance (`read.whole`) says how many pieces there are and how each looks different (`pieces`, with `faintness`, `hand`, `cut`).
- Nothing says they're different languages or eras.
- **Test:** no list contains a separator; each piece can be studied by name.

## 5. Obvious next steps (with D13's suggestions)

Jb's rule: if the next thing to do would be obvious to a person standing there, the game suggests it. Add these situations from the playtest to D13's table, and make sure each named verb is understood. Where nothing happens, answer in the world ("You set the token on the altar. Nothing happens."), not "You don't know how to …":

- an altar with things in hand → put or offer something on it; pray;
- a flute, drum or other instrument in hand → play it;
- a closed stair or door with a key or token in hand → try it in the lock (`unlock X with Y`);
- a river or stream with an empty vessel → fill it (and a chipped pitcher says why it won't hold water);
- a cold brazier or hearth with wood, once a flame exists → light it, and light a torch from it;
- something heavy or blocked → push, pull, lever (with a hook or bar);
- unknown verbs that are clearly actions (`break`, `put`, `play`, `pray`, `unlock`) → understood, with an in-world answer.

Which of these do something in the world beyond the answer is a DESIGN-Q for Jb. Build the understanding and the honest answers.

## 6. Bugs from the playtest

- **`go back` is broken in two ways:**
  - It reverses only the last leg, so repeating it walks back and forth between two points. It should retrace the path the player came by, step by step, towards where they started this journey.
  - After a blocked move ("The river bars your way north."), `go back` printed nothing at all.
- **Site summaries name things that can't be examined:** "Here: pots, beads and bronze." and "Here: bones, pack and staff." followed by "You see no pack here." Either make them examinable (a group: "examine the grave goods") or word them as scene, not objects.
- **Exit names with a state in brackets:** "stair down (closed)" became part of the name. "push stair" → "You see no stair down (closed) here."
- **Parser:**
  - "look in jar" → a plain room look;
  - "examine marks" → "You see no marks here." (should examine the marks on the thing in view, or ask which);
  - "enter workshops" → the town description (a district), not an error or a way in.
- **Like items don't stack:**
  - "Two clay provisions." on looking;
  - "Some provisions. Some provisions." on examining;
  - "some provisions, some provisions" in the inventory.
- **Body messages arrive a turn late:** "Your thirst eases." and "Your hunger fades." appear on the reply after drinking or eating.
- **`touch door` gave two replies** ("The door is cool under your hand." then the cue line). It should give one, folded together.
- **Travel wording:** "a long walk, a short while" reads as a contradiction. Tie how far and how long together, or give one.
- **Walks overshoot** places already in view. The partner passed the mound twice. A walk towards something in view stops when you reach it.
- **`help` listed "west"** as a verb. Directions belong under "go".
- **The MCP `save` returned about 196,000 characters**, too large for an agent's tool output. The MCP server should write the save to a file and return its path, or return a compact form. Check the snapshot size too.

## 7. Notes, no action

- **The decipherment works.** The partner found, among other things, a text written twice, words dropped from one version that fill a scraped tablet, and an old-to-new chain on one wall. It said the game never contradicted a real pattern, and that cleaning let it test its guesses.
- **Jb reasoned about the world from evidence:** the season (flowers, new leaves, lengthening days, frost out of season), the calendar notch, the cross on the map, the emblems. The field book grew a "world" tab for it. This is evidence for X01.
- **The partner asked for an early goal or hook** after about 370 moves. Jb has said a slow build is intended. Recorded here for Jb, nothing to build.

## Done when

- Sections 1–6 built and tested, with the second playtest's transcript as a fixture.
- Release; HANDOFF; LOG; DECISIONS for the DESIGN-Qs. Then back to waiting for Jb.

## Checklist

- [ ] Looking as a progression; never loops
- [ ] Say it once; take stock; recognition; places known from afar; cue said once
- [ ] Groups, chains, the sweep, studying fully
- [ ] Pieces of writing studied separately
- [ ] Obvious next steps added to D13's table; verbs understood with in-world answers
- [ ] Playtest bugs (go back, site summaries, bracketed names, parser, stacking, late body messages, touch, travel wording, overshoot, help, MCP save size)
- [ ] Fixture; release; HANDOFF; LOG
