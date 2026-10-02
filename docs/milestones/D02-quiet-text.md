# D02 — Quiet text

**Goal:** put the "Say little; let the player dig" principle (`docs/DESIGN.md`) into the engine before the world gets richer. Every response gets an attention budget; detail moves behind actions; facts the player can infer stop being stated. After this milestone, adding more to the world adds more to *find*, not more to *read*.

**Depends on:** D01 (its brevity and depth-on-demand metrics).

**Done when:** the D02 targets are met, and the sample transcripts read as short and intriguing: a player arriving in a town gets a few lines that make them want to look around, not an inventory.

## Why

The opening of a game on the M14 build, as Jb saw it (example variants):

```
You wake in a grassland, with no memory of arriving.

Clear, in the morning.

You stand on flat ground, in grassland. The road runs here.

You stand among ruins in a grassland, in the morning. Here the worn temple, the worn house, the intact house, the ruined storehouse, the damaged cemetery, the worn tomb, the intact tomb, the worn tomb, the intact tomb, the worn tomb, the intact tomb, the intact tomb, the intact tomb, the intact tomb and the well.

In the distance: the town to the south (far) and the town to the south (horizon).

It is cold.

It is spring. Little grows.
```

What's wrong, and none of it is the placeholder wording: every system adds its own line; the scene is restated ("grassland" three times, "in the morning" twice, "You stand" twice); fifteen buildings are listed one by one; two landmarks are indistinguishable; and temperature, season and the land's state are stated instead of shown.

**This opening is a fixture.** After D02, the same moment on the same seed must render at most three facts with no repetition, such as (shape only; the words will be Jb's) a road through long grass, a ruined temple over a crowd of tombs, and the cold felt in the breath. Everything removed must still be reachable by `look around`, `look closer`, `listen` and so on. Commit the before and after to `docs/samples/D02/`.

Also, on the M14 build, arriving in a town on seed 42 prints the weather, the ground, a list of fourteen buildings, the horizon, the season, the state of the land and a creature, every time, and `look` repeats all of it. Each later system appended its line. D03–D11 will add much more to the world; without this milestone, every addition makes each response longer.

## Scope

### 1. The attention model

A single engine component decides what is said, used by every description path (rooms, areas, arrival, travel, examine):

- **Candidates:** everything perceivable here produces candidate facts with a **salience** score: size, contrast, motion, loudness, danger, rarity, relevance to what the player just did, and **novelty** (first time seen; changed since last time).
- **Budget:** each kind of response has a small budget (proposed: arrival 3 facts, `look` 4, room entry 3, travel report 1–2 plus anything that stopped the journey). Only the top candidates are rendered. Everything else stays available on request.
- **Interruptions:** things that are loud, sudden, new or dangerous may break through the budget, and only then.
- **Memory:** the engine remembers what this player has already been told about each place and thing. Repeated looks favour what's changed or not yet mentioned; unchanged things fade to a short reminder or silence.
- **Groups:** many similar things collapse into one fact ("tombs crowd the hillside") with the detail available by looking closer, instead of listing fourteen buildings.

### 2. Digging in

New and extended verbs, each with its own candidates from the same model:

- `look closer` / `examine <thing>` / `search` — more detail on a place or thing, in layers: a second examine finds more than the first.
- `listen`, `smell`, `touch <thing>`, `taste` (sparingly), `look up`, `look down`, `look around` (the wider view, when outdoors).
- `wait` and returning at another time of day or season reveal what changes.
- `check myself` (replacing the status line) — how the body feels, in sensations, with detail on what's wrong.
- The parser accepts natural variants (`listen to the river`, `what's that smell`, `look at the sky`).

Senses are real channels in the simulation: sound has sources and carries by distance and walls; smell has sources (water, smoke, rot, flowers, animals) and drifts with wind; touch reports temperature, wetness, texture.

### 3. Infer, don't state

Replace stated facts with evidence the player can read:

- **Season and time:** no "It is spring." The world shows it: blossom, lambs, meltwater, birdsong at dawn, length of the day, frost in the morning. The `time.status` slot goes; the age of the player is only felt (stiffness, slower recovery) and seen if they check themselves.
- **Regional state:** no "The land is parched. Little grows." Cracked mud, dry stream beds, withered crops, dust.
- **Needs:** no status line. Sensations when a threshold is crossed ("Your mouth is dry"), more detail on `check myself`, and consequences.
- **Weather:** said when it changes or matters, otherwise only on `look up` or when it affects what's seen.

### 4. Slot reshaping

- Replace catch-all slots that list everything (`place.site` with its structure list, `prop.cues`, `region.cues`, `body.status`, `time.status`) with one slot per *fact kind*, each with rich variables, chosen by the attention model. Jb then writes many short pieces that combine, instead of long templates that enumerate.
- Add a **digging** variant of each family (the detail you get when you look closer, listen, etc.).
- The authoring tool's preview shows responses as the attention model would assemble them, so Jb writes for the budget.

### 5. Agents and accessibility

- The JSON protocol keeps the full structured state available (behind the existing flags), so agents and tools aren't limited by the human budget.
- Screen-reader users benefit from short responses; keep the transcript and notebook export complete.

## Targets (measured by D01)

| Metric | Baseline | Target |
|---|---|---|
| Words per arrival (median, example variants) | measure | at most 60 |
| Facts per `look` (median / 95th percentile) | measure | at most 4 / 6 |
| Stated season, regional state or status lines per hour | many | 0 unless asked |
| Details available on demand per place | measure | at least 3× the facts shown on arrival |
| Repeated `look` with nothing changed | full repeat | at most 2 facts |

## Tests

- Budget never exceeded except by flagged interruptions.
- Same state and history give the same selection (determinism).
- A second `look` with no change says less than the first.
- Every fact in the world is reachable by some digging action (nothing exists only in debug output).
- Season can be inferred: each season produces at least three distinct evidence facts in each biome.
- No slot renders the season name, the regional variable names, or need levels except in `check myself` or debug.

## Checklist

- [ ] Attention model: candidates, salience, novelty, budgets, interruptions, memory, grouping
- [ ] Digging verbs and layered examine
- [ ] Sound, smell and touch as simulated channels
- [ ] Season, time, regional state and needs shown by evidence
- [ ] `check myself` replaces the status line
- [ ] Slots reshaped into fact kinds plus digging variants; authoring preview assembles responses
- [ ] JSON protocol keeps full state
- [ ] Targets met; samples committed with a note
- [ ] Tests listed above
- [ ] LOG.md entry
