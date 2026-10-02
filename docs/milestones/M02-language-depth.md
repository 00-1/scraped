# M02 — Language depth

**Goal:** turn the M01 language into the full palimpsest language: a script the player must draw, numbers they can count with, historical eras that sit on top of each other as layers, and a distinction between everyday writing and potent writing.

**Depends on:** M01. **Design gate:** Jb's answers to the M01 open questions in `docs/LOG.md`.

**Done when:** Jb can generate one language with three eras, draw its script from the glyph descriptions alone, read the same formula across eras, and spot the sound changes between them.

## Design decisions

| Decision | Proposed default |
|---|---|
| M01 open questions (four cases, bare imperatives, word-order extras) | Keep as built |
| Word separation and name marking | Both become difficulty dials (see below), default: separated, names unmarked |
| Romanisation with non-ASCII letters | Keep for spoiler/debug output only. In play, text is shown as glyph descriptions or a transliteration the player builds themselves (see Script) |
| Script type | Chosen per seed: alphabet, abjad (consonants only) or syllabary. Logographic is out of scope |
| Number of eras | 3 by default (old, middle, late), configurable 1–5 |
| How potent writing differs | A distinct register (see Registers) |

## Scope

### In

1. **Script**
   - Generate a glyph inventory mapping to phonemes (alphabet), consonants (abjad) or syllables (syllabary).
   - Each glyph is a composition of strokes from a small stroke vocabulary: hook, bar, dot, loop, crossbar, tail, and so on, with positions (over, under, beside, enclosing). Glyphs for related sounds share components where the seed's "script logic" says so (for example, voiced stops add a dot).
   - Deterministic **glyph descriptions** in structured form (stroke list + relations). Rendering those structures into English prose is a **content slot** (M03), not code. Until M03 lands, a debug renderer may describe them mechanically, marked debug-only.
   - Writing direction per seed (left-to-right, right-to-left, boustrophedon).
   - A debug SVG renderer that draws each glyph from its structure, so Jb can check that descriptions are drawable. Debug and authoring tool only, never in play.
2. **Numerals**
   - A numeral system per seed: base 10, 12, 20 or 5-10, additive or positional.
   - Numbers to at least 999 in words and, where the script allows, as numeral signs.
   - Ledgers use real totals, so sums can be checked (a classic decipherment anchor).
3. **Eras and sound change**
   - Treat the M01 output as the proto-language. Derive later eras by applying ordered, regular sound-change rules (lenition, vowel shifts, cluster simplification, final-vowel loss, merger).
   - Rules are generated per seed from a catalogue of attested change types, and every change is exceptionless.
   - Morphology can erode across eras (an affix merges or is lost and is replaced by a periphrastic form).
   - Some vocabulary is replaced between eras (new roots for a few concepts).
   - The script can change between eras too (glyph shapes simplify; new glyphs for new sounds).
   - `Language::at_era(n)` returns a full language for that era; corpus generation takes an era.
4. **Registers**
   - **Everyday** register: the M01 inscription types plus labels, signs, ledgers and letters.
   - **Potent** register: a recognisable frame (a fixed opening and closing formula, plus a dedicated verb form or particle) that marks writing as able to act. Its exact meaning stays unknown until M08 but its form is generated now.
5. **Difficulty dials** in a `Difficulty` struct carried by the language: word separation (spaces / dots / none), name marking (none / determinative glyph), script type restriction, number of eras, morphology regularity (M01's regular system vs. a few fused forms).
6. **CLI and bench**
   - `scraped-lang --seed N script --spoil` (glyph table with structures and SVG), `corpus --era E`, `eras --spoil` (the sound-change rules and example derivations).
   - Update the browser bench to show eras side by side and the glyph SVGs.

### Out

Placement of text in the world, scraping, layers stacked on surfaces (M08), player writing (M09), prose glyph descriptions (content, M03).

## Content slots introduced

Recorded here so M03 can register them: glyph description phrasing (stroke names, relations, how a full glyph is described).

## Tests

- Every era's words obey that era's phonotactics.
- Applying the sound-change rules to the proto form of every word gives exactly the later form (no hand-made exceptions).
- Every surface word in every era still has exactly one analysis.
- Glyph structures are distinct; no two glyphs have identical stroke compositions.
- Numerals round-trip: number → words → number for 0–999.
- Ledger totals are consistent.
- Snapshots for three seeds, all eras.

## Checklist

- [x] Script: stroke vocabulary, glyph generation, script logic, direction
- [x] Debug SVG glyph renderer
- [x] Numeral system and ledger totals
- [x] Sound-change rule catalogue and era derivation
- [x] Morphological erosion and lexical replacement across eras
- [x] Script change across eras
- [x] Everyday and potent registers
- [x] Difficulty dials
- [x] CLI commands and bench update
- [x] Tests listed above
- [x] LOG.md entry with open questions
