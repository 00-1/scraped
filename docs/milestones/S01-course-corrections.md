# S01 — Course corrections

**Do this before carrying on with D04.** If D04 is part-way done, finish the step in hand, commit, do S01, then resume D04.

**Goal:** fix what Jb's review of D01–D03 found. Two of these change how every later milestone is judged (the explorer bot, and reading), so they come first.

**Done when:** the D03 samples are regenerated with the new explorer and show it digging and travelling; reading reads as an impression first; the script is learnable through familiarity, naming and heard sounds; and the listed bugs are gone.

## 1. An explorer that explores like a player

Review of the D03 samples (three seeds, ten hours each): the explorer never used `look closer`, `listen` or `smell`, used `look` three times, visited three natural features in total, and spent most of its time examining every wall in the first town (93 examines that returned only a material and a name). The milestones are judged by this bot's metrics and transcripts, so it must play the way a curious person would.

- Use the whole verb set in proportion: `look around`, `look closer`, `listen`, `smell`, `look up`, `look down`, `touch`, `examine`, `read`, travel, and the group verbs (`look at the tombs`).
- Follow curiosity, not completeness: go towards whatever the last response made interesting (a sound, a smell, a standout, a landmark), and stop examining a kind of thing once examining it stops turning up anything new.
- Split time between towns, the land between and (after D04) interiors; head out to natural features and landmarks it can see.
- Add a **verb mix** metric (share of each verb family over the run) and a **places visited by kind** metric to `scraped-lang depth`, with targets: digging verbs at least 20% of commands; at least 5 natural features visited in ten hours on most seeds.
- Regenerate `docs/samples/D03/` with the new explorer (keep the old as `D03-old/` for comparison) and rewrite its `NOTES.md`. Re-check the D02 and D03 targets with the new bot and record the numbers in `docs/DEPTH.md`.

## 2. Arriving somewhere describes it

Reaching the standing stones on seed 1 printed "A rock pillar to the south. Here and there: bare earth." and nothing about the stones. When the player arrives at a place they set out for (a feature, a district, a building, a landmark, a named place), that place is the most salient fact in the arrival response. Test: on arrival at any target, the first fact rendered is about the target.

## 3. Reading: impression first

Reading is about a sixth of all commands, and `read` prints every glyph description over several pages, which is the longest text in the game. Following "Say little; let the player dig":

- `read <thing>` gives an **impression**: how many marks and lines, how they're made and their condition, whether it's scraped and what shows beneath, and which signs recur or are already known to the player. A few short facts through the attention model, no glyph list.
- `read closely` (also `read on`, `copy`, `study`) gives the sequence sign by sign, paged as now, rendered according to section 4 so it gets shorter as the player learns.
- Short texts (a few signs) may go straight to the sequence.

## 4. A script that is obscure but learnable

Jb's concern: a text as a long list of stroke descriptions may be too obscure. It should stay obscure at first, but become legible as the player learns it.

### 4a. Signs the player can talk about

- Each glyph of each era is a **sign** with a stable identity the player can refer to, independent of where it appears.
- `signs` (also `my signs`) lists the signs met so far: a short handle or the player's name for each, how many times seen, and any heard sound (4d). Paged, brief.
- Refer to a sign by its position in the text being read (`the third sign`, `sign 3`), by its handle (`the hooked bar`), or by the player's name for it.

### 4b. Familiarity compresses

- **First meetings** show the full description (the existing `glyph.describe` slot).
- After a sign has been seen a few times (proposed: 3), it is shown by a **handle**: a short name built from its most distinctive strokes ("the hooked bar", "the ringed cross"), generated to be unique within the era's script. Handles are a new slot (`glyph.handle`) with the strokes as variables, so Jb words them.
- `examine the hooked bar` or `look closer` while reading still gives the full description.

### 4c. Naming

- `call <sign> <name>` (also the existing `define`, `label`) names a sign with any short string the player chooses, usually a guessed sound. Names are never checked against the truth.
- **A named sign renders as its name alone.** A text whose signs the player has named reads as a line of their own transliteration, with word breaks: `ka ti / mo ra / ...`. Unnamed signs show by handle or full description.
- Make this discoverable without pointing at the writing mechanic: it is note-taking, like naming places. Mention it in help and the manual slots.

### 4d. Heard sounds (design gate)

Real decipherment usually found sounds before meanings. The game should offer in-world ways to learn what a sign sounds like.

**Proposed default (Jb's idea): scraping makes the sound.**

- As strokes come away under a blade, each sign gives a faint sound. A player who scrapes while listening, or in a quiet place, hears them, and the heard sound attaches to the sign, shown distinctly from the player's own names (for example ⟨ka⟩ beside or instead of a name).
- Scraping everyday writing releases nothing, but it costs legibility (scraped text is partly lost, as now). So learning sounds and keeping texts readable pull against each other, which is a real choice.
- Scraping potent writing sounds too, and then acts.
- Keep it faint and easy to miss, so it doesn't point at the mechanic: it is one of the first uncanny hints that the writing is more than marks, in keeping with "Writing is background, at first". Heard only on `listen` while scraping, or unprompted in quiet places.
- Sounds are rendered through a slot (`glyph.heard`) with the phoneme's romanisation and features as variables, so Jb decides how sounds are written.

**Alternatives or additions** (record in `docs/DECISIONS.md` for Jb):
- **Names from nature:** some animals and birds (D06) are named after their calls; a carving or label pairs the creature with its written name.
- **An earlier decipherer's notes:** traces of a previous explorer (the D06 gate) include their sign list with some sound values, some wrong. Their notes are storylets in Jb's words; the values come from the engine.
- **Acoustic places:** a whispering gallery or a ringing stone where inscriptions are tied to sounds.

Build the default; leave hooks for the alternatives.

### 4e. Metrics

Add to `scraped-lang depth`: words per `read` (impression) and per page of `read closely`, before and after a simulated player has named the commonest signs; signs met per hour; and how many signs a scraping-curious bot learns the sound of in ten hours.

## 5. Bugs

- **Identical options:** "Which do you mean: a firesteel and a firesteel?" (21 times in the D03 samples). When options are identical things, pick one; when they differ, distinguish them by their traits or position, never by an identical name.
- **Dangling reference:** "Among them stands the worn storehouse." after `out`, when the whole-place fact it refers to wasn't said. A standout fact must not depend on another fact being in the same response, or it must carry its own antecedent.
- **Raw ids:** `pry_bar`, `drain_lever` appear in player-facing names. Item and mechanism names go through slots with readable ids; add a test that no rendered response contains an underscore.
- **Closed doors with no reason:** "A door north (closed) will not move", seven times on seed 9001. Distinguish locked, barred from the other side, stuck, swollen, held by writing, and blocked by rubble, each with a cue the player can notice (a keyhole, a bar's shadow, warped wood, nothing visible). D05 adds keys; until then, the cue is enough.

## Checklist

- [ ] Explorer bot plays like a curious player; verb-mix and places-visited metrics; targets met
- [ ] D03 samples regenerated (old kept as `D03-old/`), notes rewritten, D02/D03 numbers re-checked
- [ ] Arrival describes the place arrived at; test
- [ ] `read` gives an impression; `read closely` gives the sequence
- [ ] Signs with stable identities; `signs` command; referring to signs
- [ ] Familiarity handles (`glyph.handle` slot)
- [ ] Naming; named signs render as names
- [ ] Heard sounds from scraping (`glyph.heard` slot); alternatives recorded in `docs/DECISIONS.md`
- [ ] Reading metrics
- [ ] Bugs: identical options, dangling reference, raw ids, unexplained closed doors
- [ ] LOG.md entry; then resume D04
