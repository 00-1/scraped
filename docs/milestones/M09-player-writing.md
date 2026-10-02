# M09 — Player writing

**Goal:** the player becomes an author. With the writing tool they compose new text in the language, the game parses it into meaning, checks it against what lies beneath, and when scraped it acts. With the deep-reading tool they see the layer beneath and learn where words may go.

**Depends on:** M08. **Design gates:** what counts as understanding a word; which layering rules apply.

**Done when:** Jb composes an inscription he was never shown, constructed from grammar he worked out, writes it over a suitable trace, scrapes it, and gets the effect he intended. A malformed attempt fails or misfires in a way he can learn from.

## Design decisions

| Decision | Proposed default |
|---|---|
| How the player enters text | In their own transliteration (M05) or by glyph number sequences; the game maps that to glyphs. It never accepts English |
| Understanding gate | The player can write a word only if they have *encountered* its root in at least 2 distinct texts (tracked silently). Grammatical forms need not have been seen, so composition is rewarded |
| Validation | Player text is parsed to a meaning. Text that doesn't parse is still written, and it is either inert or misfires (see below) |
| Layering rule | **Agreement:** a new text must agree with the surviving ghost words of the layer beneath (same case/number/class slots and register), as if completing it. Fresh surfaces accept anything |
| Deep reading | The tool reveals one layer beneath the top. Better versions in M10 reach further |

## Scope

### In

1. **Parser, surface → meaning:** the reverse of the M01 renderer, for the player's era (and older eras if the player writes them). Must handle every construction the renderer can produce. The M01 guarantee that every surface word has one analysis makes this tractable.
2. **Writing tool and `write` command:** `write "<text>" on <surface>`, with confirmation that echoes back the glyph descriptions (not the meaning).
3. **Understanding tracking:** a silent record of which roots and constructions the player has encountered, and where. Used for the gate; also exported in spoiler/debug mode.
4. **Layer constraints:** the agreement rule (plus register) checked against the ghost layer. Writing that violates it smudges or will not take, with a physical description, not a rule explanation.
5. **Misfires:** text that parses but is ungrammatical in a way the rules forgive, or that targets an unintended concept, produces the effect of what was *actually* written. Text that doesn't parse is inert. A small set of dangerous misfires (for example, an unintended negation) can hurt the player.
6. **Deep-reading tool:** reveals the next layer down on a surface, rendered through the damage filter at a deeper level. Placed by history like the scraping tool.
7. **Effects of player claims:** reuse the M08 engine; player-authored layers enter history (they will be read by the chronicle in M11 and can persist into the next world).
8. **Agent protocol:** `write` and `scrape` exposed with structured results so an agent partner can help compose; the agent sees only what the player sees.

### Out

Scaling scraping power and regional reach (M10), endings (M11).

## Content slots introduced

The act of writing (by material and tool), smudging or refusing, ink drying, misfire descriptions, deep-reading the layer beneath, finding the writing and deep-reading tools.

## Tests

- Parse(render(meaning)) == meaning for every generated sentence in all three snapshot seeds.
- Gate: words below the encounter threshold are refused; the threshold is configurable.
- Agreement rule fixtures: allowed and refused writes over specific ghosts.
- Misfire determinism: the same wrong text gives the same effect.
- An automated "decipherer" agent with spoiler access to grammar only (not to the world) can construct and successfully cast a target claim on most seeds, proving the system is consistent end to end.

## Checklist

- [ ] Surface → meaning parser
- [ ] Writing tool and `write` command
- [ ] Understanding tracking and gate
- [ ] Layer constraint (agreement + register)
- [ ] Misfires and inert text
- [ ] Deep-reading tool
- [ ] Player layers enter history
- [ ] Agent protocol extensions
- [ ] Slots registered with examples
- [ ] Tests listed above
- [ ] LOG.md entry
