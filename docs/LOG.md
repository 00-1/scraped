# Work log

## 2026-10-02 — Milestone 1 language engine slice

**Done**
- Cargo workspace: `crates/lang` (pure library, builds for `wasm32-unknown-unknown`) and `crates/cli` (`scraped-lang` binary).
- Seeded RNG over ChaCha8 with one stream per stage and per inscription. Range reduction is our own, using only raw `u32`s, so output doesn't depend on pointer width or `rand` internals.
- Phonology: 12–22 consonants from a weighted pool with implicational rules (no /b/ without /p/), 3–7 vowels from attested systems, syllable template (onset 0–2, coda 0–1), coda sets, stop+liquid clusters. Romanisation is checked to be uniquely decodable (Sardinas–Patterson), so every romanised word splits into sounds exactly one way.
- 99 concepts in `crates/lang/data/concepts.toml`, with semantic tags so templates pick sensible words. Roots keep an edit distance of at least 2 from each other, and **no inflected form of any root or name equals any other**, so every surface word has exactly one analysis.
- Morphology: plural; subject (unmarked), object, genitive and dative cases; past; negation. One affix each, prefix or suffix per word class per seed, number/tense nearest the root.
- Syntax: SOV/SVO/VSO, modifier order, genitive order (correlated with head direction).
- Meaning representation (`meaning.rs`), deterministic renderer, interlinear gloss, placeholder English translation.
- Inscriptions: tomb, ledger (list or "X brought N goods"), warning/command, dedication. A recurring cast of 24 named people with fixed relatives and titles appears across texts.
- CLI: `corpus [--count] [--json] [--spoil]`, `grammar --spoil [--json]`.
- Tests: determinism, prefix-stability of corpora, cross-seed variety, phonotactics of every form and corpus word, collisions, gloss ↔ meaning round trip, surface → unique analysis, snapshots for seeds 1/42/9001.

**Open questions for Jb** (each also marked `DESIGN-Q:` in code)
1. **Four cases, not 2–3.** "Made this for X" needed a dative. The alternative is an adposition.
2. **Commands** are a bare verb with no subject; there is no imperative affix.
3. **Word order extras:** numerals and "this" go where adjectives go; datives and adverbs follow head direction; appositions ("child of X", titles) always follow the name.
4. **Presentation:** words are space-separated and names are unmarked. Scriptio continua or a name marker would change difficulty a lot.
5. **Romanisation** falls back to non-ASCII letters (ŋ, ñ, š, ĥ, ë…) to stay unambiguous. Is ASCII with separators preferred?
6. **Cast size** of 24: names recur enough to cross-reference, but the same tomb can occasionally appear twice.
7. Numbers stop at ten; there is no numeral system yet.
8. `grammar` without `--spoil` refuses rather than showing a partial sheet.

**Next**
- Jb's notebook decipherment session (deliverable checks in the milestone doc).

## 2026-10-02 — Roadmap to a finished game

**Done**
- Added `docs/ROADMAP.md` and milestone specs M02–M14 in `docs/milestones/`; moved the M01 spec there.
- The finished game is defined as a complete engine plus Jb's complete content pack, joined by a slot registry and an authoring tool (M03, M13). A release gate (M14) refuses to ship any placeholder text.
- Design gates listed per milestone, each with a proposed default.
- Cost estimates calibrated on M01 (about $6).

**Next**
- Jb: decide on the M01 open questions above (or accept the defaults in M02), then start M02.

## 2026-10-02 — M02 Language depth

**Done**
- **Script** (`script.rs`): alphabet, abjad or syllabary per seed. Glyphs are sets of marks (stroke, turn, spot) from a 9-stroke vocabulary. Per-seed script logic makes voiced sounds, fricatives and nasals add a shared mark to their partner's glyph, so readers can discover it. Writing direction is left-to-right, right-to-left or boustrophedon. Every glyph is checked to be distinct. A debug SVG renderer and mechanical debug descriptions; player-facing descriptions are left for M03 content slots.
- **Numerals** (`numerals.rs`): base 10, 12, 20 or 5+10; biggest-first or smallest-first; optional bare powers ("hundred" or "one hundred"). An "and" linker is added automatically when a system would otherwise be ambiguous (for example, small-first base 5+10 makes 16 and 60 identical). All of 0–999 round-trip. Numeral signs are additive or positional. Ledgers now carry exact totals.
- **Eras** (`history.rs`, `Language::at_era`): 3 eras by default (1–5). Each era applies 2–3 ordered, exceptionless sound changes from a catalogue: mergers, vowel shifts, intervocalic lenition, palatalisation, cluster loss and final-vowel loss. Each rule also updates the era's phonotactics. Rules never repeat or undo an earlier one, and spelling is shared across eras. When change makes words collide, one gets a new root; when affixes collide, one erodes into a separate particle word. A few words are also replaced by chance. The script simplifies some glyphs and adds glyphs for new sounds.
- **Registers**: everyday (tombs, ledgers, warnings, dedications, plus new labels and letters) and potent (a fixed opening and closing word, and a particle before the verb). Potent meaning is left to M08.
- **Difficulty dials** (`difficulty.rs`): word separation (spaces, dividers, none), name determinative, forced script kind, era count, fused morphology. Dials change presentation only; tests check that they don't reshuffle the words.
- **CLI**: `script --spoil`, `eras --spoil`, `corpus --era E --glyphs`, `grammar --era E`, and flags for every dial.
- **Bench** (`tools/bench`, built by `tools/bench/build.sh`): eras side by side, glyph view, script table with drawings, all dials. It sits outside the workspace until M03's `web` crate replaces it.
- **Tests**: phonotactics in every era; every word and affix in era *n* is exactly the rules applied to era *n−1*; every surface word in every era has one analysis; glyphs are distinct; numerals and signs round-trip; ledger totals add up; dials leave words alone; snapshots for seeds 1, 42 and 9001 in all eras (grammar, script, corpus, spoilers, glyph text). The WebAssembly build gives byte-identical output to native.

**Open questions for Jb** (each also marked `DESIGN-Q:` in code)
1. **Proposed defaults kept** from the roadmap gate: the M01 answers stay as built, and separation and name marking are now dials. Please confirm or change.
2. **Abjad** writes word-initial vowels with one carrier sign. **Boustrophedon** doesn't mirror glyphs on reversed lines.
3. **Numeral signs** exist in every script, but inscriptions write numbers as words. Should ledgers use signs?
4. **Erosion and replacement rates**: one affix erodes by chance in about one era in five, and about one word in 25 is replaced per era (more when sound change forces it). Tune?
5. **Potent claims** use open, burn and break with the thing as subject ("let the gate not open"). The potent particle always sits right before the verb.
6. **Glyph line width** is fixed at 16 until surfaces have real sizes.
7. **Same meanings across eras**: inscription *n* means the same thing in every era of a corpus, which helps comparing eras in the bench. In the game, M04 decides what is actually written where.

**Content slots added:** none yet. Glyph description phrasing becomes the first slot family in M03, and the engine already exposes the structured glyphs it needs.

**Usage:** not measured from inside the session.

**Next:** M03, content system and authoring tool v1.

## 2026-10-02 — Decisions from Jb

- **All proposed defaults accepted for now**: the M01 and M02 open questions, and the design-gate defaults in `docs/ROADMAP.md`. Code keeps its `DESIGN-Q:` markers so they can be revisited.
- **Unattended work**: carry on through the milestones without pausing at the end of each; keep the browser bench updated, and add the game to it once it exists.
- The bench runs as WebAssembly in the claude.ai viewer (confirmed by Jb on build e31d1f0); it now shows its build commit and age.

## 2026-10-02 — M03 Content system and authoring tool v1

**Done**
- **`content` crate**: slot registry (`SlotDef` with descriptions, typed variables, release requirements, samplers from real seeds); TOML pack format (`[[variant]]` blocks, one file per family) with a canonical writer and a version hash; template language (variables, inline choices, chained helpers, slot calls, `[if]`/`[else]`/`[end]`, reserved `[damaged]`, language hooks); deterministic renderer that avoids immediate repeats; loud `⟦slot: …⟧` placeholders; lint (syntax, unknown variables/helpers/slots, type and impossible-value checks, unreachable conditions, too few variants, too long, near duplicates) and coverage; release check.
- **First slots**: `glyph.stroke` and `glyph.describe`, with example variants in `content/glyphs.toml` and a `content/README.md` for Jb.
- **Language hooks**: `{lang.word gate}`, `{lang.text x}`, `{lang.glyphs gate}`.
- **CLI**: `scraped-lang content lint | coverage | preview SLOT | registry | release-check`.
- **`web` crate**: one WebAssembly module with a JSON interface (`bench`, `registry`, `parse`, `write`, `lint`, `lint_variant`, `preview`). The bench now runs on it.
- **Authoring tool** (`tools/author`): slot browser with status, slot page (brief, variables with example values, requirements), editor with highlighting and lint as you type, live preview against any seed (spoilers show the glyph being described), save to the repo's `content/` folder (Chrome/Edge), import of files or zip, export zip, copy file, drafts kept in the browser.
- **CI** builds both pages and runs a headless smoke test of the authoring tool; a release-check job is allowed to fail until M14. A Pages workflow publishes both tools.

**Open questions for Jb**
1. **Pages** needs a one-time switch: repository Settings → Pages → Source: "GitHub Actions".
2. Glyph descriptions are two slots (one stroke; the whole glyph). Is that the right granularity for your writing?
3. `{lang.glyphs …}` lists glyph table numbers until display modes exist (M05).

**Content slots added:** `glyph.stroke`, `glyph.describe`.

## 2026-10-02 — M04 World generation

**Done**
- **`world` crate**: 160×160 cells of 300 m. Island or basin per seed; value-noise terrain with light erosion and fine relief; temperature by latitude and altitude; moisture carried by a prevailing wind with rain shadows; twelve biomes.
- **Water**: priority-flood drainage (every cell drains strictly downhill to the edge), rain accumulation, rivers, lakes in deep hollows, marsh in shallow ones, fords at gentle narrow stretches.
- **History** over the language's eras: factions and schisms; settlements sited by water, defence and land; dynasties named in each era's language; notable people with kinship; foundings, migrations, wars, plagues, famines, abandonments, deaths; **writing events** with a potent claim and an intended effect; one **root event** in era 0 tied to the world's trajectory. Every event records place, era, actors, cause and the evidence it should leave. Roads by least-effort routing.
- **Structures** placed by history (temples, houses, storehouses, cemeteries, archives, ruler tombs, walls and towers after wars, mines, bridges at unfordable crossings, waystations), each with an interior from authored-shape generators (house, temple, archive, tomb, tower…): rooms with purposes, exits and inscribable features with materials.
- **Texts** placed as evidence, on the least-written suitable surface, rendered in their era's language: dedications (a king list builds up on a capital's temple), epitaphs, ledgers with totals (scarce in famines), letters, warnings, labels, milestones, and potent inscriptions, with the root text in the capital's archive vault.
- **Present state**: decay by age, damp, material and abandonment; collapsed rooms, closed and blocked passages; ruins as traces; lingering intended effects recorded.
- **Debug views** (spoilers): `scraped-lang world … map | history | site | json`, PNG map, and a World tab in the bench (map, clickable sites, timeline).
- **Tests**: determinism (fingerprints for three seeds, identical in debug and release; the browser build reproduces the same histories), rivers downhill to the edge, every structure reachable from another, every text in its era's language, evidence for every event, root text present, bounds. A world takes about 0.1 s to generate in the browser.

**Open questions for Jb** (marked `DESIGN-Q:` in code)
1. **World size**: 48 km across rather than "a week's walk". Bigger costs browser time; revisit with movement (M06).
2. **Claims and the root event** use the three potent verbs M02 has (open, burn, break); M08 brings the full concept-to-property table.
3. **History density**: 5–8 first towns, 8–14 events per era, 3–6 notable people per town per era. Tune by feel once the world is walkable.
4. Rivers on very smooth slopes can still run as straight parallel lines; a better erosion pass can come later.

**Content slots added:** none (biomes, structure kinds and room purposes are data ids; their descriptions become slots in M05–M06).

## 2026-10-02 — M05 Game core and text interface

**Done**
- **`game` crate**: game state (place, inventory, moved things, time in minutes, "it", reading position, glyph labels, door states), the pipeline parse → resolve → simulate → describe, and a command log that drives save and replay. Play starts in the most-written living town that is not the capital.
- **Parser**: verbs and synonyms as data (`crates/game/data/verbs.toml`); noun phrases match the words of each thing's *displayed* name, so whatever your content calls a thing is what the player types; ordinals ("second tomb", "tomb 2"), "it", and "Which …?" questions answered by the next line.
- **Interiors**: walk between the town's buildings and their rooms by exit or by name; doors open, close and stick; collapsed rooms and buried buildings block the way.
- **Reading**: texts shown glyph by glyph through your glyph descriptions, paged (`more`); `define 3 as ka` labels a glyph and from then on it reads as your label. Labels are never checked against the truth.
- **Description slots**: every line the game says is a slot (44 now), each with an example variant; a test scans `game` and `play` for stray string literals.
- **Save/load/replay**: seed + pack version + command log; loading replays it and warns if the content changed.
- **`play` crate**: the `scraped` terminal client (word-wrapping, `save`, `load`, `transcript on|off`, `quit`) and the JSON-lines agent protocol (`--json`; `text` is exactly what a human sees, `state` summarises place, things, inventory, exits; ground truth only with `--spoil`).
- **Browser**: the bench has a **Play** tab (first tab) running the same game core, with save and load in the browser.
- **Tests**: parser verbs, synonyms, ordinals, pronouns, ambiguity; replay determinism; save/load round trip; protocol lines are JSON whose `text` matches the session; the no-prose-in-code scan; play in the browser build.

**Open questions for Jb** (marked `DESIGN-Q:` in code)
1. **Starting site**: the most-written living town that is not the capital, until M12's frame.
2. **Light**: underground rooms and night are "dim" but still readable until lamps exist (M07).
3. **Event log**: the command log is the event log for now; a richer log of what happened (for the notebook and endings) can come when there are more events than movement and reading.
4. Several buildings can share a name ("the intact tomb"); players use ordinals. Your `place.structure` variants can tell them apart once they have more to go on (M06 adds landmarks).

**Content slots added:** `place.site`, `place.structure`, `place.room`, `place.exit`, `place.out`, `thing.name`, `thing.examine`, `read.frame`, `read.glyph`, `read.more`, `read.no_more`, `read.end`, `read.nothing`, and the `say.*` family (intro, help, inventory, take/drop, doors, define, which, unknown verb, not here, no exit, saved, loaded, pack changed, wait…).

## 2026-10-02 — M06 Perception and movement

**Done**
- **Position and sight**: the player stands at a point on the land (whole metres); interiors stay room-based. Sight lines run over the height of the land (water surfaces included), with eye height, trees blocking beyond 100 m, and a reach set by weather and light. Sight is symmetric by construction.
- **Landmarks and salience**: settlements (living or ruined), lone buildings (waystations, bridges), and hills and mountains found by prominence. A look names the most striking ones in view, by size and closeness and how many look alike, with eight-point bearings and rough distance bands.
- **Edges**: rivers, streams, roads, coast, lake shores, treelines and cliffs near the player; `follow the river downstream|upstream`, `follow road east`; following keeps to a river's bank and stops at the sea, a lake, or the edge's end.
- **Intent movement**: `go to <anything in view>`, `head <direction>` (also `north`, `ne`…), `follow …`, `go back` / `retrace my steps`, `name this place <name>` then `go to <name>`, `enter`, `leave`. Routes are planned over walking time (slope, ground, roads; deep rivers need a ford or bridge; sea and lakes bar the way).
- **Travel simulation**: walks go in 300 m steps; time passes by ground and climb; anything newly in view (a settlement or building, including a settlement walked into) stops the walk.
- **Drift**: with no landmark, edge or sun to steer by (fog, rain, night, woods), each step adds heading error. The player is told what they believe they walked, never their true position, and may arrive somewhere other than intended.
- **Reports for mapping**: every journey reports bearing, rough distance (100 m under a kilometre, 500 m above) and time, as perceived; the JSON `state` carries the same, plus landmarks in view and edges near.
- **Weather and light**: three-hour spells of clear, rain or fog from local moisture (fog likelier at dawn, night and near water); dawn and dusk dim, night dark.
- **Multi-scale descriptions**: region (from a high point), area (outdoors), room (interiors), all slots.
- **Bench**: the Play tab shows, with spoilers on, the real map with your true path and position.
- **Tests**: sight symmetric and blocked by ridges; `go to` a visible landmark always arrives in clear daylight; drift zero with references, present in fog at night, reproducible; bearing reports match geometry in clear conditions; a **surveyor bot** that only sums travel reports maps the settlements it reaches within 15% of distance walked on average; a 24 km journey resolves in about 30 ms.

**Open questions for Jb** (marked `DESIGN-Q:` in code)
1. **Sight ranges**: clear daylight 20 km, rain 3 km, fog 200 m; dawn and dusk 40%; night 500 m at most.
2. **The sun as a compass**: in clear daylight outside woods the player never drifts, even with no landmark in view. Drift happens in fog, rain, darkness and forest.
3. **Weather and days**: three-hour weather spells, no seasons, days 05:00–21:00 all year.
4. **Report precision**: rounded metres and quarter hours are given to content, so maps can be drawn; content decides how to say them.
5. **Command reach**: `head` walks about 3 km, `follow` up to about 12 km, unless something stops it first.
6. **Summits**: highest within 1.5 km and 120 m above the land within 2.4 km; "mountain" from 700 m.
7. Some starting towns sit in hollows with nothing in view; the player has to climb out to see anything. Keep that, or bias starting towns towards a view?

**Content slots added:** `land.weather`, `land.area`, `land.edge_name`, `land.edge`, `land.name`, `land.landmark`, `land.horizon`, `land.region`, `travel.report`, `travel.lost`, `travel.arrive`, `travel.not_there`, `travel.interrupt`, `travel.blocked`, `travel.edge_end`, `travel.already`, `travel.no_route`, `travel.unseen`, `travel.no_edge`, `travel.indoors`, `travel.back_none`, `say.named`, `say.name_bad`.

**Also:** the authoring tool on claude.ai now saves edits to its own project store as you type (folder pickers and downloads are blocked inside the viewer); Claude reads them from there into `content/`.

## 2026-10-02 — M07 Physical world and survival

**Done**
- **`sim` crate** (the land and perception code from M06 moved here): local properties of every spot and room, meaning temperature (yearly mean, a daily swing, day-to-day spells, weather; buildings buffer it, cellars hold the mean), light, wetness, air, stability and fire. They come from the world, the time, and what the player has changed.
- **Rule table** (`crates/sim/data/rules.toml`): cold freezes still water (and flowing water in harder frost), thaw melts it, ice bears weight from 8 cm; fire and dry air dry things; fires burn fuel and starve; wet wood won't catch; flow turns wheels; darkness hides features; noise brings down unstable stone and disturbs creatures; fire deters them. M08 can trigger the same effects.
- **Mechanisms**: wells (dry in some ruins); sluices that, opened, divert a river so the crossing below can be waded, with water conserved into a side channel and the mill wheel below stopping; drain levers that empty flooded cellars; bridge levers; braziers; hearths; barred doors that a pry bar forces.
- **Items** with weight (carry limit 15): torch, lamp, oil, firesteel, wood, waterskin, cloak, provisions, berries, pry bar, and the three writing tools, which are inert until M08–M09. Making: torches from wood, rough shelters, filling a lamp.
- **Body**: warmth (clothing, fire, shelter, wetness, wind, activity), thirst, hunger, rest, wetness and injury, felt over hours, in coarse states announced when they change; healing over days; exhaustion drops you where you stand.
- **Creatures**: scavengers steal food and fear fire and noise; grazers charge if you come close; predators strike only from hiding and fear fire; deep things in dark tomb and mine rooms stir first, then strike unless there is light. Creatures that come into view stop a journey.
- **Hazards and death**: falls on dark stairs, falling stone, thin ice, deep water, cold, thirst, hunger, wounds and creatures. The cause is recorded with what you were doing, and an end-of-run stub follows (M11).
- **Obstacles**: barred doors, flooded cellars, raised bridges and deep crossings near towns; dark stairs and cold uplands come from the world itself. The starting town always holds a firesteel, a water container, provisions, wood and a pry bar.
- **Commands**: light, extinguish, drink, fill, eat, sleep, forage, gather (wood), make (fire, torch, shelter), feed (the fire), pull/push/operate, open/close (sluices, bridges), wear, remove, use, pry, shout, cross (ice, wade, swim), status.
- **Bench**: with spoilers on, the Play tab shows the body's states, light and load after each command.
- **Tests**: each rule; ice; body balance; creatures never strike from view and a torch keeps them off; a careful bot survives 4 days on all 5 test seeds; a reckless walker dies in 27–42 hours of cold or thirst; obstacles have ordinary answers (pry bar in town, drains at dry entrances, levers on dry land); opening a sluice makes the crossing wadable and conserves water; fire needs fuel; light reveals dark rooms.

**Open questions for Jb** (marked `DESIGN-Q:` in code)
1. **Pace of needs**: thirsty after 8 h, dead after 60; hungry after 16, dead after 240; tired after 18 h awake; cold below a felt 12 °C, dead after 130 degree-hours.
2. **Weather and temperature**: a fixed 13° daily swing, ±3° spells, no seasons; indoor and cellar temperatures as above.
3. **Items and numbers**: a torch burns an hour, wood two hours of fire, provisions last 12 h, berries 4; carry limit 15.
4. **Placement odds**: barred doors 30%, flooded cellars 35%, raised bridges 40%, unstable rooms in damaged/ruined buildings; the starting town's guaranteed basics.
5. **Creatures**: four archetypes, about two dozen outdoors (none within 2 km of the start), deep things in 40% of dry tomb and mine cellars (none in the starting town); deep things give one warning before striking.
6. **Risky crossings**: wading below twice river strength (one in ten swept), swimming deeper water drowns one in three, lakes too wide to swim, ice bears from 8 cm.
7. **Foraging odds** by biome; fires only outdoors or at a hearth or brazier indoors.
8. **Where the writing tools lie**: one each in an archive or temple outside the starting town.

**Content slots added:** `prop.cues`, `place.dark`, `read.dark`, `mech.name`, `mech.examine`, `mech.operate`, `mech.already`, `mech.cannot`, `fire.name`, `fire.lit`, `fire.fail`, `fire.fed`, `fire.out`, `fire.doused`, `item.examine`, `item.lit`, `item.out`, `item.doused`, `item.made`, `item.make_fail`, `item.too_heavy`, `item.wear`, `item.remove`, `item.cannot_use`, `drink.done`, `drink.none`, `fill.done`, `fill.none`, `eat.done`, `eat.none`, `sleep.done`, `forage.found`, `forage.none`, `gather.found`, `gather.none`, `body.change`, `body.status`, `body.collapse`, `hazard.fall`, `hazard.collapse`, `hazard.flooded`, `hazard.barred`, `door.pried`, `shout.done`, `cross.done`, `cross.fail`, `cross.fell_through`, `cross.swept`, `creature.name`, `creature.seen`, `creature.near`, `creature.sighted`, `creature.struck`, `creature.stole`, `creature.fled`, `death.narrate`, `end.summary`. `thing.name` and `thing.examine` gained an `item` variable.

## 2026-10-02 — M08 Writing that acts

**Done**
- **Surfaces and layers** (`crates/sim/src/writing.rs`): every text sits on its surface in a stack, oldest first. History's potent writing was cast, so it lies scraped and acts. Only the most recent scraped layer on a surface is live; layers beneath it are ghosts (inert, not yet readable); unscraped writing above it reads in full.
- **The three laws**: `scrape <thing>` with the scraper removes the whole top unscraped text, which becomes a scraped layer; if potent, its claim acts at once. Scraping an ordinary text over a live claim ends that claim (it is no longer the top scraped layer). Nothing is ever removed: stacks only grow and the scraped set only grows.
- **Claims** (`crates/sim/data/claims.toml`, the concept-to-property table): subjects map to classes (passages, rooms, land, stone) and verbs to properties. `open` holds doors open or shut, `burn` adds or takes 12° from rooms or land (enough to freeze rivers and make them crossable), `break` makes stone crumble or holds it sound. New concepts get powers by adding rows.
- **Effects engine**: claims act within a range (historic ones by their event's radius, the root inscription capped at 900 m for now; new releases by surface material). Conflicts: nearer beats farther, then newer beats older, and direct contradictions at the same place cancel. Effects show only as physical cues: strange warmth, frost against the season, doors that won't move, stone that groans loose.
- **Partial reading**: a scraped layer shows about 60% of its glyphs in daylight and 40% in dim light, deterministically per surface; lost glyphs are numbered gaps and can't be labelled. Ghost layers announce themselves ("fainter marks beneath").
- **Hands**: each scribe has a recognisable hand (six kinds), passed to the reading frame, so attentive players can tell writers apart; era spellings come from the language eras.
- **Scraping tool and the pivot**: the scraper lies in a reachable room of an archive or temple outside the starting town; beside it is a latent potent inscription whose effect is visible, safe and plain (nearby doors swing open, or the rooms warm). Four more latent inscriptions lie farther afield.
- **Debug views** (spoilers): `scraped-lang world --seed N writing --spoil`, and the bench's World tab ("Writing: why each place is strange"), listing live claims and their ranges, what each settlement is subject to, and surface stacks. Play spoilers include the claims acting where the player stands.
- **Tests**: conflicts; every potent-capable concept has a deterministic power; layers only grow; only top scraped layers act; every historic writing event leaves a scraped trace and acts on something; the scraper and a safe pivot are reachable on every test seed; scraping the pivot changes the world as claimed and nothing is lost; held doors resist; no output states a claim's meaning.

**Open questions for Jb** (marked `DESIGN-Q:` in code)
1. **Layers**: all texts on one feature form one stack in date order (king lists included); only history's potent writing starts scraped.
2. **Reach**: stone 900 m, metal 700, clay 500, wood and plaster 400, vellum 300; the root inscription acts within 900 m until M10.
3. **Claim powers**: 12° of heat; `open` and `break` on doors both hold them open; a "box" is treated as a passage; `break` on stone makes rooms unstable.
4. **Reading by eye**: 60% of glyphs in daylight, 40% dim.
5. **Latent inscriptions**: the pivot plus four, in the newest era's language; the pivot opens doors if the building has one, else warms it.
6. **Hands**: six kinds, by author.

**Content slots added:** `read.lost`, `read.scraped`, `read.ghosts`, `scrape.done`, `scrape.no_tool`, `scrape.bare`, `effect.change`, `effect.held`, `tool.found`. `read.frame` gained `hand`; `prop.cues` gained `uncanny`.

## 2026-10-02 — M09 Player writing

**Done**
- **Parser, surface → meaning** (`crates/lang/src/parse.rs`): the renderer run backwards. At each slot it generates the forms the grammar allows (every noun in each number for the case, every verb in each tense and polarity, every numeral, the potent frame) and keeps what matches, with memoisation. Works on phonemes or on glyphs, with or without word dividers. Every corpus sentence of seeds 1, 42 and 9001, in every era, parses back to its meaning (by glyphs, an abjad's dropped vowels may leave a different meaning that writes the same).
- **Writing**: `write <glyphs> on <thing>` with the stylus. Glyphs are script numbers or the player's own labels, `/` between words; never English. The confirmation echoes the glyph descriptions, not the meaning. Writing goes on blank surfaces or over a scraped trace (not over fresh writing), takes half an hour to dry, and becomes a new layer.
- **Understanding gate**: reading silently records each root and the texts it appeared in; a word can be written only once its root has been met in 2 texts (configurable; function words exempt). Grammatical forms need not have been seen.
- **Agreement**: writing over a trace must keep its register (potent over potent) and fill the same slots: the same roles, each with the same number and the same kind of noun. Otherwise it smudges and won't take.
- **Misfires**: text that doesn't parse is written but inert; text that parses acts as written (a wrong word or an unintended negation does what it says: "let the house not burn" chills the room). A garbled text inside the potent frame turns on its writer when scraped (one level of injury; "writing" is a new cause of death).
- **Deep reading**: carrying the lens shows the layer beneath the live one, fainter (35% of glyphs in daylight, 20% dim), and its words count as encountered.
- **Player layers** are stored in the game state in order, ready for M11's chronicle; their claims use the M08 engine.
- **Agent protocol**: responses carry `wrote` (accepted, refusal id, glyph numbers) and `scraped` (whether anything perceptible changed); spoilers add the player's understanding by root.
- **Tests**: parse inverts render; the gate refuses and is configurable; agreement fixtures; misfires are deterministic and negation bites; the lens reads beneath and reading records roots; a decipherer that knows only the grammar composes "let the house burn", writes it on a blank wall, scrapes it and warms the room on at least 4 of 5 seeds.

**Open questions for Jb** (marked `DESIGN-Q:` in code)
1. **Era**: players write in the newest era's language and script only.
2. **Gate**: 2 texts; partly scraped readings count; potent formulae and "and" need no encounters.
3. **Agreement**: checked against the whole trace beneath (register, roles, number, kind of noun), not only its surviving words.
4. **Where writing goes**: walls, steles, altars, niches, lintels and similar features; not over unscraped writing.
5. **Lens**: 35% / 20% of the deeper layer visible.
6. **Backlash**: a potent-framed text that doesn't parse hurts its writer when scraped; there is no "forgiving" parse of word-order mistakes yet (they don't parse, so they're inert).
7. Players can't write personal names yet.

**Content slots added:** `write.done`, `write.refused`, `write.unknown_mark`, `write.hesitate`, `write.smudge`, `write.backlash`, `scrape.wet`, `read.deep`; `death.narrate` and `end.summary` gained the cause `writing`.

## 2026-10-02 — M10 A world on a trajectory

**Done**
- **Regions** (`crates/sim/src/region.rs`): drainage basins cut into ~10 km blocks (small ones merged), each with life, water, stability and climate, and with upstream, downstream and neighbouring regions. One step a day: water relaxes to rain plus upstream minus warmth, life follows water and warmth and spreads, stability falls with drought and spreads its cracks, climate relaxes unless pushed. Values are whole thousandths, so skipping time is exact.
- **Trajectories**: dying worlds start worn and head for 60% of their land's life and water; stagnant worlds hold at 85%; recovering worlds grow back; balanced worlds hold. No other people (living world: none, as proposed).
- **Great inscriptions**: the root plus the two widest-reaching writing events of history, provided they can be reached. Each pushes its region and neighbours (winter, drought, flood, binding, crevasse, holding). Rubble on the way to them has been dug through; their buildings never flood.
- **Regional → local**: climate and seasons shift air temperature (and so ice and the body); river flow scales with the region's water against where it began (rivers shrink, crossings open or close); foraging follows the region's life.
- **Scaling tools**: scraper (1), fine scraper (2), old scraper (3), first scraper (4), each farther out (the first beside the root). Surfaces bearing a great inscription need the first scraper to scrape or write on. A release with a fine scraper or better reaches three times as far; with the old scraper it pushes its region; with the first, its region and two around. Countering a great inscription (scraping fresh writing over it, or writing and releasing the opposite claim) ends its push.
- **Trade-offs**: coupling (water downstream, life following water and warmth, cracks spreading) makes large changes ripple; the debug view lists every driver and, per region, what pushed it.
- **Revisits**: the game remembers each outdoor cell's regional state when last seen and says when a band has changed ("the water here was high; now it is low"); changes within a band say nothing.
- **Time**: `wait 3 hours`, `wait a day`, `wait 2 weeks`, `wait a season`; a 360-day year of four seasons (summer warmer, winter colder); the player ages from 25, which shows in `status`, and wounds heal more slowly past 30.
- **Evidence of the big picture**: shortage ledgers in the storehouses of the regions that have lost most life, plus history's famine ledgers, ruins and letters.
- **Debug views**: `scraped-lang world --seed N regions --spoil --days 360` and the bench's World tab ("Regions over half a year"); play spoilers carry the region, its variables and what pushes it.
- **Tests**: ten years alone is stable; skipping days equals stepping them; dying worlds lose life and recovering ones gain over a year; holding water upstream changes downstream; every great inscription is reachable on foot and through its building, resists weaker scrapers, and can be countered with the first scraper; countering changes its region's course; revisits notice only band changes.

**Open questions for Jb** (marked `DESIGN-Q:` in code)
1. **Regions**: basins cut into 32-cell blocks, merged below 40 cells.
2. **Coupling numbers and headings**: 60% / 85% / 100% / 115% of natural life and water for dying / stagnant / balanced / recovering; a few percent change per day.
3. **Great inscriptions**: the root plus two; the root reaches three regions out, others two. A perpetual winter can kill a region's life within a year; is that the intended harshness?
4. **Claim → region**: warmth ±6°, opening/sealing water ±0.3, breaking/holding stability ±0.4.
5. **Tools**: four scrapers, all found (none made); the stylus works at any scale if the right scraper is carried.
6. **Seasons and age**: 360-day year from spring; start age 25; healing slows by a sixtieth a year past 30; no death of old age yet (M11's endings).
7. **Evidence**: up to four shortage ledgers; no migration graffiti yet.

**Content slots added:** `region.cues`, `region.changed`, `great.site`, `great.release`, `scrape.too_weak`, `time.status`; `write.refused` gained `too_great`; `tool.found` and `thing.name` gained the new scrapers.

## 2026-10-02 — M11 Endings and legacy

**Done**
- **Endings**: death (as before), leaving, writing yourself in, old age (80) and being overtaken (the region you stand in collapses: stability and life both under 0.1). Each is recorded with its cause, command, time and place, and narrated through its own slot; any command after the end shows the summary.
- **The deepest text**: the root inscription's surface becomes the deepest stack in the world. Beneath the root lie an account of the departure ("the self departed. Let the self depart."), an account of the cause ("the king scraped the tablet. The walls did not break. The self did not depart."), and repeats of the middle sentence until no other stack is as deep. Two new concepts, `self` and `depart`, occur nowhere else, so they can be learnt only here. The plain lens never shows these accounts; the new **first lens**, placed nearly as far out as anything, shows every faint layer at once (80% of glyphs in daylight, 60% dim).
- **Leaving**: write "let the self depart" (the late era's words) and scrape it. The run ends by choice and the world's state is frozen as the outcome.
- **Writing yourself in**: any other potent claim on the self ("let the self not depart" is the plainest), scraped, ends the run with the player as a trace in the world.
- **Run record** (`Game::record`): buildings entered, places named, texts read, texts written, every claim released (who wrote it, who scraped it, with what power), the player's acts on the regions (day, released or silenced, scale, what it pushes), and each region's start, end and the same world left alone.
- **Zoomed-out summary**: the regions are run a second time from the start with only history's pushes, so every change is attributed to the world, to the player, or to both. Regions that changed alike are grouped into one line, the player's doing first, then the causal chain, all through slots.
- **Chronicle**: up to six clauses in the newest era (arrival; up to two of the largest changes the player caused, or a released claim restated; the people's verdict, honour or fear; the ending), shown as glyph numbers or the player's own labels in the same form `write` takes. Spoiler output adds the meaning, romanisation and an English gloss. It parses back to exactly its meaning on seeds 1, 42 and 9001.
- **Legacy**: `scraped --legacy [FILE]` writes the run's final inscription (the last thing the player wrote that parses) when it ends, and places the previous one in a new world as a faint layer beneath a reachable history cast outside the starting town, older than everything on it. Saves carry the legacy they were made with. The bench has a Legacy toggle (kept in the browser).
- **Notebook**: `export [PREFIX]` writes the transcript, the named places and the run record as JSON; the bench shows all three, with copy buttons, when a run ends. The browser API has `play_end`.
- **Tests**: every ending reached by a scripted run; reading the deepest text through the first lens is enough to write the departure (and nothing less is); the summary matches the simulation, attributes a counter to the player, and a world left alone attributes nothing; the chronicle parses back; legacy appears only when enabled, as a ghost, and survives save and load; the notebook holds the run. The bench smoke test plays to the end of a run.
- Adding the two concepts shifts some generated words and glyphs for existing seeds (snapshots and world fingerprints updated). Meanings now deserialise, for the legacy file.

**Open questions for Jb** (marked `DESIGN-Q:` in code)
1. **The departure**: "let the self depart"; writing yourself in is any other potent claim on the self. Is "self" the right idea, or should the player have a name?
2. **The deepest text's wording** (above), and padding by repeating its middle sentence.
3. **First lens**: reads every faint layer at once, at 80% / 60%; placed in a far archive or temple.
4. **Old age at 80; collapse** when stability and life are both under 0.1 where you stand.
5. **Chronicle mapping**: life falling is "the fields burned", water rising "the people drank the water", ground failing "the walls broke", warmth "the sun burned"; verdict honour or fear by the net change in life, water and stability.
6. **Legacy placement**: beneath a reachable history cast outside the starting town (the root if none), in the first era's language; it never acts.
7. **Chronicle display**: glyph numbers with the player's labels, "/" between words.

**Content slots added:** `end.left`, `end.written_in`, `end.old_age`, `end.overtaken`, `end.region`, `end.calm`, `end.act`, `end.chronicle`, `read.legacy`, `notebook.heading`, `say.export_offer`, `say.exported`, `say.legacy_kept`. `end.summary` is reworked (ending, counts); `read.deep` gained `count`; `tool.found` gained the first lens.
