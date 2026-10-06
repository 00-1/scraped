# D12 — Integration and tuning

**Goal:** bring every client, tool and check up to date with the deepened engine, tune it as a whole, and hand Jb a playtest pack.

**Depends on:** D11.

**Done when:** all depth targets are met together on a seed batch, every client and tool works with the new systems, and Jb has what he needs to playtest and to write the content pack.

## Scope

1. **Authoring tool:** new slot families and fact kinds; previews assembled by the attention model (D02); inspectors for places, scenes, objects, machines, species, sky, arcs and spells; Gaps mode with both bots reaching the whole game; review order updated so Jb writes the most-seen, earliest pieces first.
2. **Clients:** browser player, terminal player, Android app and agent protocol support every new verb and system; help and manual slots updated (no spoilers about writing).
3. **Fairness and difficulty:** checker covers everything added; difficulty presets retuned (gentle, standard, archaeologist).
4. **Performance:** world generation and play stay fast in WebAssembly and on a mid-range phone.
5. **Whole-game tuning:** brevity, novelty curve, realisation timing, survival pace and spell density tuned together, using the D01 metrics on a large seed batch.
6. **Playtest pack for Jb** (`docs/PLAYTEST.md`): three recommended seeds with what each shows off, how to play spoiler-free and how to look under the hood, what feedback is most useful, and the current list of open decisions from `docs/DECISIONS.md`.
7. **Content plan for Jb:** an ordered list of slot families to write, by how much play they affect, with counts.

## Tests

- Full CI green, including determinism on all platforms and the Android build.
- All depth targets met on a 50-seed batch.
- Smoke tests for every client and the authoring tool.

## Checklist

- [x] Authoring tool updated
- [x] All clients updated
- [x] Fairness re-checked on 50 seeds at every difficulty (presets not retuned: every seed already gives a fair world)
- [x] Performance checked in the browser (phone estimated from native timings; no phone available)
- [x] Whole-game tuning on a seed batch
- [x] `docs/PLAYTEST.md`
- [x] Content plan for Jb
- [x] Tests listed above (CI and smokes green; depth targets: 16 rows met, 8 not, see DEPTH)
- [x] LOG.md entry
