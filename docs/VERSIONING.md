# Versioning

The engine's version is `MAJOR.MINOR.PATCH`, set once in the workspace `Cargo.toml`. The player program shows it (`scraped-player --version`) and every save records it: the version that made the world, the one that last wrote it, and every build each stretch of play ran on. The content pack has its own hash; changing Jb's text never breaks a save.

## What each kind of change may do

- **Patch:** wording, presentation and fixes that change no rule and no generation. Saved worlds **replay identically**.
- **Minor:** new or changed rules, behaviour, items or slots. Saved worlds **load from their snapshot and carry on** under the new rules. Places not yet generated (an interior not yet visited) may come out under the new rules; that's accepted.
- **Major:** a change to world generation that alters what is already generated, or to the save format without a converter. Saved worlds need a new world, unless the release ships an `upgrade` step for the old format.

**Prefer minor.** When a change would be major, look first for a way to make it minor: keep the old behaviour for existing worlds, version a generator step, or write a converter.

## How it's checked

- **The save corpus**, `tests/saves/<version>/`: real saves from each release, each with its world's fingerprint (`scraped-lang saves add NAME --seed N`; Jb's playtest saves are welcome here too).
- **`cargo test`** loads every corpus save of the current major version from its snapshot and plays on (`crates/game/tests/corpus.rs`).
- **`scraped-lang saves check`** says what the change needs: patch if every save replays identically, minor if one loads but replays differently, major if one no longer reads or its world generates differently. CI prints it on every push.
- **The release workflow** runs `saves check --against <last tag>` and refuses a tag whose version bump is smaller than the change needs.

## Opening worlds

The app and the player program refuse a world made by a newer major version (asking to update), and carry on a world from an older minor version from its snapshot, noting the new build in the save's history.
