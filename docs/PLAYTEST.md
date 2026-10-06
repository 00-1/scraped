# Playtest pack

For Jb: what to play, how to play it without spoiling it, how to look under the hood afterwards, and what is most useful to send back. Where to get a build, and how to send a save, are at the top of `docs/HANDOFF.md`.

## Three seeds

Each is a different shape of world. All three have sample transcripts in `docs/samples/` (the explorer's first hours under D09 and D10, the scholar's season under D11), so you can compare your play with the bots' afterwards, not before.

| Seed | The world | What it shows off |
|---|---|---|
| **1** | 20 settlements, 273 buildings of 43 kinds, three eras of writing, seven great interiors (the largest 467 spaces) | Breadth: the most towns and kinds of building, and the longest walk to the world's edge. A good first world for exploring without writing at all. |
| **42** | 14 settlements, 192 buildings of 39 kinds, ten great interiors | Density: great interiors close together, and a sealed cemetery gate that the scholar opens in its samples. A good world for the late game. |
| **9001** | 13 settlements, 206 buildings of 43 kinds, eleven great interiors | Weather and life: a cold road where something keeps wolves off, frost where it shouldn't lie, and the most great interiors. A good world for noticing strangeness. |

Each has three sealed places and a way out without writing (the world's rim). On all three, the chain of sealed places ends at a great inscription's building.

Play on **standard** first. **Gentle** gives a friendlier language and more legible writing; **archaeologist** gives more worn writing and a harder script.

## Playing spoiler-free

- In the browser or the app, start a new world and give it the seed. In a terminal: `scraped-player --seed 42`.
- Keep a notebook outside the game. The game is not your notebook (`docs/DESIGN.md`): it won't keep a glossary for you.
- Nothing in the game will tell you what writing does. If something strange happens, it's worth writing down where you were and what you'd just done.
- `help` and the manual only list verbs. They say nothing about writing.

## Looking under the hood (after playing)

All of these spoil the world. Use the developer build (`cargo run -q -p scraped-cli --release -- ...`, shown as `scraped-lang`):

- `scraped-lang world --seed 42 writing --spoil`: every written surface, every live spell and its condition, and why each strange place is strange.
- `scraped-lang world --seed 42 history --spoil`: the world's eras, people, events and stories.
- `scraped-lang world --seed 42 map --spoil --png map.png`: the map.
- `scraped-lang --seed 42 grammar --spoil`: the language's grammar sheet.
- `scraped --seed 42 --json --spoil`: play with the ground truth (meanings, glosses) beside every response.
- `scraped-lang replay SAVE`: replay a save exactly, for a bug.
- `scraped-lang bots --seeds 42 --bot scholar`: how far the scholar bot gets in 90 days.

## What feedback helps most

In order of usefulness:

1. **Where you stopped wanting to go on**, and why: boredom, confusion, death that felt unfair, text that was too long.
2. **Text that says too much.** Any response that explains instead of showing, especially one that points at writing too soon. Name the command and quote the response.
3. **Moments that worked.** A discovery that felt like yours: what you saw, and what you worked out from it.
4. **Wording bugs.** Grammar, plurals, articles ("A graves"), repeated phrases. Send the save.
5. **Your notebook**, if you are willing: it shows what a player can actually piece together, and what they can't.

Remember that nearly all the text is still example text written to test the engine (`docs/CONTENT-PLAN.md` says what to write first). Judge what is said and when, more than how it's worded.

## Open decisions

Every default the engine runs on, with where it lives, is in `docs/DECISIONS.md`. The newest, and the ones play will bear on most:

- **The slow realisation (D10):** how common grime is on writing; one everyday spell in three left latent; the world's rim as the way out without writing.
- **Problems only writing solves (D11):** three sealed places per world; what a ward says; "greatly" for later wards; the chain's fixed order; a great inscription at the chain's end.
- **The magic, deepened (D09):** how a culture picks its powers; what a spell acts on; which conditions the world can judge; how many everyday spells there are.
- **Before Jb plays (S03):** the v0.x release gate only reports missing text.
- **Shared play and versions (C01):** the fixed seal key; the GitHub repo as the sync place.
