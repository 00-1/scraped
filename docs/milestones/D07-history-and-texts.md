# D07 — History and what the writing says

**Goal:** make reading worth it. A deeper history of people, institutions and projects, written down in many genres, so that once a player can read, the texts tell stories: lives, quarrels, disasters, laws, prayers, instructions. The stories are spread across many sites and match the physical evidence (D03 scenes, D04 objects).

**Depends on:** D06.

**Done when:** the D07 targets are met, a scholar-bot transcript with spoilers reads like a history you'd want to piece together, and a player following one story arc has to travel to several places to complete it.

## Scope

### 1. Deeper history

Extend the history simulation:

- **People** with lives: birth, family, trade, office, deeds, feuds, friendships, death. Notable people recur across many texts and places.
- **Institutions:** temples and their priesthoods, guilds, councils, courts, schools, the scribes themselves. They own buildings, keep records, make rules and quarrel.
- **Economy:** goods, workshops and trade routes linked to D03 buildings and D04 objects; prices, shortages and gluts that ledgers record.
- **Religion:** deities and spirits tied to places and the sky (D05), festivals, myths, offerings.
- **Projects:** building an aqueduct, draining a marsh, raising a wall, carving a road. Started, funded, finished or abandoned, each leaving physical traces.
- **Disasters and responses:** flood, fire, earthquake, plague, famine, war, and what people did about them.
- **Law:** decrees, disputes and judgements.
- More event kinds: today there are 7; target at least 25.

### 2. Text genres

Generated as meaning first (as now), rendered through the D06 grammar. Target at least 20 genres, for example:

- Chronicles and annals; king and priest lists.
- Laws and decrees; court records and judgements; contracts, sales and loans.
- Letters, as threads between people over years.
- Prayers, hymns, myths and oracles.
- **Instructions and procedures**, including how to work the machine they're written beside (D04). Reading them is a reward with a direct use.
- Inventories, ledgers with totals, receipts, tax lists.
- **School exercises and word lists**, some pairing older and newer forms of words: anchors between eras.
- Maps with labels (D04); boundary stones; milestones with distances; building inscriptions with dates.
- Epitaphs that tell something of a life; graffiti and personal marks; curses and blessings.
- Astronomical tables and calendars (D05).

Tombs, today 59% of all texts, should fall to under 20%.

### 3. Story arcs

- Each world generates a set of **arcs**: a person's life, a family feud, a failed project, a disaster and its aftermath, a crime and its trial, a love affair told in letters, a schism. Each arc is told in several texts of different genres, in different places and sometimes different eras.
- Arcs match the world: the siege an annal describes left the D03 scene of arrowheads at that gate; the merchant's letters mention the warehouse where his seal (D04) is found.
- Some arcs lead to places (a letter tells where something was hidden), so reading pays off in exploration.
- Expose arcs to storylets, so Jb can write set pieces into them.

### 4. Placement

- Texts go where they would have been written or kept: archives and libraries hold the most, but every building type has its own (a kitchen has labels, a mill has its tally, a gate has its dedication).
- Reading order matters: the fairness checker (M14) is extended so arcs and constructions are learnable in a plausible exploration order.

## Content slots introduced

Few new slots: texts are generated language. Add framing slots for new genres and surfaces (a ledger's columns, a map's labels, a letter's seal), through the attention model.

## Targets (measured by D01)

| Metric | Baseline | Target |
|---|---|---|
| Event kinds | 7 | at least 25 |
| Text genres per world | 7 | at least 20 |
| Largest genre's share | 59% (tombs) | at most 20% |
| Distinct sentence shapes per world | about 90 | at least 600 |
| Mean words per text | about 6 | at least 15, with a long tail of long texts |
| Story arcs per world | 0 | at least 10, each in at least 3 places |
| People with traces in 3+ places | few | at least 30 |

## Tests

- Every text's meaning is consistent with the history (no text contradicts an event, except deliberately: propaganda and lies are tagged).
- Every arc's texts and physical evidence agree.
- Instructions describe their machine correctly.
- Fairness: every construction used in arcs is attested earlier in a reachable order.
- Round trip on all texts.

## Checklist

- [ ] People, institutions, economy, religion, projects, disasters, law
- [ ] At least 25 event kinds
- [ ] At least 20 text genres, generated as meaning
- [ ] Story arcs across sites and eras, tied to scenes and objects
- [ ] Texts placed where they belong
- [ ] Fairness checker extended to arcs and order
- [ ] Framing slots
- [ ] Targets met; samples committed with a note
- [ ] Tests listed above
- [ ] LOG.md entry
