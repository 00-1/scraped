# S01 — Course corrections

**Do this before carrying on with D04.** If D04 is part-way done, finish the step in hand, commit, do S01, then resume D04.

**Goal:** fix what Jb's review of D01–D03 found. Two of these change how every later milestone is judged (the explorer bot, and reading), so they come first.

**Done when:** the D03 samples are regenerated with the new explorer and show it digging and travelling; reading works in the layers of `docs/DESIGN.md` ("How the script is perceived"); signs have stable shape impressions and can be learned by their heard sounds; arbitrary labels are gone; and the listed bugs are gone.

## 1. An explorer that explores like a player

Review of the D03 samples (three seeds, ten hours each): the explorer never used `look closer`, `listen` or `smell`, used `look` three times, visited three natural features in total, and spent most of its time examining every wall in the first town (93 examines that returned only a material and a name). The milestones are judged by this bot's metrics and transcripts, so it must play the way a curious person would.

- Use the whole verb set in proportion: `look around`, `look closer`, `listen`, `smell`, `look up`, `look down`, `touch`, `examine`, `read`, travel, and the group verbs (`look at the tombs`).
- Follow curiosity, not completeness: go towards whatever the last response made interesting (a sound, a smell, a standout, a landmark), and stop examining a kind of thing once examining it stops turning up anything new.
- Split time between towns, the land between and (after D04) interiors; head out to natural features and landmarks it can see.
- Add a **verb mix** metric (share of each verb family over the run) and a **places visited by kind** metric to `scraped-lang depth`, with targets: digging verbs at least 20% of commands; at least 5 natural features visited in ten hours on most seeds.
- Regenerate `docs/samples/D03/` with the new explorer (keep the old as `D03-old/` for comparison) and rewrite its `NOTES.md`. Re-check the D02 and D03 targets with the new bot and record the numbers in `docs/DEPTH.md`.

## 2. Arriving somewhere describes it

Reaching the standing stones on seed 1 printed "A rock pillar to the south. Here and there: bare earth." and nothing about the stones. When the player arrives at a place they set out for (a feature, a district, a building, a landmark, a named place), that place is the most salient fact in the arrival response. Test: on arrival at any target, the first fact rendered is about the target.

## 3. Reading in layers

Reading is about a sixth of all commands, and `read` prints every sign's stroke-by-stroke description over several pages, which is the longest and most mechanical text in the game. Jb's decisions (`docs/DESIGN.md`, "How the script is perceived" and "The game is not your notebook") replace this with three layers:

1. **`read <thing>`: the whole text, at a glance.** An impression of the mass of writing: how much there is, in how many lines or bands, how it's made (cut, incised, painted, scratched), its condition, whether it was scraped and something shows beneath, and perhaps that a few shapes keep recurring. A few short facts through the attention model. No sign-by-sign listing.
2. **`read closely` (also `read on`, `study`, `look closer` while reading): an impression of each character in turn**, paged as now. Each sign is given as a short **shape impression**: something a person would see and remember (tall and hooked; a ring like an eye; a squat cross with a dot beside it), with word breaks shown. A sign whose sound the player has heard is shown by its sound instead (section 4).
3. **`examine the fourth sign`: a fuller impression** of that one sign (its proportions, its most distinctive part, what it resembles, how it's cut), still how it looks rather than how it's built.
4. **`trace the fourth sign` (also `copy`, `make a rubbing of`): the exact strokes**, as the current `glyph.describe` gives them, precise enough to draw. Jb: breaking a sign down stroke by stroke is a task in itself, so tracing is a deliberate activity:
   - it takes in-game time (proposed: several minutes per sign; tracing a whole text is a long job, and can be interrupted);
   - it needs enough light, and a sign that's legible (worn or scraped signs trace partly or not at all);
   - it may use materials (charcoal, a cloth or paper for rubbings) where the world has them, as a DESIGN-Q;
   - the result is given once, in the moment; the game doesn't keep it (the player's notebook does).

Short texts (a few signs) may go straight to layer 2.

## 4. Signs: impressions and heard sounds

### 4a. Shape impressions

- Generate an **impression** for every sign of every era from its strokes: overall silhouette (tall, squat, round, angular, open, closed), its dominant stroke, its most distinctive mark, and where it fits, a resemblance (a comb, a bird's foot, an eye, a hook on a bar).
- Impressions are **stable**: the same sign always gives the same impression, so a player can recognise repeats and match signs across texts by themselves.
- Impressions should tell signs apart: on gentle and standard difficulty, no two signs in one era's script share an impression; on archaeologist, a few deliberately confusable pairs are allowed and recorded.
- Related signs (the script logic from M02, where voiced sounds add a shared mark) should read as related ("like the hooked sign, with a dot").
- Rendered through a new slot, `glyph.impression`, with the features above as variables, so Jb words them. `glyph.describe` and `glyph.stroke` remain for layer 3.
- Eras: the script changes between eras (M02), so a sign's impression in an older era may differ, which is a real clue for a player comparing layers.

### 4b. No lists, no labels

- **No `signs` command** and no in-game list of signs met. Keeping track is the player's job.
- **Retire arbitrary labels:** remove `define`/`label`/`call` for signs, and the labels in game state. Keep save files loading (ignore old labels). Update help and the manual slots.
- **Writing without labels** (DESIGN-Q, proposed default): the player writes with **heard sounds** (`write "ka ti mo" on the wall`), and can include signs they haven't heard by **tracing** them from a text in view (`trace the fourth sign of the stele`), which takes the time and light that tracing takes. Agents use the same inputs. Update `crates/game/src/composing.rs`, the protocol docs and the tests.

### 4c. Heard sounds (the one place the game keeps something)

- **Signs make their own sounds.** As a sign's strokes come away under a blade, it gives its sound, faintly. A player who is listening (`listen` while scraping, or anywhere quiet) hears each sign as it goes.
- **The sound attaches to the sign.** From then on, wherever that sign appears (in that era's script), layer 2 shows it by its sound, romanised, instead of its shape impression. This is the one exception to "the game is not your notebook", and it is recorded in `docs/DECISIONS.md` as such: without it, matching a heard sound to one of dozens of described shapes would be unreasonably obscure.
- **It's a choice.** Scraping everyday writing releases nothing, but the scraped text is partly lost (as now), so learning sounds costs legibility.
- Scraping potent writing sounds too, and then acts.
- **Faint, easy to miss:** the sounds are one of the first uncanny hints that the writing is more than marks, in keeping with "Writing is background, at first". They don't announce themselves; a player has to notice.
- Sounds go through a slot (`glyph.heard`) with the romanisation and phonetic features as variables, so Jb decides how sounds are written and how hearing them feels.
- Syllabaries give syllables; alphabets and abjads give single sounds. Logographic signs, if ever added, would need another source.

**Alternatives or additions** for Jb (record in `docs/DECISIONS.md`, leave hooks, don't build):
- **Names from nature:** some animals and birds (D06) named after their calls, with a carving or label pairing the creature and its written name.
- **An earlier decipherer's notes:** traces of a previous explorer (the D06 gate), giving some sound values, some wrong. Their notes are storylets in Jb's words.
- **Acoustic places:** a whispering gallery or ringing stones tied to inscriptions.

### 4d. Metrics

Add to `scraped-lang depth`: words per `read` (layer 1) and per page of `read closely` (layer 2), with no sounds known and with the commonest signs heard; the share of signs in an era's script with a unique impression; and how many sign sounds a scraping-curious bot hears in ten hours.

## 5. Bugs

- **Identical options:** "Which do you mean: a firesteel and a firesteel?" (21 times in the D03 samples). When options are identical things, pick one; when they differ, distinguish them by their traits or position, never by an identical name.
- **Dangling reference:** "Among them stands the worn storehouse." after `out`, when the whole-place fact it refers to wasn't said. A standout fact must not depend on another fact being in the same response, or it must carry its own antecedent.
- **Raw ids:** `pry_bar`, `drain_lever` appear in player-facing names. Item and mechanism names go through slots with readable ids; add a test that no rendered response contains an underscore.
- **Closed doors with no reason:** "A door north (closed) will not move", seven times on seed 9001. Distinguish locked, barred from the other side, stuck, swollen, held by writing, and blocked by rubble, each with a cue the player can notice (a keyhole, a bar's shadow, warped wood, nothing visible). D05 adds keys; until then, the cue is enough.

## Checklist

- [x] Explorer bot plays like a curious player; verb-mix and places-visited metrics; targets met (digging 0.22, met; features visited 2.6 of 5: not met, see `docs/DEPTH.md`)
- [x] D03 samples regenerated (old kept as `D03-old/`), notes rewritten, D02/D03 numbers re-checked
- [x] Arrival describes the place arrived at; test
- [x] Reading in layers: whole text, sign impressions, a fuller impression on examining one sign
- [x] Tracing as a task: time, light, legibility, optional materials; the only source of exact strokes
- [x] Shape impressions per sign and era (`glyph.impression` slot); uniqueness by difficulty
- [x] Arbitrary labels and their commands removed; old saves still load
- [x] Writing with heard sounds and traced signs (DESIGN-Q); composing, protocol and tests updated
- [x] Heard sounds from scraping (`glyph.heard` slot), attached to signs; recorded in `docs/DECISIONS.md` as the exception
- [x] Alternatives recorded in `docs/DECISIONS.md`
- [x] Reading metrics
- [x] Bugs: identical options, dangling reference, raw ids, unexplained closed doors
- [x] LOG.md entry; then resume D04
