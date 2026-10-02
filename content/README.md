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
`and`, `or`, `not`, parentheses, variable names, numbers and quoted text:
`spot == 'top' and count > 2`.

Where several variants apply, one is chosen by weight. The same seed and
situation always give the same choice, and the same variant is not used
twice in a row for a slot when there is an alternative.

## Checks

The authoring tool lints as you type. From the command line:

```sh
cargo run -q -p scraped-cli -- content lint
cargo run -q -p scraped-cli -- content coverage
cargo run -q -p scraped-cli -- content preview glyph.describe --seed 42 --count 5
cargo run -q -p scraped-cli -- content release-check
```
