# Agent protocol

*Scraped Again* can be played by programs as well as people: the JSON-lines
protocol of the terminal client, or the MCP server. Both carry exactly what a
human player sees and nothing more (S06): no hidden lists, ids, clock or
figures. Ground truth (meanings, maps) and the state summary are only in the
developer tools, for debugging.

## JSON lines (`scraped --json`)

```sh
scraped --json --seed 42            # or --code K5G0-9ZQ1, --difficulty gentle
```

Send one command per line, either bare (`look`) or as `{"cmd": "look"}`.
Every response is one line of JSON:

| Key | Since | Meaning |
|---|---|---|
| `protocol` | 1 | The protocol version. A client should refuse versions it doesn't know. |
| `text` | 1 | Exactly what a human player sees. |
| `ended` | 2 | Present, and true, once the run has ended. |
| `state` | 1 | Developer builds only (`scraped`, not `scraped-player`): a summary of the state, for bots and debugging. Removed from the player build in version 2. |
| `truth` | 1 | Only with `--spoil` (developer builds): position, live claims, understanding, regions, storylets; `attention`, every fact the description weighed (slot, key, salience, score, whether new, variables), said or not; at the end, the run record and the chronicle's gloss. |

Session commands are handled by the client, not the game: `save [FILE]`,
`load [FILE]`, `transcript on|off`, `export [PREFIX]`, `code`, `quit`.

Agents type what a player types. Since S01 that includes reading in layers
(`read X`, `read closely`, `examine sign 4`, `trace sign 4`) and writing by
sound: `write kati mo on wall` spells words in the script of the day from
the signs whose sounds have been heard; `#4` copies in sign 4 of the last
text read. Glyph table numbers are no longer accepted, and `define` is gone.

**Versioning.** Adding keys does not change the version; removing or
changing the meaning of one does. Version 1 is the first release.

## MCP (`scraped-mcp`)

```sh
scraped-mcp --content path/to/content
```

A stdio MCP server (protocol revision 2025-06-18) with five tools:

| Tool | Arguments | Does |
|---|---|---|
| `new_game` | `seed` or `code`, optional `difficulty` | Starts a game; returns the opening. |
| `act` | `command` | One command, as a player types it. |
| `save` | | The game as a save (seed, difficulty, commands). |
| `load` | `save` | Loads a save from `save`. |
| `seed_code` | | The world's shareable code. |

Each result has the player's text as `content` and the JSON-lines response
(with `protocol`) as `structuredContent`.

An agent playing *with* a person gets what the person gets, plus the
fair-play note (`docs/coop/FAIR-PLAY.md`); there are no house rules (S06).
