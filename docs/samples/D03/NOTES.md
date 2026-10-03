# D03 samples (regenerated in S01): notes

The curious explorer's first ten hours on seeds 1, 42 and 9001, with the example text. Regenerate with `scraped-lang samples D03 --hours 10`. The samples from before S01 are kept in `../D03-old/` for comparison. The world is the same; the explorer and the reading are new.

## What the new explorer does

- **It digs where it arrives, but not everywhere.** On reaching a spot outside it looks, often looks around, and sometimes listens, smells, or looks up or down. In a room it often looks closer, and sometimes listens, smells or touches. Over seeds 1–10 these are about a fifth of its commands (none before S01).
- **It follows what it notices.** On seed 9001, after taking in the town, it goes to the terraces it was told of, then to a burial mound, an old road and a dead forest. On seed 42 it goes to the terraces by its town. It heads towards sounds and smells from somewhere, and looks at groups of alike buildings ("look at the tombs").
- **It stops examining what gives nothing.** After three walls or tables that turned up nothing, it stops examining that kind of thing. Before S01 it spent most of its time examining every wall in the first town.
- **It splits its time.** It sees about four buildings of a town once it has a firesteel and a cloak, leaves any building after about 45 minutes, and goes out to features and landmarks. Over seeds 1–10 that is 13 buildings in about 1.5 towns, and 2.6 features, in ten hours. The features target (5) is not met: the body sends it back to shelter in the afternoon and to sleep, and features lie far apart.

## Reading in layers

- `read the stele` is now a glance: "On a stone stele, sixty-six signs in twelve groups, about six lines, in a cramped hand." About 22 words, against a page of stroke descriptions before.
- `read closely` goes sign by sign by impression ("an angular sign, a wedge turned up, with a tail"). Related signs read as related ("like an angular sign, a cross, with a hook turned up"). Recurring signs are easy to spot down a page (seed 9001's stele repeats a handful).
- `examine sign 1` gives a fuller impression; `trace sign 6` (seed 42) gives the exact strokes and takes minutes.
- The explorer never scrapes, so it hears no sounds. The scholar, with a scraper in hand, hears about 33 signs' sounds in ten hours.

## What still reads flat (for Jb and later milestones)

- **The example text is placeholder.** "A terraces, in granite", "A provisions lies here" are example wording, not Jb's. The new slots (`read.whole`, `glyph.impression`, `glyph.closer`, `glyph.heard`, `trace.*`, `write.unheard`, `sign.name`) each have a description and variables for Jb.
- **Impressions are long when a sign is told by its relation** ("like an angular sign, a wedge turned up, with a tail, centre, with a hook turned up, centre"). Jb's wording of `glyph.impression` can shorten this, for example by naming the related sign's most striking stroke only.
- **A close page is still about 170 words** (16 signs). Once common signs are heard it falls to about 110. Whether a page should hold fewer signs is for Jb.
- **Doors held by writing say only that they won't move.** That is the cue (nothing to see), but it may read as a bug until the player learns it.
