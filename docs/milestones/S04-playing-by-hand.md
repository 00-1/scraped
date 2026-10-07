# S04 — Playing by hand

**When:** now, before Jb's playtest. **Size:** small. Fixes only; no new systems. Release a patch or minor version when done, so Jb plays the fixed build.

**Goal:** a person typing ordinary commands shouldn't spend their first session fighting the parser. These come from a review that played v0.2.0 (`scraped-player --seed 42`) by hand for about an hour of game time. The bots never hit these bugs because they use exact command phrasing. Also included: three fixes from the D11 review that never reached D11.

## 1. A bot that types like a person

Add a third bot, the **hand player**, which plays like the explorer but phrases commands the way people do. For example:

- `x`, `get`, `take all`, `pick up the knife`, `enter the temple`, `go in`, `leave`;
- `go to the sinkhole` from inside a building;
- names with "the", plurals ("walls"), and words from the last response.

Count every response that is a parser failure ("You see no…", "Read what?", "There is no… here"). Write them to `docs/samples/S04/failures.md` with the command that caused each and the state it was typed in. **Target:** under 5% of the hand player's commands fail on seeds 1, 42 and 9001, and none of the failures listed below appear. Keep the bot in CI.

## 2. Bugs seen in play (seed 42, v0.2.0)

1. **Room contents come one per look.** `look`, `look around`, `look closer` and `search` in a room each reveal one more item: a knife, then a torch, then oil, then a ring. The player has to repeat `look` until it stops changing. Per D02 ("whole, then groups, then individuals"), loose things in a room are given together as a group ("A knife, a torch and a flask of oil lie here"); digging finds what is hidden, not what is lying in plain view.
2. **`take all`** fails with "You see no all here". Support `take all` and `take all <kind>`.
3. **Examining a building from outside walks you into it.** `examine observatory` put the player in its lower hall. Examining something should never move the player.
4. **Going to something outside from indoors** fails with "You see no the sinkhole here". If the target was in view outside, go out and go there, or say that you're inside.
5. **Errors repeat the article:** "You see no the sinkhole here", "There is no the river here to follow". Strip the player's article from names quoted back.
6. **Number and article slips:**
   - "You sleep for 1 hours."
   - "You carry a greens."
   - "You take an oil."
   - "You need to be outside to head."
   - "You take a knife." followed by "You find a knife."

   Mass nouns take "some". Verbs quoted back read as the player typed them. The take message isn't repeated.
7. **Empty variable:** `go back` printed "You should have reached  by now". No slot may render an empty variable; add a lint-time or test-time check across the bots' runs.
8. **Lost in daylight:** "With nothing to steer by, you are no longer sure of your way" at mid-morning with the sun up. The sun steers by day, and the still star by night under a clear sky.
9. **Related signs still have no findable base:** "like a wide, mixed sign, with a curl, with a curl" (one sign shows the same part twice). Half a dozen signs on that page begin "a wide, mixed sign, with…". The S03 test passes, but the base is still unclear in play. Name the base by sound or by a resemblance no other sign shares. Where it has neither, describe the sign on its own instead of "like…". Never repeat a part.
10. **Cleaning speaks of scraping:** `clean wall` with a knife on an already-scraped wall said "There is nothing left on a clay wall to scrape." Cleaning messages don't name scraping (per "Writing is background, at first"). And a wall with half its signs left isn't "nothing left".
11. **The first text met is very long:** the first table read on seed 42 has 153 signs over 10 pages. Check what share of texts in the start town run over 3 pages. If most do, add short everyday texts near the start, or record it as a DESIGN-Q.

## 3. From the D11 review (never merged)

The bots slipped across D07–D10: explorer survival from 9 of 10 to 6, the scholar from 6 of 10 to 4. `docs/DEPTH.md` puts both causes in the game.

1. **Hidden magic doesn't kill a new player.** A spell that can threaten life (cold, dark, bad air, beasts drawn) is always felt before it harms: the first time the player is in it, they get a cue through `spell.cue`. No such spell lies on the starting area or on the shelters nearest the start. Test: no explorer death in the first three days is caused by a spell the player was never cued about.
2. **Counter words can be learned.** Every word needed to undo a ward or a held door (door, gate, tomb, box, "open", "greatly", and each sealed place's word) appears in enough readable texts for the understanding gate to pass. Proposed default: at least five texts, two of them before the first place it gates (DESIGN-Q). The fairness checker covers it.
3. **No more lowering bars.** Re-measure the explorer and scholar on seeds 1–10 and record the cause of each remaining failure (game or bot) in `docs/DEPTH.md`.

## Done when

- The hand player is under 5% failures, and none of the bugs above appear in its runs or in the explorer's.
- The tests in section 3 pass; bot lines are re-measured with causes recorded.
- A release with these fixes; HANDOFF updated; LOG entry. Then back to waiting for Jb's playtest.

## Checklist

- [x] Hand-player bot, failure report, under 5%, in CI
- [x] Room contents grouped; `take all`
- [x] Examine never moves you; going to outside things from indoors
- [x] Articles, numbers, mass nouns, empty variables
- [x] Steering by sun and star
- [x] Related signs: findable base or none; no repeated parts
- [x] Cleaning messages don't name scraping
- [x] Long first texts checked
- [x] Uncued deadly spells; learnable counter words; bot causes recorded
- [x] Release; HANDOFF; LOG
