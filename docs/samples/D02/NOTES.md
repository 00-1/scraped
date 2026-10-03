# D02 samples: notes

The curious explorer's first three hours on seeds 1, 42 and 9001 after D02, with the example text. Regenerate with `scraped-lang samples D02`. Compare `docs/samples/D01/`.

## The fixture: seed 42, the opening

**Before (D01):**

```
You wake in a grassland, with no memory of arriving.

Clear, in the morning.

You stand on slope ground, in grassland. The road runs here and the lakeshore lies to the west.

You stand among ruins in a grassland, in the morning. Here the worn temple, the worn house, the intact house, the worn house, the damaged storehouse, the damaged cemetery, the intact tomb, the intact tomb, the worn tomb, the worn tomb, the intact tomb, the intact tomb, the intact tomb and the well.

In the distance: the flat mountain to the east (middle).

It is spring. The land is parched. Little grows.
```

**After (D02):**

```
You wake in a grassland, with no memory of arriving.

An intact town of several tombs. Among them stands the worn temple. Here and there: bare earth.
```

That's three facts and no repetition: the town as one whole, one standout, and the dry land shown by its ground rather than stated. Everything that was cut can still be found by digging:

```
> look
An intact town of several tombs. Among them stands the damaged storehouse.
> look
An intact town of several tombs. Here and there: cracked mud.
> look around
The road runs here. The flat mountain to the east (middle).
> look closer
Several tombs, in mixed repair. Among them stands the damaged cemetery. A few houses, in mixed repair. The well stands here. The worn house stands out. Grassland, slope ground.
> look at the tombs
The intact tomb, to the east. The intact tomb, to the east. The intact tomb, to the southeast. Four others beyond.
> count the tombs
You count seven tombs.
> listen
Birds. Faintly, lake from the west.
> smell
A smell of flowers. A smell of water, from the west.
> look up
The sky: sun low in the east. Birds wheel overhead.
> look down
Here and there: bare earth. Slope grassland underfoot, dry.
> touch ground
The air is cool and draughty; the ground is dry.
> check myself
You feel well enough.
```

The season is shown only by evidence (birds wheeling overhead, green shoots, young animals), and needs only in `check myself` and the sensations felt as a threshold is crossed.

## What reads well
- Arrivals are short: 17 words and 3 facts at the median, down from 30 words and 14 facts. A `look` is 12 words, down from 88.
- Repeated looks say less, and they say what hasn't been said yet (another standout, another sign of the land), never the same list again.
- Rooms read in one line: what the room is, what stands out in it, and the ways out.
- Journeys end with the report, whatever stopped the walk, and at most two facts about the place reached.

## What still reads flat (for Jb and later milestones)
- **Members of a group look alike.** "The intact tomb, to the east" twice in a row: members differ only by bearing and condition. D03 (places with character) gives buildings more to tell them apart by.
- **The example text is thin.** "Faintly, lake from the west" and "The sky: sun low in the east" are placeholder examples that print the ids. Jb's variants are what will make these read well; every new slot has its description and variables in the authoring tool, with the new "In a response" panel showing each piece among its neighbours.
- **Things in rooms are sparse.** Many examines still return only the name ("A stone hearth."). A second examine adds size, material and condition (`thing.closer`), but most things have nothing more to find until D05.
