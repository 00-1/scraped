# D14 — Finding your way

**When:** after S07, before D15. **Size:** one milestone.

**Goal (Jb, 2026-10-11):** finding your way outdoors should be confusing at first and masterable, not confusing because of noise. Both playtests showed noise:

- the same `go east` was "a long walk" once and "a short walk" the next;
- arriving somewhere gave a random selection of facts, so a return looked like a new place;
- the home village was "the plain town" from outside and "an intact farming village" on arrival;
- `go back` toggled between two points;
- rivers barred the way without warning.

Design: `docs/DESIGN.md`, "Finding your way".

**Done when:** a patient player (and a wayfarer bot) can leave the start, travel by landmarks and paths, and come back on purpose. Returns are recognised. Getting lost happens for reasons the player can learn to avoid.

## Decided by Jb

- **Heading in a direction and stopping when you hit something interesting stays.** It's how it works now and it's right.
- **If you can see something in the direction you head, the default is to head to that thing,** and arrive there.
- It should be confusing initially, until mastered. Not made easy immediately.

## Design gates (proposed defaults; build, mark DESIGN-Q, list in the LOG)

- **Telling unnamed places apart:** by a fixed signature of their most distinctive features, with no game-given names (proposed). The alternative is short descriptive handles ("the ford by the split oak").
- **Sense of direction:** by day under open sky the character knows roughly where the sun is, so cardinal directions hold. Under cloud, in forest, in fog or at night without the still star, they drift (proposed).
- **How lost you can get, by difficulty:** gentle drifts little and corrects at landmarks; standard as designed; archaeologist drifts more and gives fewer cues (proposed).

## Scope

1. **Stable outdoor places.** The land is divided into recognisable places: a ford, a ridge, a meadow by a stream, a crossroads, a shore, a clearing. Each has a fixed **signature**: its two or three most distinctive features, always noticed first on arrival and always the same. Generated with the world, so they're stable across visits.
2. **Recognition** (S07): a place you've been is recognised on return, by its signature, in recognition wording. Towns and features are known by what you met them as, from afar and close up.
3. **Heading and stopping.** `go east` heads east and stops at the first interesting thing: a place's signature feature, a feature, a town, a change of ground, a path crossed, something seen. If something visible lies roughly that way, it's the default destination, and arriving there is the stop. The same command from the same place, in the same conditions, always gives the same result.
4. **Travelling by something:** `go to <landmark or place in view>`, `follow the river / road / ridge / shore` (up or down, either way), and `head east until <the river / the road / dark>`. Paths are easy to follow and safe to retrace.
5. **Landmarks as the backbone.** Visible landmarks report bearing and coarse distance consistently from each place, and change as you move, so a patient player can triangulate. Skyline features (peaks, towers, smoke) are seen from far; small ones only near.
6. **Disorientation, for reasons.** Drift happens in forest, fog, heavy rain, at night without the still star, and in open country with no landmark in view. With sun, star, a path or a landmark you hold course. The character feels unsure when drifting (a slot), and stops feeling it on reaching a recognised place or a path. Keenness for land and sky (D13) reduces drift. No randomness without one of these causes.
7. **`go back` retraces the route** you came by, place by place, towards where this journey began (with S07's fix). After a blocked move it says why there's nowhere to go back to, or goes back the way you came.
8. **Barriers seen before they block.** A river, cliff or sea is noticed as you approach ("a river ahead, running east to west"), before "the river bars your way". Fords and bridges are places you can head for.
9. **Travel words agree.** How far and how long are both body terms (S06) and consistent with each other and with the ground (a long walk over open ground isn't "a short while"). Travel time follows distance and terrain.
10. **Through slots:** signatures, approach notices, drift and recognition are Jb's words.

## Targets

- The same command from the same place gives the same result (test over seeds and conditions).
- A wayfarer bot, using only what the text says, returns to the start from a day's walk away on at least 8 of 10 seeds, and finds a place it visited before by its signature on most.
- In the explorer's runs, "I've been here" is always recognised (no return described as first sight).
- Drift happens only under its causes (test).

## Tests

- Signatures stable across visits and saves.
- Approach notices before every barrier.
- `go back` retraces multi-leg routes.
- Samples: `docs/samples/D14/`, a wayfarer bot's journey out and back on seed 42 in clear weather and in fog, with a short note.

## Checklist

- [x] Stable outdoor places with signatures; recognition
- [x] Heading stops at interesting things; heads to what's in view; deterministic
- [x] Travel by landmark, path, and until
- [x] Landmarks consistent and shifting
- [x] Disorientation only for reasons; by difficulty
- [x] `go back` retraces; barriers seen ahead; travel words agree
- [x] Wayfarer bot; targets; samples; LOG; DECISIONS
