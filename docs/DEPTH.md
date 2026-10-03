# Depth roadmap

The first roadmap (`docs/ROADMAP.md`, M01–M14) built every system the design asked for, each at the simplest level that met its spec. This roadmap makes them deep. Its aim, in Jb's words: **the environment should be genuinely interesting and rewarding to explore in its own right**, and the writing should start as background flavour that the player only gradually realises matters (`docs/DESIGN.md`, "Writing is background, at first"). And however rich the world gets, **the text stays short and intriguing; the player digs for detail** (`docs/DESIGN.md`, "Say little; let the player dig").

Each milestone has a spec in `docs/milestones/`. Work through them in order.

## Where the engine is now

Measured on the M14 build (seed 42 unless noted).

| Area | Now | Problem |
|---|---|---|
| Building types | D03: 50 kinds, by each town's role | Was: every town the same handful of temples, houses, tombs and storehouses |
| Interiors | 3–6 rooms joined by compass exits, no geometry | Nothing large or intricate to explore inside; maps can't be checked |
| Room purposes | about 7 | Interiors feel interchangeable |
| Natural features | D03: 36 kinds of feature and old mark, placed by cause | Was: nothing between towns worth walking to |
| Landmark identity | "the mountain", "the town" | Five identical names in one view; maps can't tell them apart |
| Biomes | 12 | Fine for now |
| Items | 20, all practical | Nothing to find for its own sake |
| Mechanisms | 6 | Single-step, local |
| Creatures | 4 archetypes | No ecology |
| History events | 7 kinds | Little for texts and ruins to be evidence of |
| Text kinds | 7; 59% tombs; about 90 sentence shapes per world | Reading is unrewarding even once deciphered |
| Lexicon | 113 concepts | Too small for rich texts or place names |
| Spells (potent texts) | 6 per world; 3 verbs (open, burn, break); 3 properties | The heart of the game is thin |
| Discovery | Scraper placed beside a "pivot" spell; a storylet announces the tool | Points straight at the mechanic, contrary to the design |
| Response length | D02: an attention budget per response (arrival 3 facts, `look` 4); the rest is found by digging | Was: each system appended a line |

## Metrics

Measured by `scraped-lang depth --seeds A-B [--hours H] [--json]` (and the web crate's `depth` call, shown in the bench's Depth tab). World measures read the generated world. Variety, brevity and depth on demand come from the curious explorer's first H hours (24 by default), rendered with the example variants, so they measure the engine, not the prose.

| Metric | Definition |
|---|---|
| `places.structure_kinds` | Distinct kinds of building in the world |
| `places.room_purposes` | Distinct room purposes across all interiors |
| `places.natural_kinds` | Distinct natural features: edges other than roads (river, stream, lake, coast, treeline…) and summit kinds |
| `places.landmarks` | Landmarks: settlements, lone buildings, summits |
| `places.landmarks_unique_share` | Share of landmarks whose rendered name no other landmark in the world shares |
| `places.per_km2` | Buildings and summits per km² of land |
| `things.kinds` | Distinct kinds of thing (features and items) |
| `things.per_structure` | Things per building |
| `things.mechanisms` | Mechanisms (wells, sluices, levers, braziers…) |
| `things.longest_chain` | Steps in the longest mechanism chain (a lever and what it moves is 2) |
| `life.species` | Distinct creature kinds |
| `life.with_signs` | Kinds that leave signs (tracks, nests) to find without seeing them |
| `history.event_kinds`, `history.events` | Kinds of history event, and how many |
| `history.people_with_traces` | Named people met in more than one text |
| `writing.texts`, `writing.genres` | Texts, and genres among them |
| `writing.largest_genre_share` | The commonest genre's share of all texts |
| `writing.sentence_shapes` | Distinct meanings once names and numbers are abstracted away |
| `writing.words_per_text` | Mean words per text |
| `writing.concepts` | Distinct concepts attested in the texts |
| `magic.live_spells`, `magic.claim_types` | Live (released) spells at the start, and distinct kinds of claim among them |
| `magic.places_with_writing_cause` | Buildings within reach of a live spell |
| `magic.strange_without_writing_share` | Share of strange places with no writing cause (natural oddities) |
| `variety.combinations` | Distinct (slot, variable combination) pairs the explorer met; per slot family in the JSON |
| `brevity.<kind>.words_*`, `brevity.<kind>.facts_*` | Median and 95th percentile of words, and of facts, per response, for `look`, `arrival` (into a new place), `travel` and `other`. A fact is a rendered slot that says one thing (no list variable), outside the parser's replies, whose text was in the response |
| `depth_on_demand.per_place` | At each arrival somewhere (D02 on): how many facts digging there could turn up (median): what `look closer` and `look around` would weigh, sounds and smells, the ground, and writing to read. Until D02 it counted things to examine in each room, plus one for each with writing |
| `depth_on_demand.per_fact_shown` | At each arrival, those facts for each fact shown on arriving (median) |
| `play.novel_per_hour` | New things perceived per hour: first renders of a (slot, kind) pair |
| `play.minutes_between_new_kinds` | Median minutes between new kinds of thing (first word of the kind, per slot family) |

### Baseline (D01, seeds 1–10, explorer 24 h)

| Area | Baseline |
|---|---|
| Places | 8.7 building kinds; 22.2 room purposes; 5.8 natural kinds; 29.3 landmarks, 85% uniquely named; 0.08 places per km² |
| Things | 37.2 kinds; 6.6 per building; 57 mechanisms; longest chain 2 |
| Life | 4 species; none leave signs |
| History | 6.2 event kinds, 215 events; 72 people met in more than one text |
| Writing | 262 texts in 8 genres; the largest genre is 52%; 103 sentence shapes; 5.3 words per text; 70 concepts |
| Magic | 9.5 live spells of 5.4 kinds; 65 buildings within a spell's reach; no natural oddities |
| Variety | 240 slot × variable combinations met in 24 hours |
| Brevity (words, median / p95) | look 88 / 117; arrival 30 / 109; travel 91 / 127 |
| Brevity (facts, median / p95) | look 29 / 45; arrival 14 / 41; travel 26 / 49 |
| Depth on demand | 3.4 details per room; senses 0 |
| Novelty | 3.6 new things per hour; a new kind of thing every 1.1 minutes (median) early on |

### After D02 (seeds 1–10, explorer 24 h)

A fact is now counted once even when a name is rendered inside it.

| Metric | D01 | D02 | Target |
|---|---|---|---|
| Words per arrival (median / p95) | 30 / 109 | 17 / 21 | at most 60 |
| Facts per arrival (median / p95) | 14 / 41 | 3 / 4.4 | 3 |
| Words per `look` (median / p95) | 88 / 117 | 12 / 14 | |
| Facts per `look` (median / p95) | 29 / 45 | 2 / 2.3 | at most 4 / 6 |
| Facts per journey (median / p95) | 26 / 49 | 4.2 / 5.1 | report, what stopped it, 2 more |
| Stated season, regional state or status lines | many | none (tested) | 0 unless asked |
| Facts digging could find per arrival | 3.4 per room | 13.1, 4× those shown | at least 3× |
| A repeated `look` with nothing changed | full repeat | at most 2 facts (tested) | at most 2 |

### After D03 (seeds 1–10, explorer 24 h)

| Metric | Before | D03 | Target |
|---|---|---|---|
| Building kinds per world | 8.7 (of 11) | 38.2 (of 50) | at least 25 of 35+ |
| Natural feature kinds per world | 5.8 | 22.4 | at least 12 |
| Scenes per settlement (median / least) | 0 | 4 / 3, plus 18.8 outside towns | at least 3, plus some outside |
| Towns sharing a role, size and layout | most | 0 | none |
| Town roles per world | 1 | 5.7 of 8 | |
| New kinds of thing in the fifth hour | – | 6.0 | still finding new kinds in hour 5 |
| Words per arrival (median) | 17 | 17 | at most 60 (D02) |
| WebAssembly world generation | – | under 0.1 s | under a few seconds |

New metrics: `places.scenes_per_settlement(_min)`, `places.scenes_outside`, `places.towns_sharing_layout`, `places.town_roles`, `play.new_kinds_hour5`. `places.natural_kinds` now counts natural features (not old marks), and `places.per_km2` counts features.

### The bots (D01)

`scraped-lang bots --seeds A-B [--bot explorer|scholar] [--hours H]`.

- **Curious explorer** (`crates/game/src/bots.rs`): survives three days on 8 of seeds 1–10 (16 of 1–20; after D03, 8 of 10 and 17 of 20). Since D03 it goes into kinds of building it has seen least first, makes for towns from noon, and shelters indoors from the evening or when cold. Deaths are cold: worlds whose start lies under a cold spell, nights caught in the open.
- **Scholar** (grammar spoilers, no map; body kept well): in 90 days reads the deepest text on 5 of seeds 1–10 and a great inscription on 6. The spec asks for 8. What stops it is in the game, not the bot:
  - **Held doors.** Old writing holds whole towns' doors shut. The only counter is to write "open" with a passage word (door, gate, tomb, box), and those words are met in fewer than two texts even after 100–170 texts read, so the understanding gate never lets the scholar write them.
  - **Scarce light.** Torches burn an hour, lamps four. Deep rooms are often dark, and the scholar must come back later with fuel.
  - **Distance.** The first lens is placed in roughly the 90th-percentile building by distance from the start, so reaching it means exploring most of the world.

  D07 (a larger lexicon) and D09 (the magic, deepened) should lift this; the test holds at 5 of 10 until then.

## Milestones

| # | Milestone | Delivers |
|---|---|---|
| D01 | [Instruments](milestones/D01-instruments.md) | Depth and brevity metrics, a curious-explorer bot and a scholar bot that reach the late game, sample transcripts, a decisions list, distinct landmarks |
| D02 | [Quiet text](milestones/D02-quiet-text.md) | An attention budget for every response, senses and digging as actions, facts shown by evidence instead of stated, slots reshaped into short combinable pieces |
| D03 | [Places with character](milestones/D03-places.md) | Natural features, many more building and settlement types, deeper interiors with secrets, scenes that tell what happened |
| S01 | [Course corrections](milestones/S01-course-corrections.md) | From Jb's review of D01–D03: an explorer bot that plays like a person, arrivals that describe the place, reading in layers (the whole text, then each sign's shape; exact strokes only by tracing, a task in itself), signs learned by the sounds they make when scraped, no arbitrary labels, and bug fixes |
| D04 | [Great interiors](milestones/D04-great-interiors.md) | Vast, intricate buildings and cave systems on a real spatial model: hundreds of spaces, many levels, loops, shortcuts, secrets; architecture that grew through history; navigation and mapping as a pleasure |
| D05 | [Things and mechanisms](milestones/D05-things-and-mechanisms.md) | A rich object model, keys and caches, old maps, multi-step machines, and non-linguistic puzzles: symbols, measures, calendars |
| D06 | [A living world](milestones/D06-living-world.md) | Ecology with signs and rhythms, weather and seasons with consequences, the sky, natural phenomena |
| D07 | [A language for long texts](milestones/D07-language.md) | Clause combining, subordinate and relative clauses, moods and aspects, derivation and compounds, a cultural lexicon of hundreds of words, meaningful place names |
| D08 | [History and what the writing says](milestones/D08-history-and-texts.md) | Deeper history with people, institutions and projects; twenty-plus text genres; story arcs told across many sites, matching physical evidence |
| D09 | [The magic, deepened](milestones/D09-magic.md) | A broad concept-to-power table, targeted and conditional spells, writing as the old civilisation's technology, a world saturated with old spells |
| D10 | [The slow realisation](milestones/D10-slow-realisation.md) | Ordinary tools, accidental discovery, evidence to notice, rewarding play without writing, a non-writing way to leave |
| D11 | [Problems only writing solves](milestones/D11-writing-payoff.md) | Late-game places and composition puzzles that reward real fluency, layered constraints, the great inscriptions as destinations |
| D12 | [Integration and tuning](milestones/D12-integration.md) | All clients and tools updated, fairness re-checked, targets met together, performance, a playtest pack and content plan for Jb |

Order of priority is deliberate: first the instruments and quiet text (D01–D02), so every later addition is measured and adds to what can be found rather than to what is read; then the world, outside and in (D03–D06); then what it says (D07–D08); then the magic and how it's discovered (D09–D10); then the late-game payoff (D11).

## Rules for every depth milestone

These add to `CLAUDE.md`.

1. **Depth, not checkboxes.** Each milestone states numeric targets measured by the D01 instruments. A milestone is done when the targets are met *and* the sample transcripts read as interesting, not when the checklist is ticked.
2. **Sample transcripts.** Each milestone commits spoiler-free transcripts from the curious-explorer bot on three seeds to `docs/samples/<milestone>/`, so Jb can read what play now feels like. Write a short honest note beside them: what got better, what still reads flat.
3. **Generated, not listed.** Variety comes from systems that combine (history × geography × culture × era), not from long hand-typed lists. Lists of kinds are fine as vocabulary; the interest must come from how they combine and from causes in history.
4. **Rich variables for content.** Every new thing the player can perceive exposes enough variables (material, size, condition, maker, era, distinguishing feature, what happened here) that Jb's templates can be specific. A slot whose only variable is `{kind}` produces the flat text this roadmap exists to fix.
5. **Everything has a cause.** If something is in the world, history or physics put it there, and the evidence of why is findable.
6. **Writing stays in the background.** No new system may point the player at the writing mechanic. Reviews check this.
7. **Say little; let the player dig.** Every new thing the player can perceive goes through the D02 attention model, with a salience and a digging layer. Richer worlds must not mean longer responses: brevity targets hold in every milestone after D02.
8. **The game is not your notebook.** No new system may keep lists or records for the player (no glossaries, journals, clue lists, sign lists). Any exception must be argued for and recorded in `docs/DECISIONS.md` (see `docs/DESIGN.md`).
9. **Keep what works.** Determinism, meaning-first language, slots instead of prose, browser and Android builds, fairness and tests all stay green throughout.

## Design gates

Each spec lists its gates with a proposed default, as before.

| Gate | Needed by | Proposed default |
|---|---|---|
| Is anyone else here? | D06 | No living people. Traces of earlier explorers (camps, belongings, notes) can appear as storylets in Jb's words |
| Can a player leave without writing? | D10 | Yes, by a long, hard physical journey to the world's edge, found through exploration. Writing offers other, stranger ways out |
| How common is old magic? | D09 | Every settlement has several small live spells, most places between have a few, and each region has at least one large one |
| Is writing the old civilisation's technology? | D09 | Yes: everyday writing kept larders cold, lamps lit, mills turning. The ruins of that infrastructure are much of the world's strangeness |
| How long before a typical player realises? | D10 | Many hours. Target: the naive bot has its first accidental release after 3–10 hours of play, and nothing in the first hour points at writing |
