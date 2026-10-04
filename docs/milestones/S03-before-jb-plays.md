# S03 — Before Jb plays

**When:** now, before D10 (finish and commit any D10 step in hand first). **Size:** small. Fixes only; no new systems, no new design. One short session.

**Goal:** Jb is playing the game himself on 2026-10-05. Give him a current build he can open, and fix the small wording bugs that show in the D09 samples.

## 1. A build Jb can play (do this first)

- Tag the first release, **v0.1.0**, so the release workflow publishes the player builds and the Pages site lists them (`players.html`). From now on, later milestones are held to `docs/VERSIONING.md`. This answers the question in the D09 LOG entry: yes.
- Refresh the Android preview APK from the current build (the `android-preview` release), so the app has D09 and C01.
- Check that the browser player on the Pages site starts a new world and plays a few commands.
- Add three lines at the top of `docs/HANDOFF.md`: where Jb plays (browser link, APK link), which version it is, and how to send a save back for a bug (`scraped-lang replay`).

## 2. Wording bugs seen in the D09 samples

1. **Plurals.** Groups and counts use the singular, or add -es twice: "A few tawny crow", "Many green locust", "Two clay wick", "Two clay provisionses". Give every creature, plant and object name a plural (regular by rule, irregular and mass nouns listed in data). Groups and counts use it. Test: render every name in the plural for three seeds; none ends in "-seses", and every plural differs from its singular unless the name is listed as unchanging.
2. **Plural features as subjects.** "A field walls to the southwest." A feature whose name is plural takes no article ("Field walls to the southwest"). Test with every feature kind.
3. **Exact counts in `read.whole`.** "This writing was scraped; 20 of 46 glyphs are lost." This breaks "Say little; let the player dig": layer 1 is an impression. Replace the `lost` and `glyphs` variables with a coarse amount (`lost`: a few, some, about half, most, nearly all), and update the slot description and its example variant. `read closely` already shows each lost sign as "(worn away)", so a player who wants the exact count can still find it.
4. **Related signs with no findable base.** "like a squarish, angular sign, with a curl above, with a dot beside": several signs could be "a squarish, angular sign". The base should be said the way the player meets it elsewhere: by its sound if heard, otherwise by the impression that sets it apart (its resemblance, if it has one). For example, "like the star sign, with a dot beside". Test: for every related sign, the base's wording matches exactly one sign in that script.
5. **Content lint error:** `say.take_fixed` has a variant that uses `cause`, which the slot doesn't declare (found in C01). Fix it in the slot (add the variable) rather than in Jb's text.
6. **Sample pages:** a few transcript blocks in `docs/samples/D09/` leave a code fence open (after `out` on seed 1). Fix it in the sample writer.

## Done when

- The tests above pass, and the D09 samples are regenerated with none of these bugs. The rest of each transcript may only change where these fixes touch it.
- v0.1.0 is tagged and published, the APK is refreshed, and the HANDOFF lines are in.
- LOG entry; then D10.

## Checklist

- [x] v0.1.0 tagged; player builds on Pages; browser player checked
- [x] Android preview APK refreshed
- [x] HANDOFF: where and how Jb plays
- [x] Plurals for every name; groups and counts use them
- [x] Plural feature names without an article
- [x] `read.whole`: coarse amount lost, not exact counts
- [x] Related signs name a base that can be found
- [x] `say.take_fixed` lint error
- [x] Sample code fences
- [x] D09 samples regenerated; LOG entry; then D10
