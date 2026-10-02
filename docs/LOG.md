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
