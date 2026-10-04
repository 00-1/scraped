# D07 — A language for long texts

**Goal:** give the language the grammar and vocabulary that real, varied texts need: sentences that join, subordinate and qualify; words built from other words; a lexicon that comes from the culture; and place names that mean something.

**Depends on:** D06 (so the lexicon can name the world's species, sky, crafts and places).

**Done when:** the language can express every meaning D08's texts need, the renderer and parser still round-trip everything, and a skilled player can still decipher it: every new construction is regular and attested often enough to learn.

## Scope

### 1. Clauses and sentences

- **Coordination:** and, but, or, then.
- **Subordination:** when, because, if/then, until, before, after, so that, although.
- **Relative clauses:** "the man who built the gate", "the water that we drink".
- **Reported speech and quotation:** "X said that…", direct quotes in letters and court records.
- **Comparison and degree:** more than, the most, as … as.
- **Quantifiers and negation scope:** all, some, none, each, many, few.
- **Moods:** imperative, optative ("may it…", the natural home of the potent register), conditional, interrogative.
- **Aspect and tense:** past, non-past, plus perfective/imperfective or habitual, chosen per seed from attested systems.
- Each construction is generated per seed (particles, affixes or word order), regular, and recorded in the grammar sheet.

### 2. Word building

- **Derivation:** agent nouns (builder), place nouns (bakery), instruments (opener), abstracts (kingship), diminutives, adjectives from nouns.
- **Compounds:** head-first or head-last per seed (riverstone, gatekeeper).
- Derivation and compounding are regular and productive, so a player who learns them can build or guess words they were never shown. That is the "generativity over lookup" principle in `DESIGN.md`.

### 3. Lexicon from culture

- The concept list grows from about 110 fixed concepts to a **generated lexicon of 400–700 concepts**: a core of universal concepts (body, kin, numbers, basic verbs) plus domains drawn from this world's culture, crafts, religion, law, trade, ecology (D06 species), sky, and places.
- The culture decides which domains are rich (a seafaring people has many words for wind and boats).
- Semantic fields and relations (part of, kind of, opposite) are recorded so texts, place names and the magic (D09) can use them.

### 4. Names

- **Personal names** built from words (as many real cultures do), with naming systems per seed (patronymics, clan names, titles, epithets).
- **Place names** that mean something: "Red Ford", "Hill of the Two Gates". Place names come from the landmark traits (D01) and history, are written on milestones, boundary stones and maps, and evolve across eras with the sound changes. A player who has walked to Red Ford has a strong anchor for "red" and "ford".

### 5. Dates, numbers and the calendar

Dates by the culture's calendar (D05, D06): regnal years, festivals, months. Ordinals and fractions where trade needs them.

### 6. Keep everything that works

- Renderer and parser both cover every construction; round trip on all generated text.
- Every surface word still has exactly one analysis, or ambiguity is controlled and recorded per difficulty preset.
- Sound changes (eras) apply to everything, including derivation and names.
- Difficulty presets can turn constructions down (gentle) or up (archaeologist).

## Tests

- Round trip (meaning → text → meaning) on a large random sample per seed, all eras.
- Each construction attested at least N times in a world's readable texts (with D08), checked by the fairness checker.
- Derivations and compounds are fully regular per seed.
- Place names match their landmarks' traits in meaning.
- Snapshots for three seeds updated and reviewed.

## Checklist

- [x] Coordination, subordination, relative clauses
- [x] Reported speech, comparison, quantifiers
- [x] Moods, tense and aspect systems
- [x] Derivation and compounds
- [x] Generated cultural lexicon with semantic relations
- [x] Personal names and meaningful place names, across eras
- [x] Dates and calendar language
- [x] Parser parity and round trip
- [x] Grammar sheet, bench and inspector updated
- [x] Tests listed above (attestation per construction comes with D08's texts)
- [x] LOG.md entry
