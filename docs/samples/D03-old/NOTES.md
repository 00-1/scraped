# D03 samples: notes

The curious explorer's first ten hours on seeds 1, 42 and 9001, with the example text. Regenerate with `scraped-lang samples D03 --hours 10`. Ten hours rather than three, because the point of D03 is what turns up over hours.

## What changed since D02

- **Towns have roles.** The opening now says what kind of place it is: "An intact holy city of a few houses" (seed 1), "A worn holy city of several tombs" (seed 9001). Seed 42's town is a refuge on a hilltop. Over seeds 1–10, a world has 5 or 6 of the 8 roles, and no two towns of one role and size share a layout.
- **New kinds of building, found by going in.** In the first town, the explorer goes into a processional way, a school, a scriptorium and a garden (seed 1). On seed 42 it finds a cistern, a kiln, a granary and an orchard. On seed 9001 it finds a library and a mausoleum. In the next town on seed 9001 it finds a granary, an orchard, a brewery, a kiln and a wayside shrine. A world has about 38 kinds of building (target: at least 25 of 35+).
- **The land between towns has places in it.** The explorer walks to a rock pillar and standing stones (seed 1), and to an ancient tree (seed 9001). A world has about 22 natural feature kinds, against about 3 before. The tall ones (waterfalls, rock pillars, sea stacks, glaciers, cairns, standing stones) are landmarks.
- **`look around` in a town gives its layout:** "A refuge by a hilltop, terraced. Parts: upper town, farmyards, graves and old town." `go to the farmyards` walks there.
- **Scenes wait to be found.** Each town has 3 to 5 (a meal left on a table, tools dropped, a barricade, a flood line), and there are about 19 out on the land. They weigh little at first glance, so they mostly turn up on `look closer`.
- **Still finding new kinds of thing in the fifth hour:** 6 new kinds in hour 5, averaged over seeds 1–10.

## What still reads flat (for Jb and later milestones)

- **The example text is placeholder.** "A wood tree", "A standing stones to the southeast", "Here: bones, bundle and bowl" print ids. Each new slot has a description and its variables in the authoring tool for Jb's own pieces.
- **New buildings are thin inside.** A mill is one room with a millstone and a waterwheel. D04 replaces every interior.
- **A scene is only its list of things.** What the things say together (the barricade facing the door, the meal's bowls still set) is for Jb's variants of `place.scene`, which gets the scene's kind, its things and its cause.
- **Reading still dominates the first hours** in towns with much writing (seed 42).
