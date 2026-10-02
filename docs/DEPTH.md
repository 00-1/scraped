# Depth roadmap

The first roadmap (`docs/ROADMAP.md`, M01–M14) built every system the design asked for, each at the simplest level that met its spec. This roadmap makes them deep. Its aim, in Jb's words: **the environment should be genuinely interesting and rewarding to explore in its own right**, and the writing should start as background flavour that the player only gradually realises matters (`docs/DESIGN.md`, "Writing is background, at first"). And however rich the world gets, **the text stays short and intriguing; the player digs for detail** (`docs/DESIGN.md`, "Say little; let the player dig").

Each milestone has a spec in `docs/milestones/`. Work through them in order.

## Where the engine is now

Measured on the M14 build (seed 42 unless noted).

| Area | Now | Problem |
|---|---|---|
| Building types | 11 | Every town is the same handful of temples, houses, tombs and storehouses |
| Interiors | 3–6 rooms joined by compass exits, no geometry | Nothing large or intricate to explore inside; maps can't be checked |
| Room purposes | about 7 | Interiors feel interchangeable |
| Natural features | rivers, lakes, coast, summits | Nothing between towns worth walking to |
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
| Response length | Arrival lists weather, ground, every building, horizon, season, land state, creatures | Each system appended a line; more depth would mean walls of text |

## Milestones

| # | Milestone | Delivers |
|---|---|---|
| D01 | [Instruments](milestones/D01-instruments.md) | Depth and brevity metrics, a curious-explorer bot and a scholar bot that reach the late game, sample transcripts, a decisions list, distinct landmarks |
| D02 | [Quiet text](milestones/D02-quiet-text.md) | An attention budget for every response, senses and digging as actions, facts shown by evidence instead of stated, slots reshaped into short combinable pieces |
| D03 | [Places with character](milestones/D03-places.md) | Natural features, many more building and settlement types, deeper interiors with secrets, scenes that tell what happened |
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
8. **Keep what works.** Determinism, meaning-first language, slots instead of prose, browser and Android builds, fairness and tests all stay green throughout.

## Design gates

Each spec lists its gates with a proposed default, as before.

| Gate | Needed by | Proposed default |
|---|---|---|
| Is anyone else here? | D06 | No living people. Traces of earlier explorers (camps, belongings, notes) can appear as storylets in Jb's words |
| Can a player leave without writing? | D10 | Yes, by a long, hard physical journey to the world's edge, found through exploration. Writing offers other, stranger ways out |
| How common is old magic? | D09 | Every settlement has several small live spells, most places between have a few, and each region has at least one large one |
| Is writing the old civilisation's technology? | D09 | Yes: everyday writing kept larders cold, lamps lit, mills turning. The ruins of that infrastructure are much of the world's strangeness |
| How long before a typical player realises? | D10 | Many hours. Target: the naive bot has its first accidental release after 3–10 hours of play, and nothing in the first hour points at writing |
