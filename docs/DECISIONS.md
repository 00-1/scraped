# Open decisions

Every design question still waiting for Jb, in one place: each `DESIGN-Q` marker in the code, each open question from `docs/LOG.md`, and the design gates of the depth roadmap (`docs/DEPTH.md` and the D milestone specs). Nothing here has been resolved; each row shows what the game does **today** until you say otherwise.

**How to use it**

- Read down a table. If the current default is fine, leave it; it stays as built.
- To change one, tell Claude ("make torches burn two hours", "players should have a name, not 'self'"), or edit the row's *Current default* and ask Claude to make the code match.
- *Where* points at the code (or doc) that holds the decision. *Affects* is the milestone that will build on it ("now" means it shapes the game as it stands).
- Rows marked **(accepted)** are M01–M02 questions whose defaults you accepted on 2 October; their markers stay in the code so they can be revisited.

## Language

| Question | Current default | Where | Affects |
|---|---|---|---|
| How is "made this for X" said? | A fourth case ending for "for" (a dative), not a separate word like "for" (accepted) | `crates/lang/src/morphology.rs:28` | D07 |
| How does a command look? | A bare verb with no subject; there is no command ending (accepted) | `crates/lang/src/corpus.rs:514` | D07 |
| Where do numbers, "this", adverbs and "for X" go? | Numbers and "this" sit where adjectives do; adverbs and "for X" follow the verb/object order (accepted) | `crates/lang/src/syntax.rs:74` | D07 |
| Where do titles and "child of X" go? | Always after the name, in every language (accepted) | `crates/lang/src/render.rs:345` | D07 |
| Where does the spell word sit? | Always right before the verb, whatever the word order (accepted) | `crates/lang/src/render.rs:219` | D09 |
| How are ambiguous sounds spelt in romanisation? | A single special letter (ŋ, ñ, š, ĥ…) rather than plain letters with a separator like `n'g` (accepted) | `crates/lang/src/phonology.rs:737` | now |
| How is a text presented by default? | Spaces between words, names unmarked, script chosen by seed, three eras, regular endings (accepted) | `crates/lang/src/difficulty.rs:52` | now |
| How does an abjad write a word that starts with a vowel? | One "carrier" sign marks it (accepted) | `crates/lang/src/script.rs:24` | D07 |
| Does back-and-forth writing mirror the glyphs? | No; reversed lines keep the same glyph shapes (accepted) | `crates/lang/src/script.rs:39` | now |
| Should ledgers write numbers as signs? | Every script has number signs, but texts spell numbers as words (accepted) | `crates/lang/src/numerals.rs:183` | D05, D08 |
| How fast do word endings wear away? | Besides forced changes, one ending wears away by chance in about one era in five (accepted) | `crates/lang/src/lib.rs:215` | D07 |
| How often are words replaced between eras? | About one word in 25 per era by chance, plus any forced by sound changes (accepted) | `crates/lang/src/lib.rs:238` | D07 |
| How many named people recur in texts? | 24; names recur enough to cross-check, and the same tomb can now and then appear twice (accepted) | `crates/lang/src/corpus.rs:20` | D08 |
| How long is a line of glyphs? | Always 16 glyphs, whatever the surface (accepted) | `crates/lang/src/corpus.rs:213` | D04, D08 |
| How does content show a word's glyphs? | `{lang.glyphs …}` lists glyph table numbers; no drawn or described form yet | `crates/lang/src/slots.rs:245` | now |
| Which era does the player write in? | Only the newest era's language and script | `crates/game/src/composing.rs:75` | D11 |
| Can players write personal names? | Not yet | `crates/game/src/composing.rs` | D07, D11 |

## Writing and magic

| Question | Current default | Where | Affects |
|---|---|---|---|
| Which verbs can spells use? | Only open, burn and break, with the thing as subject ("let the gate not open") | `crates/lang/src/corpus.rs:477`, `crates/world/src/history.rs:833` | D09 |
| What can each spell verb do? | Burn adds or takes 12°; open and break on doors both hold them open; break on stone makes rooms unstable | `crates/sim/data/claims.toml:91` | D09 |
| Is a box a door? | Yes: a box counts as a passage (its lid) until containers exist | `crates/sim/data/claims.toml:28` | D05 |
| How do layers stack? | All texts on one feature form one stack in date order (king lists too); only history's spells start scraped | `crates/sim/src/writing.rs:467` | D09 |
| How far does a released spell reach? | By surface: stone 900 m, metal 700, clay 500, wood and plaster 400, vellum 300 | `crates/sim/src/writing.rs:236` | D09 |
| How far does the root inscription act locally? | Within 900 m (its wider pull is through the regions) | `crates/sim/src/writing.rs:774` | D09 |
| How does a better scraper change a release? | Fine scraper or better: three times the reach; old and first scrapers also push whole regions | `crates/game/src/writing.rs:144` | D09, D10 |
| What does a spell do to a region? | Warmth ±6°; opening or sealing water ±0.3; breaking or holding ground ±0.4 | `crates/sim/src/region.rs:111` | D09 |
| Which inscriptions are "great"? | The root plus the two widest-reaching other spells in history | `crates/sim/src/region.rs:556` | D11 |
| How far do great inscriptions reach? | The root three regions out, the others two | `crates/game/src/site.rs:186` | D11 |
| How many scrapers, and can they be made? | Four, of rising power, all found, none made | `crates/sim/src/items.rs:154` | D10 |
| How much of a scraped text can be read by eye? | 60% of glyphs in daylight, 40% in dim light | `crates/game/src/writing.rs:100` | D10 |
| How much does the lens show of the layer beneath? | Lens: 35% daylight, 20% dim. First lens: 80% and 60% | `crates/game/src/writing.rs:107` | D10 |
| What does the first lens show? | Every faint layer at once | `crates/game/src/writing.rs:76` | D10 |
| How many scribal hands are there? | Six, one per author; anonymous spells get one by era and building | `crates/game/src/writing.rs:20` | D08 |
| How long does fresh writing take to dry? | Half an hour | `crates/game/src/writing.rs:16` | D11 |
| When does a player know a word well enough to write it? | After meeting it in 2 texts; partly scraped readings count; spell formulae and "and" are free | `crates/game/src/composing.rs:49` | D11 |
| What must new writing over a trace match? | The whole trace beneath, even lost words: register, roles, number and kind of noun | `crates/game/src/composing.rs:428` | D11 |
| Where can the player write? | Walls, steles, altars, niches, lintels and similar; never over unscraped writing | `crates/game/src/composing.rs:55` | D10, D11 |
| What happens when a spell is garbled? | A spell-framed text that doesn't parse hurts its writer when scraped; word-order slips are simply inert | `crates/game/src/composing.rs:161` | D09 |
| How is the scraper's discovery set up? | A "pivot" spell beside the scraper with a plain, safe effect, plus four more latent spells farther out | `crates/sim/src/writing.rs:281` | D10 |
| How common is old magic? (gate) | Several small live spells per settlement, a few in the land between, at least one large one per region | `docs/DEPTH.md:69`, `docs/milestones/D09-magic.md:5` | D09 |
| Was writing the old civilisation's technology? (gate) | Yes: everyday spells kept larders cold, lamps lit, mills turning; their ruins are much of the strangeness | `docs/DEPTH.md:70`, `docs/milestones/D09-magic.md:5` | D09 |

## World and history

| Question | Current default | Where | Affects |
|---|---|---|---|
| How big is the world? | 48 km across (two to three days' walk), not "a week's walk"; bigger costs browser time | `crates/world/src/terrain.rs:14` | D03 |
| How busy is history? | 5–8 first towns, 8–14 events per era, 3–6 notable people per town per era | `crates/world/src/history.rs:555`, `crates/world/src/history.rs:657`, `crates/world/src/history.rs:628` | D08 |
| Rivers on smooth slopes | Can still run as straight parallel lines; a better erosion pass later | `crates/world/src/water.rs` | D03 |
| How are regions drawn? | Drainage basins cut into 32-cell blocks (about 10 km); basins under 40 cells join a neighbour | `crates/sim/src/region.rs:125` | now |
| What counts as a summit? | Highest point within 1.5 km, 120 m above the land within 2.4 km; "mountain" from 700 m, else "hill" | `crates/sim/src/outdoors.rs:806` | D01 |
| Where does play start? | The most-written living town that isn't the capital | `crates/game/src/site.rs:85` | D03, D10 |
| Starting towns in hollows | Some start with nothing in view; kept as is rather than biasing towards a view | `crates/game/src/site.rs:85` | D01 |
| Buildings with the same name | Several can share a name ("the intact tomb"); players use "second tomb" | `crates/game/src/site.rs` | D01, D03 |
| How common are obstacles? | Barred doors 30%, flooded cellars 35%, raised bridges 40%, unstable rooms in damaged buildings; the starting town always has firesteel, water container, provisions, wood and pry bar | `crates/sim/src/fixtures.rs:137` | D05 |
| Can great inscriptions be blocked by rubble? | No: the way to each has been dug through | `crates/sim/src/fixtures.rs:306` | D04 |
| Where do the writing tools lie? | Scraper, stylus and lens each in an archive or temple, never in the starting town | `crates/sim/src/fixtures.rs:563` | D10 |
| Where do the stronger tools lie? | Fine scraper about halfway out, old scraper far, first scraper with the root, first lens nearly as far as anything | `crates/sim/src/fixtures.rs:599` | D10 |
| How are shortages shown? | Up to four shortage ledgers in storehouses of the most depleted regions; no migration graffiti yet | `crates/sim/src/writing.rs:392` | D08 |
| Is anyone else here? (gate) | No living people; traces of earlier explorers can appear as storylets in Jb's words | `docs/DEPTH.md:67`, `docs/milestones/D06-living-world.md:5` | D06 |

## Travel, senses and interface

| Question | Current default | Where | Affects |
|---|---|---|---|
| How far can you see? | Clear daylight 20 km, rain 3 km, fog 200 m; dawn and dusk 40% of that; night 500 m at most | `crates/sim/src/outdoors.rs:275` | D02, D06 |
| Does the sun keep you on course? | Yes: in clear daylight outside woods you never drift; drift comes with fog, rain, dark and forest | `crates/game/src/travel.rs:572` | D06 |
| How does weather work? | Three-hour spells of clear, rain or fog from local moisture; days run 05:00–21:00 all year | `crates/sim/src/outdoors.rs:247` | D06 |
| How precise are travel reports? | Rounded metres and quarter hours go to content; your text decides how to say them | `crates/sim/src/outdoors.rs:203` | D02 |
| How far does one command walk? | `head` about 3 km, `follow` up to about 12 km, unless something stops it | `crates/game/src/travel.rs:18`, `crates/game/src/travel.rs:22` | D02 |
| How dark is it indoors? | Underground rooms and indoors at night count as "dim" but readable (marker predates lamps) | `crates/game/src/site.rs:358` | D04 |
| How is the chronicle shown? | As glyph numbers or the player's own labels, "/" between words | `crates/game/src/ending.rs:433` | now |

## Survival and body

| Question | Current default | Where | Affects |
|---|---|---|---|
| How fast do needs bite? | Thirsty after 8 h, dead after 60; hungry after 16 h, dead after 240; tired after 18 h awake; cold below a felt 12°, dead after 130 degree-hours; a wound heals a level a day | `crates/sim/src/body.rs:55` | D06 |
| Does age slow healing? | Past 30, healing slows by a sixtieth a year (none at 90) | `crates/sim/src/body.rs:151` | now |
| How does temperature swing? | 13° between night and day, ±3° from day to day, with seasons on top | `crates/sim/src/env.rs:53` | D06 |
| How warm are buildings and cellars? | Underground: the yearly mean less 1°; buildings 2° above the mean, with some of the outside swing | `crates/sim/src/env.rs:449` | D04 |
| Items and their numbers | A torch burns an hour, wood gives two hours of fire, provisions last 12 h, berries 4 | `crates/sim/src/items.rs:36` | D05 |
| How much can you carry? | 15 (a torch weighs 1) | `crates/sim/src/items.rs:175` | D05 |
| What creatures are there? | Four kinds: scavengers (steal food, fear fire and noise), grazers (charge up close), predators (strike from hiding, fear fire), deep things (dark underground, fear light, stir once before striking) | `crates/sim/src/creatures.rs:34` | D06 |
| How many creatures, and where? | About two dozen outdoors, none within 2 km of the start; a deep thing in 40% of dry tomb and mine cellars | `crates/sim/src/fixtures.rs:450` | D06 |
| How dangerous are dark stairs? | One climb in three ends in a fall (one or two levels of injury) | `crates/game/src/physical.rs:1243` | D04 |
| How risky is crossing water? | Wading below twice river strength (one in ten swept and hurt); swimming deeper rivers drowns one in three; lakes too wide to swim | `crates/game/src/physical.rs:1273` | D06 |
| How thick must ice be? | 8 cm bears you; thinner breaks | `crates/sim/src/rules.rs:220` | D06 |
| How do rivers respond to a region drying or flooding? | Flow scales with the region's water, between a fifth and double | `crates/sim/src/env.rs:258` | D06 |
| How easy is foraging? | Forest 60%, shore 50%, grassland 45%, marsh 40%, scrub 35%, pine 30%, tundra 15%, desert and rock 10%; fires only outdoors or at a hearth or brazier | `crates/game/src/physical.rs:946` | D06 |

## Regions, time and endings

| Question | Current default | Where | Affects |
|---|---|---|---|
| How long is a year? | 360 days: four 90-day seasons from spring; summer 5° warmer, winter 6° colder | `crates/sim/src/region.rs:28` | D06 |
| How old is the player at the start? | 25 | `crates/game/src/trajectory.rs:14` | now |
| When does a life end of old age? | At 80 | `crates/game/src/ending.rs:28` | now |
| When is the player overtaken by collapse? | When ground and life where they stand both fall under 0.1 | `crates/game/src/ending.rs:33` | now |
| What does land naturally hold? | Water follows moisture; life follows water and warmth, best near 15° | `crates/sim/src/region.rs:277` | D06 |
| How worn is a world at the start? | Dying: a quarter less life and water; stagnant: 15% less; recovering: a tenth less | `crates/sim/src/region.rs:363` | now |
| Where is each kind of world heading? | Dying 60% of natural life and water, stagnant 85%, balanced 100%, recovering 115% | `crates/sim/src/region.rs:445` | now |
| How do regions affect each other? | Water flows downstream, life follows water and warmth and spreads, cracks spread with drought; a few percent a day | `crates/sim/src/region.rs:421` | D06 |
| What are the words for leaving? | "Let the self depart"; is "self" right, or should the player have a name? | `crates/sim/src/writing.rs:495` | D07 |
| What counts as writing yourself in? | Any other spell about the self, scraped ("let the self not depart" is the plainest) | `crates/game/src/ending.rs:195` | D11 |
| What does the deepest text say? | "The king/priest/scribe scraped the tablet. The walls did not break. The self did not depart.", padded by repeating the middle sentence | `crates/sim/src/writing.rs:495` | D08 |
| If the root can't be reached, what is the recopy? | A scraped recopy in a reachable room, as an account that never acts | `crates/sim/src/writing.rs:510` | D04 |
| Where does a previous run's legacy go? | Beneath a reachable history spell outside the starting town, in the first era's language; it never acts | `crates/sim/src/writing.rs:633` | now |
| How are changes told in the chronicle? | Life falling: "the fields burned"; water rising: "the people drank the water"; ground failing: "the walls broke"; warmth: "the sun burned"; verdict honour or fear by net change | `crates/game/src/ending.rs:585` | D07 |
| How long is the chronicle? | At most six clauses: arrival, up to two outcomes, the people's verdict, the ending | `crates/game/src/ending.rs:620` | D07 |
| Can a player leave without writing? (gate) | Yes, by a long, hard journey to the world's edge found by exploring; writing offers stranger ways out | `docs/DEPTH.md:68`, `docs/milestones/D10-slow-realisation.md:5` | D10 |

## Content and tools

| Question | Current default | Where | Affects |
|---|---|---|---|
| Are two glyph slots the right split? | One slot for a single stroke, one for the whole glyph | `crates/lang/src/slots.rs:28` | now |
| When does a slot "need more" variants? | When a player would see the same variant 4+ times in a run | `crates/game/src/coverage.rs:165` | now |
| What counts as an echo? | A phrase of four or more words shared between slots | `crates/content/src/voice.rs:123` | now |
| In what order does Review mode list families? | Opening (story, say, place, thing), early (land, travel, reading, survival…), late (tools, writing, regions, endings) | `crates/game/src/slots.rs:2142` | now |
| Which pack changes break saves? | Any change to a storylet's rules or effects; text never does | `crates/content/src/voice.rs:251` | now |
| How close must you come to an outdoor storylet? | 300 m, about one travel step | `crates/game/src/storylets.rs:68` | now |
| How many storylets per building? | One; placed in pack order, so earlier ones choose first | `crates/game/src/storylets.rs:320` | D03 |
| Where does a storylet's generated writing go? | A new stone inscription in the building's furthest reachable room, or outdoors at the spot | `crates/game/src/storylets.rs:337` | D08 |
| Where does a hook storylet's text appear? | After the command's own text; the opening replaces the wake-up; the ending comes before the summary | `crates/game/src/lib.rs:482` | now |
| Should "first…" beats ever repeat? | No, once a run | `crates/game/src/storylets.rs:469` | D10 |
| Are the bots good enough? | Simple; endings and late writing show as "never reached" until D01's bots | `crates/game/src/coverage.rs` | D01 |
| How are the player's interface labels written? | One slot (`ui.label`) with an id per control | `crates/game/src/slots.rs:1990` | now |

## Apps, release and integrations

| Question | Current default | Where | Affects |
|---|---|---|---|
| Publishing the web tools | Needs a one-time switch: repository Settings → Pages → Source: "GitHub Actions" | `.github/workflows/pages.yml` | now |
| Should players be told when a seed is swapped for a fair one? | No; unfair seeds are replaced silently | `crates/play/src/main.rs:105`, `crates/play/src/mcp.rs:76` | D12 |
| What is the first release tag? | Not chosen (`v0.1.0`?); the release gate waits for your text | `.github/workflows/release.yml` | now |
| Android signing | Until `ANDROID_KEYSTORE_B64` and `ANDROID_KEYSTORE_PASSWORD` are set, each build needs an uninstall to update | `docs/ANDROID.md:152` | now |
| Android package name | `org.scrapedagain` (permanent once published) | `android/app/build.gradle.kts:22` | now |
| Which commands do the Android chips offer? | look, exits, out, read/take/examine plus things in view, inventory, status, wait; shown as typed verbs, not slot labels | `android/app/src/main/java/org/scrapedagain/AppModel.kt:220` | now |
| Can a cloud agent play on your phone? | Local network only; the internet would need a relay service | `android/app/src/main/java/org/scrapedagain/AgentServer.java` | now |
| The backdrop's look | Lamp colour on dark, ink colour on light; a minute-long breath | `crates/android/src/atmosphere.rs:62` | now |

## Balance and difficulty

| Question | Current default | Where | Affects |
|---|---|---|---|
| What do the difficulty presets set? | Gentle: alphabet, names marked, two eras. Standard: the defaults. Archaeologist: word dividers, fused endings, four eras | `crates/lang/src/difficulty.rs:72` | D07 |
| What makes a world fair for its preset? | At least 8 anchors and 2 texts per concept; gentle needs 3 per concept and under 5% look-alike words | `crates/game/src/fairness.rs:376` | D12 |
| How many words must link oldest and newest writing? | 10 shared words | `crates/game/src/fairness.rs:39` | D07, D12 |
| How long do regions settle before play? | Two years under history's spells | `crates/sim/src/region.rs:395` | now |
| Where does ground start? | Mid "high" (0.7), so small drift isn't news | `crates/sim/src/region.rs:380` | now |
| How fast does life change? | Half a percent of the way a day (a quarter in winter), so seasons, not weeks | `crates/sim/src/region.rs:490` | now |
| How harsh can a great inscription be? | A perpetual winter can kill a region's life within a year; is that intended? | `crates/sim/src/region.rs:421` | D11 |
| How long before a player realises writing matters? (gate) | Many hours: first accidental release after 3–10 hours; nothing in the first hour points at writing | `docs/DEPTH.md:71` | D10 |

## Instruments (D01)

| Question | Current default | Where | Affects |
|---|---|---|---|
| Should the scholar bot have to survive? | No: its body is kept well, so it measures how far the late game reaches; the explorer measures survival | `crates/game/src/bots.rs` (`sustain`) | D01–D12 |
| How deep must the scholar get? | The spec asks for the deepest text on 8 of 10 seeds; it gets 5, held back by held doors whose counter-words are too rare to learn, scarce light and far-flung tools. The test holds at 5 until D07/D09 | `crates/game/tests/depth.rs` | D07, D09, D10 |
| Can a held town be opened without writing? | No: doors held by old writing give only to a written "open" claim, so in some worlds whole towns stay shut until the player learns rare words | `crates/game/src/lib.rs` (`door`), `crates/sim/src/env.rs` (`held`) | D09, D10 |
| How long does light last? | A torch an hour, a lamp four (oil refills it); deep rooms are dark | `crates/sim/src/items.rs` | D04, D05 |
| How is a landmark told apart? | By the first of its traits (shape, walls, size, setting, tallest building, roads, cover, height…) that no alike landmark within 40 km goes by, or a pair of them | `crates/sim/src/traits.rs` | D03 |
| Which words name a landmark loosely? | All its traits, and each half of a compound bearing; exact words win when several match | `crates/game/src/travel.rs`, `crates/game/src/parser.rs` | now |

## Quiet text (D02)

| Question | Current default | Where | Affects |
|---|---|---|---|
| How many facts does each response say? | Arrival 3, room entry 3, `look` 4, the end of a journey 2 (after the report and whatever stopped it), `look closer` and `look around` 6; one of each kind of fact in short responses; with nothing new, at most 2 | `crates/game/src/attention.rs` (`Response::budget`) | D03–D12 |
| What breaks through the budget? | Unstable ground, a dangerous creature close by, a great inscription | `crates/game/src/attention.rs` (`interrupting`) | D06, D11 |
| How fast do told facts fade? | Said and unchanged: a quarter of their weight for six hours, then 60% as a reminder; changed facts weigh almost as much as new ones | `crates/game/src/attention.rs` (`attend`) | now |
| Which signs show each season? | A list of evidence per season and biome (blossom, lambs, meltwater, birdsong at dawn, frost, fallen leaves…), at least 3 in each; one is picked per day and place | `crates/game/src/attention.rs` (`SEASON_EVIDENCE`) | D06 |
| Which signs show a region's state? | Water and life away from usual (cracked mud, dry stream beds, sodden ground, standing water; bare earth, withered, thick or rampant growth), unstable ground (fresh rockfalls, cracked ground), colder or warmer than usual (frost or heat haze out of season) | `crates/game/src/attention.rs` (`REGION_EVIDENCE`) | D06 |
| How does the wind blow? | It turns every six hours, at random but deterministically; smells drift downwind | `crates/game/src/senses.rs` (`wind_from`) | D06 |
| Vague counts | none, one, two, a few (3–4), several (5–8), many (9–15), dozens (16–40), a crowd (more) | `crates/game/src/attention.rs` (`vague`) | now |
| How many members does `look at the tombs` name? | Three at a time, those not yet named first, then round again | `crates/game/src/senses.rs` (`examine_group`) | D03 |
| May storylets use the season and the region's state? | Yes, as variables for conditions; the example text never says them | `crates/game/src/attention_tests.rs` | now |

## Places with character (D03)

| Question | Current default | Where | Affects |
|---|---|---|---|
| What rock lies where? | Basalt in sparse volcanic patches, granite on the heights, clay in wet lowlands, sandstone in dry country, limestone and slate between | `crates/world/src/geology.rs` | D04, D06 |
| How many of each feature, how far apart? | At most 4 of most kinds (8 springs; 5 caves, sinkholes, rapids, meadows, groves, cairns; 2 glaciers, deltas, salt flats, petrified forests), at least 4 km apart, none in a town's heart | `crates/world/src/features.rs` (`limits`) | D06 |
| How is a town's role decided? | In order: the capital; a port near the sea; a fortress where war came (size 2+); a mining camp by rock (60%); the two holiest towns; up to three refuges; a market town where three roads meet; else a farming village | `crates/world/src/towns.rs` (`plan`) | D08 |
| Which buildings does each role call for? | A list per role (palace, council hall… for a capital; harbour, lighthouse… for a port); the first always, then size + 1 more (4 more in the capital), plus a gatehouse per gate of a walled town | `crates/world/src/structures.rs` (`role_kinds`) | D04 |
| What is out on the land? | A signal station on high ground near each town that saw war, a wayside shrine a quarter of the way along long roads, a hermitage far from roads near each holy city, refuge and capital, an aqueduct from high ground to a capital that has it | `crates/world/src/structures.rs` (`place_more`) | D04 |
| How many scenes, and which? | Each town shows up to 3 + size of its events, then its last days and its place's history until it has 3 + size / 2; outside towns: battlefields, lost travellers by waystations, kept shrines, grave goods at burial mounds | `crates/world/src/scenes.rs` | D08 |
| How do scenes show? | Little weight at first glance (found by `look closer`), much when looking closer | `crates/game/src/places.rs` | now |
| How far are features noticed? | Within 300 m on arriving; within 1.5 km on `look around` (tall ones are landmarks from afar) | `crates/game/src/places.rs` | now |
| What stops a journey? | Towns, and landmarks of weight 10 or more (towers, temples, waterfalls, sea stacks, cairns no) | `crates/game/src/travel.rs` | now |
| What do the newer buildings hold? | Items by what the building was for: food and wood in stores and markets, tools and firesteels in workshops, lamps and oil in learned and holy places, torches and cloaks in forts | `crates/sim/src/fixtures.rs` | D05 |
| Interiors of the new kinds | A placeholder line of rooms from a table, a third of the optional rooms left out | `crates/world/src/structures.rs` (`planned`) | D04 |

## Great interiors (D04, in progress)

| Question | Current default | Where | Affects |
|---|---|---|---|
| Which buildings grow into great interiors? | Palaces, temples of holy cities, barracks of fortresses and capitals, libraries and archives, catacombs, mines, cisterns (as underground cities in refuges), labyrinths: at most one palace and two of each other kind per world, largest towns first; and every cave mouth (the largest become cave systems) | `crates/world/src/interiors.rs` (`great_kind`, `grow`) | D05, D09 |
| How big and deep? | Palace 380–560 spaces over levels −2 to 3; precinct 240–360; fortress 240–380; library 220–340; necropolis 280–420 to −5; mine 220–360 to −7; cistern 200–280; underground city 300–460 to −6; labyrinth 200–260; cave 220–420 to −6 | `crates/world/src/interiors.rs` (`shape_of`) | now |
| How does each era build? | A seeded grammar per era: bay module 2–4 cells, mirrored wings (65%), courtyards (50%), corridor width 1–2; styles rough-hewn, dressed stone, vaulted, brick and plaster, fine ashlar, painted; later eras block three old doors and cut new ones | `crates/world/src/interiors.rs` (`Grammar`) | D08 |
| Hidden spaces | At least 6 per great building: bays reached only by a hidden panel, each with a visible twin across its corridor, so a careful plan shows the gap; in caves, crawls behind rubble off chambers | `crates/world/src/interiors.rs` (`hide`, `grow_cave`) | D11 |
| How is a hidden way found? | `look closer` (search) in its room always finds it | `crates/game/src/interior.rs` | now |
| How fast does one walk inside? | 70 m a minute in light, 30 by feel | `crates/game/src/lib.rs` (`go_way`) | now |
| Lazy generation? | No: every interior is built with the world. The whole world builds in about 0.2 s in WebAssembly and saves are the commands typed, so lazy generation would add machinery for nothing yet. Revisit if worlds grow | `crates/world/src/interiors.rs` (`grow`) | D06 |
| Landmarks inside | In each great interior: the largest few spaces (one per 80, 2 to 6) are "vast", the two highest of the rest "lofty", and spaces whose purpose occurs only once "lone"; corridors and passages never. Given to `room.whole` and `room.name` | `crates/world/src/interiors.rs` (`landmarks`) | D05 |
| Bad air | About a third of the spaces four levels down or deeper in mines, caves, necropolises and refuges; a level of injury every 20 minutes there | `crates/world/src/interiors.rs` (`foul_air`), `crates/game/src/interior.rs` (`breathe`) | D06 |
| Getting stuck | A crawl with a load over two thirds of what can be carried takes 10 minutes more | `crates/game/src/lib.rs` (`go_way`) | D05 |
| Getting lost inside | Only by feel: in the dark a move gives its direction but no distance. Moves in light are honest, so a careful map comes out true; drift inside (as outdoors) is left for later | `crates/game/src/lib.rs` (`move_report`) | D06 |
| A cave system in every world | If no cave mouth is large enough, the largest land cave mouth grows as a system | `crates/world/src/interiors.rs` (`grow`) | now |
| How long does the explorer stay inside? | 45 minutes in an ordinary building, 3 hours once it has seen more than eight spaces; it marks a great interior's entrance and walks back to it by name | `crates/game/src/bots.rs` | now |


## Course corrections (S01)

| Question | Current default | Where | Affects |
|---|---|---|---|
| **The exception to "the game is not your notebook"** | Sounds heard while scraping attach to their signs: from then on a close reading shows a heard sign by its sound, romanised, instead of its look. Nothing else is kept for the player (no sign list, no labels). Without this, matching a heard sound to one of dozens of described shapes would be unreasonably obscure | `crates/game/src/writing.rs` (`hear_signs`), `crates/game/src/reading.rs` | now |
| When are signs heard? | While scraping, where nothing louder than faint can be heard, or anywhere right after `listen`; every sign of the scraped text that has a sound | `crates/game/src/writing.rs` (`hear_signs`), `crates/game/src/senses.rs` (`quiet`) | D06 |
| How is a sign's impression made? | From its strokes: outline (round, tall, wide, angular, slight, plain), the heaviest stroke and its turn, then the rarest other marks, their turns and the main stroke's place only as far as needed to tell every sign of the script apart; a sign that is another plus one or two marks is told as "like that sign, with…"; a resemblance where one fits (eye, wheel, crook, comb, arrow, seed) | `crates/lang/src/impression.rs` | now |
| Confusable signs on archaeologist | Detail capped at the outline, main stroke and one other mark: about one alike pair per script | `crates/lang/src/impression.rs` (`impressions`) | now |
| How long does tracing take? | 4 minutes a sign; a whole text is traced 8 signs at a go and carries on where it left off; needs the light reading needs; lost signs can't be traced. No materials for now (charcoal, cloth or paper for rubbings could come in) | `crates/game/src/reading.rs` (`TRACE_MINUTES`, `trace`) | D05 |
| Short texts | 6 signs or fewer go straight on to a close reading after the glance | `crates/game/src/reading.rs` (`SHORT`) | now |
| How does the player write without labels? | Words by their sounds (`write kati mo on wall`), spelled as the script of the day spells them, signs without a sound (an abjad's vowel carrier) added by the spelling; every sounded sign must have been heard. Signs not heard are copied in from the last text read by number (`#4`), at the cost of tracing them; name markers and dividers too | `crates/game/src/composing.rs` (`spell_written`) | D11 |
| How does the end chronicle show? | Each sign by its sound where heard, else a dot, `/` between words | `crates/game/src/ending.rs` (`chronicle_glyphs`) | D12 |
| What does writing echo? | Each sign written by its sound where heard, else its impression | `crates/game/src/composing.rs` | now |

| Can a feature seen at a distance be gone to? | Yes, by name, within 1.5 km (the `look around` range); close by is 300 m | `crates/game/src/places.rs` (`place_targets`) | D06 |
| How does the explorer split its time? | About four buildings of a town once it has a firesteel and a cloak; 45 minutes in any building; then features and landmarks it has noticed, odd ones (pillars, stones) before heights and towns in the morning | `crates/game/src/bots.rs` (`TOWN_BUILDINGS`, `BUILDING_MINUTES`) | now |
| Ids in text | Templates see ids with spaces ("pry bar"), in variables, enum values and condition words alike; code keeps its ids | `crates/content/src/slot.rs` (`readable`) | now |
| Several ways one way | Each is listed by its name ("the western door north") among the exits; one alone by its direction | `crates/game/src/lib.rs` (summary) | now |
| Alike options | When "which?" options read the same, one of each is offered; if only one is left, it is taken | `crates/game/src/lib.rs` (`with_target`) | now |
| Why a door won't move | Rubble in the doorway, the space beyond fallen in, or not a door at all (`say.door_stuck`'s `cause`); held by writing shows nothing (`effect.held`); barred from the far side gives a finger's width (`hazard.barred`). Locked, stuck and swollen doors wait for D05's keys | `crates/game/src/lib.rs` (`door`) | D05 |
| Can a drop trap the player? | No: each building is checked as if writing held every door shut, and a drop that lands where only doors lead on is made climbable | `crates/world/src/interiors.rs` (`trapped`) | D04 |

**Other ways to learn sounds** (recorded for Jb, hooks only, not built):
- *Names from nature:* some animals and birds (D06) named after their calls, with a carving or label pairing the creature and its written name.
- *An earlier decipherer's notes:* traces of a previous explorer (the D06 gate), giving some sound values, some wrong, as storylets in Jb's words.
- *Acoustic places:* a whispering gallery or ringing stones tied to inscriptions.

## Things and mechanisms (D05)

| Question | Current default | Where | Affects |
|---|---|---|---|
| Which objects, and where? | 106 kinds in 18 families (vessels, tools, coins, seals, measures, jewellery, figurines, games, instruments, weapons, armour, clothing, lights, boxes, the sky, medicine, writing, everyday), placed by each room's purpose: kitchens vessels, treasuries coins, shrines figurines and so on; up to three a room, fewer in poor or ruined places | `crates/world/src/objects.rs` (`KINDS`, `families_for`, `place`) | D08 |
| Whose is it? | Temples' things are the temple's; houses' the household's family line (its eldest known member); elsewhere the town's people, or a family; about half carry their owner's emblem; a third of things in towns with smiths carry a maker's mark | `crates/world/src/objects.rs` (`place`) | D08 |
| Emblems | A motif (24), a device (6) and a border (5), fixed by the seed and the owner; an era's things share a border style, so a player can date them | `crates/world/src/objects.rs` (`emblem`) | D08 |
| Locks and keys | About one coffer or casket in three locked, its key (with the owner's emblem, like the lock) in another room or house of the same owner, never in a strongroom; the door into a palace's or temple's strongroom locked; none in archives or libraries, and nothing the player must find lies behind a lock | `crates/world/src/objects.rs` (`hide_and_lock`) | now |
| Caches | One per town, under a floor stone or in a wall of a house or temple room, the owner's emblem cut beside it as its sign; looking closer finds it | `crates/world/src/objects.rs`, `crates/game/src/objects.rs` (`cache_facts`) | now |
| Buried things and old maps | Up to four a world: something buried by a feature within 12 km of a town with a library, archive, palace or temple, and a map of that town's era there with a cross on it; digging needs no tool and takes half an hour | `crates/world/src/objects.rs`, `crates/game/src/objects.rs` (`dig`, `read_map`) | D06 |
| Works | In up to three great buildings of 40+ spaces: water works (valve, mill wheel, gear lever, winch) in cisterns and palaces, weight works (counterweight, gear lever, hoist, winch) elsewhere, spread through the building in working order; the last part raises the gate of a dead-end room | `crates/sim/src/fixtures.rs` (`place_works`), `crates/game/src/works.rs` | now |
| Calendar doors | In up to two temples: a dead-end room's door opens only within a day of its festival day, shown as a deep notch on a calendar stone by the entrance; a year is four seasons of 90 days | `crates/sim/src/fixtures.rs`, `crates/game/src/works.rs` | D06 |
| Measures | A weight shows a numeral sign of its era (as an impression) and its heft in units of the lightest | `crates/game/src/objects.rs` (`weight_mark`) | D07 |
| Ordinary uses | A rope climbs back up a hole or shaft; other uses (pole, hook, bell, mirror) not yet | `crates/game/src/lib.rs` (`go_way`) | D06 |

## A living world (D06)

| Question | Current default | Where | Affects |
|---|---|---|---|
| Is anyone else here? | No living people (the gate's proposed default); earlier explorers' traces are left to Jb's storylets | — | D08 |
| Which species? | Body plans from a vocabulary of about 90 animals and 95 plants, each suited to habitats (open, woods, pine, wet, dry, cold, fresh water, sea, deep) and climates; each habitat of 25+ cells holds a set number of each role (one more of its first role, birds and flowers when it covers a fifth of the land). Familiar body plans (deer, birch) are ids: templates may name them as this world's beasts and plants | `crates/world/src/life.rs` (`ANIMALS`, `PLANTS`, `roles`) | now |
| Traits | Each species a colour and a mark; animals two signs of their role, a call, a home, when they are about; migrants come in spring and summer (in warm lands, autumn and winter); plants flower and fruit in set seasons and are good for one thing (food, fibre, dye, fuel, remedy) or harmful | `crates/world/src/life.rs` (`habits`, `plant_habits`) | now |
| Populations | A damped hunter-and-prey cycle per habitat, stepped by season, with the region's life as the land's capacity; each species has a tolerance of regional life below which it is gone; about one in eight is gone at the start and returns only where a region recovers past its start | `crates/world/src/life.rs` (`Cycle`), `crates/game/src/life.rs` (`life_here`) | D09 |
| How shy | Birds and insects are seen readily by someone looking about; grazers now and then far off; hunters almost never. Signs show on looking closer or down; the animals roaming nearby leave fresh tracks where the ground holds them, a warning before they are seen; waiting outdoors (up to three hours) may bring one out | `crates/game/src/life.rs` (`boldness`, `life_facts`), `crates/game/src/trajectory.rs` (`wait`) | now |
| Foraging, fishing, snares | Forage keeps the old odds by land and region (the plants say what is found: berries, nuts, fungi or greens, all a four-hour meal); only safe plants are taken. Fishing needs no gear, takes 90 minutes by water and catches what lives there (a third as well in winter); a snare takes 20 minutes and can hold a small animal after six hours. No cooking | `crates/game/src/life.rs` | D10 |
| Weather | Fronts form each week: two to five rain fronts by the land's wetness (one more in autumn and winter), storms mostly in autumn and winter, fog banks in spring and autumn, heat spells in summer; they cross the map with the prevailing wind. Rain falls as snow where the day's mean air is below 1 °C | `crates/sim/src/weather.rs` (`fronts`) | now |
| Consequences | A ford floods after 5.5 hours of rain in two days (storms count double); snow lies at 3 cm an hour (5 in storms), melts a fifth of a cm an hour per degree above freezing, and closes high ground at 20 cm; storms bring down trees in one forest patch in twenty they cross, lying two weeks; lakes draw back after under four hours of rain in thirty summer days; frost ice is as before. Forced weather (tests) closes nothing | `crates/sim/src/weather.rs`, `crates/sim/src/env.rs` (`closed`) | now |
| Day length | The clock still runs dawn 05:00–21:00 all year; shorter and longer days are evidence only | `crates/sim/src/outdoors.rs` (`time_of_day`) | D12 |
| The sky | Twelve figures of stars named in the oldest language after things it has words for, one holding the still star in the north; three figures to a season, high in the south at midnight; a 30-day moon rising with the sun when new and at sunset when full; three to five planets moving through the figures, the innermost an evening or morning star; eclipses every five to seven moons (the sun's at new moon midday, the moon's at the next full moon); a comet every 300–700 days for 20; two showers of falling stars a year | `crates/world/src/sky.rs` | D07, D08 |
| Steering by night | On a clear night outside the woods the still star keeps the heading true, as the sun does by day; a near-full moon lets the eye reach 1.5 km | `crates/game/src/travel.rs` | now |
| Natural wonders | Marsh lights, booming dunes, tidal bores, steam vents, echoing gorges, mirages, aurora, glowing shores, fogbows, singing arches; at most two of each (one aurora), 4.5 km apart, each where the land makes it and shown at its time | `crates/world/src/phenomena.rs`, `crates/game/src/skies.rs` | D09 |
| Strange places | Counted as sites: each live spell and each natural wonder (before D06, every structure within a spell's reach) | `crates/game/src/depth.rs` | D09 |

## A language for long texts (D07)

| Question | Current default | Where | Affects |
|---|---|---|---|
| How deep can clauses nest? | One level: a main clause may hold one relative, adverbial or reported clause, but those hold none; long texts chain sentences instead (deeper nesting makes attachment ambiguous in many word orders) | `crates/lang/src/parse.rs` (`CLAUSE_DEPTH`) | D08 |
| Word order of the new constructions | Correlated with verb-final or not: "that" faces the verb, linking words stand between the two clauses, relative clauses on the adjectives' side with the relative word closing them on the far side, the compared-with phrase outside the adjectives; adverbial clauses mostly first | `crates/lang/src/syntax.rs` | now |
| Aspect and mood | A quarter of languages mark tense only; the rest tense plus one aspect (perfective, imperfective or habitual). Optative, conditional and question each have a marker; commands are a bare verb. Each marker is a particle about 30% of the time; the difficulty dial makes them all words (gentle) or all affixes (archaeologist) | `crates/lang/src/morphology.rs`, `crates/lang/src/difficulty.rs` (`Markers`) | now |
| Derivation | Six affixes (agent, place, instrument, abstract, diminutive, adjective-from-noun), always affixes next to the root; compound order drawn freely per seed (the real tendency, head-last in verb-final languages, isn't followed) | `crates/lang/src/morphology.rs` | now |
| Compounds | Formed on eight heads (stone, road, gate, house, field, water, song, day) with modifiers of fitting kinds; a language has every such compound whose parts it names | `crates/lang/src/concepts.rs` (`COMPOUNDS`) | D08 |
| Which words a world has | A core every language has, plus five culture fields: two rich and three partial, drawn by weight (10 each plus the world's coast, open land and rivers), plus its species. 566–603 concepts per world | `crates/lang/src/lib.rs` (`Culture`), `crates/lang/data/lexicon.txt` | D08, D09 |
| Personal names | Two words compounded (45%), a word and a derivation (20%) or either (35%) per world; patronymics, clan names and epithets are not yet built | `crates/lang/src/names.rs` | D08 |
| Place names | Heads from the site (ford on a river, shore by the sea, hill on high ground…), modifiers from colour, age, size, holiness and a few things of the land | `crates/world/src/history.rs` (`place_words`) | now |
| Dates | "In the Nth year/day of …" with ordinals as the numeral and an ordinal word; in the language now, used by texts from D08 | `crates/lang/src/meaning.rs` (`Role::Time`) | D08 |
| Storylet writing | A storylet's inscription picks among core nouns only, so every world has the word | `crates/game/src/storylets.rs` | now |

## Sign impressions (S02)

| Question | Current default | Where | Affects |
|---|---|---|---|
| Which features, in what order of noticeability | Proportions, curves and a resemblance always; then as needed: the part that stands out, pieces, enclosed spaces, symmetry, spare or busy, lean, weight, where it sits on the line, which way its lines run, further parts, and last which way the parts face (forward/back along the writing, high/low, standing/lying) | `crates/lang/src/impression.rs` (`FEATURES`) | now |
| Resemblances | 31 everyday things (eye, wheel, shield, lamp, egg, key, crook, fish-hook, tree, star, comb, ladder, doorway, bridge, moon…), the first whose rule fits | `crates/lang/src/impression.rs` (`resembles`) | now |
| Spare or busy | Drawn stroke length plus 15 per stroke: up to 90 spare, from 160 busy | `crates/lang/src/impression.rs` (`shape`) | now |
| Archaeologist | Each sign gains at most one feature beyond its proportions, curves and resemblance, so a few pairs read alike | `crates/lang/src/impression.rs` (`impressions`) | now |

## History and texts (D08)

| Question | Current default | Where | Affects |
|---|---|---|---|
| How much history beyond kings and wars | A second pass on its own random streams (so the older history stays as it was): per era, two to five new people a town (merchants, builders, healers, smiths, families, marriages), a temple in every town, a council and a guild in larger ones, a court in the largest, a school in the capital, three to five gods, and stories: 3–5 thefts tried, 2–3 feuds, 2–3 works, 1–2 disasters, 2–3 merchants' journeys, 1–2 omens, 2–3 pairs writing letters, a few lives, festivals and decrees | `crates/world/src/society.rs` | now |
| Who judges | The court's head; where a town has no court, the elders; where no elders, the priests; never one of the parties | `crates/world/src/society.rs` (`court_near`, `trial`) | now |
| Which genres, where | 27 genres, each kept in fitting buildings (annals in archives, decrees in council halls and copied to every town of the realm, court records in courthouses, instructions beside the kiln, loom or anvil they explain…), else in the town's archive, temple or a house | `crates/world/src/genres.rs` (`home`) | now |
| Unmarked graves | Half the common folk's graves bear no epitaph (tombs were 59% of all writing) | `crates/world/src/history.rs` | now |
| How each text is put | Each event's clause varies by the event (a title here, a word of colour, a word of when, the aspect), the same in every copy | `crates/world/src/genres.rs` (`vary`) | now |
| Names across eras | A name of words is said with each later era's words; a coined name carries the sound changes; no new name may sound like an older one said anew | `crates/world/src/lib.rs`, `crates/world/src/history.rs` | now |
| Stories and physical evidence | Disasters leave a flood line, a burnt house or cracked walls in their town; a merchant's seal lies in a store of the town he traded with. Letters do not yet point to hidden caches | `crates/world/src/scenes.rs`, `crates/world/src/objects.rs` | D10 |
| Fairness for constructions | Every construction a story's texts use is met in at least the learning threshold of readable texts; the order of reading is not modelled | `crates/game/src/fairness.rs` | D11 |
| Text layout | `read.whole` gets the layout (columns, entries, sealed, list, verses, running) from the genre | `crates/game/src/reading.rs` (`form`) | now |
| Word lists | List one semantic field's words; pairing older and newer forms of a word is not built | `crates/world/src/genres.rs` | D09 |

