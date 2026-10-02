# Content pack

Every word of English the player reads comes from here. The engine never
contains prose: it declares **slots** (places where it needs text), and the
files in this folder fill them with **variants**.

Edit these files with the authoring tool (it shows every slot, what it is
for, its variables, and live previews from real generated worlds), or by hand
in any text editor.

## Files

One file per slot family: `glyphs.toml` holds every `glyph.*` slot, and so
on. Each variant is a `[[variant]]` block:

```toml
[[variant]]
slot = "glyph.stroke"
text = "{a stroke}, turned {turn}, {spot}"
when = "turn != 'none'"   # optional: only used when this holds
weight = 2                # optional: twice as likely as weight 1
```

- Variants marked `example = true` were written by an agent to show the
  intended shape. They count as placeholders: the release check refuses a
  pack that still relies on them. Replace or delete them.
- A slot with no variant shows `⟦slot: name⟧` in development builds.
- `notes = "…"` at the top of a file is for your own notes.

Long text can use triple quotes:

```toml
text = '''
First line,
second line.'''
```

## Template language

| Write | Gets |
|---|---|
| `{turn}` | the value of a variable |
| `{red\|green\|blue}` | one of the choices, picked the same way every time for the same situation |
| `{a stroke}` | "a hook", "an arc" |
| `{cap x}` | x with a capital first letter |
| `{plural word n}` | "jar" or "jars", by n |
| `{count n jar}` | "one jar", "three jars" |
| `{number n}` | "forty-two" |
| `{list strokes}` | "a, b and c" |
| `{bearing deg}` | "north-east" |
| `{distance m}` | "about 300 metres" |
| `{duration min}` | "about an hour" |
| `{cap list strokes}` | helpers chain, right to left |
| `{>other.slot}` | another slot's text, with the same variables |
| `{lang.word gate}` | the generated language's word for a concept |
| `[if n > 2]…[else]…[end]` | part of a variant that depends on a condition |
| `{{`, `}}`, `[[` | a literal `{`, `}` or `[` |

Conditions (in `when` and `[if …]`) use `==`, `!=`, `<`, `>`, `<=`, `>=`,
`has` (a list holds a value), `and`, `or`, `not`, parentheses, variable
names, numbers and quoted text: `spot == 'top' and count > 2`,
`carrying has 'torch'`.

Where several variants apply, one is chosen by weight. The same seed and
situation always give the same choice, and the same variant is not used
twice in a row for a slot when there is an alternative.

## Storylets

A storylet is a hand-written event or place the game weaves into every
generated world. It is a `[[storylet]]` table in any file (the examples are
in `story.toml`); its text is the slot `story.<id>`, written as ordinary
variants. The authoring tool has a form for it and can show where it lands
in a real world.

```toml
[[storylet]]
id = "shrine"
about = "An old temple far from the start, with a warning about water."
at = "structure"          # anywhere | structure | outdoors | hook
# hook = "opening"        # with at = "hook": opening, first_scraped_seen,
                          # tool_found, first_release, first_write,
                          # great_reached, deepest_found, ending
when = "carrying has 'torch'"   # optional condition
after = "opening"               # optional: another storylet first
effects = ["flag shrine_seen"]  # give <item>, flag <name>, unflag <name>, open
# repeat = true           # may happen more than once

[storylet.place]          # where it may be placed (structure / outdoors)
structure = ["temple"]    # kinds of building
biome = ["grassland"]     # kinds of land
era = "old"               # old | middle | new
near_water = true
away = true               # never in the starting town

[storylet.inscription]    # writing in the world's own language, put there
register = "everyday"     # everyday (a warning) | potent (a claim)
about = "water"           # a concept or a kind of thing
era = "old"               # default: the building's own era
```

Its text and condition can use: `place`, `biome`, `era`, `indoors`, `day`,
`season`, `time`, `age`, `life`, `water`, `stability`, `climate`,
`carrying`, `happened`, `flags`, `tool`, `ending`, `read`, `written`,
`released` and `inscription` (the name of the thing bearing its writing).

## Checks

The authoring tool lints as you type. From the command line:

```sh
cargo run -q -p scraped-cli -- content lint
cargo run -q -p scraped-cli -- content coverage
cargo run -q -p scraped-cli -- content preview glyph.describe --seed 42 --count 5
cargo run -q -p scraped-cli -- content release-check
```
