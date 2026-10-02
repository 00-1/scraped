# D01 — Instruments

**Goal:** make depth measurable before adding it. Build the metrics, the bots and the sample-transcript pipeline every later milestone is judged by, collect the open design questions in one place, and fix the landmark identity problems found in review.

**Depends on:** M14 (done).

**Done when:** `scraped-lang depth` reports a baseline for ten seeds, both bots play deep into a world, baseline sample transcripts are committed, `docs/DECISIONS.md` lists every open `DESIGN-Q`, and no `look` ever lists two landmarks the player can't tell apart.

## Scope

### 1. Depth metrics

A `scraped-lang depth --seeds A-B [--json]` command (and a `depth` call in the web crate) reporting, per world and averaged:

- **Places:** distinct structure kinds, room purposes, natural feature kinds; landmarks; share of landmarks with a unique description; places per km² worth visiting (anything that isn't empty terrain).
- **Things:** distinct object kinds; objects per structure; mechanisms and how many steps their longest chain has.
- **Life:** species, and how many show signs (tracks, nests) without being seen.
- **History:** event kinds and counts; people with more than one trace in the world.
- **Writing:** texts by genre; share of the largest genre; distinct sentence shapes (meaning with names and numbers abstracted); mean words per text; concepts attested.
- **Magic:** live spells; distinct claim types; places whose strangeness has a writing cause; share of strange places with *no* writing cause (natural oddities).
- **Description variety:** for each slot family, distinct variable combinations met by the explorer bot.
- **Brevity:** words per response and facts per response (things, cues and states mentioned), as median and 95th percentile over the explorer bot's play, for `look`, arrival and travel separately. Measured with the example variants, counting facts from the slot variables actually rendered, so the number reflects the engine, not the prose.
- **Depth on demand:** how many further details are available by examining, listening, smelling and so on, per place visited. The aim of D02 onwards is for this to grow while brevity stays flat.

Each metric has a short definition in `docs/DEPTH.md` and a baseline recorded there.

### 2. Bots

- **Curious explorer** (never scrapes deliberately, never writes): plays like a new player who finds the world interesting. It goes towards unvisited landmarks, enters buildings, examines and reads things, operates mechanisms, follows edges, keeps itself alive, and sometimes cleans or scrapes surfaces for ordinary reasons once that's possible (D09). It records a **novelty curve**: new things perceived per in-game hour, and the time between new kinds of thing.
- **Scholar:** has spoiler access to the grammar but not the map. It reads, finds tools, scrapes and writes, and must reach the great inscriptions and the deepest text on most seeds. This fixes the coverage gap: today 103 of 172 slots are never reached by the bots.
- Both bots run headless in the CLI (`scraped-lang bots --seeds … --hours N`) and in the authoring tool's Gaps mode.

### 3. Sample transcripts

`scraped-lang samples <milestone>` writes spoiler-free explorer transcripts for seeds 1, 42 and 9001 (first three hours of play each) to `docs/samples/<milestone>/`. Commit the D01 baseline.

### 4. Decisions list

Collect every `DESIGN-Q` marker and every open question from `docs/LOG.md` into `docs/DECISIONS.md`: one row per question, with the current default, where it lives in code, and which milestone it affects. Group them by topic. This is for Jb to work through; do not resolve them.

### 5. Landmark identity

- Every landmark gets distinguishing traits generated from what it is: for a summit its shape, height band, rock colour and what's on it; for a settlement its size, walls, tallest building, condition; for a lone building its kind and silhouette. Expose them as slot variables (`land.name` and friends) so Jb's text can say "the split peak" or "the walled town".
- Traits are stable, so the same landmark reads the same from every viewpoint, which is what makes paper maps possible.
- The parser accepts directional and trait qualifiers: `go to the town to the west`, `go to the walled town`, `the second mountain`. "Which do you mean?" lists options by their distinguishing traits and bearings, never as two identical names.

## Tests

- Metrics are deterministic and snapshot-tested for three seeds.
- The scholar bot reaches the deepest text on at least 8 of 10 test seeds.
- The explorer bot survives at least 3 in-game days on at least 8 of 10 seeds.
- No `look` on any test seed lists two landmarks with the same rendered description (using the example variants).
- Qualified references (`to the west`, trait words, ordinals) resolve correctly.

## Checklist

- [ ] Depth metrics in CLI and web, defined and baselined in `docs/DEPTH.md`
- [ ] Curious-explorer bot with novelty curve
- [ ] Scholar bot reaching the late game; Gaps mode uses both bots
- [ ] Sample transcript command; D01 baseline committed
- [ ] `docs/DECISIONS.md`
- [ ] Landmark traits, slot variables and parser qualifiers
- [ ] Tests listed above
- [ ] LOG.md entry
