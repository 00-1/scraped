# Playtests

Each playtest Jb shares is kept in `docs/playtests/<date>/`, with no personal details:

- `transcript.md`: every command and the game's text, verbatim (no `state` objects).
- `save.txt`: the save from the end of the session, so the world can be rebuilt.
- Any notes the players kept (for example `field-notes.html`).
- `README.md`: the build, the client, the difficulty, who played, and where the findings went.

The transcript's commands become a fixture (`tests/fixtures/`), replayed on the save's world by a test in `crates/game/tests/depth.rs`, so the bugs it showed stay fixed. The players' notes are the evidence for X01, the record-keeping extension.

| Date | Build | Client | Fixture |
|---|---|---|---|
| [2026-10-11](2026-10-11/README.md) | v0.5.0 | Android app, AI partner over MCP | `tests/fixtures/s06-playtest.txt` |
