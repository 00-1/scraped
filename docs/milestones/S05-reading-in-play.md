# S05 — Reading in play

**When:** now, before Jb's playtest. **Size:** small. Fixes only. Release a patch or minor version when done.

**Goal:** fix what a hand replay of v0.4.0 on seed 42 still showed, and make the bots' checks match what a person sees. S04 passed its own tests, but some of its fixes don't hold in the real player build.

## 1. Checks that look at the real text

The S04 hand player reported 0% failures, yet about 60 hand-typed commands hit several. The related-sign test passed while `read closely` showed "like a wide, mixed sign, with a curl, with" (cut off).

- Run the slip checks on the **player build's** output (the `scraped-player` binary, as released), not only through the engine calls.
- The hand player also reads closely (several pages, on several surfaces) and uses `go in` right after examining a building.
- Count as failures: any "Which do you mean" the bot didn't need, a choice joined with "and", any response ending in a dangling word ("with", "and", "of", "the"), and any rendered list item that is empty.
- Add the transcript below (seed 42, v0.4.0) as a fixture that must pass.

## 2. Bugs (seed 42, v0.4.0, by hand)

1. **Related signs, cut off and unfindable.** Sign 6 of the observatory's wood table reads "like a wide, mixed sign, with a curl, with" and stops. Most "like…" signs still name a base that matches many signs ("like a wide, angular sign, with a curl beside"). In the real reading view, "like X" must name an X that matches exactly one sign in that script (by sound if heard, otherwise by a resemblance or impression no other sign shares). Otherwise drop "like" and describe the sign on its own. Test this over every page of every readable text on seeds 1, 42 and 9001.
2. **Things in a room that are never shown.** In the observatory's lower hall, `look`, `look around` and `look closer` never mention the knife, torch, oil and ring that `take all` then picks up. Every loose thing in plain view appears within `look around`.
3. **`go in` after examining.** "The intact observatory stands before you." followed by `go in` asks which of eight buildings. The building just examined, or the one stood before, is the default.
4. **Choices joined with "and":** "Which do you mean: the worn temple and the temple quarter?" Choices are joined with "or".
5. **Missing articles:** "Faintly, river from the southeast." and "Here: bones, bundle and bowl."
6. **Long texts on small things, and long texts first.** A clay jar carries 170 signs in fifteen lines. On the natural route through seed 42's start town (observatory, temple, storehouse), the first three texts run 10, 32 and 11 pages, and none of the new short texts is met. Fix both:
   - A surface's text length fits the thing it's on (a jar or box a label or a few words; a table or chest a few lines; long texts on walls, steles and tablets).
   - The short texts sit where a player goes first: the entrance room and the first surfaces reported there, in the buildings nearest where the player wakes.

   Test: on seeds 1, 42 and 9001, the first three texts the explorer and hand player read each fit in three pages.

## Done when

- The checks in section 1 run on the player build and pass, with the seed 42 fixture.
- None of the bugs above appear in the bots' runs on seeds 1, 42 and 9001.
- A release; HANDOFF updated; LOG entry. Then back to waiting for Jb's playtest.

## Checklist

- [ ] Slip checks run on the player build; hand player reads closely and uses `go in`; seed 42 fixture
- [ ] Related signs: "like" only with a base that matches one sign; never cut off
- [ ] Loose things always shown by `look around`
- [ ] `go in` defaults to the building in front of you
- [ ] Choices joined with "or"; missing articles
- [ ] Text length fits its surface; short texts met first
- [ ] Release; HANDOFF; LOG
