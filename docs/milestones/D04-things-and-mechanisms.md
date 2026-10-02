# D04 — Things and mechanisms

**Goal:** fill the world with things worth finding and machines worth working out, and give players puzzles that need no language at all: symbols, numbers and the calendar. These are the "other intriguing things" a player can pursue for hours, and they quietly train the skills that writing will later need.

**Depends on:** D03.

**Done when:** the D04 targets are met, a non-reading player has several non-writing puzzles to pursue on every seed, and the explorer samples show objects and machines being found, examined and used for their own sake.

## Scope

### 1. Objects with histories

- A general object model: kind, material, size, maker, owner, era, condition, decoration, contents, and the event that left it here.
- Many kinds, generated from the culture's crafts and wealth: vessels, tools of each trade, coins, seals, weights, jewellery, figurines, games and toys, instruments, weapons and armour as relics, clothing, furniture, lamps, keys, boxes and chests, astronomical instruments, medical tools, writing materials.
- Containers hold things; caches are hidden in places (under floors, in walls, buried at landmarks).
- Objects can be examined closely to reveal small details (maker's marks, wear, repairs).
- Ordinary uses where they make sense: a rope to climb, a pole to probe ice, a hook to reach, a bell to ring, a mirror to signal or to direct light.

### 2. Keys, locks and old maps

- Locks with keys that are elsewhere, placed by who owned them.
- **Old maps** found in the world: generated from the world as it was in their era, described in the slots' words (an outline, landmarks, a route), partial and sometimes wrong for today (a river that has moved, a town now gone). They invite comparison with the player's own map.

### 3. Machines and works

Multi-step mechanisms built from parts, some spanning several rooms or sites:

- **Water:** cisterns, aqueduct valves, sluice networks, canal locks, fountains, waterwheels driving machinery.
- **Weight and motion:** counterweights, lifts, drawbridges, portcullises, cranes, turntables.
- **Signal and sound:** bells, gongs, horns, signal fires seen between stations.
- **Light:** mirrors and light shafts, lamps that must be lit in order.
- **Time:** water clocks, sundials, calendar devices.

Each machine has a state that the physical simulation (M07) understands, and working one out is a puzzle of observation and order, not of reading.

### 4. Non-linguistic decipherment

Generated consistently from history, so that understanding them is real:

- **Symbols:** each faction, family, temple and era has emblems that appear on seals, coins, banners, doors and tombs. A player can learn who built or owned what without reading a word.
- **Measures:** weights and measuring vessels with marks; ledgers (which the player can't read yet) sit next to weighed goods. The numeral signs are learnable from objects alone.
- **Calendar and sky:** the culture's calendar (months, festivals) shown on calendar devices, temple alignments and sundials, linked to the sky (D05).

These are puzzles in their own right with payoffs (a cache only the right emblem leads to; a door that opens on the festival day; a set of weights that reveals a hidden storeroom), and they give later language work its first anchors.

## Content slots introduced

Everything new is a candidate for the D02 attention model, with a salience and a digging layer: most of it should be found by exploring and looking closer, not announced. Object examine and close-examine (with rich variables), containers, caches, keys and locks, old map description, each machine family's states and operation, emblem descriptions, measures, calendar devices.

## Targets (measured by D01)

| Metric | Baseline | Target |
|---|---|---|
| Object kinds per world | about 20 | at least 120 |
| Objects per structure (mean) | about 1 | at least 6 |
| Longest mechanism chain | 1 step | at least 4 steps on most seeds |
| Non-writing puzzles per world | 0 | at least 8 with payoffs |
| Old maps per world | 0 | at least 2 |

## Tests

- Every object's presence traces to an owner, maker, event or trade.
- Every key's lock exists; every cache is findable from clues in the world.
- Every machine is solvable from observation alone; a test bot solves each by trying actions and watching state.
- Emblem consistency: the same owner always has the same emblem.
- Old maps match the world of their era.

## Checklist

- [ ] Object model and kinds from culture
- [ ] Close examination and ordinary uses
- [ ] Containers and caches
- [ ] Keys and locks
- [ ] Old maps
- [ ] Machines in each family, multi-room and multi-site
- [ ] Emblems, measures, calendar devices, with payoffs
- [ ] Slots and variables
- [ ] Targets met; samples committed with a note
- [ ] Tests listed above
- [ ] LOG.md entry
