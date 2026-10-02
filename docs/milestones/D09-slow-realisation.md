# D09 — The slow realisation

**Goal:** make the discovery of writing's power the player's own, slow and unannounced, as `docs/DESIGN.md` ("Writing is background, at first") requires. Remove everything that points at the mechanic, make the tools ordinary, let the realisation come from evidence and accident, and make sure a run without it is still a full and rewarding run.

**Depends on:** D08. **Design gate:** can a player leave without writing? (Proposed default: yes, by a long, hard physical journey found through exploration.)

**Done when:** in bot play, nothing in the first hour points at writing; the naive explorer's first accidental release comes after hours, not minutes; and a player who never writes still has goals, payoffs and a meaningful ending.

## What exists now that goes against the design

- The scraper is placed beside a "pivot" spell chosen to be safe and obvious (M08).
- The `tool_found` storylet hook fires on picking up a writing tool, and the example text announces it ("It fits your hand as if it remembered it").
- The scraper, stylus and lens are named as writing tools; `scrape` exists only for writing.
- The fairness goals start with "scraper" and "first_release", treating them as the path.

## Scope

### 1. Ordinary tools

- **Scraping** is an ordinary action with ordinary tools: knives, chisels, adzes, pumice, sand, a stiff brush, a whetstone. Players use them for ordinary reasons: cleaning moss, soot or lichen off a surface to read or see it; scraping hides; shaving wood for kindling; removing paint. `clean`, `scrub` and `scrape` all work on surfaces.
- Better tools exist and matter (they bite harder and reach older, stronger writing, as M10's scaling), but they are found as good tools of a craft (a scribe's fine knife, a mason's chisel), not as magic items.
- **The lens** is a magnifier and fire-starter, useful for close examining (D02) and making fire (M07); seeing deeper layers is something the player may notice it does.
- **Writing materials** (ink, styluses, brushes, chalk, charcoal) are ordinary finds in scriptoria, schools and workshops. The player can make marks of their own with them in their own transliteration (M05), and may later realise those marks can act.
- Rename item ids and slot variables so nothing internal leaks into player-facing names (no `first_scraper`).

### 2. Discovery from evidence

- Remove the pivot placement. Unscraped potent texts are placed by history (D08), not beside a tool.
- **Accidental release:** cleaning a surface fully, scraping for an ordinary purpose, or a creature, flood or collapse removing writing can release a spell. The first a player meets is likely to be accidental, and its effect may be small, delayed, or out of sight.
- **Patterns to notice:** strange places tend to have scraped writing nearby; similar strangeness goes with similar fragments; old maps (D04) mark places that are now strange; texts (D07) talk about writing that did things, in the language. The evidence is there to find, not delivered.
- **No narrator knowledge.** Slots never connect effect to text. Storylet hooks may mark moments (first accidental release, first deliberate one) for Jb's writing, but the engine's descriptions don't explain.
- Different players should realise at different points; the bots measure the spread.

### 3. A rewarding run without writing

- Goals that need no writing: explore and map the world; find every kind of place; solve machines and non-linguistic puzzles (D04); follow the history through scenes, objects and emblems; learn the sky; survive seasons; reach remote places (summits, islands, deep caves); find the old explorers' traces (D05 gate).
- **A non-writing way to leave:** a long, hard physical journey to the world's edge, found through exploration (a pass, a crossing, a ship), as the gate proposes.
- The end-of-run summary (M11) values exploration as well as writing: places found, mysteries understood, how far the player got, what they left behind.

### 4. Pacing

- Naive-explorer bot (from D01, now able to `clean` surfaces for ordinary reasons) plays 20 hours on many seeds. Record: time to first accidental release, time to first deliberate release (if the bot can detect the pattern from its own observations), and what it was doing when it happened.
- Tune placement, frequency of accidental releases, and the visibility of effects to hit the targets.

### 5. Fairness, revised

The M14 fairness goals change from a path (scraper → first release → …) to **reachability without a path**: each goal is reachable, and the evidence needed to understand it is findable, in more than one order.

## Targets

| Metric | Target |
|---|---|
| Responses in the first hour that mention writing as more than background | 0 |
| Naive bot's first accidental release | between 3 and 10 hours on most seeds |
| Non-writing goals with payoffs per world | at least 10 |
| Seeds where a non-writing departure is reachable | all |
| Spread of realisation times across seeds and bots | wide; no single scripted moment |

## Tests

- No player-facing slot variable or item name contains internal writing-mechanic terms.
- The pivot placement and its tests are gone; nothing places an unscraped spell next to a tool.
- Cleaning a surface with ordinary tools releases potent writing exactly as scraping does.
- A non-writing scripted run reaches the world's edge and ends there.
- Fairness revised and passing on a seed batch.

## Checklist

- [ ] Ordinary scraping tools and ordinary scraping uses (`clean`, `scrub`)
- [ ] Lens and writing materials made ordinary
- [ ] Internal names removed from player-facing text
- [ ] Pivot removed; tool-found storylet reframed
- [ ] Accidental releases, including by creatures, floods and collapse
- [ ] Evidence patterns placed by history
- [ ] Non-writing goals and a non-writing departure
- [ ] End summary values exploration
- [ ] Pacing measured and tuned
- [ ] Fairness revised
- [ ] Samples committed with a note
- [ ] Tests listed above
- [ ] LOG.md entry
