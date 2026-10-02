# M04 — World generation

**Goal:** generate a complete, bounded world from a seed: land, climate, water, a civilisation's history across the language's eras, the places it built, and the state those places are in now. Pure data; no player yet.

**Depends on:** M02 (eras), M03 (slots for anything descriptive).

**Done when:** Jb can generate a world, view a debug map, read a history timeline, and see that settlements, roads and inscriptions sit where history put them, with the right era's language.

## Design decisions

| Decision | Proposed default |
|---|---|
| World shape | A bounded basin or island, chosen per seed, about a week's walk across |
| Resolution | A continuous heightfield sampled on a fine grid (for example 1 cell ≈ 50 m), with structures as separate placed objects that have interiors |
| One civilisation or several | One civilisation with factions that split across eras; one language family |
| Starting trajectory | Generated now as a seed-level parameter (dying / stagnant / balanced / recovering); the simulation that uses it arrives in M10 |

## Scope

### In

1. **Terrain and climate:** layered noise for height, then erosion-lite (hydraulic smoothing) for believable valleys; temperature from latitude and altitude; moisture from prevailing wind and rain shadow; biomes from temperature × moisture × altitude.
2. **Water:** rivers traced downhill from high-rainfall sources, joining and widening; lakes where flow pools; coast. Fords and narrow points flagged (they matter for movement and history).
3. **History simulation** over the eras from M02:
   - Founding sites chosen by water, defensibility and resources; growth, trade routes, roads.
   - Events: settlement founding and abandonment, wars, plagues, famines, schisms, rulers and dynasties (named with generated language), migrations, and **writing events**: inscriptions cast with potent writing and what they were for.
   - Each event records place, era, actors and cause, so it can later appear as evidence (a ledger of shortages, a tomb, a defaced name).
   - Writing events change the world's state (a river diverted, a region chilled). They are recorded as **intended effects** now; M08 makes them mechanical.
   - The **root event** (DESIGN.md "the deepest text"): one event per world that explains its present condition, placed deepest in time.
4. **Structures:** settlements, temples, archives, tombs, waystations, bridges, mines, walls, placed by history. Each has a generated **interior layout** (rooms, connections, levels) from a small set of authored-shape generators (temple, house, archive, tomb, tower). Room purpose is data; description is content.
5. **Texts in the world:** every structure gets the writing its history implies: dedications on temples, ledgers in storehouses, tombs in cemeteries, signs on roads, letters in houses. Rendered in the era it was written in. Surfaces have materials. (Layering, scraping and visibility come in M08; for now each text is one entry with an era and an author.)
6. **Present state:** decay by age and climate (collapsed roofs, buried rooms, washed-out bridges), lingering effects of old writing events recorded as region properties, and the generated starting trajectory.
7. **Debug output**, spoiler-only:
   - `scraped world --seed N map` as a PNG (height, biomes, rivers, structures, roads) and as ASCII for terminal use.
   - `history` as a timeline; `site <id>` with interior map and texts.
   - All of the above as JSON, and visible in the browser bench.

### Out

Player, movement and perception (M05–M06), local physical simulation (M07), effects mechanics (M08), regional change over time (M10).

## Content slots introduced

None required for play yet. Names of biomes, structure types and room purposes are data ids; their descriptions become slots in M05–M06.

## Tests

- Determinism across platforms (snapshot world JSON and a hash of the map for three seeds).
- Rivers always flow downhill and reach a lake or the coast.
- Every structure is reachable over land, by road or by a ford/bridge from at least one other structure (no unreachable sites unless deliberately sealed, which is recorded).
- Every text's language matches its era, and every name is a valid word of its era.
- Every history event that implies evidence has at least one text or physical trace in the world.
- World size and feature density stay within configured bounds.
- Generation time stays reasonable in WebAssembly (target: a few seconds).

## Checklist

- [ ] `world` crate scaffold
- [ ] Terrain, climate, biomes
- [ ] Rivers, lakes, coast, crossings
- [ ] History simulation with events and the root event
- [ ] Structure placement and interior generators
- [ ] Texts placed by history, in era
- [ ] Present-state decay and lingering effects
- [ ] Debug map, timeline and site views in CLI and bench
- [ ] Tests listed above
- [ ] LOG.md entry
