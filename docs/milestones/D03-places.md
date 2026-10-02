# D03 — Places with character

**Goal:** make the land worth walking across and every site worth entering. Natural features between settlements, many more kinds of building and settlement, interiors with depth and secrets, and scenes that show what happened in a place before any text explains it.

**Depends on:** D02 (the attention system decides how all this is shown).

**Done when:** the D03 targets below are met, and the explorer bot's sample transcripts show a player who keeps finding new kinds of place for hours, with no two towns feeling alike.

## Scope

### 1. Natural features

Generated from terrain, water, climate and geology, not scattered at random:

- **Water:** springs (some warm), waterfalls, rapids, gorges, oxbow lakes, deltas, tidal flats, sea stacks, sea caves, bogs.
- **Rock and ground:** caves and cave systems (entrances, chambers, underground rivers, links between distant openings), sinkholes, cliffs with ledges, natural arches, scree, rock pillars, boulder fields, salt flats, a glacier or snowfield where cold enough.
- **Life:** ancient lone trees, groves, dead or petrified forest, reed beds, flower meadows in season.
- **Old human marks on the land:** standing stones, cairns, terraces, field walls, old roads and cuttings, quarries, spoil heaps, burial mounds.
- A light **geology** layer (rock types by region) so caves sit in limestone, slate quarries in slate, and so on, giving the land consistent regional character.

Each feature is a landmark or place with its own traits (D01) and its own interior where it has one (caves).

### 2. Buildings and settlements

- **Many more structure kinds**, each with an interior generator and room purposes, placed by history's needs: mill, granary, bakehouse, brewery, bathhouse, cistern, aqueduct, fountain house, market hall, warehouse, harbour and jetty, lighthouse, smithy, kiln and pottery, tannery, dye works, weaving house, scriptorium, school, library, observatory, palace, council hall, courthouse, prison, barracks, armoury, gatehouse, signal station, hermitage, wayside shrine, ossuary and catacombs, garden and orchard, amphitheatre, mausoleum, labyrinth or processional way. Target at least 35 kinds in total.
- **Settlement character:** each settlement gets a role from history and geography (port, holy city, mining camp, fortress, market town, farming village, capital, refuge), which decides its buildings, layout and size.
- **Layout:** streets and squares, districts, walls and gates where history had wars, a reason for the town's shape (river crossing, hilltop, harbour). Moving through a town has places in it, not just a list of buildings.
- **Underground:** cellars, crypts, drains, tunnels between buildings, catacombs, mine workings; some joined to natural caves.

### 3. Interiors with depth

- Multi-level buildings with stairs, balconies, lofts and basements.
- **Secrets that mapping reveals:** sealed or hidden rooms whose existence shows up as gaps on a careful map (a wall too thick, a room missing from a symmetrical plan), and ways into them (a loose panel, a crawlspace, a route from a neighbouring building or from underground).
- Partial collapse that cuts off parts of a building, reachable another way.
- Furnishings appropriate to the room's purpose and the culture's wealth and era.

### 4. Scenes

A **scene** is a small arrangement of things that shows what happened in a place, generated from a history event or from the place's last days: a barricade from the inside, a meal left on the table, tools dropped mid-task, a plague pit, scorch marks and arrowheads after a siege, bones with belongings, a flood line on the walls, a hurried burial, a shrine still kept by someone long gone.

- Scenes are structured facts (what, where, which event, what objects), rendered through slots, never prose in code.
- Each scene is consistent with the history and with any texts about the same event (D07 will tie them together more tightly).
- Expose scenes as storylet placement targets, so Jb can write set pieces into them.

## Content slots introduced

Everything new is a candidate for the D02 attention model, with a salience and a digging layer: most of it should be found by exploring and looking closer, not announced. Natural feature looks and interiors; new structure kinds and room purposes (as variables on the existing slot families, with richer variables, not one slot per kind); street and district descriptions; secret-passage discovery; scene descriptions with variables for each part of the scene.

## Targets (measured by D01)

| Metric | Baseline | Target |
|---|---|---|
| Structure kinds per world | 11 | at least 25 of 35+ |
| Room purposes per world | about 7 | at least 40 |
| Natural feature kinds per world | about 3 | at least 12 |
| Scenes per settlement | 0 | at least 3, plus scenes outside settlements |
| Settlements sharing the same role and layout | most | none on the same seed within the same role and size |
| Explorer novelty | baseline | new kinds of place still being found in hour 5 |

## Tests

- Every feature's placement follows its cause (springs where water meets the surface, caves in soluble rock, etc.).
- Every hidden room is reachable and its existence is inferable from the map (a test bot that maps rooms finds the gap).
- Every scene traces to an event or a place's history.
- All interiors are connected or deliberately sealed, as recorded.
- WebAssembly world generation stays under a few seconds.

## Checklist

- [ ] Geology layer
- [ ] Natural features and caves
- [ ] New structure kinds with interior generators
- [ ] Settlement roles and layouts, streets and districts
- [ ] Underground networks
- [ ] Multi-level interiors, collapse, secrets
- [ ] Scenes from history, as storylet targets
- [ ] Slots and variables
- [ ] Targets met; samples committed with a note
- [ ] Tests listed above
- [ ] LOG.md entry
