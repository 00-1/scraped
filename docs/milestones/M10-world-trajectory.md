# M10 — A world on a trajectory

**Goal:** the world is going somewhere, and the player's writing changes where. Regions carry slow variables that drift over days and seasons; great inscriptions from history drive the big trends; better scraping tools let the player reach them. Change is noticed by coming back.

**Depends on:** M09. **Design gate:** is there a living world (descendants who speak)?

**Done when:** on a dying-world seed, Jb sees evidence of decline, locates one of the great inscriptions, acquires the power to affect it, and over a season of play sees a region recover (or collapse further), with a trade-off somewhere else.

## Design decisions

| Decision | Proposed default |
|---|---|
| Regional variables | Life, water, stability and climate per region |
| Time scale | Regions update daily; meaningful change takes weeks to seasons. Long travel and waiting make time pass |
| Living world | None for the first release. Other people exist only as traces; creatures and plants respond to regional change |
| Lifespan | The player ages; a long life is several in-game years. Age shows in description and slowly worsens recovery |

## Scope

### In

1. **Regional simulation:** regions (from M04 biome and watershed boundaries) with the four variables, coupled to neighbours (water flows downstream, instability spreads, life follows water and warmth). Driven by the starting trajectory, active claims and seasons.
2. **Feeding local state:** regional variables bias local properties (M07): rivers shrink, ice persists, scrub returns, ground cracks. The local view is how the player sees regional change.
3. **Great active inscriptions:** a few world-scale claims from history (perpetual winter, a drowned city, a silenced region, a crevasse that widens), each with a location, a deep surface stack and a regional effect. Generated per seed from the history.
4. **Scaling scraping:** scraping tools of increasing power (found or made), gating which surfaces and claim scales the player can affect: door/room → site → region → great inscription. Writing scales the same way.
5. **Trade-offs:** coupling makes most large interventions cost something somewhere else, and the debug view explains the chain.
6. **Change on revisits:** the game keeps a per-place memory of how it was last described, and descriptions can draw contrast ("The pool you remember is gone; cracked mud remains") through slots with *before/after* variables.
7. **Time passage:** waiting and long journeys advance the simulation efficiently; seasons; ageing.
8. **Inferring the big picture:** evidence of regional state spread through the world (ledgers of shortages generated from history, abandoned farms, migration graffiti), so the player can infer what's happening (DESIGN.md "Seeing the whole").
9. **Debug views:** regional variable maps over time, causes of change.

### Out

The end-of-run summary and chronicle (M11).

## Content slots introduced

Regional state cues in area descriptions; before/after contrast phrases; seasons; ageing; great-inscription sites; the feeling of a world-scale claim releasing.

## Tests

- Regional simulation is stable (no oscillation or blow-up) over 10 in-game years with no player.
- Each starting trajectory produces its intended direction without intervention.
- Every great inscription is reachable and affectable with the strongest tool.
- Time skipping gives the same result as stepping day by day.
- Revisit contrast only appears when a real change crossed a threshold.

## Checklist

- [ ] Regions, variables and coupling
- [ ] Regional → local property bias
- [ ] Great active inscriptions from history
- [ ] Scaling scraping and writing tools
- [ ] Trade-off coupling and debug explanation
- [ ] Per-place memory and revisit contrast
- [ ] Time passage, seasons, ageing
- [ ] Evidence of the big picture placed in the world
- [ ] Slots registered with examples
- [ ] Tests listed above
- [ ] LOG.md entry
