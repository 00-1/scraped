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

## 2026-10-02 — M12 Authored spine and set pieces

**Done**
- **Storylets in the pack**: `[[storylet]]` tables in any content file, with `at` (anywhere, structure, outdoors, hook), a spine `hook`, a `when` condition, `after`, placement rules (building kinds, land, era, near water, away from the start), effects and a request for generated writing. Each storylet's text is the slot `story.<id>`, so it is written, linted, covered and previewed like any other slot. Files round-trip through the authoring tool and the project database.
- **Conditions** use the template language, plus a new `has` operator for lists (`carrying has 'torch'`, `flags has 'met_keeper'`, `happened has 'shrine'`).
- **Placement** at world creation: deterministic from the seed and the storylet's id, at most one storylet per building, only buildings that can be entered and walked to.
- **Generated language**: a storylet can ask for an everyday warning or a potent claim about a concept or kind of thing, in an era; the engine builds the meaning, renders it in that era and puts it on a new inscription in the building, readable like any other (and, if potent, releasable). It parses back to its meaning.
- **Triggering**: on entering the building, within 300 m of an outdoor spot, anywhere the condition holds, or at a spine beat. Storylets happen once unless marked `repeat`.
- **Effects** through the game's own systems: give an item, set or clear a flag, open the building's ways (unbar and clear).
- **Spine hooks**: opening (replaces the bare wake-up when written), first scraped text seen, each tool found, first release, first write, reaching a great inscription, finding the deepest text, and every ending (before the summary).
- **Authoring tool**: "New storylet" in the sidebar; a storylet form with dropdowns and checkboxes for every rule; spine beats shown as chips (written or not); "Place it" generates a world and shows the building, its distance and bearing from the start, the generated writing (gloss with spoilers) and the text as it reads there.
- **Lint**: bad `at`, missing or unknown beat, unknown building kind, land or era, unreadable condition or unknown variables, `after` naming no storylet or itself, unknown or conflicting effects, writing with nowhere to go or about nothing the language has, missing text, duplicate ids, unwritten spine beats, and (in `scraped-cli content lint`, which generates worlds) storylets that fit no building.
- **Web API**: `storylet_schema`, `storylet_preview`; `registry`, `lint`, `lint_variant` and `preview` take the pack. Play spoilers list where storylets were placed and what has happened.
- **Examples**: one placed storylet (a far temple with a warning about water) and one per spine beat, all marked as examples for Jb to replace.
- **Tests**: condition evaluation, `after` and flags chaining, deterministic placement that follows its rules, generated writing that parses back and triggers on entering, a scripted run reaching every spine beat, and lint fixtures for each problem.

**Open questions for Jb** (marked `DESIGN-Q:` in code)
1. **Nearness** for outdoor storylets: 300 m.
2. **One storylet per building**; storylets placed in pack order, so earlier ones get first pick.
3. **Generated writing** for a storylet goes on a new stone inscription in the building's furthest reachable room (or outdoors at the spot).
4. **Hook storylets' text** appears after the command's own text; the opening replaces the wake-up, the ending comes before the summary.
5. Should "first_…" beats ever repeat? They fire once a run.

**Content slots added:** one per storylet (`story.<id>`), declared from the pack. Examples: `story.opening`, `story.shrine`, `story.scraped_seen`, `story.tool`, `story.release`, `story.first_write`, `story.great`, `story.deepest`, `story.ending`.

## 2026-10-02 — M13 Authoring tool v2

**Done**
- **Render traces**: the renderer records every slot it renders (which variant, which `[if]` branches, the text, and whether it was called from another slot); the game keeps each with its variables and seed, so a line can be rendered again exactly.
- **Coverage** (`scraped_game::coverage`): two deterministic bots, a wanderer and a scholar who starts with the writing tools, play any number of worlds. Every slot gets hits per player-hour, gap renders (placeholder or example) per player-hour (the ranking), per-variant and per-branch counts, the common variable combinations that fell through to a generic variant, and repetition (how often the same variant shows in a typical run). Slots no bot reached are listed.
- **Playtest in the tool**: the game runs beside the editor; every line links to the slot, variant, branches and variables that wrote it ("why did I see this?"), and clicking opens that variant. Edits replay the whole run at once with the new text (a run is its seed and commands, so its state never changes); rewind to any command; play from a pasted save; copy the save.
- **Hot reload**: `Game::set_pack` swaps the text of a running game; storylet rules stay as they were when the world was made (`play_pack` in the browser API).
- **Inspectors** (spoilers): language (glyphs, lexicon search, grammar sheet per era), world (map, history, surfaces and live claims, regions over half a year), run (understanding by root against the threshold, words not yet met, the run record, storylets, claims here, regions).
- **Voice tools**: Jb's recurring words and phrases (his own variants only), echo warnings when a four-word phrase appears in two slots (also in lint), length and rhythm by family.
- **Review mode**: the sidebar can list families in the order a player meets them (opening, early, late) with done counts, and "Next to write" jumps to the next slot that isn't done.
- **Pack diff**: the Changes tab compares the pack with the committed one the tool was built with, variant by variant, and warns when storylet changes mean saves will replay differently (text changes never do).
- **Tests**: coverage is deterministic and ranked, and accounts for every slot; every line of output comes from a render and every render re-renders exactly from its variant and variables (two bots, three seeds); a hot reload mid-run leaves the state identical and changes the text. The authoring smoke test (run in CI) now plays, follows a line to its template, edits it and sees the run update, runs the bots, and opens every tab.

**Open questions for Jb** (marked `DESIGN-Q:` in code)
1. **Repetition**: a slot "needs more" when a player would see the same variant 4+ times a run.
2. **Echoes**: phrases of four words or more shared between slots.
3. **Review order**: story, say, place, thing (opening); land, travel, read, glyphs, survival, hazards, mechanisms, creatures (early); tools, scraping, writing, regions, great inscriptions, time, death, endings (late).
4. **Saves and pack changes**: any storylet change counts as breaking saves.
5. The bots are simple; slots behind deep play (endings, late writing) show as "never reached" until better bots exist.

**Content slots added:** none.

## 2026-10-02 — M14 Fairness, clients and release

**Done**
- **Solvability checker** (`scraped_game::fairness`): for each goal (the scraper, the first release, writing a claim, a great inscription, the deepest text, leaving) it traces what must be reachable on foot and through buildings, which words must appear in enough readable texts, and which constructions must be attested in the newest era; for leaving, the departure words must be in the deepest texts and enough words shared between the oldest and newest writing to carry them across. Worlds that fail give way to the first fair seed derived from them (`fair_seed`); the clients do this unless told `--raw`.
- **It found a real problem**: on most seeds the root inscription, and so the deepest text, lay behind fallen rooms. Where the root can't be reached, the deepest accounts now lie beneath a scraped recopy of the root, in a reachable room of the same building.
- **Difficulty**: presets gentle (an alphabet, names marked, two eras), standard (the defaults) and archaeologist (word dividers, fused inflection, four eras), carried in saves and seed codes. Metrics per world: readable texts, evidence density (texts per concept), ambiguity (newest-era roots spelled alike), anchors (numerals, formulae, repeated names) and the bridge between oldest and newest writing. Each preset has a band. `scraped-lang fair --seeds 1-30 --all`: 88 of 90 worlds fair as made, and all 90 have a fair world.
- **Balance**: regions settle for two years under history's pushes before play, so water, ground and climate start where the great inscriptions have long held them, and life stays as worn as the trajectory left it. Ground starts mid-band, and life moves half a percent of the way a day. A world left alone now changes a band in about 6% of regions a month (it was nearly all). Survival is unchanged: the careful bot lasts four days, the reckless one dies in a day or two, and the coverage wanderer mostly dies of cold in about 20 hours.
- **Browser player** (`tools/play`, published to Pages): a quiet reading page; the log is a live region; keyboard-first (focus starts in the command box, Up/Down recall, Alt+S saves, Alt+M opens the menu); text size, high contrast, light/dark; three save slots plus export/import; transcript and notebook downloads; new games by seed, code or difficulty; legacy; links with `#code=…`. Its labels are Jb's `ui.label` slot.
- **Terminal player**: `--difficulty`, `--code`, `--raw`; `code` shows the world's code. Release binaries for Linux, macOS and Windows (`scraped` and `scraped-mcp`, with the content and protocol docs), built by the release workflow and attached to each GitHub release.
- **Agents**: JSON-lines responses carry `protocol: 1`; `docs/PROTOCOL.md` documents it. `scraped-mcp` is a stdio MCP server with five tools: new_game, act, save, load and seed_code. `docs/coop/CLAUDE.md` holds house rules for an agent playing with a person (ask before anything irreversible, never claim meanings without evidence, keep a journal).
- **Seed codes**: Crockford base 32 with the preset, a pack fingerprint and a check byte (`2K4G-30GF-GFV8-T`); typos are caught.
- **Release gate** (`.github/workflows/release.yml`): fmt, clippy, tests, the content release check, the fairness batch (50 seeds × 3 presets), the browser builds and their headless tests, and determinism on Linux, macOS and Windows; then the binaries. CI also checks determinism on all three platforms and in WebAssembly (`tools/smoke/determinism.cjs`, against `crates/game/tests/transcripts.txt`).
- **Player docs**: `manual <section>` (contents, playing, reading, notebook, writing, survival, endings) through the `manual.page` slot; the in-game `help` as before. The text is Jb's to write.
- **Tests**: crafted unsolvable worlds fail on exactly the right goal; metric bands; every preset gives fair worlds, plays and replays; seed codes round-trip and catch typos; the MCP server and protocol 1; cross-platform transcripts; and a headless test of the browser player (keyboard only, focus stays in the menu, named controls and labels, live log, saves, text size, contrast, downloads, and the same world from its code).

**Open questions for Jb** (marked `DESIGN-Q:` in code)
1. **Presets**: the dials for gentle, standard and archaeologist; the metric bands (8 anchors and density 2 for all, gentle needs density 3 and under 5% ambiguity); the bridge of 10 shared words.
2. **Unfair seeds** are replaced silently by a derived fair seed; should players be told?
3. **Balance**: two years of settling; ground starting at 0.7; life at half a percent a day.
4. **UI labels** are one slot (`ui.label`) with an id per control.
5. **Release**: the gate fails until every required slot has Jb's text, as intended. The first tag (`v0.1.0`?) is his call.

**Content slots added:** `manual.page`, `say.seed_code`, `ui.label`.

**Roadmap complete.** What's left for release is Jb's writing; the authoring tool's Gaps and Review views show where to start.

## 2026-10-02 — Android app

**Done**
- **An Android app** (`android/`, Android 8+, targets 35): the game only, no spoilers. Built by `android/build.sh` without Gradle or the SDK manager (this environment can't reach Google's SDK host). The platform jar comes from GitHub, aapt2 from Apktool's release, D8 from R8's release and apksig from Maven Central. The APK is about 1.3 MB.
- **The page**: `tools/app/index.html` runs the existing WebAssembly engine in the app's WebView, so the app plays exactly as the other clients, with the same determinism. The native shell is a few hundred lines of Java.
- **Design, from research**: interactive fiction on phones lives or dies by typing. The best-regarded Android IF app (Text Fiction) is praised for an SMS-like interface that replaces typing with touch; chat apps get the composer right. So:
  - a worlds list like a list of conversations;
  - the game's text as prose, with commands as bubbles;
  - a composer pinned above the keyboard that keeps the keyboard up after sending, with recall;
  - chips for one-tap commands and verb + thing composition;
  - tap a passage to copy it or add it to the world's notebook;
  - text size, theme (system, light, dark, sepia), typeface, and Android's font scale respected;
  - edge-to-edge with the keyboard and bars handled natively (Android 15 no longer resizes the window for the keyboard).
- **Integrations**:
  - Android's Google account backup of worlds and notebooks;
  - sync to a file the player picks (Google Drive works), with restore and import;
  - agent access: an MCP server and plain HTTP on the local network, behind a key, with two abilities only (read the text, type a command), the agent's moves shown tagged in the transcript.
- **Tests**:
  - `tools/smoke/app.cjs` drives the page on an emulated Pixel 7: making a world, typing, focus kept after sending, recall, chips, scrolling to the newest text, the notebook, an agent's moves, menus, theme, integrations, back, and reopening a world.
  - `android/test/AgentTest.java` tests the agent server over a real socket: the key, MCP and HTTP, only two tools, and bounded one-line commands.
  - CI builds the APK and keeps it as an artifact; releases attach it.
- **Not tested on a device**: there is no emulator here. The Java is compiled against the Android 35 platform and the APK is checked (manifest, aligned resources, dex, v2 signature), but the first run on a real phone may turn up something.

**Open questions for Jb**
1. **Agent access over the internet** (for Claude on claude.ai) needs a relay service; the local-network version is what exists.
2. **Command words on chips** (look, read, take, examine, inventory, status, wait, out) are the parser's verbs, shown as typed rather than as slot text. Should they be labels instead?
3. **The signing key**: set the `ANDROID_KEYSTORE_B64` and `ANDROID_KEYSTORE_PASSWORD` secrets so CI builds can update each other.
4. **Package name** `org.scrapedagain`.

- **Fix**: since the M14 balance pass, life could stall short of where it was heading, because a slow daily step rounded away in whole thousandths. It now moves at least one unit a day. A world left alone changes a band in about 13% of regions a month. The sim test that caught it hadn't been reached by the earlier test runs, which stop at the first failing crate; I now run with `--no-fail-fast`.

**Content slots added:** `app.label` (68 ids, each described for Jb).

## 2026-10-02 — Native Android app (Compose + Rust)

**Done**
- The WebView wrapper is replaced by a native app in Jetpack Compose with Material 3 (`android/app`). The engine is a JNI library (`crates/android`) holding Jb's content and is called on one thread. Everything from the web app carries over:
  - the worlds list, the transcript as prose with your commands as bubbles, and the composer with chips and recall;
  - tap a passage to copy it or add it to the notebook;
  - reading settings;
  - Google backup, file sync, and agent access (MCP or plain HTTP on the local network; it can only read and act).
- **Backdrop** (Jb's choice: atmosphere, never game state). A faint lamplit room is painted by Rust (`crates/android/src/atmosphere.rs`) from the clock and the theme only. It is drawn tiny and scaled up, at 10 fps, and goes still with reduced motion.
- **Build:** `android/build.sh` runs cargo-ndk (arm64, armv7, x86_64) and then Gradle.
- **CI:** builds the APK, then plays it on an emulator (API 34). `AppTest` covers a world, typing, an agent move, HTTP with the key and the notebook. `EngineTest` checks the transcripts against every platform's.
- `android/test-agent.sh` tests the agent server off-device.
- **Reading:** a reply taller than the screen now opens at its top, under its command. The emulator test caught this: the list followed the end of the reply.
- CI is green: the APK builds and all three on-device tests pass.

**New content slots**
- `app.label` id `atmosphere` (the switch for the backdrop).

**Open questions for Jb**
1. **Signing secrets.** Until `ANDROID_KEYSTORE_B64` and `ANDROID_KEYSTORE_PASSWORD` are set, each CI APK has a different key, so updates need an uninstall. The native app can't update the WebView one in any case. Instructions are in `docs/ANDROID.md`.
2. **Agent from the cloud.** It still works on the local network only; a relay service would be needed for cloud agents.
3. **Chips.** They offer look, exits, out, read/take/examine plus the things in view, inventory, status and wait. Which verbs should they offer?
4. **Package name** `org.scrapedagain` is permanent once published.

## 2026-10-02 — Depth roadmap

**Done**
- Reviewed the M14 build by playing it (seed 42) and measuring the world: every system works, but each is at its simplest (11 building types, 6 spells per world on 3 verbs, 59% of texts are tombs, generic landmark names), arrival text lists everything at once, and the scraper sits beside a "pivot" spell with a storylet announcing it.
- Two principles from Jb added to the top of `docs/DESIGN.md`: **Writing is background, at first** (the world is worth exploring for itself; the mechanic is discovered slowly, never pointed at) and **Say little; let the player dig** (short, intriguing responses; detail through examining, listening, smelling; season and needs inferred from evidence, not stated).
- Game loop and tools in `DESIGN.md` reworded to match.
- New `docs/DEPTH.md` and milestone specs D01–D12 in `docs/milestones/`, each with numeric targets measured by D01's instruments and sample transcripts for Jb to read.

**Next**
- D01 — Instruments.
- Added **D04 — Great interiors** at Jb's request (vast, intricate buildings and caverns): a real spatial model for all interiors, architecture that grows through history, cave systems from geology, navigation and mapping inside. Later milestones renumbered D05–D12.

## 2026-10-03 — D01 Instruments

**Done**
- **Landmarks read apart** (`crates/sim/src/traits.rs`). Every landmark has traits read from the world:
  - a summit's shape, height, cover and what stands on it;
  - a settlement's walls, size, setting, tallest building, roads and bridges;
  - a lone building's setting and condition.

  Each gets a *mark*: the trait no alike landmark that could share a view goes by. `land.name` offers them all. The parser takes trait words, bearings and ordinals, and prefers the most exact match ("the town to the west" over the north-west one; "the hilltop mountain" over "the twin hilltop mountain"). "Which do you mean?" lists landmarks with their bearings.
- **The depth bots** (`crates/game/src/bots.rs`): both play only from what the player is shown, plus their own memory.
  - The **curious explorer** survives three days on 8 of 10 seeds.
  - The **scholar** (grammar spoilers, no map, body kept well) reads the deepest text on 5 of 10 seeds in 90 days. It:
    - comes back to rooms that were too dark, had writing too faint for its lens, or had a door it couldn't open;
    - counters doors held shut by old writing with an "open" claim of its own.

  The authoring tool's Gaps mode now uses both. `scraped-lang bots`.
- **Depth metrics** (`crates/game/src/depth.rs`): `scraped-lang depth`, the web `depth` call, and the bench's new Depth tab. Defined and baselined in `docs/DEPTH.md`.
- **Samples:** `scraped-lang samples D01` wrote `docs/samples/D01/` with a note.
- **`docs/DECISIONS.md`**: every open question in one place (118 rows, plus the D01 ones).
- **Game fixes the bots found:**
  - answering "which do you mean?" with a bare "first" never worked;
  - "drop" and "take" asked about things you couldn't drop or take.

  The summary (JSON protocol) also lists mechanisms now.
- **Tests:** metric snapshots for seeds 1, 42 and 9001; explorer survival and scholar depth (run in release in CI); landmark names apart in view and the same from every side; qualified references resolve.

**Findings for Jb**
1. **Sealed towns.** Old writing holds the doors of whole towns shut, and the only counter is to write "open" with door, gate, tomb or box. Those words are met in fewer than two texts even after 100–170 texts read, so the understanding gate never lets them be written. A player can find the late game sealed. D07 and D09 should fix this. Until then, the scholar test is held at 5 of 10 (D01 asks for 8).
2. **Light is very scarce.** Torches burn an hour, lamps four. Deep rooms stay dark until the player comes back with fuel.
3. **Cold kills.** Most explorer deaths are worlds whose start lies under a cold spell.
4. **The baseline reads as the review said.** A look has 88 words and 29 facts at the median, and it states facts ("It is spring") rather than showing them. D02 is next.

**Open questions:** see the new "Instruments (D01)" section of `docs/DECISIONS.md`.

**New content slots:** none. `land.name` gains variables: mark, mark2, shape, height, cover, top, walls, tallest, setting, condition.

## 2026-10-03 — D02 Quiet text

**Done**
- **The attention model** (`crates/game/src/attention.rs`). Every description path builds candidate facts. Each fact has a slot, a key, a salience, and optionally an interrupt or anchor flag. The model weighs them by novelty against `State.told`, which remembers what was said and what was only noticed. It keeps the best within the response's budget, renders each as a sentence in a natural order, and remembers what it said.
  - Memory hashes a fact's meaning, not its prose, so a hot reload never changes state.
  - Budgets: arrival 3, room entry 3, `look` 4, the end of a journey 2, `look closer` and `look around` 6. With nothing new, at most 2.
- **Places seen as a whole.** A place is one whole fact ("an intact town of several tombs") plus at most two standouts.
  - Digging moves down a level: `look closer` gives the groups, and `look at the tombs` names members three at a time, round again until every one is reached.
  - Counts are vague; `count the tombs` gives the exact number.
  - The parser understands "the tombs", "the worn tombs", "the worn ones", "a tomb", "another tomb", "the nearest tomb" and "go among the tombs".
- **Digging verbs** (`crates/game/src/senses.rs`): `look around`, `look closer`/`search`, `look up`, `look down`, `listen`, `smell`, `touch`, `taste`, `count` and `check myself`. A second `examine` finds more (`thing.closer`).
  - Sound has sources that carry by distance and walls.
  - Smell drifts with a wind that turns every six hours.
  - Touch reports material, texture, temperature and wet.
- **Shown, not stated.**
  - The season comes through as evidence: at least 3 kinds per season per biome, tested.
  - The region's state shows on the ground (cracked mud, withered growth, frost out of season).
  - Needs appear as sensations and in `check myself`.
  - Weather comes only when it changes or matters, or on `look up`.
  - Retired slots: `place.site`, `place.room`, `land.weather`, `land.area`, `land.horizon`, `prop.cues`, `region.cues`, `region.changed`, `time.status`, `creature.near`, `body.status`.
- **Authoring:** the tool's new "In a response" panel shows whole responses as the attention model assembles them, where the open slot was said (web `in_context`).
- **JSON protocol:** with `--spoil`, `truth.attention` lists every fact weighed, said or not.
- **Measures:** facts are counted once even with names inside them. `depth_on_demand` now counts what digging could find at each arrival.
- **Results (seeds 1–10):**

  | Measure | Before | After |
  |---|---|---|
  | Arrival, median | 30 words, 14 facts | 17 words, 3 facts |
  | `look`, median | 88 words, 29 facts | 12 words, 2 facts |
  | Journey, median facts | 26 | 4.2 |

  Digging finds 4× what arrival shows. The explorer survives three days on 9 of 10 seeds.
- **Samples:** `docs/samples/D02/`, with the seed 42 opening fixture before and after in `NOTES.md`.
- **Tests** (`crates/game/src/attention_tests.rs`):
  - budgets hold except for interruptions;
  - selection is deterministic;
  - repeated looks say less;
  - at most two buildings are named;
  - every group member is reachable;
  - each season has evidence;
  - no season, region or need is stated;
  - digging finds at least 3× what arrival shows;
  - spoiled truth lists everything weighed;
  - the parser understands groups.

**Open questions:** see the new "Quiet text (D02)" section of `docs/DECISIONS.md`: budgets, interruptions, fading, the evidence lists, wind, vague counts, the size of member batches, and storylet variables.

**New content slots (one example variant each):**
- place: `whole`, `standout`, `group`, `group_name`, `member`, `count`, `thing`
- land, sky, air: `land.ground`, `land.unseen`, `sky.weather`, `air.felt`, `air.moving`, `air.uncanny`
- other world facts: `ground.wet`, `danger.unstable`, `fire.near`, `evidence.season`, `evidence.region`
- room: `whole`, `ways`, `group`, `thing`
- sense: `sound`, `silence`, `smell`, `no_smell`, `touch`, `touch_air`, `taste`, `sky`, `ground`
- `thing.closer`
- body: `felt`, `well`, `age`

**For Jb**
- The example text is thin and sometimes prints ids ("Faintly, lake from the west"). These are short pieces that combine, so your variants will carry the voice. The "In a response" panel shows each one among its neighbours.
- Members of a group look alike ("the intact tomb, to the east" twice). D03 gives buildings more to tell them apart by.

**Parked** at Jb's request after D02. The next agent starts D03; see `docs/HANDOFF.md`.

## 2026-10-03 — D03 Places with character

**Done**
- **The world** (`crates/world`):
  - **Rock.** A geology layer by region: granite, slate, limestone, sandstone, basalt and clay.
  - **Natural features and old marks**, 36 kinds, each placed by its cause:
    - springs at slope feet, warm ones over basalt;
    - waterfalls and rapids where rivers drop;
    - gorges, deltas, oxbow lakes, tidal flats, sea stacks and sea caves;
    - cave mouths and sinkholes in limestone;
    - cliffs, arches, pillars, scree, boulder fields, salt flats, glaciers;
    - ancient trees, groves, dead and petrified forests, reed beds, meadows;
    - cairns on summits; standing stones and burial mounds near the oldest towns;
    - terraces, field walls, old roads and cuttings, quarries, spoil heaps.

    Caves record what they should hold, for D04.
  - **Buildings.** 39 new kinds of building (50 in all) from one table, each with a family and a placeholder room plan. Each town's role calls for its own; signal stations, wayside shrines, hermitages and aqueducts stand out on the land. The original buildings, their texts and their decay are unchanged.
  - **Towns.**
    - Each settlement has a role: capital, port, holy city, mining camp, fortress, market town, farming village or refuge.
    - It has a reason for its shape (a river crossing, a harbour, a hilltop…) and a street plan.
    - It has districts joined by streets, and walls and gates where war came.
    - No two towns of one role and size share a layout.
  - **Underground routes** recorded for D04: cellars, drains, tunnels under walls, catacombs, mine workings and cave systems.
  - **Scenes.** Each traces to an event or to its place's history; there are at least 3 per town, and more out on the land.
- **The game:**
  - **Arriving.** A town's role is in its first line ("an intact holy city of a few houses").
  - **Moving through towns.** The district underfoot is a fact; `look around` in a town gives its layout; `go to the market` walks there.
  - **Features.**
    - Those close by are noticed on arrival, and those within 1.5 km on `look around`.
    - Tall ones are landmarks; one standing in water is reached from the bank.
    - `examine` digs in.
  - **Scenes** are mostly found by `look closer`.
  - **Journeys** are stopped only by striking landmarks.
  - **Items.** The newer buildings hold items by their family.
- **Storylets** can be placed at scenes (`[storylet.place] scene = ["barricade"]`). The storylet happens in the scene's room. The authoring tool has a Scenes list for this.
- **Bench:** the world map shows features by group, town roles in the tooltips, and a Places view (roles, districts, buildings, scenes, features, underground).
- **Measures:** scenes per settlement, scenes outside towns, towns sharing a layout, town roles, and new kinds found in the fifth hour. The WebAssembly world-generation time is checked in the wasm smoke test (under 0.1 s).
- **Bots:**
  - **Explorer.** Goes into the kinds of building it has seen least first. Makes for towns from noon. Shelters indoors from the evening or when cold. Wants water, food, a cloak and a firesteel before lights.

    It survives three days on 8 of seeds 1–10 and 17 of 1–20 (D01: 16).
  - **Scholar.** Goes where writing is kept and passes by mills and smithies. Still reaches the deepest text on 5 of 10 seeds.
- **Results (seeds 1–10):**

  | Measure | Before | After |
  |---|---|---|
  | Building kinds per world | 8.7 | 38.2 |
  | Natural feature kinds | 5.8 | 22.4 |
  | Scenes per settlement (median, least) | 0 | 4, 3 |
  | Towns sharing a layout | – | none |
  | New kinds in hour 5 | – | 6 |

  D02's brevity holds: 17 words per arrival.
- **Samples:** `docs/samples/D03/` (ten hours each), with notes.
- **Tests:**
  - every feature follows its cause;
  - every scene traces to history;
  - towns never share a layout, and every town building has a district;
  - worlds are varied enough;
  - kinds' ids follow the table;
  - slot lists cover every room and thing;
  - districts can be walked to and named;
  - features are noticed and examined;
  - scenes are found by looking closer;
  - storylets can be placed at scenes.

**Open questions:** see the new "Places with character (D03)" section of `docs/DECISIONS.md`.

**New content slots (one example variant each):**
- `land.feature`, `land.feature_name`, `feature.closer`
- `place.district`, `place.district_name`, `place.layout`, `place.scene`

`place.whole` gains `role`. Room purposes and thing kinds gain the new buildings' ids.

**For Jb**
- What each role builds, and how roles are chosen, are first guesses. They are listed in DECISIONS.
- The scenes are ready for set pieces: a storylet can ask for "barricade", "meal left", "plague pit" and the rest.

## 2026-10-03 — Review of D01–D03 and S01

**Review** (from the D03 samples and metrics; the session couldn't build): brevity and place variety are much better and targets are met. Problems: the explorer bot rarely digs or leaves town, so samples under-show D02 and D03; arriving at a feature doesn't describe it; `read` is the longest output in the game; examines mostly return only a name (expected until D05); identical "Which do you mean" options, a dangling "Among them", raw ids, unexplained closed doors.

**Jb's concern:** the script as long lists of strokes may be too obscure. He wants it somewhat obscure, but learnable, and suggested that scraping a sign might make its sound.

**Added** `docs/milestones/S01-course-corrections.md`, to do before resuming D04: a curious explorer bot, arrivals that describe their place, reading as an impression first, signs with handles and names that compress as the player learns them, heard sounds from scraping (proposed default, a design gate), and the bug fixes.

**Revised the same day after Jb's feedback:** no arbitrary naming of signs (the only in-game names are real sounds, heard as signs are scraped off); no `signs` list (the game is not the player's notebook; exceptions must be argued for); and unknown writing is perceived in layers: an impression of the whole text, then an impression of each sign's shape, then exact strokes only when one sign is examined. Both principles added to `docs/DESIGN.md`. Writing with labels gives way to writing with heard sounds and copied signs (DESIGN-Q). Worth Jb's review against the new rule: named places (`name this place`, `go to <name>`) are a record the game keeps for the player.


## 2026-10-03 — S01 done; D04 part-way

**D04 step committed first** (great interiors, caves, mapper bot; tool placement routes each home once, site setup about 90 ms faster).

**S01 — Course corrections**
- **Reading in layers** (`crates/game/src/reading.rs`): `read` is a glance at the whole text; `read closely` (`study`, `read on`, `more`, `look closer` while reading) goes sign by sign by impression, or by sound once heard; `examine sign 4` gives a fuller impression; `trace sign 4` / `trace the stele` gives the exact strokes, 4 minutes a sign, in light, eight signs at a go.
- **Sign impressions** (`crates/lang/src/impression.rs`): outline, main stroke and only as many other marks as needed to tell every sign of a script apart; related signs told as "like that sign, with…"; about one alike pair per script on archaeologist.
- **Labels gone**, old saves load. **Heard sounds**: scraping where it's quiet, or after `listen`, lets each sign's sound be heard; it then reads by its sound. The one exception to "the game is not your notebook", recorded in DECISIONS with the alternatives. **Writing by sound**: `write kati mo on wall`, `#4` copies a sign of the last text read.
- **Curious explorer**: digs (0.22 of commands), follows features, sounds and smells, stops examining what gives nothing, leaves towns and buildings in time, avoids drops. Survives three days on 8 of 10. Features visited in ten hours: 2.6, short of the target of 5 (see DEPTH.md). The scholar reaches the deepest text on 5 of 10 again (bar restored).
- **Arrival** leads with the place set out for; features in view within 1.5 km can be gone to.
- **Bugs**: ids readable everywhere templates see them (a test checks no response has an underscore); alike options offered once; several ways one way listed by name; standouts stand alone; doors that won't move give a cause.
- **Also fixed**: drops into spaces where only held doors lead on (world); two hash picks that differed in WebAssembly (usize is 32 bits there). The bench has a `transcript` debug command for comparing platforms.
- D03 samples regenerated (old kept in `docs/samples/D03-old/`), notes rewritten; D02/D03 numbers re-checked in DEPTH.md.

**Open questions:** see "Course corrections (S01)" in `docs/DECISIONS.md`: tracing time and materials, writing by sound and `#N`, the chronicle's dots, features in reach at 1.5 km, the explorer's time split.

**New content slots (one example variant each):** `glyph.impression`, `glyph.closer`, `glyph.heard`, `read.whole`, `sign.name`, `trace.frame`, `trace.sign`, `trace.lost`, `trace.more`, `trace.dark`, `write.unheard`. Changed: `read.glyph` (impression, heard, sound), `write.done` (signs by sound or look), `say.door_stuck` (cause), `end.chronicle`. Removed: `say.define`, `say.define_bad`.

**Next:** resume D04: brevity inside great interiors (a move report adds a fact to each room arrival: facts per arrival 3.7 against 3), the lazy-generation decision, hazards, interior landmarks, samples and notes.

## 2026-10-03 — D04 done

**Done since S01:** brevity inside (a move report takes a place in the room's budget; facts per arrival back to 3); landmarks inside great interiors (vast, lofty, lone, in `room.whole` and `room.name`); bad air deep down, crawls slow with a heavy load; a cave system in every world; the explorer stays longer in great interiors, marks their entrance and follows passages; D04 samples (`scraped-lang samples D04 --inside`). Lazy generation decided against (the whole world builds in about 0.2 s in WebAssembly). Targets met (see DEPTH.md).

**Open questions:** "Great interiors (D04)" in DECISIONS: landmark traits, bad air, getting stuck, getting lost only by feel, no lazy generation.

**New content slots:** `hazard.air`, `hazard.air_hurt`, `move.squeeze`; `room.whole` and `room.name` gain `landmark`.

**Next:** D05.

## 2026-10-03 — D05 done

**Things and mechanisms:** 106 object kinds by craft, with owners, makers, eras, conditions and emblems (`crates/world/src/objects.rs`); close examination; containers; locked boxes and strongroom doors with keys bearing the owner's emblem; caches with an emblem sign; things buried by features and old maps with a cross; works (four-part machines through great buildings, worked in order by watching what moves) sealing a room; calendar doors that open on their day, shown on a calendar stone; weights marked with numeral signs; a rope climbs back up a drop. Targets met (see DEPTH.md). The scholar holds 5 of 10 (works and locks are kept out of archives and libraries; it skips examining objects).

**Open questions:** "Things and mechanisms (D05)" in DECISIONS: kinds and placement, emblems, how many locks, caches, maps, works and calendar doors, digging without a tool, other ordinary uses (pole, hook, bell, mirror) not built.

**New content slots:** `object.examine`, `object.closer`, `emblem.describe`, `object.not_open`, `object.locked`, `object.unlocked`, `object.opened`, `door.unlocked`, `cache.sign`, `cache.found`, `dig.found`, `dig.nothing`, `map.read`, `map.place`, `mech.idle`, `works.done`, `calendar.notch`, `move.rope`. `say.door_stuck` gains causes locked, works and calendar.

**Next:** D06.

## 2026-10-04 — D06 done

**A living world:** species generated per world from habitats and climate (about 60 animals and 49 plants), with roles, colours, marks, signs, calls, homes, seasons and a tolerance of regional life, under a damped hunter-and-prey cycle (`crates/world/src/life.rs`); signs on looking closer or down, roaming animals' fresh tracks, calls on listening, plants in season, sightings to a player who looks about or waits (`crates/game/src/life.rs`); forage names its plant, fishing and snares. Weather fronts that cross the map with the wind, and what they leave: flooded fords, snow-closed heights, storm-felled trees, low lakes, closing routes and showing on the land; a front on its way shows in the sky; storms thunder (`crates/sim/src/weather.rs`, `crates/game/src/skies.rs`). The sky: twelve figures named in the oldest language with a still star to steer by at night, a 30-day moon, planets, eclipses on a cycle, a comet, showers of falling stars (`crates/world/src/sky.rs`). Ten kinds of natural wonder placed by geography (`crates/world/src/phenomena.rs`). Explorer survival 9 of 10; scholar unchanged. Targets met (see DEPTH.md). Also fixed: picks that cast to usize before the modulo (wasm).

**Open questions:** "A living world (D06)" in DECISIONS: familiar body plans as ids, how shy animals are, foraging odds kept as before, no gear for fishing or snares, weather and consequence thresholds, day length still fixed, the sky's cycles, strange places counted as sites.

**New content slots:** `life.sign`, `life.home`, `life.plant`, `life.seen`, `fish.no_water`, `fish.caught`, `fish.none`, `snare.set`, `snare.waiting`, `snare.caught`, `snare.empty`, `weather.mark`, `weather.coming`, `sky.eclipse`, `sky.moon`, `sky.figure`, `sky.planet`, `sky.comet`, `sky.meteors`, `wonder.noticed`. Changed: `sense.sound` (call, role, size; sources call and thunder), `forage.found` (kinds, plant), `creature.name` (form, colour, mark, size), `travel.blocked` (flood, snow, fallen trees), weather vars gain storm and snow. New items: nuts, fungi, greens, fish, game.

**Next:** D07.

## 2026-10-04 — S02 added (review)

Review of D04–D06: targets met except those already reported (features visited, scholar). Sign impressions in `read closely` are still stroke lists ("a tail turned right, with a bar, right…"), against Jb's design; the cause is the impression model, not the wording. Added a small, contained steer, `docs/milestones/S02-sign-impressions.md`, to do after D07 and before D08.

## 2026-10-04 — D07 done

**A language for long texts:** coordination, eight subordinators, relative clauses, reported and quoted speech, comparison, quantifiers, four moods, a tense-and-aspect system per seed, ordinals and dates, all rendered and parsed back (round trip on random sentences in every era); six derivations and compounds per seed; a cultural lexicon of about 630 concepts from which each world takes 566–603 by its coast, land, rivers and species; personal names built from words and town names from their sites, each with its meaning, sound-changed across eras (`crates/lang/`, `crates/world/src/history.rs`). A difficulty dial puts aspect and mood markers in words (gentle) or affixes (archaeologist). The grammar sheet, the bench ("Longer sentences") and a new `scraped-lang long --spoil` show it. Explorer 8 of 10 (it now goes out to gather wood when cold indoors), scholar 6 of 10. Fixed along the way: a pivot over a great inscription, routes squeezing between closed cells at a corner, a hermitage on unreachable land, storylet writing that used words a world lacks.

**Open questions:** "A language for long texts (D07)" in DECISIONS: one level of clause nesting, how often markers are particles, compounds on eight heads, culture-field weights, name styles (no patronymics or epithets yet), place-name words. Attestation of each construction in readable texts waits for D08. The scholar loops on seeds 6 and 10 (many commands, few hours) without failing.

**New content slots:** none (names and sentences are generated language).

**Next:** S02, then D08.

## 2026-10-04 — S02 done

**Sign impressions as whole shapes:** an impression is now worked out from the sign as drawn (laid on a grid): proportions, curved or angular, spare or busy, symmetry, enclosed spaces, separate pieces, which way its weight leans along the line and sits, where it sits on the line, which way its lines run, its parts (dot, loop, crossing, point, curl, sweep, arch, line, saw edge) with a coarse place, and one of 31 everyday resemblances picked by rules on these (`crates/lang/src/impression.rs`). Each sign starts at proportions, curves and resemblance; a sign that reads like another gains the most noticeable feature that tells them apart. No stroke names, turns or exact places (a test over three seeds, all eras, every script kind); every sign still reads differently on gentle and standard, a few alike on archaeologist. `glyph.closer` gives the whole shape and the part that stands out. Also fixed: a zigzag turned half about looked the same but counted as a different sign. Samples in `docs/samples/S02/`.

**Open questions:** "Sign impressions (S02)" in DECISIONS: the resemblance rules, spare/busy thresholds, archaeologist's one extra feature.

**New content slots:** `glyph.part` (part, place, facing). Changed: `glyph.impression` (proportion, curve, resembles, parts, pieces, holes, symmetry, busy, leans, weight, sits, runs, like, added; no strokes), `glyph.closer` (proportion, curve, busy, symmetry, pieces, holes, resembles, distinctive from glyph.part; no main stroke or count).

**Next:** D08.

## 2026-10-04 — C01 added (shared play and versions)

Jb's decisions: trust the AI but make the engine hard to see inside (a stripped player build with spoilers compiled out, text embedded, opaque saves, no undo); the player build is public on GitHub Pages and worlds sync to a private location; a short fair-play note is shown to agents; versioning by major/minor, preferring minor. Because saves replay commands today, minor updates would change old worlds, so C01 also moves saves to snapshots. Added `docs/milestones/C01-shared-play-and-versions.md`, to do after D08.

## 2026-10-04 — D08 done

**History and what the writing says:** a second history pass on streams of its own (`crates/world/src/society.rs`): people's lives and marriages, temples, councils, guilds, courts and a school, gods and festivals, merchants' journeys, loans, shortages and gluts, works begun and finished or given up, floods, fires and earthquakes, decrees, thefts, disputes, judgements, feuds and reconciliations, gathered into stories (arcs). 27 genres built as meaning and placed where they would be kept (`crates/world/src/genres.rs`): annals, king lists, decrees copied to every town of a realm, court records quoting both sides, contracts, receipts, inventories, prayers, hymns, myths, oracles, instructions beside the kiln or loom, lessons, word lists, boundary stones, milestones, building inscriptions, graffiti, curses, blessings, calendars, and epitaphs that tell a life. Disasters leave scenes and journeys a merchant's seal. Names are said in each era; storylets can join a kind of story (`story = [...]`); `read.whole` knows a text's layout; `scraped-lang world --seed N --spoil stories` lists every story's texts. Targets met except sentence shapes on one small world (585 of 600); the scholar now reads the deepest text on 4 of 10 (was 6): it reads everything, and there is twice as much. Fixed along the way: names that sounded alike across eras, a seven-sentence limit in the parser, impossible kin ties and unborn people in old texts, arrival losing to a sighting on the same step, the pivot over held doors.

**Open questions:** "History and texts (D08)" in DECISIONS: how many stories an era holds, who judges, where genres are kept, half the common graves unmarked, how each event is worded, letters not yet pointing to hidden caches, reading order not modelled in fairness, word lists without old-and-new pairs. The scholar's pacing with so much to read.

**New content slots:** none. Changed: `read.whole` gains `form` (columns, entries, sealed, list, verses, running); `place.scene` gains scene kinds `burnt house`, `cracked walls` and causes `fire`, `earthquake`. Storylets gain `place.story`.

**Next:** C01, then D09.


## 2026-10-04 — C01 done

**Shared play and versions:** saves are snapshots now (`crates/game/src/saves.rs`): loading restores play instead of replaying it, so a minor update carries old worlds on. Each save is sealed (unreadable by eye), carries a turn count, a chain over its moves, the version that made the world and the one that last wrote it, and the builds each stretch of play ran on. `scraped-player` is a separate build (`--no-default-features`, profile `player`): terminal, JSON lines and MCP in one stripped binary with spoilers compiled out and only the templates baked in; the browser player is the same engine in wasm. It shows `docs/coop/FAIR-PLAY.md` to agents. Shared worlds (`crates/game/src/shared.rs`): a world file of the sealed save, every move with what it showed, and table talk, each tagged with who and when; the player program plays one (`--world FILE --as ai`, `talk`, `talk since N`, MCP `open_world`/`talk`/`talk_since`), refuses an older copy than it has seen, and merges copies (`merge OURS THEIRS`; a split keeps the other line as a branch). The Android app shares a world from its menu, syncs it through a private GitHub repo, shows who moved last and unread talk in the world list, and has a table-talk sheet. Versioning (`docs/VERSIONING.md`): a save corpus (`tests/saves/0.1.0/`), `scraped-lang saves check` reports the bump a change needs, CI runs it and the release workflow refuses a tag whose bump is too small; another major version won't open, an older minor one opens with a note. `scraped-lang replay SAVE --build VERSION=PATH` replays a game stretch by stretch on its own builds. Releases publish every player build with `SHA256SUMS`; the Pages site lists them (`players.html`). `docs/coop/PLAYING.md` tells an agent how to fetch the player, clone the worlds repo, play, merge and push.

**Not yet tried end to end:** Jb in the app and an AI in a cloud session on one world (the "done when"). The app's Kotlin is built and tested only in CI (no Android SDK in this environment); a scripted test covers the same turns through the engine calls the app makes and the player program's own code. No release has been tagged, so the Pages list is empty until the first.

**Open questions:** "Shared play and versions (C01)" in DECISIONS: the world made again from its seed (so generation changes are major), the fixed seal key, the GitHub repo as the sync place, default names, how splits are kept, the Android app keeping the full engine. Also found: `content lint` reports one error in `say.take_fixed` (a variant uses `cause`, which the slot doesn't have), from before C01.

**New content slots:** `say.upgraded` (from, to). New `app.label` ids: shared, share_world, moved_last, talk, talk_hint, talk_none, worlds_repo, worlds_repo_hint, worlds_repo_name, worlds_token, worlds_token_set, worlds_you, worlds_save, sync_now, syncing, synced, world_split, world_refused.

**Next:** D09.

## 2026-10-04 — D09 done (targets partly met)

**The magic, deepened:** every concept with a meaning that suggests one has a power, by principles in `crates/lang/data/powers.toml`: a verb does what it says, a noun is what a spell gives or takes away, an animal is drawn, a plant made to grow. Fifteen qualities, and each world's culture holds to one of a concept's fitting powers. Spells take their shape from their words (`crates/sim/src/writing.rs`): a target (and "this", "no", a named town), what is given or taken, degree, extent, and conditions and exceptions (when, if, until, unless: night, day, rain, winter, summer, someone entering, someone carrying the seal). Ten classes of target, each taking only the qualities that make sense for it. The old civilisation wrote its technology into its buildings (`crates/world/src/charms.rs`): larders kept cold, lamps lit at night, mills turning, wells, fields, doors that open only for the household, wards on tombs, written when each building was new and since renewed, weakened or undone by later hands, as charms, wards and invocations. Light and wetness change rooms and land, mills turn, things won't lift, beasts and plants come and go, and everything else shows as evidence (`spell.cue`). Misfired potent writing acts without the one word that doesn't fit. The scholar now reads the deepest text on 5 of 10; the writing tools never lie where writing keeps rooms dark or doors shut.

**Not met:** claim types 56.4 of 60 (three attempts); a large spell in about a tenth of regions, as most regions have no building to carry one; live spells 135 on the smallest world. The explorer survives three days on 7 of 10 (was 8): it shelters in houses an old cast keeps cold. D08's words per text fell to 13.3 (spells are one clause). Numbers in `docs/DEPTH.md`.

**Open questions:** "The magic, deepened (D09)" in DECISIONS: how a culture picks powers, what a spell acts on, degree and extent, which conditions the world can judge, misfires, how many everyday spells and what later hands did to them, household doors, large spells, spell genres, which qualities act and which only show. Also: the save corpus for 0.1.0 was regenerated, since 0.1.0 was never released and D09 changes generation (a major change). Should the first player release be tagged now, so later milestones are held to the versioning policy?

**New content slots:** `spell.cue` (quality, rising, class, thing, strength, with, with_rising, indoors), `effect.fast` (thing, how). Changed: `effect.change` takes all fifteen qualities and ten classes. New words in every language: greatly, slightly, widely, and "unless".

**Next:** D10.

## 2026-10-04 — S03 added (review)

Review of D08, C01 and D09: large and solid, but 279 of 280 text variants are still examples, so the transcripts can't yet show whether it plays well. Jb is playing tomorrow. Added `docs/milestones/S03-before-jb-plays.md`, to do before D10: tag v0.1.0 and refresh the APK so Jb has a current build, and fix the wording bugs in the D09 samples (plurals, plural feature names, exact counts in `read.whole`, related signs whose base can't be found, the `say.take_fixed` lint error, open code fences in samples).

## 2026-10-04 — S03 done (before Jb plays)

**A build to play:** v0.1.0 is released (the session may not push tags, so `.github/RELEASE` on the branch names the tag and the release workflow makes it); the release workflow publishes the player programs, the browser player and the APK, and the Pages site lists them. The save corpus for 0.1.0 was made again from this build, since nothing had been released. `content release-check` still fails on purpose (the text waits on Jb), so for v0.x tags it only reports (`DESIGN-Q:` in `release.yml`); from v1.0.0 it blocks. HANDOFF opens with where Jb plays, which version, and how to send a save back.

**Wording fixes:** every creature, plant and object name has a plural (regular by rule; irregular and unchanging names in `crates/content/data/plurals.toml`), and groups and counts use it. Feature names that are plural (rapids, standing stones, field walls…) take no article. `read.whole` says roughly how much is lost (a few, some, about half, most, nearly all) instead of exact counts. A related sign names its base by its sound once heard, otherwise by a resemblance no other sign shares, otherwise by its own impression when that is unique. `say.take_fixed` declares `cause`. The sample writer closes every code fence whatever the transcript holds. The D09 samples were regenerated; none of the bugs remain in them.

**Open questions:** the v0.x release gate above.

**New content slots:** none. Changed: `read.whole` `lost` is a coarse amount and `glyphs` is gone; `glyph.impression` gains `like_sound` and `like_resembles`; `land.feature` and `feature.closer` gain `plural`; `say.take_fixed` gains `cause`.

**Next:** D10 (in hand: its first commit is in, and the slow bots fall short with it: explorer 6 of 10, scholar 2 of 10).

## 2026-10-05 — D10 done (targets partly met)

**The slow realisation:** the tools are ordinary. Knives, pumice and old chisels scrape a little; a penknife, a mason's chisel and a graver bite deeper; a lens and a jeweller's loupe magnify. Knives, pumice and styluses lie where people kept them. `clean` (also scrub, wipe) takes moss, lichen, soot or dust off a surface by hand; with an edged or abrasive tool it takes the whole top layer, exactly as a scrape. Grime hides some signs, mostly where a spell waits. The pivot is gone: a third of everyday spells were written and never cast, falling stone and floodwater can set them loose, and a `first_accident` storylet hook marks the first. A way out without writing: the last land before the world's edge, reachable on foot on every seed, felt as you near it (`land.rim`), and `go beyond` ends the run. The end summary counts kinds of building, rooms, secrets, kilometres and the rim. Fairness asks for reachability, not a path (tool, latent, edge). A great site's air no longer names writing. In the first hour no seed brings writing forward; the explorer's first release comes from cleaning with a knife, 4.5–7.7 h in on most seeds.

**Not met:** the scholar reads the deepest text on 3 of 10 (5 after D09) and the explorer survives three days on 6 of 10 (7), after two attempts each; the bot tests now hold those lines. Two bot bugs were fixed on the way (a door hidden or held shut walked into for ever; every bearing tried while snowed in). Numbers in `docs/DEPTH.md`.

**Open:** creatures don't yet strip writing; old maps don't mark strange places and texts don't speak of writing that acted (evidence patterns, unticked); grime is common enough that cleaning becomes routine (see the D10 sample notes). "The slow realisation (D10)" in DECISIONS.

**New content slots:** `read.grime`, `clean.done`, `clean.nothing`, `say.no_beyond`, `land.rim`, `end.beyond`. Changed: `end.summary` gains kinds, rooms, secrets, walked, rim; `great.site`'s example no longer names writing. Storylet hook `first_accident`.

**Next:** D11.

## 2026-10-05 — v0.1.1

A patch release of the D10 build for Jb's first play (`saves check --against v0.1.0`: every corpus save replays identically). Released through `.github/RELEASE`. v0.1.0 had published with player builds and the APK.

## 2026-10-06 — D11 done

**Problems only writing solves:** each world has up to three sealed places (2.9 on seeds 1–10), chosen from its buildings away from the start and holding no tool. A ward at the way in ("let this gate not open", greatly) holds them; only what is live on the ward's own surface counts, so the counter-spell must be written over the ward (agreeing with it, M09), left to dry and scraped. After the first place a plain counter is not enough: it must say "greatly". A counter may carry a condition and then opens only while it holds ("when night comes"). The places form a chain: beneath each ward (the lens reads it) and inside each place, an account names the next place's word; the first's lies deep beneath its own ward (the loupe) and in one ordinary building. Where a great inscription's building holds no tool (5 of 10 worlds), it ends the chain, sealed from within (its ward on the entrance room's wall holds every way on), and its account names the town where the root inscription was cut: the thread to the deepest text. The fairness check gains a `sealed` goal that walks the chain in order (all of seeds 1–20), and counts a great inscription behind it as reachable once the chain can be walked. The scholar remembers a way in that would not give, comes back once it can write the words, and opens it: at least one on 7 of 10 seeds. It reads the deepest text on 4 of 10 (3 after D10). The sealed places' texts are numbered from `SEALED_BASE`, so old saves keep their ids: a minor change. `samples D11 --bot scholar` writes excerpts of a season.

**Not built:** the other kinds the spec names (a drowned district, a valley held in winter, a hidden library, a bridge that holds while a spell lasts); a chain that can be walked in more than one order; agreement with layers deeper than the live one. Also seen: "A graves" (plural room names take an article), and a collapsed door saying "will not move" in the same example words as a held one.

**Open questions:** "Problems only writing solves (D11)" in DECISIONS.

**New content slots:** none (the way in that won't give uses `effect.held`).

**Next:** D12.

## 2026-10-06 — D12 done (targets partly met)

**Integration and tuning:** every depth target from D01 to D11 measured together on 50 seeds (`docs/DEPTH.md`, "All together"): sixteen rows met, eight not. Of those eight, six were worked at in their own milestones and stay short (digging share 0.17, features visited, claim kinds 49.9, regions with a large spell, explorer survival 6 of 10, scholar's deepest 4 of 10); two clash with later design and go to Jb (strange places without writing against D09's saturation; words per text against one-clause spells). Fairness: every one of 50 seeds gives a fair world at every difficulty (39–42 fair as made). Performance: 1.5–2 s for a new world natively, 4.8 s in the browser (15 s with the CPU slowed fourfold), 0.05 s a command. Clients: help lists the ordinary verbs (senses, clean, eat, drink, fire, sleep, manual) and nothing about writing; the Android chips offer clean and listen; the agent's `act` examples no longer show writing. Authoring tool: the review order follows how often players meet each family; the writing inspector lists sealed places and their chain; Gaps mode already plays both depth bots. For Jb: `docs/PLAYTEST.md` (three seeds, spoiler-free play, looking under the hood, what feedback helps) and `docs/CONTENT-PLAN.md` (slot families by play, from `scraped-lang content plan`). Names already plural take no article ("graves").

**Not done:** difficulty presets were checked (every seed fair at each) but not retuned; no phone was available, so the phone figure is an estimate from the native timings.

**Open questions:** "Integration and tuning (D12)" in DECISIONS.

**New content slots:** none. Changed: `say.help`'s description and example list the ordinary verbs.

**Next:** the depth roadmap is complete. A v0.2.0 release (D11's sealed places need a minor bump), then Jb's playtest and content.

## 2026-10-06 — v0.2.0

A minor release of the D11 and D12 build (`saves check --against v0.1.1`: older saves load and carry on), with a 0.2.0 corpus save. Released through `.github/RELEASE`.

## 2026-10-07 — S04 added (review)

Reviewed D11–D12 and played v0.2.0 by hand on seed 42. The depth is there, but a person typing ordinary commands hits parser and wording bugs the bots never meet: room contents revealed one per `look`, no `take all`, examining a building walks you in, "You see no the sinkhole here", "1 hours", an empty name after `go back`, lost in daylight, related signs still with no findable base, cleaning that speaks of scraping. The D11 review fixes (uncued deadly spells, learnable counter words, no lowering bot bars) were never merged. Added `docs/milestones/S04-playing-by-hand.md` with all of these and a hand-player bot, to do before Jb's playtest.
