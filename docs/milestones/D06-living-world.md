# D06 — A living world

**Goal:** the world moves without the player. Plants and animals live by their own rules, weather and seasons have consequences, and the sky is worth looking at. Some of what's strange in the world is natural, so magic is not the only explanation for oddities.

**Depends on:** D05. **Design gate:** is anyone else here? (Proposed default: no living people; traces of earlier explorers may appear as storylets in Jb's words.)

**Done when:** the D06 targets are met, and the explorer samples show a player noticing animals, signs, weather and sky as part of exploring, through the D02 attention model (mostly found by listening, looking up, waiting and examining, not announced).

## Scope

### 1. Ecology

- **Species generated per world** from biomes and climate, with roles: grazers, browsers, predators, scavengers, burrowers, birds (resident and migrating), fish, insects and other small life. Plants: trees, shrubs, grasses, flowers, fungi, water plants, each with seasons (leaf, flower, fruit, die-back).
- **Behaviour:** daily and seasonal rhythms (dawn chorus, nocturnal hunters, ruts, migrations), territories, dens, nests and roosts as places, reactions to the player, fire, noise and each other.
- **Signs:** tracks in mud and snow, droppings, feathers, browsed bark, burrows, calls. Most animals are known by their signs long before they're seen.
- **Use:** forage, fish and trap at a simple level, consistent with M07's coarse needs. Some plants are useful (fibre, dyes, fuel, remedies), some harmful. No crafting trees.
- **Response to change:** life follows the M10 regional variables, so a recovering region gains species and a dying one loses them, visibly.
- Creature archetypes from M07 become species with roles; the "something stranger in deep places" stays, now with signs and lore.

### 2. Weather and seasons with consequences

- Weather systems that move across the map: fronts, storms, fog banks, heat, snowfall that lies and melts.
- Consequences: rivers rise after rain and flood fords; snow closes passes and shows tracks; drought exposes old foundations in a lake; storms bring down trees and ruined walls; frost makes ice that can bear weight.
- Seasons change the land as evidence (D02): what grows, what's in flower, which birds are present, the length of the day.

### 3. The sky

- Generated stars and constellations (named in the language, eventually readable in texts), a moon or moons with phases, planets with motion, occasional eclipses, comets and meteor showers on a schedule.
- Usable for navigation at night (a pole star or equivalent) and for the calendar: the culture's festivals, temple alignments and calendar devices (D05) follow the sky.
- `look up` is a real activity: the night sky rewards attention.

### 4. Natural phenomena

A set of genuinely natural oddities, placed by geography: marsh lights, singing or booming dunes, tides and tidal bores, hot springs and steam vents, echoing gorges, mirages, aurora at high latitude, bioluminescence, fogbows. They give the world wonder of its own, and they make magic harder to spot: a player can't assume every oddity is writing.

## Content slots introduced

All through the attention model: species signs, sightings and behaviour (as variables on shared slot families), plant states by season, weather events and their consequences, sky objects and events, natural phenomena, foraging, fishing and trapping.

## Targets (measured by D01)

| Metric | Baseline | Target |
|---|---|---|
| Species per world | 4 archetypes | at least 40 animals and 40 plants |
| Animals known by signs before being seen (explorer) | 0 | most |
| Weather consequences per in-game month | about 0 | at least 3 that change routes or places |
| Natural phenomena per world | 0 | at least 5 |
| Strange places with no writing cause | 0% | at least 25% of all strange places |

## Tests

- Populations stay within bounds over 10 in-game years with no player.
- Signs only appear where the species lives.
- Sky positions are consistent with a fixed calendar; eclipses are predictable from the cycle.
- Floods, snow and drought change travel as intended.
- Natural phenomena never have a writing cause in debug views.

## Checklist

- [x] Species and plants generated from biomes and climate
- [x] Behaviour, rhythms, territories and dens
- [x] Signs and tracks
- [x] Simple foraging, fishing and trapping
- [x] Life follows regional change
- [x] Moving weather systems and their consequences
- [x] The sky: stars, moon, planets, events; night navigation
- [x] Natural phenomena
- [x] Slots and variables, through the attention model
- [x] Targets met; samples committed with a note
- [x] Tests listed above
- [x] LOG.md entry
