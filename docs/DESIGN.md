# Scraped Again — Design Doc

Living design document. Source of truth for intent; code should follow it, and changes to design should be agreed with Jb and recorded here.

## Concept

Scraped Again is a text-only exploration game about deciphering a lost, procedurally generated language in a forgotten world where writing has power.

- **Title:** *palimpsest* comes from Greek *palin* (again) + *psao* (to scrape). The title is the definition.
- **Interface:** output is text only; input is simple typed prompts.
- **Tools required:** the game is effectively unplayable without external tools: pen and paper, software, or an AI agent. It must always be solvable by a patient human with a notebook.
- **Agent-friendly, not agent-dependent:** easy to hook an agent up to, but designed for humans first.
- **Spirit:** classic story-driven text adventures, with roguelike depth, procedural worlds and the ability to change the environment.

## Writing is background, at first

Decided by Jb (2026-10-02), and it overrides anything below that suggests otherwise.

The world must be genuinely interesting and rewarding to explore in its own right. Writing is the core mechanic, but a player must be able to play for a long time without knowing that, because there are plenty of other intriguing things to find, survive and work out. To a new player the writing is background flavour: marks on old stones, like the ruins around them. Over time the player may notice that there is more to it, and give it a second look.

What this means in practice:

- **Nothing points at the writing.** No tutorial, no announcement, no item that is obviously "the magic tool", no scripted first spell placed beside it.
- **The world rewards exploration without writing.** Places, objects, mechanisms, creatures, weather, the sky, the history visible in ruins: each is worth exploring for its own sake, with its own mysteries and its own payoffs.
- **The strangeness comes first, unexplained.** Old spells shape the world everywhere, and at first they are indistinguishable from the world's other oddities. Their cause is something the player may work out later, from patterns in their own notes.
- **Discovery is gradual and the player's own.** The realisation that writing acts should emerge from evidence (a strange place always has scraped writing nearby; cleaning a stone to read it changed something), not from a scripted moment. Different players should realise at different times, and some may never.
- **Tools are ordinary first.** What can scrape is first a knife, a chisel, a pumice stone, used to cut, pry or clean. What can magnify is first a lens for starting fires and reading small marks. Their deeper use is discovered, not announced.
- **A run without writing is still a run.** A player who never learns the mechanic can still explore, survive, map, solve mechanisms, piece together the history, and end their time in the world in a meaningful way.

## Say little; let the player dig

Decided by Jb (2026-10-02). As the world gets richer, the text must not get longer.

- **Short and intriguing.** Each response is a few lines: the one or two things that matter most right now, said so the player wants to know more. Never a wall listing every element of the scene.
- **Detail is earned.** More detail comes from digging in: examining, looking closer, listening, smelling, touching, waiting, coming back at another time. The world holds far more than any single response shows.
- **Infer, don't state.** The game rarely states facts the player could work out. It doesn't say "it is spring"; there is blossom, and lambs, and meltwater. It doesn't say "you are hungry" every turn; the player notices weakness, or checks themselves.
- **Senses are actions.** Sound, smell, touch and close looking are things the player does (`listen`, `smell`, `touch`, `look closer`), not lines appended to every description. Something only intrudes unasked when it's loud, sudden, new or dangerous.
- **Change over inventory.** What's new or different gets mentioned; what's the same as last time stays quiet unless asked about.
- **The body speaks rarely.** Needs show as sensations when they cross a threshold, and as detail when the player checks themselves, not as a status line.

## The game is not your notebook

Decided by Jb (2026-10-03).

- **The player keeps the records.** The game keeps no lists of what the player has learned or seen: no sign lists, no glossaries, no journals of clues. Tracking, cross-referencing and mapping are the player's work, on paper or with their own tools. That need is part of the game (see "Tools required" above).
- **Help only where obscurity would become unreasonable.** There is no exact rule. Each case where the game does keep something for the player must be argued for and listed in `docs/DECISIONS.md`. Current cases: a sign's heard sound stays attached to it (below); travel reports give rough bearings and distances.
- **No arbitrary labels.** The game doesn't let the player attach their own names to signs. Their guesses live in their notebook. The only names a sign takes in the game are what it truly sounds like, learned from the world.

## How the script is perceived

Decided by Jb (2026-10-03). Unknown writing is experienced the way a person would experience it, in layers:

1. **The whole text, at a glance:** an impression of the mass of writing. How much, in what lines, how cut or painted, how worn, perhaps that some shapes recur. Never a list of signs.
2. **Looking closely:** an impression of each character in turn. A shape you could recognise again (tall and hooked, a ring like an eye, a squat cross with a dot), consistent for the same sign every time, so a player can learn to spot repeats themselves. Not a geometric specification.
3. **Examining one sign** gives a fuller impression of that sign: still how it looks, not how it's built.
4. **Tracing a sign is a task in itself.** Breaking a sign down into its exact strokes, precise enough to draw, is a deliberate piece of work, like taking a rubbing or making a careful copy. It takes time, needs good light and a legible sign, and is something the player chooses to sit down and do. Only tracing gives the stroke-by-stroke description.

A sign whose sound the player has heard (see "Heard sounds") is shown by that sound from then on.

### Heard sounds

Signs make their own sounds: as a sign's strokes come away under a blade, it gives its sound, faintly. A player who is listening hears it, and from then on that sign is shown by its sound, romanised. This is the one way signs get names in the game, and it is real: the sound is the sign's true value. Scraping costs legibility, so learning sounds and keeping texts readable pull against each other. (Details and alternatives in `docs/milestones/S01-course-corrections.md`.)

## The three laws

The world runs on three rules about writing.

1. **Writing holds power.** Words are effectively spells, but fresh writing is latent and does nothing on its own.
2. **Scraping releases it.** Scraping writing away, always in full, brings its effect into being, visibly in the environment.
3. **Nothing is lost.** Writing can be scraped and overwritten, but never destroyed. A close enough reader can always piece together what was there.

### Core verbs

- **Read:** easy if unscraped; only partial if scraped. Recovering more of scraped text is where decipherment and deeper reading progress.
- **Write:** only possible once you understand the words, and constrained by what lies beneath.
- **Scrape:** always the whole text; releases its effect.

There is no partial scraping. Precision lives in what you write, not in how you erase.

### Tools

The player starts with no tools. Unscraped text is read freely, and the top scraped layer is partly visible by eye alone. Other verbs are unlocked by finding tools.

- **Reading deeper:** no tool for the top layer. A later tool lets the player read one layer deeper, revealing what can be written over what.
- **Scraping:** anything with an edge or abrasive surface can scrape, and the player will scrape for ordinary reasons (cleaning moss or soot off a stone to read it). Better scrapers bite harder and reach older, stronger writing. Possibly surface-specific (vellum, stone, metal).
- **Writing:** inks or styluses, probably last, since writing needs understanding anyway.

Tools can carry story: a scribe's knife bearing its owner's mark, or a lens left by an earlier decipherer.

### Game loop

The loop below is the *possible* arc, not a sequence the game steers the player through (see "Writing is background, at first").

1. Explore the environment, for its own sake: places, objects, mechanisms, creatures, the land and sky.
2. Notice writing everywhere, mostly as background; some of it scraped, partly legible.
3. Notice strange things in the world, alongside ordinary mysteries. Nothing says which are which.
4. Perhaps notice the pattern: strangeness and scraped writing go together. Connecting text to effect takes work.
5. Perhaps scrape a stone for an ordinary reason, and see something change.
6. Learn enough of the language to write, and write and scrape your own text.
7. Read deeper layers, revealing what can be written over what.

The early phase works as a two-sided cipher: partial text hints at its effect, and the effect hints at the text.

These laws explain the world itself: it is full of half-scraped writing because past people cast with it, so faded traces are spells in effect. Unscraped writing found in the world is a spell waiting to be released; scraping one and watching the environment change is a natural way for players to stumble on the mechanic. Every gap is evidence of a choice someone made, whether censorship, rivalry, maintenance or war.

A candidate central mystery: the civilisation fell because of the mechanic, editing its world until the text became incoherent.

## Layers

Only the top layer of writing is live; older layers sit beneath it as faint traces, and the player's ability to read them grows over the game.

### Reading deeper as progression

Perception improves in stages, for example:

1. Noticing that traces exist at all.
2. Reading the most recent scraped layer.
3. Reading older, fainter layers.
4. Telling different hands apart: scribes, eras, factions.

Tools, techniques or deeper understanding of the script could each unlock a stage. Each overwriting makes older traces fainter, so the deepest history is the hardest to recover.

In text output, traces might appear as their own register: `DO NOT ⟦faint: OPEN⟧ THE LOWER GATE`, or in prose: *beneath the newer hand, ghostly strokes curve where a word once stood.*

### Old text constrains new text

What lies beneath governs what can be written over it. Some possible rules:

- **Agreement:** new words must agree grammatically with surviving ghost words (case, number, class), as if completing an old sentence.
- **Reused strokes:** new writing must fit the marks left behind, so only words sharing a root or consonant skeleton can be written there.
- **Register:** a sacred or legal inscription only accepts writing of the same kind.

This makes reading the ghosts essential before writing, and means each surface offers a different, limited set of possible edits.

### Words from above, rules from below

The top scraped layer teaches vocabulary; the layer beneath teaches where those words can be written. Early on, players collect words they cannot yet use. Reading deeper turns that knowledge into ability.

- **Recursive:** each layer constrains the one above, so depth of reading is effectively the player's power level.
- **Inference before sight:** players can guess a lower layer's rules from patterns in where words appear, then confirm or overturn them once they can read it.
- **Every surface is a puzzle:** identical-looking surfaces may accept different words because of what lies beneath.
- **Obtuse by design:** no softening for new players. Going in cold, players mostly explore, seeing faded writing everywhere without knowing why. Effects can show long before causes are understood (a door that won't open, a stream flowing uphill past an inscription), so a player's early notes later become evidence once they realise writing acts.

## The language

The language is built the way conlangers build real languages, not as random strings, and each stage becomes a step in the generator.

1. **Sounds:** a plausible sound inventory and rules for how syllables form.
2. **Script:** alphabet, syllabary or word-signs. Glyphs are described in words, never shown: as impressions, and as exact strokes only when the player sets out to trace a sign (see "How the script is perceived").
3. **Word-building:** how roots and affixes combine. Clean, regular affixes are most decipherable; messier forms suit hard mode.
4. **Grammar:** word order and how plurals, tense, possession and so on are marked.
5. **Vocabulary from culture:** generate the civilisation first, so its lexicon reflects what it cared about.
6. **History:** evolve a proto-language through regular sound changes. Each era becomes a palimpsest layer.

**Meaning first, then form.** The game builds a sentence as meaning and renders it through the grammar. It always knows the true meaning, so it can check guesses and parse player-written words.

**Decipherment anchors:** repeated text types (tomb formulae, ledgers, laws), numerals, bilingual fragments and context clues.

**Generativity over lookup:** grammar knowledge should let players build words they were never shown. Learning a word should not simply open its matching door.

References: Mark Rosenfelder's *Language Construction Kit*; sound-change tools such as Lexurgy; generators such as Vulgarlang. Prior art in games: *Heaven's Vault*, *Chants of Sennaar*.

## Authorship

The game's personality comes from hand-authored writing; procedure supplies variety. No AI-written prose at runtime.

- **Split by layer:** the narrator, room descriptions and explorer's voice are hand-written. Layout, the language and where fragments lie are generated. Ancient texts are the right place for procedural output, since they should feel alien.
- **Templates, not paragraphs:** hand-written fragments with slots that recombine (Tracery-style), as Caves of Qud does for its histories.
- **Set pieces placed procedurally:** self-contained hand-written events and locations slotted into the generated world (the Fallen London storylet model).
- **Authored spine:** hand-written opening, frame story, key revelations and ending.
- **Taste in the parameters:** which sound aesthetics, cultures and kinds of loss can exist.

Damaged, partial text naturally hides procedural seams.

## Player artefacts

The game has no visuals, but a completed run should leave the player with a compelling, unique notebook of their own making.

- **Drawn script:** glyphs described compositionally (*a hooked stroke over two dots, closed by a bar*), so every player draws their own alphabet.
- **Maps that reveal:** consistent directions, height and landmarks by bearing, so careful maps expose hidden rooms or alignments.
- **Diagram-shaped knowledge:** dynasties as family trees, rewritings as timelines, word families as root trees, sound changes as charts. Spread across fragments so the insight happens in the diagram.
- **Unique per run:** procedural worlds mean no wiki can do it for you. Players can share notebooks alongside seeds.

Every artefact must pay off, never busywork: a map exposes something, a grammar table lets you build a word you need.

A later run might also contain faint traces of a previous run's writing.

## The world

The loop only matters if the environment gives reasons to change it. This is the largest piece of work still to flesh out.

- **Systemic, not scripted:** each place has a few physical properties (temperature, light, water and flow, sound, stability) that interact. Cold freezes water, ice can be crossed, heat warps wood, noise disturbs things.
- **Needs arise naturally:** crossing a river, reaching a high window, keeping a lamp lit. Writing is one way to push properties, alongside ordinary actions like opening sluices.
- **Lots to do before the realisation:** exploring, mapping, ordinary mechanisms, alternative routes, remnants of daily life, and a larger mystery (who you are, why you're here, where you're going). The writing is background texture for hours.
- **Ordinary writing too:** signs, ledgers, letters, graffiti and labels in the same language, with no power. This is where most decipherment happens. Potent writing is a special case (register, ink or surface) that players learn to recognise.
- **Aftermath:** the world's strangeness is the accumulated result of past writing, a puzzle box built by earlier hands.

Systemic properties also suit procedural generation: places can be generated as property sets and obstacles.

### World generation

A full procedural world map with biomes and structures, in the spirit of Minecraft, presented and interacted with only through text.

1. **Terrain and climate:** noise-based height, temperature and moisture, producing biomes.
2. **Water:** rivers following terrain downhill; lakes and coasts.
3. **History:** where the civilisation settled, traded and fought; which places belong to which language era; where writing was used and what it did.
4. **Structures:** settlements, roads, temples, archives, placed by history rather than at random.
5. **Present state:** decay, plus lingering effects of old spells (a lake frozen by an ancient inscription).

**From continuous world to text:** extract a navigable layer. Outdoor areas are coarse and moved between by direction, described by biome, features and distant landmarks. Structures are dense interiors, room by room. Travel works at several scales (region, area, room).

**Cautions:** open terrain gets monotonous in text, so feature density matters more than size. A bounded world (island, valley, basin) keeps pen-and-paper mapping tractable.

### Movement

Free, intent-based movement over a continuous world, governed by one rule: **you can only be surprised by what you couldn't see.** Short moves within sight are predictable; long moves through the unseen carry risk.

- **Continuous position:** the character has a real location on the generated map; text is only the view.
- **Intent commands:** `go to the tree`, `follow the river upstream`, `head north`, `return to the ruined tower`, `retrace my steps`.
- **Options from perception:** `look` lists salient landmarks (scored by size, contrast, distance) and followable edges (roads, rivers, treelines, cliffs) from the real map.
- **Simulated travel:** long moves resolve step by step. Things en route may intercept; losing sight of landmarks in fog, darkness or dense forest causes drift, which can lead somewhere unexpected.
- **Mappable:** travel reports bearings and rough distances ("roughly north-east for about an hour"), so players can dead-reckon.
- **Naming places:** players can name locations and travel to them by name later, so their own words become commands.
- **Honest uncertainty:** after getting lost, the game does not say where you are; landmarks are how you find out.

Compared with fixed north/south/east/west moves, this is harder to build, but its parts are well-understood: a visibility check, landmark scoring, pathfinding, and a drift chance per step.

## Ending

No fixed story ending, since every world, language, history and culture is new. Instead: an authored shape with generated content.

- **The deepest text:** each world's history has a root event, the inscription that broke, sealed or ended the civilisation, buried in the deepest layers. Finding and reading it is always the destination; what it says is unique per world.
- **Great active inscriptions:** a few major spells from history still running (a perpetual winter, a drowned city, a silenced region) act as generated goals. Players choose which to pursue, counter or leave alone.
- **The final word:** the player composes and scrapes a final inscription. The outcome emerges from the systems: what was written, over what, and how well the language was understood.
- **Legacy:** the final inscription persists faintly in the next world, seeding a future mystery.

Decided: the game ends when the player's time in the world ends. The world goes on; their chapter closes. Then comes the zoomed-out summary. Success means having changed the world as you wanted, and having produced interesting materials with your external tools.

### How your time ends

- **Leaving:** finding or writing a way out, possibly a departure inscription scraped deliberately.
- **Death:** from the environment, from something that intercepts you, or from your own spell going wrong.
- **Becoming part of the world:** writing yourself in; your name becomes an inscription for future decipherers.
- **Time:** a lifespan as a soft clock, since long journeys and slow regional change mean years can pass.
- **Being overtaken:** a collapse you didn't stop reaches you.

The final word is one way among these, not the only ending.

### Survival frame

Essentially a survival game: it ends when you die, or when you choose to leave via a late-game discovery.

- **Survival gives the environment stakes:** cold matters because you can freeze; rivers matter because you must drink or cross them.
- **Early purpose:** before writing is understood, staying alive, finding shelter and exploring carry the game.
- **Writing as survival tool:** learning it becomes urgent, not academic.
- **Permadeath:** a new world each run; the notebook (and perhaps a faint trace in the next world) survives you.
- **Leaving is earned:** a rare late discovery, and choosing it over staying is itself a decision.
- **Caution:** keep needs coarse and meaningful (warmth, water, food, injury, rest), felt over hours rather than minutes, so they don't become chores. Harshness is a tuning question.

### Arc: from observer to author

Influence over the world grows with understanding, and leads to the player deciding the world's fate.

- **Scraping scales with significance:** early tools affect small inscriptions (a door, a room); later tools reach larger, older, more potent writing (a district, a river, a region), up to the great active inscriptions and the deepest text.
- **Understanding gates power:** tools are not enough; you can only write what you understand, over what the layers beneath allow.
- **The fate is a layer too:** whatever the player decides (restore, end, seal, rewrite) does not erase what came before; it is the newest writing over the old.

### A world on a trajectory

The world's starting state is procedurally generated, and its success or decline is shaped by how the player wields growing power. Influence is visible in the environment: life returning to a barren waste, or a vast crevasse opening and the land crumbling into it.

- **Generated starting state:** dying, stagnant, precariously balanced or slowly recovering. The player's role differs per world.
- **Slow regional processes:** regions track broad variables (life, water, stability, climate) that change over time. Inscriptions push trends and effects spread, rather than flipping switches.
- **Seen on revisits:** change is noticed by returning to known places. Comparing old notes with new descriptions is how players measure their influence.
- **Trade-offs:** healing one region may harm another, so fate is a judgement rather than a moral binary.
- **The ending is a state:** the world's condition when the player finishes is the outcome, described from its own simulation.

### Seeing the whole

The world's overall state is not known at the start; the player sees only their small local view. The big picture is inferred from local evidence (dry riverbeds, abandoned settlements, ledgers recording shortages, migrations), the same way the language is pieced together from fragments.

On completion, the game gives a zoomed-out assessment, in text, of how the world ended up: the first time the whole picture is shown.

Idea: present that assessment as a chronicle in the world's own language, written by those who came after. The player can mostly read it by then, and understanding their own ending is the final test.

## Open questions

- [ ] **What does "influence" mean?** One option: inscriptions are claims the world tries to honour ("the bridge holds"), so effects follow meaning and precision matters.
- [ ] **What limits writing?** Candidates: known vocabulary, correct grammar, surfaces and materials, scale, constraints from underlying layers, the world resisting contradiction.
- [ ] **How do conflicting inscriptions resolve?** Newer over older, nearer over farther, precise over vague, or something stranger?
- [ ] **Do ghost layers ever act?** Fully inert, or a faint pull on the world?
- [ ] **Which layering rules apply?** Agreement, reused strokes, register, or a mix.
- [ ] **Is there a living world?** Descendants speaking an evolved form, with formulaic interactions.
- [ ] **What counts as "understanding" a word well enough to write it?**
- [ ] **Is there metaprogression between runs?**

## Next steps

- [ ] Hand-write a handful of rooms and fragments, as if the language already existed, to find the game's voice.
- [ ] Settle what "influence" means, since most other rules follow from it.
- [ ] Prototype a thin slice of the language engine (see `milestones/M01-language-slice.md`).
- [ ] Try deciphering the prototype's output with only a notebook, and judge whether it's fun.
