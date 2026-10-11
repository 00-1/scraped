# D13 — Ways of noticing

**When:** after S06. **Size:** one milestone.

**Goal:** build "Noticing and knowing" (`docs/DESIGN.md`): what the character notices unasked depends on a play style and on habit, verbs are suggested by situations, and every style can reach the slow realisation through its own kind of evidence.

**Done when:** four bots, one per style, each play ten worlds. Each one's transcripts read differently in what arrives unasked, and each finds evidence of old spells in its own category before it finds it in marks.

## Design gates (Jb)

Build on the proposed defaults, mark `// DESIGN-Q:`, list them in the LOG:

- **How a style is chosen.** Proposed: a short choice when a world starts (scribe, forager, mason, wayfarer, or none), in Jb's words through slots, recorded with the world. The alternative is no choice: everyone starts even and drifts.
- **How fast habit moves keenness.** Proposed: a category attended to a dozen times over two or three days of play becomes noticeably keener; a week of neglect fades it halfway to the floor.
- **Whether a starting style keeps an edge.** Proposed: yes, a small one, so a style is never wholly lost.

## Scope

1. **Keenness** per category (marks; sounds; smells; living things; buildings, materials and wear; land, routes and sky; things and mechanisms), in the player's half of the save. The attention model weighs candidate facts by keenness. The budget per response is unchanged.
2. **Habit:** deliberate attention to a category (the verbs and targets that count, listed per category) raises it; neglect lowers it towards a floor. Never shown as a number. One optional slot per category for the first time it sharpens (Jb's words; may stay empty).
3. **Play styles** as starting keenness (and the matching start, per the gate above).
4. **Suggested verbs:** situations that suggest each verb (a table: verb → situations), one suggestion per response at most, through slots, shown a couple of times and then dropped until the situation changes, never after the verb is used. Keenness makes a category's suggestions come sooner. Scraping and writing are never suggested before the player has done them.
5. **Evidence in every category.** Each kind of old spell leaves evidence in at least two categories, through `spell.cue` and its follow-ups (S06). Proposed coverage: cold or heat in materials and air; light in sights and the sky; beasts drawn or kept off in living things; doors held or opened in buildings; growth in living things and smells; sounds where a spell wakes.
6. **Bots per style** (scribe, forager, mason, wayfarer), built on the explorer, each attending more to its category. Metrics: share of unasked facts by category; first evidence of a spell, and its category; first release; survival.
7. **`help` and suggestions together:** every verb in the parser has at least one situation that suggests it (test), and each style bot meets it on most seeds.

## Targets

- For each style, the share of unasked facts from its category is at least twice the neutral share, while words per arrival stay within D02's limits.
- Each style bot meets evidence of an old spell in its own category before meeting it through marks, on at least 7 of 10 seeds.
- Every verb is suggested at least once in ten hours on most seeds (per verb; report the exceptions).
- No suggestion claims what the character couldn't do then (a test over the slot variables: no "read" before recognition, no "scrape" before a scrape).

## Tests

- Keenness and habit are deterministic and survive save and load.
- Two players in one shared world keep their own keenness.
- Suggestion rules: shown at most a couple of times; never after use; never for scraping or writing first.
- Samples: `docs/samples/D13/`, one hour per style on seed 42 from the same start, with a short note on how they differ.

## Checklist

- [ ] Keenness, habit, styles (gates as DESIGN-Q)
- [ ] Suggested verbs, table and slots
- [ ] Evidence of spells in every category
- [ ] Four style bots and metrics
- [ ] Targets met or reported
- [ ] Samples; LOG; DECISIONS
