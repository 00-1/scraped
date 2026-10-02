# Roadmap

From the language engine to the complete game, plus the authoring tool Jb uses to write every piece of human prose.

`docs/DESIGN.md` says what the game is. This file says what order to build it in. Each milestone has its own spec in `docs/milestones/`; agents work on one milestone at a time.

## What "finished" means

The game is finished when two things are true:

1. **The engine is complete.** Every system in `DESIGN.md` is built: a generated language with eras and a script, a generated world with history, free movement, survival, writing that acts when scraped, player writing, a world that changes over a run, and the endings. It runs in a terminal, in a browser, and through a JSON protocol for agents.
2. **The content pack is complete.** Every piece of prose the player can see was written by Jb, through the authoring tool. The release check refuses to build if any reachable text is a placeholder.

The engine is agent work. The content pack is Jb's work. The authoring tool is what connects them: the engine declares exactly what prose it needs, and the tool shows Jb what is missing, lets him write it, and previews it in real generated worlds.

## Milestones

| # | Milestone | Delivers | Jb can… |
|---|---|---|---|
| 01 | [Language slice](milestones/M01-language-slice.md) ✅ | Sounds, roots, morphology, word order, formulaic inscriptions, CLI, browser bench | decipher a corpus on paper |
| 02 | [Language depth](milestones/M02-language-depth.md) ✅ | Script and glyph descriptions, numerals, sound-change eras, registers, difficulty dials | draw a script and compare eras |
| 03 | [Content system and authoring tool v1](milestones/M03-content-and-authoring.md) | Content pack format, slot registry, template language, linter, browser authoring tool | write prose and see it rendered |
| 04 | [World generation](milestones/M04-world-generation.md) | Bounded terrain, climate, biomes, rivers, history, settlements, structures, inscriptions placed by history | inspect a generated world and its history |
| 05 | [Game core and text interface](milestones/M05-game-core.md) | Game state, command parser, time, save/replay, terminal client, JSON agent protocol, interiors | walk around inside one generated site |
| 06 | [Perception and movement](milestones/M06-perception-and-movement.md) | Landmarks, salience, intent movement, simulated travel, drift, bearings, naming places | cross the whole world and map it |
| 07 | [Physical world and survival](milestones/M07-physical-world-and-survival.md) | Local properties and their interactions, mechanisms, items, needs, threats, death | survive, or not |
| 08 | [Writing that acts](milestones/M08-writing-that-acts.md) | The three laws, layers, partial reading, effects engine, scraping tool | notice effects, then trigger a spell by scraping |
| 09 | [Player writing](milestones/M09-player-writing.md) | Writing tool, player text parsed to meaning, understanding gate, layer constraints, deep reading, misfires | compose and cast new inscriptions |
| 10 | [A world on a trajectory](milestones/M10-world-trajectory.md) | Regional simulation, time passing, great active inscriptions, scraping that scales, change seen on revisit | change the fate of a region |
| 11 | [Endings and legacy](milestones/M11-endings-and-legacy.md) | Ways a run ends, leaving, the deepest text, the zoomed-out summary and chronicle, traces in the next world | finish a run |
| 12 | [Authored spine and set pieces](milestones/M12-spine-and-set-pieces.md) | Storylet engine, opening and frame, set pieces placed into generated worlds | slot hand-written events into any world |
| 13 | [Authoring tool v2](milestones/M13-authoring-tool-v2.md) | Coverage by reachability, playtest-from-any-state, world and language inspectors, repetition analysis | finish the content pack efficiently |
| 14 | [Fairness, clients and release](milestones/M14-fairness-clients-release.md) | Solvability checks, difficulty tuning, browser player, agent harness, release gate | ship it |

Dependencies are mostly linear. Two exceptions:

- **M03 comes early on purpose.** From M04 onwards every milestone adds content slots instead of placeholder prose, so Jb can write alongside the build rather than facing a mountain of slots at the end.
- **M12 and M13 can run in parallel with M10–M11** once M09 is done.

## Design gates

Some milestones need a decision from Jb before an agent can build them properly. Each spec lists its gates with a **proposed default**. An agent may build on the default, marking the code `DESIGN-Q:`, but Jb should confirm or change the decision before the milestone is closed.

| Gate | Needed by | Proposed default |
|---|---|---|
| Resolve M01's open questions (cases, commands, separators, romanisation) | M02 | Keep what M01 built; add the presentation options as difficulty dials |
| What "influence" means | M08 | Inscriptions are claims the world tries to honour; effects follow meaning |
| What marks writing as potent | M08 | A distinct register: a fixed ritual frame and a special verb form |
| Do ghost layers act? | M08 | No. Only the top live layer acts |
| How conflicting inscriptions resolve | M08 | Nearer beats farther; newer beats older at equal distance |
| What counts as understanding a word | M09 | The player has seen the word in at least N distinct contexts, and their text parses to a valid meaning |
| Which layering rules apply | M09 | Agreement with the surviving ghost words |
| Is there a living world (descendants who speak)? | M10 | No, for the first release |
| Metaprogression between runs | M11 | Only the faint trace of the last run's final inscription |
| The frame story: who the player is and why they are here | M12 | Jb writes it; the engine only needs the hooks |

## Principles that apply to every milestone

These extend the rules in `CLAUDE.md`.

- **Determinism.** A run is fully defined by its seed, the content pack version and the player's commands. Saves are replays.
- **No prose in code.** Any text the player can read comes from the content pack through a declared slot (see M03), or is generated language. The only exceptions are debug and spoiler output.
- **The world is the truth; text is a view.** Systems simulate state. Descriptions are rendered from that state, never stored as strings.
- **Spoiler discipline.** Ground truth (meanings, maps, effects) is available through debug and spoiler flags and the authoring tool, never in normal play output.
- **Pen-and-paper solvability.** Anything the player needs to understand must be learnable from evidence the world actually contains. M14 checks this automatically.
- **Keep the browser build working.** Library crates stay WebAssembly-compatible, and CI builds them.

## Planned crate layout

| Crate | Purpose | From |
|---|---|---|
| `lang` | Language engine | M01 |
| `content` | Content pack loading, slots, template rendering, lint | M03 |
| `world` | World generation and history | M04 |
| `sim` | Physical properties, survival, effects, regional simulation | M07 |
| `game` | Game state, commands, perception, movement, writing, endings | M05 |
| `cli` | Developer tools: corpus, grammar, world dumps, content lint | M01 |
| `play` | Terminal client and JSON agent protocol | M05 |
| `web` | WebAssembly bindings shared by the browser player and the authoring tool | M03 |
| `tools/author` | The authoring tool front end (static web app) | M03 |
| `content/` | Jb's content pack (data, not code) | M03 |

## Cost and pacing

Calibration: M01 cost about **$6** of agent usage. The estimates below are rough multiples of that. Later milestones touch more existing code and more systems at once, so they cost more than their size alone suggests.

| # | Relative size | Rough cost |
|---|---|---|
| 02 | 1.5–2× | $8–12 |
| 03 | 2–2.5× | $10–15 |
| 04 | 2–3× | $12–18 |
| 05 | 2–2.5× | $10–15 |
| 06 | 2–2.5× | $10–15 |
| 07 | 2–3× | $12–18 |
| 08 | 2–3× | $12–18 |
| 09 | 2–2.5× | $10–15 |
| 10 | 2–2.5× | $10–15 |
| 11 | 1.5–2× | $8–12 |
| 12 | 1.5–2× | $8–12 |
| 13 | 2–2.5× | $10–15 |
| 14 | 2–3× | $12–18 |
| **Total** | | **roughly $130–200**, plus fixes after playtests |

At about $94, expect to get through roughly **M02 to M08**: a full language, the content pipeline and authoring tool, a generated world you can explore and survive in, and writing that acts. That is a playable core. M09 onwards needs further budget.

Ways to keep costs down:

- Run one milestone per session, ending at its checklist. Don't let a session drift into the next milestone.
- Do the hand check and settle design gates before starting the next milestone; rework is the most expensive thing.
- If a milestone is going over, split it at a checklist boundary rather than pushing on.
- Ask the agent to report usage at the end of each milestone in `docs/LOG.md`, so these estimates can be corrected as real numbers come in.
