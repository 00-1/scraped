# M11 — Endings and legacy

**Goal:** every run ends, and the ending means something. The player's time in the world ends by death, by leaving, or in other ways; then the game finally shows the whole picture: how the world ended up, and what the player did to it.

**Depends on:** M10. **Design gate:** metaprogression between runs.

**Done when:** Jb finishes runs by at least three different endings and reads a summary and chronicle that accurately reflect what happened, including a chronicle in the world's own language that he can mostly read.

## Design decisions

| Decision | Proposed default |
|---|---|
| Ways a run ends | Death, leaving (a late discovery), old age, being overtaken by collapse, writing yourself into the world |
| How leaving works | A departure inscription: a potent claim whose concepts are only learnable from the deepest text. Writing and scraping it ends the run by choice |
| Final summary | Two parts: a zoomed-out assessment (authored templates fed by simulation) and a chronicle in the language, written as by those who came after |
| Metaprogression | Only legacy: the final inscription (if any) appears as a faint trace in the next world generated with legacy enabled. No unlocks |

## Scope

### In

1. **End conditions:** detection and recording for each way a run ends, with full context.
2. **The deepest text:** the root event from M04 gets a physical location, the deepest surface stack in the world, and readable only with the strongest deep-reading tool. Reading it reveals (in the language) the cause of the world's state and the concepts needed to leave.
3. **Leaving:** the departure claim and its effect (ends the run, with the world's state frozen as the outcome).
4. **Writing yourself in:** a player text naming themselves in the potent register, scraped, ends the run with the player becoming a trace in the world.
5. **The run record:** a complete, structured account of the run: places discovered, named places, texts read, claims released (by whom), regional changes caused, how it ended.
6. **Zoomed-out summary:** the final regional state compared with the starting state, region by region, and the causal chain back to the player's actions. Rendered entirely through slots.
7. **The chronicle:** a short text in the late era of the language, generated from the run record as meaning and rendered through the grammar, describing the player's era and deeds as seen by successors. Shown first untranslated; a spoiler toggle reveals a gloss after.
8. **Legacy:** a small legacy file (outside the save) that carries the final inscription into a future world: placed as a faint, very deep layer somewhere plausible.
9. **Notebook export:** at the end, offer the transcript, the list of named places and the run record as files, for the player's records.

### Out

Authored frame story and opening (M12).

## Content slots introduced

Death narration (per cause, already partly in M07) leading into the end; leaving; ageing out; being overtaken; writing yourself in; the summary's framing and per-region assessment phrases; the chronicle's presentation; legacy trace descriptions.

## Tests

- Every end condition is reachable by a scripted run on a test seed.
- The summary agrees with the simulation state (regression on recorded runs).
- The chronicle parses back to the meaning it was generated from (round trip via M09 parser).
- Legacy traces appear in the next world when enabled and never when disabled.

## Checklist

- [x] End conditions and recording
- [x] Deepest text placement and content
- [x] Departure claim
- [x] Writing yourself in
- [x] Run record
- [x] Zoomed-out summary through slots
- [x] Chronicle in language
- [x] Legacy file and placement
- [x] Notebook export
- [x] Slots registered with examples
- [x] Tests listed above
- [x] LOG.md entry
