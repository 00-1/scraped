# Depth roadmap

The first roadmap (`docs/ROADMAP.md`, M01–M14) built every system the design asked for, each at the simplest level that met its spec. This roadmap makes them deep. Its aim, in Jb's words: **the environment should be genuinely interesting and rewarding to explore in its own right**, and the writing should start as background flavour that the player only gradually realises matters (`docs/DESIGN.md`, "Writing is background, at first"). And however rich the world gets, **the text stays short and intriguing; the player digs for detail** (`docs/DESIGN.md`, "Say little; let the player dig").

Each milestone has a spec in `docs/milestones/`. Work through them in order.

## Where the engine is now

Measured on the M14 build (seed 42 unless noted).

| Area | Now | Problem |
|---|---|---|
| Building types | D03: 50 kinds, by each town's role | Was: every town the same handful of temples, houses, tombs and storehouses |
| Interiors | 3–6 rooms joined by compass exits, no geometry | Nothing large or intricate to explore inside; maps can't be checked |
| Room purposes | about 7 | Interiors feel interchangeable |
| Natural features | D03: 36 kinds of feature and old mark, placed by cause | Was: nothing between towns worth walking to |
| Landmark identity | "the mountain", "the town" | Five identical names in one view; maps can't tell them apart |
| Biomes | 12 | Fine for now |
| Items | 20, all practical | Nothing to find for its own sake |
| Mechanisms | 6 | Single-step, local |
| Creatures | 4 archetypes | No ecology |
| History events | 7 kinds | Little for texts and ruins to be evidence of |
| Text kinds | 7; 59% tombs; about 90 sentence shapes per world | Reading is unrewarding even once deciphered |
| Lexicon | 113 concepts | Too small for rich texts or place names |
| Spells (potent texts) | 6 per world; 3 verbs (open, burn, break); 3 properties | The heart of the game is thin |
| Discovery | Scraper placed beside a "pivot" spell; a storylet announces the tool | Points straight at the mechanic, contrary to the design |
| Response length | D02: an attention budget per response (arrival 3 facts, `look` 4); the rest is found by digging | Was: each system appended a line |

## Metrics

Measured by `scraped-lang depth --seeds A-B [--hours H] [--json]` (and the web crate's `depth` call, shown in the bench's Depth tab). World measures read the generated world. Variety, brevity and depth on demand come from the curious explorer's first H hours (24 by default), rendered with the example variants, so they measure the engine, not the prose.

| Metric | Definition |
|---|---|
| `places.structure_kinds` | Distinct kinds of building in the world |
| `places.room_purposes` | Distinct room purposes across all interiors |
| `places.natural_kinds` | Distinct natural features: edges other than roads (river, stream, lake, coast, treeline…) and summit kinds |
| `places.landmarks` | Landmarks: settlements, lone buildings, summits |
| `places.landmarks_unique_share` | Share of landmarks whose rendered name no other landmark in the world shares |
| `places.per_km2` | Buildings and summits per km² of land |
| `things.kinds` | Distinct kinds of thing (features and items) |
| `things.per_structure` | Things per building |
| `things.mechanisms` | Mechanisms (wells, sluices, levers, braziers…) |
| `things.longest_chain` | Steps in the longest mechanism chain (a lever and what it moves is 2) |
| `life.species` | Distinct creature kinds |
| `life.with_signs` | Kinds that leave signs (tracks, nests) to find without seeing them |
| `history.event_kinds`, `history.events` | Kinds of history event, and how many |
| `history.people_with_traces` | Named people met in more than one text |
| `writing.texts`, `writing.genres` | Texts, and genres among them |
| `writing.largest_genre_share` | The commonest genre's share of all texts |
| `writing.sentence_shapes` | Distinct meanings once names and numbers are abstracted away |
| `writing.words_per_text` | Mean words per text |
| `writing.concepts` | Distinct concepts attested in the texts |
| `magic.live_spells`, `magic.claim_types` | Live (released) spells at the start, and distinct kinds of claim among them |
| `magic.places_with_writing_cause` | Buildings within reach of a live spell |
| `magic.strange_without_writing_share` | Share of strange places with no writing cause (natural oddities) |
| `magic.concepts_with_powers`, `magic.properties`, `magic.live_properties` | Concepts a world's languages name that have a power; qualities the power table can push; qualities live spells push (D09) |
| `magic.conditional_spells`, `magic.settlements_without_spell`, `magic.regions_with_large_spell_share` | Live spells with a condition; settlements no live spell reaches; share of regions holding a spell reaching 1.5 km or more (D09) |
| `variety.combinations` | Distinct (slot, variable combination) pairs the explorer met; per slot family in the JSON |
| `brevity.<kind>.words_*`, `brevity.<kind>.facts_*` | Median and 95th percentile of words, and of facts, per response, for `look`, `arrival` (into a new place), `travel` and `other`. A fact is a rendered slot, outside the parser's replies, whose text was in the response; a list ("Ways out: …") is one fact, and names said inside another fact are part of it (since D04) |
| `depth_on_demand.per_place` | At each arrival somewhere (D02 on): how many facts digging there could turn up (median): what `look closer` and `look around` would weigh, sounds and smells, the ground, and writing to read. Until D02 it counted things to examine in each room, plus one for each with writing |
| `depth_on_demand.per_fact_shown` | At each arrival, those facts for each fact shown on arriving (median) |
| `play.novel_per_hour` | New things perceived per hour: first renders of a (slot, kind) pair |
| `play.minutes_between_new_kinds` | Median minutes between new kinds of thing (first word of the kind, per slot family) |

### Baseline (D01, seeds 1–10, explorer 24 h)

| Area | Baseline |
|---|---|
| Places | 8.7 building kinds; 22.2 room purposes; 5.8 natural kinds; 29.3 landmarks, 85% uniquely named; 0.08 places per km² |
| Things | 37.2 kinds; 6.6 per building; 57 mechanisms; longest chain 2 |
| Life | 4 species; none leave signs |
| History | 6.2 event kinds, 215 events; 72 people met in more than one text |
| Writing | 262 texts in 8 genres; the largest genre is 52%; 103 sentence shapes; 5.3 words per text; 70 concepts |
| Magic | 9.5 live spells of 5.4 kinds; 65 buildings within a spell's reach; no natural oddities |
| Variety | 240 slot × variable combinations met in 24 hours |
| Brevity (words, median / p95) | look 88 / 117; arrival 30 / 109; travel 91 / 127 |
| Brevity (facts, median / p95) | look 29 / 45; arrival 14 / 41; travel 26 / 49 |
| Depth on demand | 3.4 details per room; senses 0 |
| Novelty | 3.6 new things per hour; a new kind of thing every 1.1 minutes (median) early on |

### After D02 (seeds 1–10, explorer 24 h)

A fact is now counted once even when a name is rendered inside it.

| Metric | D01 | D02 | Target |
|---|---|---|---|
| Words per arrival (median / p95) | 30 / 109 | 17 / 21 | at most 60 |
| Facts per arrival (median / p95) | 14 / 41 | 3 / 4.4 | 3 |
| Words per `look` (median / p95) | 88 / 117 | 12 / 14 | |
| Facts per `look` (median / p95) | 29 / 45 | 2 / 2.3 | at most 4 / 6 |
| Facts per journey (median / p95) | 26 / 49 | 4.2 / 5.1 | report, what stopped it, 2 more |
| Stated season, regional state or status lines | many | none (tested) | 0 unless asked |
| Facts digging could find per arrival | 3.4 per room | 13.1, 4× those shown | at least 3× |
| A repeated `look` with nothing changed | full repeat | at most 2 facts (tested) | at most 2 |

### After D03 (seeds 1–10, explorer 24 h)

| Metric | Before | D03 | Target |
|---|---|---|---|
| Building kinds per world | 8.7 (of 11) | 38.2 (of 50) | at least 25 of 35+ |
| Natural feature kinds per world | 5.8 | 22.4 | at least 12 |
| Scenes per settlement (median / least) | 0 | 4 / 3, plus 18.8 outside towns | at least 3, plus some outside |
| Towns sharing a role, size and layout | most | 0 | none |
| Town roles per world | 1 | 5.7 of 8 | |
| New kinds of thing in the fifth hour | – | 6.0 | still finding new kinds in hour 5 |
| Words per arrival (median) | 17 | 17 | at most 60 (D02) |
| WebAssembly world generation | – | under 0.1 s | under a few seconds |

New metrics: `places.scenes_per_settlement(_min)`, `places.scenes_outside`, `places.towns_sharing_layout`, `places.town_roles`, `play.new_kinds_hour5`. `places.natural_kinds` now counts natural features (not old marks), and `places.per_km2` counts features.

### After S01 (seeds 1–10)

The explorer was rewritten to play like a curious person (S01), so earlier numbers were re-checked with it (explorer 24 h unless said).

| Metric | D03 (old explorer) | S01 | Target |
|---|---|---|---|
| Digging verbs (`look closer`, `look around`, `listen`, `smell`, `look up`/`down`, `touch`, group looks), share of commands, 10 h | about 0 | 0.22 (at least 0.20 on 9 of 10) | at least 0.20 |
| Natural features visited in 10 h (within 300 m) | 0–2 | 2.6 (5 on 1 of 10) | at least 5 on most seeds: **not met** |
| Buildings entered / towns in 10 h | 15–20 / 1 | 13 / 1.5 | |
| Words per arrival (median / p95) | 17 / – | 20 / 32 | at most 60 (D02) |
| Facts per arrival (median / p95) | 3 / 4.4 | 3.7 / 8.1 | 3 (D02) |
| Facts per `look` (median / p95) | 2 / 2.3 | 2 / 6.3 | at most 4 / 6 (D02) |
| Facts digging could find per arrival | 13.1, 4× those shown | 12.1, 3.1× | at least 3× (D02) |
| New kinds of thing in the fifth hour | 6.0 | 5.9 | still finding (D03) |
| Words per `read` (a glance) | 100s (every sign) | 22 | |
| Words per page of `read closely`: none heard / commonest 12 heard | – | 174 / 113 | |
| Signs with an impression of their own (least over eras) | – | 1.00 (archaeologist: about one alike pair per script) | all, on gentle and standard |
| Sign sounds a scraping scholar hears in 10 h (scraper in hand) | – | 33 | |
| Explorer survives three days | 8 of 10 | 8 of 10 | 8 of 10 |
| Scholar reads the deepest text in 90 days | 5 of 10 (3 during D04) | 5 of 10 | 5 of 10 (8 asked) |

Why features fall short: the explorer sees a town first (it must find a firesteel and a cloak), its body sends it back to shelter from the afternoon or when cold, it sleeps three or four hours in the ten, and features lie 4 km or more apart, so most journeys end at towns or landmarks. Seeds with a mild start and features near the road reach 4–5. Facts per arrival rose with D04's move reports inside buildings (a fact before each room); D04's "brevity inside great interiors" item is to bring that back to 3.

New metrics: `play.verbs.<family>_share` (look, dig, examine, read, move, handle, writing, body, other), `play.visited.features`, `play.visited.feature_kinds`, `play.visited.buildings`, `play.visited.towns`, `reading.words_per_read`, `reading.words_per_page`, `reading.words_per_page_heard`, `reading.unique_impression_share`, `reading.heard_scholar_10h`.

### After D04 (seeds 1–10, explorer 24 h)

| Metric | Before | D04 | Target |
|---|---|---|---|
| Spaces in the largest interior | about 6 | 492 | at least 400 |
| Great interiors (200+ spaces) per world | 0 | 9.5, a cave system in every world | at least 4, with a cave system |
| Levels in the deepest interior | 2 | 7.4 | at least 6 |
| Independent loops in a great interior (least) | 0 | 26.9 | at least 15 |
| Hidden spaces per great interior (least) | 0 | 6, each with a visible twin | at least 5, inferable |
| Mapper bot plan error / topology | – | 0 / exact | within 10% / exact |
| Facts per arrival (median / p95) | 3 / 4.4 | 3 / 3.4 | 3 (D02) |
| Facts per `look` (median / p95) | 2 / 2.3 | 2 / 6 | at most 4 / 6 (D02) |
| Time to walk every reachable space of the largest (mapper, never resting) | minutes | 34 h | several in-game days |

The mapper walks without sleeping, eating or getting lost, so 34 hours of walking is several days of play. Since D04 a fact count treats a list ("Ways out: …") as one fact, as the attention model does.

### After D05 (seeds 1–10)

| Metric | Before | D05 | Target |
|---|---|---|---|
| Kinds of thing per world (object kinds alone) | 66 | 168.5 (103) | at least 120 |
| Things per structure (mean) | 30 | 51 | at least 6 |
| Longest mechanism chain | 2 | 4 (works) | at least 4 on most seeds |
| Non-writing puzzles per world, with payoffs | 0 | 32 (locked boxes with contents, locked doors, caches, buried caches, works, calendar doors) | at least 8 |
| Old maps per world | 0 | 4 | at least 2 |
| Facts per arrival (median / p95) | 3 / 3.4 | 3 / 3.3 | 3 (D02) |

New metrics: `things.puzzles`, `things.maps`, `things.object_kinds`; `things.longest_chain` counts works.

### After D06 (seeds 1–10)

| Metric | Before | D06 | Target |
|---|---|---|---|
| Species per world (animals / plants) | 4 archetypes | 59.8 / 48.8 | at least 40 / 40 |
| Animals known by signs before being seen (explorer, 24 h; insects aside) | 0 | 0.84 of 3.4 met | most |
| Weather consequences per 30-day month that change routes or places | about 0 | 114 (flooded fords, snow-closed heights, windthrow, low lakes; counted per 5 km area and week) | at least 3 |
| Natural wonders per world | 0 | 9.5 (at least 5 on every seed tested) | at least 5 |
| Strange places with no writing cause | 0% | 49% | at least 25% |
| Facts per arrival (median / p95) | 3 / 3.3 | 3 / 3.3 | 3 (D02) |
| Facts per `look` (median / p95) | 2 / 6 | 2 / 6 | at most 4 / 6 (D02) |

Strange places are now counted as sites (each live spell, each natural wonder) rather than every structure within a spell's reach. The explorer meets few animals in a day (3.4) because it spends most of its time in towns and buildings. New metrics: `life.animals`, `life.plants`, `life.met`, `life.known_by_signs_first`, `weather.consequences_per_month`, `magic.natural_wonders`.

### After D07 (seeds 1–10)

| Metric | Before | D07 | Target |
|---|---|---|---|
| Concepts in a world's lexicon | 110 | 566–603 (core, five culture fields, species, derived words, compounds) | 400–700 |
| Constructions (coordination, eight subordinators, relatives, reported and quoted speech, comparison, quantifiers, four moods, aspect, ordinals, dates) | 0 | all, each rendered and parsed back | every one regular and round-tripping |
| Derivations / compound order | none | six affixes per seed; head-first or head-last | regular per seed |
| Names that mean something (people, towns) | 0% | 100% (`name_meaning`; place names from the site) | all |

Fairness attestation per construction in readable texts waits for D08's texts.

### After D08 (seeds 1–10)

| Metric | Before | D08 (mean; worst seed) | Target |
|---|---|---|---|
| Event kinds | 7 | 31.4; 29 | at least 25 |
| Text genres per world | 7 | 27.3; 26 | at least 20 |
| Largest genre's share | 59% (tombs) | 17%; 19% (epitaphs) | at most 20% |
| Distinct sentence shapes per world (each sentence of a text) | about 90 | 668; 585 | at least 600 (not met on one small world) |
| Mean words per text | about 6 | 17.9; 16.5 | at least 15 |
| Story arcs per world, told in 3+ places | 0 | 41.8 arcs, 27.1 in 3+ places; 21 | at least 10 in 3+ places |
| People named in writing in 3+ places | few | 85.9; 73 | at least 30 |

About 650 texts a world. Sentence shapes fall short on the smallest worlds; two attempts at more varied wording raised the mean from 619 to 668. New metrics: `story.arcs`, `story.arcs_in_3_places`, `history.people_in_3_places`; `writing.sentence_shapes` now counts each sentence of a text.

### After D09 (seeds 1–10)

| Metric | Before | D09 (mean; worst seed) | Target |
|---|---|---|---|
| Concepts with powers | 3 verbs | 280.8; 269 | at least 120 |
| Properties spells can change | 3 | 15 (14 live in a typical world) | at least 15 |
| Live spells per world | 9.5 | 226; 135 | at least 150 (not met on the smallest world) |
| Distinct claim types per world | 5.4 | 56.4; 46 | at least 60 (not met) |
| Conditional (dormant) spells per world | 0 | 55.9; 23 | at least 20 |
| Settlements with no live spell | most | 0; 0 | none |
| Regions holding a large spell (1.5 km or more) | — | 11%; 6% | each region has one (not met) |
| Largest genre's share (D08) | 17% | 18%; 21% | at most 20% |
| Mean words per text (D08) | 17.9 | 13.3 | at least 15 (no longer met) |

Kinds stay short of 60 after three attempts at more varied everyday spells: a world's buildings and the culture's choice of powers (where "keep" binds rather than keeps, say) bound the combinations, and doubling the spells to reach more kinds pushed spells past half of all writing. Most regions have no building to carry a large spell. Spells are short (a clause), so words per text fell. Arrivals and looks stay as brief as before (arrival median 19.3 words, 3 facts). New metrics: `magic.concepts_with_powers`, `magic.properties`, `magic.live_properties`, `magic.conditional_spells`, `magic.settlements_without_spell`, `magic.regions_with_large_spell_share`.

### After D10 (seeds 1–10)

| Metric | D09 | D10 | Target |
|---|---|---|---|
| Responses in the first hour that bring writing forward (scraping, writing, a great inscription's force) | – | 0 on every seed | 0 |
| Explorer's first release (72 h) | – | 4.5–7.7 h on 6 seeds; 25 h on one; never on 3 | 3–10 h on most seeds |
| Scholar's first release (90 days) | – | 9.6–109 h | |
| Non-writing goals with payoffs per world | – | 12.9 | at least 10 |
| Seeds where the rim is reachable on foot | – | 10 of 10 | all |

The explorer's first release always comes from cleaning a surface with a knife ("clean clay wall"), hours in; never from a deliberate scrape. Realisation is spread: the explorer from 4.5 h to never, the scholar from 10 h to four and a half days. New metrics: `pacing.first_release_hours` (24 if never), `pacing.pointers_first_hour`, `goals.non_writing`, `goals.rim_reachable`. A great site's `great.site` is atmosphere, not a pointer, once its example no longer names writing.

### After D11 (seeds 1–10)

| Metric | D10 | D11 | Target |
|---|---|---|---|
| Sealed places per world (opened only by writing) | 0 | 2.9 | a late game in each world |
| Worlds whose chain ends at a great inscription | – | 5 of 10 | |
| Worlds whose sealed chain the fairness check can walk | – | 20 of 20 (seeds 1–20) | all |
| Scholar opens at least one sealed place (90 days) | – | 7 of 10 | |
| Scholar reads the deepest text (90 days) | 3 of 10 | 4 of 10 | |

The spec sets no numbers; these say how far the late game reaches. Each sealed place is visible and reachable to its door without writing (tests); the scholar opens one from attested words alone, by the grammar it was given. New bot fields: `sealed_met`, `sealed_opened`.

### The bots (D01)

`scraped-lang bots --seeds A-B [--bot explorer|scholar] [--hours H]`.

- **Curious explorer** (`crates/game/src/bots.rs`): survives three days on 6 of seeds 1–10 after D10 (since D10 it cleans what it finds grimy; seed 6 now runs out of food on the land, fires burning out, and dies walking to shelter), 7 after D09 (on seeds 1, 3 and 4 it shelters in houses an old cast keeps cold and goes in and out of them until the cold takes it; two attempts to have it give up on a cold shelter made things worse), 8 after D07 (worlds changed with the new lexicon; 9 after D06, 8 after D03, S01 and D05; 16 of 1–20 before D03). Since D07 it goes out by day to gather wood when cold under a roof with no fire. Since D06 it fishes when hungry by water (warm, by day), won't forage while shivering, waits outdoors now and then, and looks up once a night. Since S01 it plays like a curious person: it digs in with a few senses wherever it arrives (not every sense, not everywhere), follows the features, sounds and smells a response turns up, looks at groups of alike buildings, stops examining a kind of thing once that stops giving anything, reads closely now and then and sometimes examines or traces a sign. It sees four buildings of a town once it has a firesteel and a cloak, then goes out to the land; it leaves any building after 45 minutes and won't drop down holes. Since D03 it goes into kinds of building it has seen least first, makes for towns from noon, and shelters indoors from the evening or when cold. Deaths are cold: worlds whose start lies under a cold spell, nights caught in the open.
- **Scholar** (grammar spoilers, no map; body kept well): in 90 days reads the deepest text on 4 of seeds 1–10 after D08 (6 after D07, 5 after D06): there is twice as much to read and it reads it all, skimming only past the third page, so it finds the lens later and a great inscription on 9 (6). The spec asks for 8. What stops it is in the game, not the bot:
  - **Held doors.** Old writing holds whole towns' doors shut. The only counter is to write "open" with a passage word (door, gate, tomb, box), and those words are met in fewer than two texts even after 100–170 texts read, so the understanding gate never lets the scholar write them.
  - **Scarce light.** Torches burn an hour, lamps four. Deep rooms are often dark, and the scholar must come back later with fuel.
  - **Distance.** The first lens is placed in roughly the 90th-percentile building by distance from the start, so reaching it means exploring most of the world.

  D07 (a larger lexicon) and D09 (the magic, deepened) should lift this; the test holds at 4 of 10.

  After D10 it reads the deepest text on 3 of 10 (seeds 5, 8, 9). Worlds changed (tools, grime, latent spells), so seeds won and lost are not like for like with D09 (now 2, 4 lost; 5 kept). Two bot bugs were fixed along the way: it walked into a door hidden or held shut for ever (it now notes a blocked way before anything else), and it tried every bearing when snowed in without letting time pass. Two attempts; not met.

  After D09 it reads the deepest text on 5 of 10. Spells now hold doors, darken rooms and leave stacks of layers everywhere: the writing tools no longer lie where writing keeps rooms dark or doors shut, and with a stronger lens the scholar goes back first to the deepest stack it has seen.

## Milestones

| # | Milestone | Delivers |
|---|---|---|
| D01 | [Instruments](milestones/D01-instruments.md) | Depth and brevity metrics, a curious-explorer bot and a scholar bot that reach the late game, sample transcripts, a decisions list, distinct landmarks |
| D02 | [Quiet text](milestones/D02-quiet-text.md) | An attention budget for every response, senses and digging as actions, facts shown by evidence instead of stated, slots reshaped into short combinable pieces |
| D03 | [Places with character](milestones/D03-places.md) | Natural features, many more building and settlement types, deeper interiors with secrets, scenes that tell what happened |
| S01 | [Course corrections](milestones/S01-course-corrections.md) | From Jb's review of D01–D03: an explorer bot that plays like a person, arrivals that describe the place, reading in layers (the whole text, then each sign's shape; exact strokes only by tracing, a task in itself), signs learned by the sounds they make when scraped, no arbitrary labels, and bug fixes |
| D04 | [Great interiors](milestones/D04-great-interiors.md) | Vast, intricate buildings and cave systems on a real spatial model: hundreds of spaces, many levels, loops, shortcuts, secrets; architecture that grew through history; navigation and mapping as a pleasure |
| D05 | [Things and mechanisms](milestones/D05-things-and-mechanisms.md) | A rich object model, keys and caches, old maps, multi-step machines, and non-linguistic puzzles: symbols, measures, calendars |
| D06 | [A living world](milestones/D06-living-world.md) | Ecology with signs and rhythms, weather and seasons with consequences, the sky, natural phenomena |
| D07 | [A language for long texts](milestones/D07-language.md) | Clause combining, subordinate and relative clauses, moods and aspects, derivation and compounds, a cultural lexicon of hundreds of words, meaningful place names |
| S02 | [Sign impressions as whole shapes](milestones/S02-sign-impressions.md) | Small steer after D07: signs read as shapes (proportions, curves, symmetry, loops, a standout part, a resemblance), never as strokes |
| D08 | [History and what the writing says](milestones/D08-history-and-texts.md) | Deeper history with people, institutions and projects; twenty-plus text genres; story arcs told across many sites, matching physical evidence |
| C01 | [Shared play and versions](milestones/C01-shared-play-and-versions.md) | After D08: a hardened public player build, snapshot saves, major/minor versioning enforced in CI, shared worlds in a private repo with table talk, and a fair-play note for agents |
| D09 | [The magic, deepened](milestones/D09-magic.md) | A broad concept-to-power table, targeted and conditional spells, writing as the old civilisation's technology, a world saturated with old spells |
| S03 | [Before Jb plays](milestones/S03-before-jb-plays.md) | Small steer after D09: tag the first release so Jb can play, and fix wording bugs (plurals, exact counts in reading, related signs with no findable base) |
| D10 | [The slow realisation](milestones/D10-slow-realisation.md) | Ordinary tools, accidental discovery, evidence to notice, rewarding play without writing, a non-writing way to leave |
| D11 | [Problems only writing solves](milestones/D11-writing-payoff.md) | Late-game places and composition puzzles that reward real fluency, layered constraints, the great inscriptions as destinations |
| D12 | [Integration and tuning](milestones/D12-integration.md) | All clients and tools updated, fairness re-checked, targets met together, performance, a playtest pack and content plan for Jb |

Order of priority is deliberate: first the instruments and quiet text (D01–D02), so every later addition is measured and adds to what can be found rather than to what is read; then the world, outside and in (D03–D06); then what it says (D07–D08); then the magic and how it's discovered (D09–D10); then the late-game payoff (D11).

## Rules for every depth milestone

These add to `CLAUDE.md`.

1. **Depth, not checkboxes.** Each milestone states numeric targets measured by the D01 instruments. A milestone is done when the targets are met *and* the sample transcripts read as interesting, not when the checklist is ticked.
2. **Sample transcripts.** Each milestone commits spoiler-free transcripts from the curious-explorer bot on three seeds to `docs/samples/<milestone>/`, so Jb can read what play now feels like. Write a short honest note beside them: what got better, what still reads flat.
3. **Generated, not listed.** Variety comes from systems that combine (history × geography × culture × era), not from long hand-typed lists. Lists of kinds are fine as vocabulary; the interest must come from how they combine and from causes in history.
4. **Rich variables for content.** Every new thing the player can perceive exposes enough variables (material, size, condition, maker, era, distinguishing feature, what happened here) that Jb's templates can be specific. A slot whose only variable is `{kind}` produces the flat text this roadmap exists to fix.
5. **Everything has a cause.** If something is in the world, history or physics put it there, and the evidence of why is findable.
6. **Writing stays in the background.** No new system may point the player at the writing mechanic. Reviews check this.
7. **Say little; let the player dig.** Every new thing the player can perceive goes through the D02 attention model, with a salience and a digging layer. Richer worlds must not mean longer responses: brevity targets hold in every milestone after D02.
8. **The game is not your notebook.** No new system may keep lists or records for the player (no glossaries, journals, clue lists, sign lists). Any exception must be argued for and recorded in `docs/DECISIONS.md` (see `docs/DESIGN.md`).
9. **Keep what works.** Determinism, meaning-first language, slots instead of prose, browser and Android builds, fairness and tests all stay green throughout.

## Design gates

Each spec lists its gates with a proposed default, as before.

| Gate | Needed by | Proposed default |
|---|---|---|
| Is anyone else here? | D06 | No living people. Traces of earlier explorers (camps, belongings, notes) can appear as storylets in Jb's words |
| Can a player leave without writing? | D10 | Yes, by a long, hard physical journey to the world's edge, found through exploration. Writing offers other, stranger ways out |
| How common is old magic? | D09 | Every settlement has several small live spells, most places between have a few, and each region has at least one large one |
| Is writing the old civilisation's technology? | D09 | Yes: everyday writing kept larders cold, lamps lit, mills turning. The ruins of that infrastructure are much of the world's strangeness |
| How long before a typical player realises? | D10 | Many hours. Target: the naive bot has its first accidental release after 3–10 hours of play, and nothing in the first hour points at writing |
