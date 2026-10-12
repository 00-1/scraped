# S08 — Hand check of v0.9

**When:** now. **Size:** small. Fixes only. Release when done.

**Goal:** fix what a hand replay of the start of seed 42 on v0.9.0 showed. Much of it worked well. The storehouse's hollow wall gave a firesteel within ten commands. Looking progressed and ended with "Nothing more catches your eye." The style offer read naturally. These points didn't:

## 1. The home town isn't recognised on return

- From the square, I typed `head east` (river ahead) and then `go back`, which reported "You travel west, a long walk." The arrival said "Grassland, flat ground. Here and there: withered growth." Only a `look` showed "The square." A second `go back` then said "You haven't come from anywhere yet."
- `head northwest` then `go back` did the same: the arrival gave the land's signature, not the town.

Fix:
- Arriving in a town or at a building gives the town first, in recognition wording if visited. The land's signature is for open country, not settlements.
- `go back` keeps the whole journey, so a second `go back` continues to retrace it. "You haven't come from anywhere yet" only appears at the true start of a journey.
- **Test:** for each start town on seeds 1, 42 and 9001: head out each of four ways, then `go back`. The arrival names the town and recognises it, and a second `go back` behaves.

## 2. The carving's proud stone can't be worked

The opening says "One stone of the carving sits proud of the rest." What I tried:

- `touch stone` → touched the carving;
- `push carving` → "You see no stone carving here." (`examine carving` had worked two commands before);
- `pull stone` → "You see no stone carving here.";
- `lift stone` → "You try the carving. Nothing comes of it.";
- `take stone` → "A stone carving will not move.";
- `press the stone` → "You don't know how to 'press'.";
- `pull the proud stone` → "You see no proud stone here."

Fix:
- `push` and `pull` must find the same things `examine` finds. Add `press` (and `prise`/`pry`, `lever`).
- The part a payoff cue names ("one stone", "the proud stone") can be named and worked.
- If a payoff needs a tool, the cue or the failed attempt says what kind ("It won't shift with bare hands; something to lever it might.", Jb's words).
- **Test:** for every payoff on seeds 1–10, a scripted solve using only words from its cue and failure messages succeeds.

## 3. Signs of spells about things that aren't there

In the storehouse's storeroom: "The statue sounds wrong here." There's no statue in that room.

- A spell's sign (`spell.sign`) names only a thing the player can perceive where they are.
- **Test** over the bots' runs: every thing named in a sign is in the room or in view.

## 4. The sweep

- The first `search` in the square said "Nothing more catches your eye. A closer look might." Later `search` commands each examined one of three identical carvings, each answered "A stone carving. Marks are cut into it." It never said which carving it was looking at, or that the three were alike.
- Fix: `search` runs its steps until something new, not one step per call. It never answers "nothing more" while unexamined things remain. Alike things are examined as a group, or told apart by position ("the carving by the well").

## 5. Smaller

- **"Here: bones, bundle and bowl."** still reads as a list of objects. Word scene parts as scene (Jb's slot), in the scene's own wording.
- **Suggestions out of nowhere.** "You could smell." after stepping outside: a suggestion follows from something perceived (a smell on the air), as D13's rule says. Check `suggest.verb` cases for a perceived cause.
- **Chains in the Android app** show as one bubble. Show one bubble per step, as on the other clients (S07 decision).
- **The mason reaches no ending** (D15: the works are only in great buildings). Add reachable large payoffs for the mason near most starts (a well, roof or bridge to repair; a smaller building's works), or report why not.

## Done when

- Tests above pass on seeds 1, 42 and 9001.
- The hand replay below gives the town on return and solves the proud stone:
  ```
  forager / look ×5 / take stock / search ×3 / go storehouse / touch wall / take all / out / head east / go back / go back / head northwest / go back / push stone
  ```
  Add it as a fixture.
- Release; HANDOFF; LOG.

## Checklist

- [ ] Towns recognised on return; `go back` retraces the whole journey
- [ ] Payoff parts can be named and worked; push/pull/press/prise; tool hints
- [ ] Spell signs name only what's present
- [ ] Sweep runs to something new; alike things grouped or told apart
- [ ] Scene parts worded as scene; suggestions from a perceived cause; Android chains per step; mason ending
- [ ] Fixture; release; HANDOFF; LOG
