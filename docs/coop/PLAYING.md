# Playing a shared world (for agents)

How an agent, for example Claude in a Claude Code cloud session, takes its
turn in a world it shares with a person. Read `FAIR-PLAY.md` first; the
player program shows it to you too.

## 1. Fetch the player program

Every release puts the player program on its GitHub release page, with a
`SHA256SUMS` file, and the Pages site lists every release's builds. On Linux:

```sh
base=https://github.com/00-1/scraped/releases/latest/download
curl -sSLO $base/scraped-player-linux-x86_64
curl -sSLO $base/SHA256SUMS
sha256sum --ignore-missing -c SHA256SUMS
chmod +x scraped-player-linux-x86_64
./scraped-player-linux-x86_64 --version
```

Use the version the world needs: the program refuses a world played on a
newer major version, and says so.

## 2. Clone the worlds repo

Worlds live in a private repository that holds only world files (one
`NAME.world` per world, plus `NAME.BRANCH.world` for a branch), never source
code. Your partner gives your session access to it.

```sh
git clone https://github.com/OWNER/WORLDS.git worlds && cd worlds
```

## 3. Play

```sh
./scraped-player-linux-x86_64 --world NAME.world --as ai --json
```

Each line you send is a command (`look`, `read stele`, `go temple`, `help`)
or `{"cmd": "..."}`; each reply is one JSON line. Every move is written into
the world file as you make it, tagged `ai` with the time. Two more lines
carry table talk, which isn't a move:

- `talk TEXT` leaves your partner a message.
- `talk since N` reads the messages after the first N (0 for all); the reply
  says which N to use next time.

Read the table talk before your first move, and say what you did and why
before you stop. Through MCP instead (`--mcp`), the same are the tools
`open_world`, `act`, `talk` and `talk_since`.

The program keeps a note of the newest copy of each world it has played
(under `$SCRAPED_HOME`, else `~/.scraped`) and refuses an older one: there
is no undo. Pull before you play.

## 4. Push

```sh
git add NAME.world && git commit -m "ai: N moves" && git push
```

If the push is refused because your partner played meanwhile:

```sh
git fetch && git show origin/HEAD:NAME.world > theirs.world
./scraped-player-linux-x86_64 merge NAME.world theirs.world
```

The merge keeps the newer copy and both sides' table talk. If you both
played on from the same point, the world has split: yours stays, theirs is
written beside it as a branch, and your partner chooses which line to keep
(or both). Commit what the merge wrote, then push.

Don't copy world files to try moves out: see `FAIR-PLAY.md`.
