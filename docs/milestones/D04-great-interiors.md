# D04 — Great interiors

**Goal:** vast, intricate buildings and caverns to explore from the inside: palaces, temple precincts, fortresses, catacombs, mines, underground cities and natural cave systems with hundreds or thousands of spaces, multiple levels, loops, shortcuts, dead ends and secrets. Navigating them, and mapping them on paper, should be one of the game's great pleasures.

**Depends on:** D03 (which places the structures, cave mouths and underground routes this milestone builds).

**Done when:** the D04 targets are met; every world has several great interiors that take hours to explore; a careful paper map of one matches its true plan; and the explorer samples show getting lost, finding the way, and the delight of a shortcut back to a known place.

## Why a new model

Today an interior is a handful of rooms (a temple is a forecourt, a hall, a sanctum, perhaps two chapels and a crypt) joined by compass exits, with no geometry. That can't hold a vast building: rooms have no size or position, so a map can't be checked, distances and directions mean nothing, and big places would become long chains of interchangeable rooms.

## Scope

### 1. A spatial model

- Interiors become **real geometry**: spaces with positions, footprints, heights and levels on a fine 3D grid (proposed: 2 m cells, levels of varying height), joined by openings (doors, arches, stairs, ramps, ladders, shafts, crawlways, windows, holes).
- **Spaces** are rooms, halls, corridors, galleries, stairwells, courtyards, light wells, chambers and passages. Large spaces are large: a pillared hall can take minutes to cross in the dark, and can be partly seen from a balcony above.
- Directions and distances in descriptions and travel reports come from the geometry, so they are honest and mappable.
- All structures, ordinary and great, move to this model. The current room-graph interiors are replaced, keeping their texts, features and scenes in place.
- **Lazy generation:** the outline of every structure is generated with the world; the full interior of a great structure is generated deterministically on first approach, so WebAssembly start-up stays fast.

### 2. Architecture that grows

Great buildings are generated the way real ones come to exist:

- An **architectural grammar per culture and era:** axes and symmetry, courtyards, modules (cells, bays, aisles), proportions, materials, decoration, how stairs and vaults are built.
- **Growth through history:** an original core, then extensions, rebuilds, new wings in later eras' styles, blocked doors, filled windows, subdivided halls, abandoned sections, collapse, flooding, burial. The intricacy has causes, and the building is itself a palimpsest: older masonry under newer, a sealed door outlined in a later wall.
- **Kinds of great interior,** placed by history and D03's settlement roles: palace complex, temple precinct or monastery, fortress and its tunnels, library or archive complex, necropolis and catacombs, mine (adits, shafts, galleries, chambers, flooded lower levels), cistern and aqueduct tunnels, underground city or refuge, labyrinth, and the town's own undercroft of cellars, drains and passages.
- Spaces have purposes from the building's life (kitchens, dormitories, cells, scriptoria, treasuries, shrines, stores, workshops, guardrooms) and are furnished accordingly, which D05 fills with objects and machines.

### 3. Caves

Natural cave systems generated from geology and water (D03), not from building rules:

- Passages follow fractures and bedding planes; tubes, canyons, crawls, chimneys, pits, chambers where passages meet or the roof collapsed.
- Underground rivers and lakes, sumps (flooded passages), waterfalls, mud, and seasonal flooding (D06).
- Formations: stalactites and stalagmites, flowstone, crystals, columns. Fossils in the walls.
- Several entrances, some far apart; draughts between them.
- **Where people met caves:** mines broke into caverns, catacombs extended natural tunnels, shrines were built in cave mouths, a refuge city was carved out of a cave system.

### 4. Navigating inside

- **Moving:** by openings (`through the arch`, `down the stair`, `east`), by intent within what's known (`go to the pillared hall`, `go back to the stair`, `follow the passage`), and by feel in darkness (one step at a time).
- **Interior landmarks:** distinctive spaces and features (the great stair, the hall of columns, the dry fountain) get traits like outdoor landmarks (D01), so the player can name and return to them.
- **Getting lost:** in large, repetitive or dark places, uncertainty grows (as with outdoor drift, M06). The game says what the player believes, not where they are.
- **Finding the way:** light is a resource; `listen` for water or wind; `feel` for draughts that lead out; follow walls; mark places (`mark this wall`, chalk or scratches, which also become things later players' notes can refer to); name places.
- **Vertical travel:** climbing, ropes, descending shafts, drops that can't be climbed back up (one-way), balconies that look down on places you can't yet reach.
- **Loops and shortcuts:** many routes join up; doors and gates that open only from one side become shortcuts once reached from behind.
- **Hazards:** falls, collapse, flooding, bad air, getting stuck, getting lost with no light. Through M07's survival systems.
- **Brevity:** large spaces are described briefly with a sense of scale (echo, light failing before the far wall), and detail comes from digging (D02). Sequences of similar corridors compress ("the passage winds on, branching twice") instead of repeating descriptions.

### 5. Secrets and structure to discover

- **Hidden spaces** inferable from a careful map: a gap in a symmetrical plan, a wall too thick, a stair that rises higher than any known room, a draught from solid stone. Ways in: a loose panel, a crawlway, a route from a neighbouring building or from below.
- **Inaccessible-but-visible** places (a gallery above, a door across a chasm, a grating over a lit room) that invite finding the way round.
- Places sealed by collapse or by old writing, entered much later (D11).

### 6. Tools and checks

- **Debug floor plans:** an SVG plan per level for any interior, in the bench and the authoring tool's inspector (spoilers), with the player's route and beliefs overlaid.
- **Mapper bot:** explores an interior using only what the text says (openings, directions, distances, landmarks) and builds a plan. Its plan must match the true plan within tolerance; this proves interiors are mappable on paper.
- Metrics for D01: spaces per great interior, levels, loops (independent cycles), dead ends, longest shortest-path to the deepest space, hidden spaces, one-way connections.

## Content slots introduced

Through the attention model, with digging layers: space looks by kind and size and condition; openings by kind and state; scale cues; compressed passage sequences; interior landmarks; darkness and moving by feel; marks the player makes; vertical movement; hazards underground; cave features and formations; architectural style and era differences (masonry, vaults, decoration) as variables, so the player can learn to tell an old wing from a new one by its stones.

## Targets (measured by D01)

| Metric | Baseline | Target |
|---|---|---|
| Spaces in the largest interior | about 6 | at least 400 |
| Great interiors (200+ spaces) per world | 0 | at least 4, including at least 1 cave system |
| Levels in the deepest interior | 2 | at least 6 |
| Independent loops in a great interior | 0 | at least 15 |
| Hidden spaces per great interior | 0 | at least 5, each inferable from the plan |
| Mapper bot plan error | n/a | within 10% of distances; topology exact for spaces it visited |
| Brevity (D02 targets) | — | still met inside great interiors |
| Time to explore the largest interior (explorer bot) | minutes | several in-game days |

## Tests

- Geometry is consistent: no overlapping spaces, every opening joins two spaces that touch, stairs join levels at matching heights.
- Every space is reachable, or deliberately sealed as recorded with its way in.
- Hidden spaces are inferable (the mapper bot's plan shows the gap).
- Cave passages follow the geology and water rules.
- Architectural growth by era is visible in recorded styles.
- Lazy generation is deterministic: generating an interior at different times in a run gives the same result.
- Performance: generating a great interior on approach takes well under a second in WebAssembly; saves stay small (state as changes, not whole interiors).

## Checklist

- [x] Spatial model: spaces, geometry, levels, openings; all structures migrated with their texts and scenes
- [x] Lazy, deterministic generation of great interiors (decided against: the whole world, interiors included, builds in about 0.2 s in WebAssembly; see DECISIONS)
- [x] Architectural grammar per culture and era; growth through history
- [x] Each kind of great interior
- [x] Cave systems from geology and water; where people met caves
- [x] Interior movement, landmarks, getting lost, finding the way, marks (getting lost is by feel in the dark only; see DECISIONS)
- [x] Vertical travel, one-way routes, loops and shortcuts
- [x] Underground hazards
- [x] Brief descriptions of scale; compressed passages
- [x] Hidden and visible-but-unreachable spaces
- [x] Debug floor plans in bench and authoring inspector
- [x] Mapper bot
- [x] Slots and variables, through the attention model
- [x] Targets met; samples committed with a note
- [x] Tests listed above
- [x] LOG.md entry
