# Work log

## 2026-10-02 — Milestone 1 language engine slice

**Done**
- Cargo workspace: `crates/lang` (pure library, builds for `wasm32-unknown-unknown`) and `crates/cli` (`scraped-lang` binary).
- Seeded RNG over ChaCha8 with one stream per stage and per inscription. Range reduction is our own, using only raw `u32`s, so output doesn't depend on pointer width or `rand` internals.
- Phonology: 12–22 consonants from a weighted pool with implicational rules (no /b/ without /p/), 3–7 vowels from attested systems, syllable template (onset 0–2, coda 0–1), coda sets, stop+liquid clusters. Romanisation is checked to be uniquely decodable (Sardinas–Patterson), so every romanised word splits into sounds exactly one way.
- 99 concepts in `crates/lang/data/concepts.toml`, with semantic tags so templates pick sensible words. Roots keep an edit distance of at least 2 from each other, and **no inflected form of any root or name equals any other**, so every surface word has exactly one analysis.
- Morphology: plural; subject (unmarked), object, genitive and dative cases; past; negation. One affix each, prefix or suffix per word class per seed, number/tense nearest the root.
- Syntax: SOV/SVO/VSO, modifier order, genitive order (correlated with head direction).
- Meaning representation (`meaning.rs`), deterministic renderer, interlinear gloss, placeholder English translation.
- Inscriptions: tomb, ledger (list or "X brought N goods"), warning/command, dedication. A recurring cast of 24 named people with fixed relatives and titles appears across texts.
- CLI: `corpus [--count] [--json] [--spoil]`, `grammar --spoil [--json]`.
- Tests: determinism, prefix-stability of corpora, cross-seed variety, phonotactics of every form and corpus word, collisions, gloss ↔ meaning round trip, surface → unique analysis, snapshots for seeds 1/42/9001.

**Open questions for Jb** (each also marked `DESIGN-Q:` in code)
1. **Four cases, not 2–3.** "Made this for X" needed a dative. The alternative is an adposition.
2. **Commands** are a bare verb with no subject; there is no imperative affix.
3. **Word order extras:** numerals and "this" go where adjectives go; datives and adverbs follow head direction; appositions ("child of X", titles) always follow the name.
4. **Presentation:** words are space-separated and names are unmarked. Scriptio continua or a name marker would change difficulty a lot.
5. **Romanisation** falls back to non-ASCII letters (ŋ, ñ, š, ĥ, ë…) to stay unambiguous. Is ASCII with separators preferred?
6. **Cast size** of 24: names recur enough to cross-reference, but the same tomb can occasionally appear twice.
7. Numbers stop at ten; there is no numeral system yet.
8. `grammar` without `--spoil` refuses rather than showing a partial sheet.

**Next**
- Jb's notebook decipherment session (deliverable checks in the milestone doc).

## 2026-10-02 — Roadmap to a finished game

**Done**
- Added `docs/ROADMAP.md` and milestone specs M02–M14 in `docs/milestones/`; moved the M01 spec there.
- The finished game is defined as a complete engine plus Jb's complete content pack, joined by a slot registry and an authoring tool (M03, M13). A release gate (M14) refuses to ship any placeholder text.
- Design gates listed per milestone, each with a proposed default.
- Cost estimates calibrated on M01 (about $6).

**Next**
- Jb: decide on the M01 open questions above (or accept the defaults in M02), then start M02.

## 2026-10-02 — M02 Language depth

**Done**
- **Script** (`script.rs`): alphabet, abjad or syllabary per seed. Glyphs are sets of marks (stroke, turn, spot) from a 9-stroke vocabulary. Per-seed script logic makes voiced sounds, fricatives and nasals add a shared mark to their partner's glyph, so readers can discover it. Writing direction is left-to-right, right-to-left or boustrophedon. Every glyph is checked to be distinct. A debug SVG renderer and mechanical debug descriptions; player-facing descriptions are left for M03 content slots.
- **Numerals** (`numerals.rs`): base 10, 12, 20 or 5+10; biggest-first or smallest-first; optional bare powers ("hundred" or "one hundred"). An "and" linker is added automatically when a system would otherwise be ambiguous (for example, small-first base 5+10 makes 16 and 60 identical). All of 0–999 round-trip. Numeral signs are additive or positional. Ledgers now carry exact totals.
- **Eras** (`history.rs`, `Language::at_era`): 3 eras by default (1–5). Each era applies 2–3 ordered, exceptionless sound changes from a catalogue: mergers, vowel shifts, intervocalic lenition, palatalisation, cluster loss and final-vowel loss. Each rule also updates the era's phonotactics. Rules never repeat or undo an earlier one, and spelling is shared across eras. When change makes words collide, one gets a new root; when affixes collide, one erodes into a separate particle word. A few words are also replaced by chance. The script simplifies some glyphs and adds glyphs for new sounds.
- **Registers**: everyday (tombs, ledgers, warnings, dedications, plus new labels and letters) and potent (a fixed opening and closing word, and a particle before the verb). Potent meaning is left to M08.
- **Difficulty dials** (`difficulty.rs`): word separation (spaces, dividers, none), name determinative, forced script kind, era count, fused morphology. Dials change presentation only; tests check that they don't reshuffle the words.
- **CLI**: `script --spoil`, `eras --spoil`, `corpus --era E --glyphs`, `grammar --era E`, and flags for every dial.
- **Bench** (`tools/bench`, built by `tools/bench/build.sh`): eras side by side, glyph view, script table with drawings, all dials. It sits outside the workspace until M03's `web` crate replaces it.
- **Tests**: phonotactics in every era; every word and affix in era *n* is exactly the rules applied to era *n−1*; every surface word in every era has one analysis; glyphs are distinct; numerals and signs round-trip; ledger totals add up; dials leave words alone; snapshots for seeds 1, 42 and 9001 in all eras (grammar, script, corpus, spoilers, glyph text). The WebAssembly build gives byte-identical output to native.

**Open questions for Jb** (each also marked `DESIGN-Q:` in code)
1. **Proposed defaults kept** from the roadmap gate: the M01 answers stay as built, and separation and name marking are now dials. Please confirm or change.
2. **Abjad** writes word-initial vowels with one carrier sign. **Boustrophedon** doesn't mirror glyphs on reversed lines.
3. **Numeral signs** exist in every script, but inscriptions write numbers as words. Should ledgers use signs?
4. **Erosion and replacement rates**: one affix erodes by chance in about one era in five, and about one word in 25 is replaced per era (more when sound change forces it). Tune?
5. **Potent claims** use open, burn and break with the thing as subject ("let the gate not open"). The potent particle always sits right before the verb.
6. **Glyph line width** is fixed at 16 until surfaces have real sizes.
7. **Same meanings across eras**: inscription *n* means the same thing in every era of a corpus, which helps comparing eras in the bench. In the game, M04 decides what is actually written where.

**Content slots added:** none yet. Glyph description phrasing becomes the first slot family in M03, and the engine already exposes the structured glyphs it needs.

**Usage:** not measured from inside the session.

**Next:** M03, content system and authoring tool v1.
