# C01 — Shared play and versions

**When:** straight after D08, before D09. (Jb may start a separate session for it sooner; if so, that session works only on C01.) **Size:** one milestone. Keep to scope.

**Goal:** let Jb and an AI play the same world, when either likes, without either needing the other online and without any public server; make it hard for the AI to cheat by accident or on a whim; and let engine updates during playtesting carry on existing worlds wherever possible.

**Done when:** Jb plays a world in the app; an AI in a Claude Code cloud session picks it up from the shared location using the public player program, plays, leaves table talk and pushes; Jb sees its moves and notes in the app; and a minor engine release loads both their copies and they carry on.

## Decisions from Jb (2026-10-04)

- **Trust the AI, but make the engine hard to see inside.** No referee server. The AI plays a stripped, hardened player program, with house rules asking it to play fair.
- **The player program is public**, downloadable from the GitHub Pages site. **Worlds sync to a private location.**
- **A short fair-play note** is shown to any agent that plays.
- **Versions:** a major version bump needs a new world; changes should ship as minor versions wherever possible.

## Scope

### 1. The player program (hardened)

A separate build for playing only, `scraped-player` (terminal client, JSON-lines protocol and MCP server in one binary), plus the same in WebAssembly for the browser player:

- **Spoilers compiled out:** debug and spoiler features (`--spoil`, `truth`, grammar sheets, world maps, `writing`/`regions`/`site` views, coverage traces) are behind a cargo feature the player build doesn't enable. The options don't exist in it.
- **Text baked in:** the content pack is embedded, as rendered templates only. No slot descriptions, authoring notes or ids that describe mechanics.
- **Stripped:** release mode, symbols stripped, no debug output.
- **Opaque worlds:** see section 3.
- **Release:** CI builds it for Linux, macOS, Windows and WebAssembly on each release tag, and publishes it on the GitHub Pages site with a checksum and its version. The developer CLI (`scraped-lang`) keeps every spoiler tool for Jb and the build agents.

### 2. Shared worlds

- **The world file** holds the opaque save (section 3), the transcript (what each side saw), and **table talk**: messages between Jb and the AI that aren't game commands. Every move and message is tagged with who made it (`jb`, `ai`) and when.
- **Private sync location** (DESIGN-Q, proposed default): a **private GitHub repo used only for worlds** (no source code), which a Claude Code cloud session can clone and push to, and which the app syncs with using a token scoped to that one repo. Alternative: the app's existing sync-to-a-file (for example Google Drive) if the AI's environment can reach it. Build the default; keep the sync layer swappable.
- **Turn-taking:** whoever has the newest copy plays. If both played from the same point (Jb offline on the bus while the AI played), the app shows that the world has **split**, and offers to keep one line or keep both as **branches** of the world. Branches are cheap: they share history up to the split.
- **In the app:** a world list that shows which worlds are shared, who moved last, and unread table talk; a table-talk view beside the transcript; a sync status.
- **For the AI:** the player program's MCP server and JSON protocol gain `talk` (leave a message) and `talk_since` (read messages since a point), alongside the existing read-the-text and type-a-command. A short `docs/coop/PLAYING.md` explains how an agent fetches the player program, clones the worlds repo, plays and pushes.

### 3. Saves that survive updates

Today a save is the seed plus the command list, and loading replays it. Almost any rule change then makes an old save replay into a different world. Change this:

- **Snapshot saves:** a save holds a snapshot of the game state (world as generated so far, including lazily generated interiors already visited; player; simulation; memory of what's been said), plus the command list and transcript as history. Loading restores the snapshot; it does not replay.
- **Opaque:** the save is serialised compactly and scrambled with a key built into the player program, so the seed and state can't be read or edited by eye. This is to discourage, not to secure.
- **No undo:** each save carries a turn counter and a chain hash of its moves. The player program refuses to continue a world from an older copy than the newest it has seen for that world (it records the newest per world locally), and table talk flags any rewind.
- **Version stamp:** each save records the engine version that last wrote it and the version that created the world.

### 4. Versioning policy

Write `docs/VERSIONING.md` and enforce it in CI:

- **Engine version** `MAJOR.MINOR.PATCH`, set in the workspace `Cargo.toml`, shown by the player program and recorded in saves. The content pack keeps its own hash, and content changes never break saves (as now).
- **Patch:** wording, presentation and fixes that don't change rules or generation. Saved worlds **replay identically**.
- **Minor:** new or changed rules, behaviour, items, slots. Saved worlds **load from their snapshot and carry on** under the new rules. Places not yet generated (unvisited interiors, for example) may come out under the new rules; that's accepted.
- **Major:** changes to world generation that would alter what's already generated, or to the save format without a converter. Saved worlds need a new world, unless the release ships a converter (`upgrade` step) for the old format.
- **Prefer minor.** When a change would be major, look first for a way to make it minor (keep the old behaviour for existing worlds, version a generator step, write a converter).
- **CI checks:** a corpus of real saves from each release (`tests/saves/<version>/`, including a few from Jb's playtests if he adds them). On every change: patch-level replay test, minor-level load-and-continue test, and a report saying which bump the change needs. The release workflow refuses a tag whose version bump is smaller than the change requires.
- **The app and player program** refuse to open a world from a newer major version (asking to update), and upgrade a world from an older minor version on load, noting it in the transcript.

### 5. Fair play

- **`docs/coop/FAIR-PLAY.md`:** a short note, in the worlds repo and shown by the player program to any agent (in the MCP server's instructions and the first JSON reply). Proposed wording, for Jb to edit:

  > You're playing *Scraped Again* with a person. Play from what the game shows you. Don't look for the game's source code or data, don't reverse-engineer the player program or its save files, don't replay or copy worlds to see what a move would do, and don't use spoilers from anywhere. Keep your own notes; share your guesses and reasons with your partner. If you're unsure whether something is fair, ask.

- Update `docs/coop/CLAUDE.md` (the house rules) to point to it and to the new table-talk tools.

## Tests

- The player build contains no spoiler commands or JSON fields (test against the built binary and the WebAssembly module).
- Snapshot round trip: save, load, continue gives the same transcript as playing straight through.
- A world saved by the previous minor release loads and continues; a patch release replays identically (save corpus).
- Rewinding to an older copy of a world is refused.
- Splits are detected; both branches load.
- Sync: a scripted app-side and agent-side session alternate moves through a local test repo.

## Checklist

- [ ] Player build with spoilers compiled out, text embedded, stripped; published on GitHub Pages per release
- [ ] Snapshot saves; opaque; turn counter and chain hash; version stamps
- [ ] `docs/VERSIONING.md`; save corpus; CI bump check; release refuses too-small bumps
- [ ] Shared world file with transcript and table talk, tagged by who
- [ ] Private worlds repo sync (default) behind a swappable sync layer; app sync, world list, table talk view
- [ ] Splits and branches
- [ ] MCP and protocol: `talk`, `talk_since`; `docs/coop/PLAYING.md`
- [ ] `docs/coop/FAIR-PLAY.md` shown to agents; house rules updated
- [ ] Tests listed above
- [ ] LOG.md entry; then D09
