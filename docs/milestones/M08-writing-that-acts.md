# M08 — Writing that acts

**Goal:** the heart of the game. Writing holds power, scraping releases it, and nothing is ever lost. Old scraped writing is visible in part and its effects linger in the environment; rare unscraped potent writing can be released with the scraping tool.

**Depends on:** M07. **Design gates:** what "influence" means, what marks writing as potent, whether ghost layers act, how conflicts resolve (see ROADMAP).

**Done when:** Jb, without spoilers, notices a strange environmental effect, finds the half-visible writing behind it, later finds the scraping tool, scrapes an unscraped potent inscription, and watches the world change in a way that matches what the text said.

## Design decisions

| Decision | Proposed default |
|---|---|
| What influence means | An inscription is a **claim** in the potent register ("the water stands still", "the door does not open", "the hall is warm"). When released, the world tries to make the claim true within a range. Effects follow meaning: subject, predicate, polarity, modifiers |
| Range and strength | Depend on surface size and material, the scraping tool's power (M10 scales this), and how specific the claim is. Vague claims are weak or unpredictable |
| What counts as potent | Only text in the potent register (M02), on a suitable surface. Everyday text never acts |
| Ghost layers | Inert. Only the most recent scraped layer on a surface is live |
| Conflicts | Nearer beats farther; at equal distance, newer beats older; directly contradictory claims at the same place cancel |
| Reading by eye | The top scraped layer shows a fraction of its glyphs, more for shallow scrapes and better light |

## Scope

### In

1. **Surfaces and layers:** every text from M04 now sits on a surface as a stack of layers (oldest at the bottom), with era, author/hand, register and state (unscraped / scraped). History from M04 decides the stacks: who wrote over whom, and why.
2. **The three laws as mechanics:**
   - *Writing holds power:* potent-register text is a latent claim.
   - *Scraping releases it:* `scrape <surface>` with a scraping tool removes the whole top unscraped text, which becomes a live scraped layer and its claim takes effect.
   - *Nothing is lost:* scraped layers persist as traces; nothing in the game deletes text.
3. **Claim semantics:** a mapping from potent-register meanings to the property and mechanism model in M07 (temperature, flow, light, stability, sound, openness, growth…). A **concept-to-property table** as data, so new concepts can be given powers without code.
4. **Effects engine:** active claims hold properties within range; they show up in descriptions only through physical cues, never as "the text says X". Historic writing events from M04 become live claims here, which is what makes the world strange.
5. **Partial reading:** scraped text renders through the damage filter (M03 markup): some glyphs visible, some lost, deterministically per surface and light. Unscraped text is read in full. The top scraped layer only; deeper layers need the M09 tool.
6. **Hands:** different authors and eras leave recognisable differences (glyph variants, era spellings, characteristic phrases) that attentive players can learn to tell apart.
7. **Tools:** the scraping tool becomes functional. Where it's found is placed by history (a scribe's workshop, a scraper's grave), reachable but not trivial. Surfaces may need different tools for different materials (default: one tool for the first release, with materials as a hook).
8. **The pivot moment:** the first unscraped potent inscription the player is likely to meet is near the scraping tool's location and has a visible, safe, unambiguous effect. Placement logic only; the prose is Jb's.
9. **Debug views:** surface stacks, active claims with ranges, and a per-site "why is this place strange" explanation, spoiler-only.

### Out

Player writing and the deep-reading tool (M09); claim scaling, great inscriptions and regional effects (M10).

## Content slots introduced

Surface descriptions by material and condition; damaged-text framing ("Beneath the soot, a few strokes survive:"); physical cues for every claim-driven property change (frost forming against the season, water standing still, an unnatural warmth); the act of scraping; the moment a claim takes effect; finding the scraping tool.

## Tests

- Scraping never deletes text; layer stacks only grow.
- Claim → effect mapping is deterministic and covered for every potent-capable concept.
- No description ever states a claim's meaning directly (lint test on rendered output against glosses).
- Conflicts resolve as specified.
- Every historic writing event leaves both a trace and a perceptible effect.
- On every seed, the scraping tool and a safe first potent inscription are reachable by ordinary means.

## Checklist

- [ ] Surfaces with layer stacks from history
- [ ] Scrape action and the three laws
- [ ] Concept-to-property table and claim semantics
- [ ] Effects engine with ranges and conflicts
- [ ] Historic writing events become live claims
- [ ] Partial reading through the damage filter
- [ ] Hands and era differences
- [ ] Scraping tool placement and the pivot moment
- [ ] Debug views
- [ ] Slots registered with examples
- [ ] Tests listed above
- [ ] LOG.md entry
