# M06 — Perception and movement

**Goal:** free, intent-based movement across the whole generated world, built on one rule: *you can only be surprised by what you couldn't see.* Players navigate by landmarks, follow rivers and roads, get lost in fog and forest, and map the world on paper from bearings and distances.

**Depends on:** M05.

**Done when:** Jb can leave the starting site, travel to a landmark he can see, follow a river, get lost in the dark, find his way back by landmarks, and draw a map that matches the debug map.

## Design decisions

| Decision | Proposed default |
|---|---|
| Position | Continuous coordinates on the world grid; interiors stay room-based |
| Granularity of a "place" | The player is always somewhere; `look` describes the local area (about 100–300 m) plus what's visible further away |
| Bearings and distance reporting | Eight compass points and rough distances/durations ("about an hour"), never exact numbers |
| Compass availability | The player knows north from the start (sun and stars). Losing it in fog or underground is part of drift |

## Scope

### In

1. **Visibility:** line of sight over the heightfield with an eye height; range limited by weather, light and vegetation density; structures and features have heights and silhouettes.
2. **Salience:** score what's visible by size, contrast, uniqueness and distance. `look` reports the local area and the most salient distant landmarks with bearings and rough distances.
3. **Followable edges:** rivers, streams, roads, paths, treelines, cliffs, coasts, walls. `follow the river upstream`, `follow the road north`.
4. **Intent commands:**
   - `go to <landmark>`, `head <direction>`, `follow <edge> <direction>`, `go back` / `retrace my steps`, `go to <named place>`, `enter <structure>`, `leave`.
   - Short moves within sight resolve directly. Long moves are simulated in steps.
5. **Travel simulation:** pathfinding over terrain costs (slope, water, vegetation); a step loop that can be interrupted by anything newly seen (a structure, a hazard, an inscription, and later threats in M07); time passes with distance and terrain.
6. **Drift:** when no landmark or edge is visible (fog, night, dense forest), each step adds heading error. The player may arrive somewhere other than intended, and the game reports only what they perceive, never their true position.
7. **Mapping support:** every travel report includes bearing and rough distance actually travelled *as the character perceives it* (which drifts too). Arrival at a new area always produces a description with its own landmark bearings.
8. **Naming places:** `name this place <name>`, then `go to <name>`. Named places are stored with the true position but the player only "knows" them as remembered; reaching them still needs a route.
9. **Weather and light at the coarse level:** time of day, day length, fog and rain from climate, enough to drive visibility and drift. (Temperature and needs come in M07.)
10. **Multi-scale description:** region sense (rough biome and terrain from a high point), area (local look) and room (interiors).

### Out

Survival, threats and intercepting creatures (M07), regional change (M10).

## Content slots introduced

Area look by biome and terrain; distant landmark phrases by type and distance band; edge descriptions; travel narration (start, uneventful passage, arrival); getting-lost cues; weather and light at a glance; naming confirmations; "you can't see that from here".

## Tests

- Visibility is symmetric and blocked by terrain.
- `go to X` always arrives at X in clear daylight with a valid path.
- Drift is zero with a visible landmark and grows without one; seeded and reproducible.
- Bearing reports agree with true geometry within the stated rounding, in clear conditions.
- An automated "surveyor" bot that only uses `look` and travel reports builds a map whose site positions match the true map within tolerance. This proves the world is mappable on paper.
- Performance: a long journey resolves fast enough for the browser.

## Checklist

- [ ] Visibility over heightfield, weather and light limits
- [ ] Salience scoring and landmark reporting
- [ ] Followable edges
- [ ] Intent movement commands
- [ ] Pathfinding and step-wise travel with interruptions
- [ ] Drift and honest uncertainty
- [ ] Bearings and distance reporting
- [ ] Naming places
- [ ] Time of day, day length, fog and rain
- [ ] Multi-scale descriptions as slots
- [ ] Surveyor bot test
- [ ] LOG.md entry
