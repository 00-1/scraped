# Milestone 1 — Language engine slice

**Goal:** prove that a procedurally generated language can be (a) internally consistent, (b) feel like one language rather than random strings, and (c) be deciphered by a patient human with only a notebook. Nothing else.

**Done when:** Jb can run the CLI with a seed, get a corpus of inscriptions, and over an evening of notebook work start cracking roots, affixes and word order — and it feels good.

## Scope

### In
1. **Phonology**
   - Generate a consonant and vowel inventory from a seed (draw from weighted, realistic options; 12–22 consonants, 3–7 vowels).
   - Syllable structure template (e.g. `CV`, `CVC`, `(C)V(C)`), with simple constraints (allowed onsets/codas).
   - Romanisation: a deterministic mapping from phonemes to Latin letters/digraphs for output.
2. **Lexicon**
   - A fixed starter list of ~60–100 concepts (English glosses), grouped by domain: people, nature, places, objects, actions, qualities, numbers 1–10. Store as data (e.g. `concepts.toml`), not code.
   - Generate a root for each concept, obeying phonotactics, avoiding near-duplicates (minimum edit distance).
3. **Morphology** — one regular, agglutinative system
   - Nouns: plural, and 2–3 cases (e.g. subject, object, genitive/location).
   - Verbs: past vs non-past; negation.
   - Affixes are generated per seed (prefix or suffix, chosen per seed), always regular, no irregular forms.
   - Simple sandhi is optional; if added, it must be regular.
4. **Syntax**
   - Word order chosen per seed (SOV / SVO / VSO), with adjective–noun order and genitive order also per seed.
5. **Meaning representation**
   - A small structured type for sentences: predicate, arguments with roles, tense, polarity, number, possessor, modifiers.
   - A renderer: meaning → surface string, deterministic.
   - The engine can return an interlinear gloss for any generated sentence (debug/spoiler output).
6. **Inscription templates** — formulaic text types that give decipherment anchors
   - **Tomb formula:** "[Name], child of [Name], lies here" style, with varying names and relations.
   - **Ledger line:** quantities (numerals) of goods.
   - **Warning/sign:** "do not [verb] the [noun]" and positive commands.
   - **Dedication:** "[Person] made this [object] for [person/place]".
   - Names are generated words that obey phonology.
7. **CLI** (`crates/cli`)
   - `scraped-lang --seed N corpus [--count 40]` → prints a corpus of inscriptions, one per line, romanised only.
   - `--json` → same as JSON objects.
   - `--spoil` → adds glosses/meanings (for testing; never the default).
   - `scraped-lang --seed N grammar --spoil` → prints the full generated grammar sheet (inventory, affixes, word order, lexicon).

### Out (do not build yet)
- Script/glyph descriptions, sound-change eras and layers, scraping/ghost text, world, movement, survival, effects of writing, web front end, player writing/parsing.

Design the types so these can be added later (e.g. leave room for an `Era` on the language, and for parsing surface → meaning), but don't implement them.

## Tests
- Determinism: same seed → identical corpus and grammar sheet (snapshot tests for 2–3 seeds).
- Different seeds produce noticeably different languages (inventory, word order, affix shapes).
- All generated words obey their seed's phonotactics.
- No two roots in a lexicon collide; affixes don't collide with each other.
- Round trip: every rendered sentence's gloss matches its meaning representation.

## Deliverable checks for Jb
- [ ] Run `corpus` for a seed and try to decipher it on paper without `--spoil`.
- [ ] Then check against `--spoil`. Note what was too easy, too hard or unfair.
- [ ] Feed findings back into DESIGN.md / next milestone.

## Checklist (agent updates this)
- [x] Workspace scaffold (`crates/lang`, `crates/cli`), CI-style checks pass
- [x] Phonology + romanisation
- [x] Concept list as data + root generation
- [x] Morphology
- [x] Syntax + meaning representation + renderer + gloss
- [x] Inscription templates
- [x] CLI commands
- [x] Tests listed above
