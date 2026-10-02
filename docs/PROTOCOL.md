# Agent protocol

*Scraped Again* can be played by programs as well as people: the JSON-lines
protocol of the terminal client, or the MCP server. Both carry exactly what a
human player sees, plus a structured summary; ground truth (meanings, maps)
is only given with `--spoil`, for debugging.

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
| `state.place` | 1 | `outside`, or `structure S room R`. |
| `state.things`, `state.carried`, `state.exits` | 1 | Names as the player sees them. |
| `state.minutes` | 1 | Minutes since the first midnight. |
| `state.landmarks`, `state.edges`, `state.weather` | 1 | Outdoors only: what is in view, by rough bearing and distance. |
| `state.travelled` | 1 | The last journey as the player perceived it. |
| `state.wrote`, `state.scraped` | 1 | What the last `write` or `scrape` did. |
| `state.body`, `state.light`, `state.load` | 1 | Needs by coarse state, light, weight carried. |
| `state.dead` | 1 | How the run ended, once it has. |
| `truth` | 1 | Only with `--spoil`: position, live claims, understanding, regions, storylets; at the end, the run record and the chronicle's gloss. |

Session commands are handled by the client, not the game: `save [FILE]`,
`load [FILE]`, `transcript on|off`, `export [PREFIX]`, `code`, `quit`.

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

For an agent playing *with* a person, see `docs/coop/CLAUDE.md`: house rules
for co-op play.
