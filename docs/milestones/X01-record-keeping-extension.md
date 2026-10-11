# X01 — Record-keeping extension

**When:** not scheduled. **Don't start this until Jb says so;** he isn't sure yet whether to build it. Recorded so the idea is captured and nothing built meanwhile closes it off. Design: `docs/DESIGN.md`, "Possible future: a record-keeping extension".

**Goal:** an optional companion to the base game that keeps records for players who want it. The base game stays as it is: it keeps no records, and players can play bare or with their own tools.

## Start from the playtests

Playtests show how players actually collect and organise evidence: what they record, how they label marks and texts, how they draw maps, and what they wish the game did for them. Keep each playtest's notes in `docs/playtests/<date>/`, next to its transcript. Before designing anything here, read them. The first is `docs/playtests/2026-10-11/field-notes.html`: a pannable map with object cards, texts as grouped sign labels, a sign list with guesses and where each was seen, and open questions. Shape the extension's records on what players built for themselves.

## Scope (when started)

1. **Markup in the text.** Tags around places, directions, distances, things carried, marks and tracings, for example `{place:the worn temple}`, `{dir:east}`, `{mark:…}`. Rules:
   - Removing every tag leaves exactly the plain text.
   - Tags only label kinds a reader would recognise anyway. Never strange, magic, important or new.
   - No identities: two things with the same name are not told apart, and mentions of one thing are not linked.
   - Jb's slot text stays untouched. The engine tags only the variables it fills in.
   - Off by default in the terminal. The app can hide tags, colour them or make them tappable.
2. **The extension reads only the player's text** (the transcript, with markup). It has no access to game state, and the same records come from a saved transcript as from live play (test).
3. **What it records:**
   - tracings, filed by place;
   - texts seen, by place;
   - a map by dead reckoning from what the text says ("far to the east", "a morning's walk north"), which drifts and which the player can correct.
4. **Levels** (Jb to choose which to offer):
   - a searchable transcript;
   - the map;
   - tracings and texts by place;
   - a catalogue of mark shapes met.
5. **Recorded with the world,** like difficulty, and shown in shared worlds and the end summary.
6. **The base game doesn't lean on it.** Bots keep playing bare, and no part of the game becomes tedious on the assumption that most players use it.

## Done when

Jb has decided to build it. Write the targets and tests then.
