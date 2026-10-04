# S02 — Sign impressions as whole shapes

**When:** after D07 is done, before D08. **Size:** small and contained. One component (`crates/lang/src/impression.rs`), the `glyph.impression` slot's variables and its example variant. Don't touch other systems. One focused session; if a target is still missed after two or three attempts, report it and move on to D08.

## The problem

Jb's design (`docs/DESIGN.md`, "How the script is perceived") says that reading closely gives an impression of each sign as a person would see it, a shape you could recognise again, and that the exact strokes come only from tracing. The D06 samples show impressions are still stroke specifications with a word in front:

> 9. like a plain sign, a tail turned right, with a bar, right, with a hook turned right, centre

The cause is in the engine, not the example wording. An impression is an outline word (six values), the main stroke and its turn, and further marks with their positions, rendered through `glyph.stroke`, the tracing vocabulary. Resemblance has six values and rarely applies. Even Jb's own wording couldn't make these read as shapes.

## What to change

1. **Work out whole-shape features from the sign's geometry** (the strokes as drawn by the debug renderer), not from its stroke list:
   - proportions (tall, wide, squarish, slight);
   - curved, angular or mixed;
   - spare or busy;
   - symmetry (side to side, top to bottom, none);
   - enclosed spaces (none, one, two…);
   - separate parts (one piece, a piece and a dot, two pieces…);
   - one standout feature, said as a part of the shape (a dot above, a long tail, a crossing, an opening at the top, a ring inside) with at most a coarse place (above, below, inside, beside), never left/right/centre coordinates;
   - a resemblance where one fits, from a larger vocabulary of everyday things (about 30, such as a comb, a ladder, a bird's foot, an eye, a lamp, a key, a fork, a doorway, a wave, a seed), matched by rules on the features above.
2. **Tell signs apart with the fewest features needed**, adding the next most noticeable feature only where two signs would otherwise read alike. Gentle and standard: every sign in a script reads differently (as now). Archaeologist: a few alike pairs (as now).
3. **No stroke vocabulary in impressions.** `glyph.impression` must not render through `glyph.stroke` or expose stroke names (bar, tail, arc, zigzag, hook, wedge…), turns or spots. Those stay only in tracing (`glyph.describe`, `glyph.stroke`).
4. **Related signs still read as related:** "like that sign, with a dot above", with the added part said as a shape part.
5. Update the slot description and variables for Jb, and the one example variant. Keep `read.closer` (examine one sign) in step: a fuller impression, still no strokes.

## Done when

- A test shows no impression (example variants, all eras, three seeds, every script kind) contains a stroke name or a turn word.
- Unique-impression share is unchanged (1.0 on gentle and standard).
- `docs/samples/S02/` holds one `read closely` page per seed (1, 42, 9001), before and after, with a two-line note. No other samples needed.
- LOG entry; then D08.

**Done (2026-10-04).** All four met: the stroke-word test (`impressions_never_name_strokes`), unique share 1.0 on gentle and standard (`every_sign_is_told_apart`, 40 seeds × 3 script kinds), samples, LOG.
