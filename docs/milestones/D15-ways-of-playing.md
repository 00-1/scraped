# D15 — Ways of playing

**When:** after D14 (Finding your way). **Size:** one milestone, possibly two. Keep each arc small at first and deepen later.

**Goal (Jb, 2026-10-11):** several ways of playing are satisfying in their own right, and never need to progress with writing at all. Each play style from D13 gets its own payoffs, small and large, and its own way for a run to feel complete. Writing stays one path among several, not the only one with a destination.

**Done when:** a bot per style, never scraping, studying or writing, plays ten worlds. Each reaches its own milestones and a satisfying end on most seeds. Its transcripts read as a run with a shape: early wins, growing mastery, something large achieved.

## Design gates (Jb)

Everything below is proposed. Jb decides the arcs, what each large payoff is, and how a run ends for each style. Build on the defaults, mark them `// DESIGN-Q:`, and list them in the LOG.

## Arcs (proposed)

Each arc has three things:
- **small payoffs:** S07's scattered puzzles, weighted to the style's category;
- **growing mastery:** the world becomes more knowable and more useful as you learn it;
- **large payoffs** that change what the player can do, and a way for the run to end well.

1. **Forager: living off the land.**
   - Mastery: learning which plants and creatures are where and when, by season (the player's own notes; the game keeps none). Snares, fishing and foraging improve with practice and tools. Making cordage, baskets and simple traps from what's found.
   - Large payoffs: a camp that holds through a winter; a store of food; a grove or spring that sustains; an animal companion that follows when fed and warns of danger.
   - Ending (proposed): settling. A run can end with the character staying, a camp made into a home, judged by how well it would last.
2. **Mason: buildings and mechanisms.**
   - Mastery: reading construction and wear to find hidden rooms, sealed spaces and the works that run them (D04, D05). Tools that open, lever and repair.
   - Large payoffs: running a great building's works; opening its sealed heart by mechanism, not writing; repairing a roof, bridge or well so a place can be used; mapping a great interior whole.
   - Ending (proposed): restoration. A building made sound and lived in again, or a great interior fully explored.
3. **Wayfarer: the land and the way out.**
   - Mastery: finding your way (D14): steering by sun and the still star; landmarks, routes, fords and passes; weather read ahead. The world's own units (D13) let them measure.
   - Large payoffs: finding every town on an old map; a hidden valley or pass; a summit view that shows the land whole; long journeys made safely.
   - Ending (exists, D10): the world's rim, reached on foot. Make the journey there a real arc, with landmarks on the way, not just a walk.
4. **Scribe: the marks.** The arc the game already has, through decipherment to writing.

## Shared rules

- **No arc needs writing,** and no arc is closed off by not writing. Old spells still shape every path as strangeness (D13: evidence in every category). A player may notice them and choose to ignore them.
- **Arcs cross.** A forager's camp shelters a scribe; a mason's opened room holds texts; a wayfarer's map finds a cache. Payoffs from one arc can help another.
- **The end summary (M11) tells each arc's story:** what the player mastered and achieved in each, not only what they read and wrote.
- **Fairness:** each arc's large payoffs are reachable on every seed (checker goals per arc).
- **Through slots:** every new cue, attempt, result and ending is Jb's words, with one example variant.

## Targets

- Each non-writing style bot (no scraping, studying or writing) gets its first small payoff in the first hour, and at least one large payoff in its first ten days, on at least 7 of 10 seeds.
- Each reaches its ending on at least 5 of 10 seeds within a season.
- In the explorer's runs, at least two arcs advance without any writing.

## Tests

- Arc goals in the fairness checker.
- Bots per style that never scrape, study or write.
- Samples: `docs/samples/D15/`, a condensed run per style on seed 42, with a short note on each arc's shape.

## Checklist

- [ ] Arcs per style, with gates as DESIGN-Q
- [ ] Small payoffs weighted per style (with S07)
- [ ] Mastery mechanics per arc
- [ ] Large payoffs per arc
- [ ] Endings per arc; end summary per arc
- [ ] Fairness goals; non-writing style bots; targets
- [ ] Samples; LOG; DECISIONS
