# Scraped Again for Android

The game as an Android app (Android 8 and newer). It shows the game only:
no spoilers, no tools, just the worlds you play and your notebooks.

## Installing

Download `scraped-again.apk` (the CI's `scraped-again-apk` artifact, or a
release), open it on the phone, and allow your browser or file manager to
install apps when Android asks. To update, install a newer APK over it; it
must be signed with the same key (see *Signing* below), or Android will
refuse and you must uninstall first. Uninstalling deletes your worlds
unless they were backed up or synced.

## How it's made to read

The app has a chat app's shape, because the game is a conversation:

- **Worlds** are listed like conversations: newest first, with the last
  thing the game said, the day, and when you last played. Long-press a world
  to share its code, export it or delete it.
- **In a world**, the game's text is set as prose in a book face, full
  width, the way the Claude and ChatGPT apps set replies. Your commands are
  small bubbles on the right. An agent's commands are tinted and tagged.
- **The composer** sits above the keyboard and keeps the keyboard up after
  you send, which Android apps often get wrong. It is a pill that grows for
  long `write` commands, with a send button and a button that recalls your
  last command. Autocorrect and capitals are off: commands aren't prose.
- **Chips** above the composer save typing, the thing that most often makes
  interactive fiction miserable on phones (the best-loved Android IF app,
  Text Fiction, replaced typing with touch). Some chips do something in one
  tap: `look`, the ways out, `inventory`. Others build a command: tap
  `read`, then `stele`, then send.
- **Tap a passage** to copy it or add it to the world's **notebook**: a page
  of your own for glyph tables, word lists and guesses, kept with the world.
- **Reading comfort**: text size, light, dark or sepia, a serif or plain
  face, and Android's own font-size setting is respected. The newest text
  stays in view as the keyboard opens and closes, and a *Latest* button
  appears if you have scrolled back. Back works as expected everywhere.

Everything the app says around the game comes from Jb's `app.label` slot
(content/app.toml), including the name under the icon.

## Integrations

The plug icon on the worlds screen.

- **Google backup.** Worlds and notebooks are included in Android's own
  backup to your Google account (Settings › Google › Backup must be on), and
  come back when you set up a new phone or reinstall. *Back up soon* asks
  Android to do it at its next chance; Android decides when.
- **Sync to a file.** Choose a file anywhere Android's file picker reaches,
  including Google Drive. Every change is written there. On another phone,
  *Restore from file* brings the worlds in (newer copies win). *Import*
  takes a single exported world.
- **Agent access.** Lets an AI agent play the open world. It can read the
  game's text and type commands, and nothing else: no saves, no other worlds,
  no spoilers, no code. Its moves appear in your transcript, tagged. Turn it
  on to see an address and a key; the agent must be on the same Wi-Fi, and
  the app must stay open.

### Connecting an agent

With agent access on, the screen shows something like
`http://192.168.1.23:8765/mcp` and a key. *Copy settings* copies an MCP
configuration.

Claude Code, on a computer on the same network:

```sh
claude mcp add --transport http scraped-again http://192.168.1.23:8765/mcp \
  --header "Authorization: Bearer YOUR_KEY"
```

The server has two tools: `read` (the latest exchanges, as text) and
`act` (one command; returns the game's reply). Anything that speaks HTTP
can use the same two abilities:

```sh
curl -H "Authorization: Bearer YOUR_KEY" http://192.168.1.23:8765/read
curl -H "Authorization: Bearer YOUR_KEY" --data "read stele" http://192.168.1.23:8765/act
```

*New key* stops every existing connection. For co-op house rules, see
`docs/coop/CLAUDE.md`.

The address only works on your local network. An agent in the cloud (for
example Claude on claude.ai) can't reach a phone directly; that would need a
relay service, which doesn't exist yet.

## Building

```sh
android/build.sh
```

This writes `android/dist/scraped-again.apk`. It needs a JDK (17+), node,
python3, and Rust with the wasm32 target. It does not use Gradle or the
Android SDK manager. Instead it fetches the Android 35 platform jar, aapt2
(from Apktool's release), D8 (R8's release) and apksig (Maven Central) into
`android/.tools`. It then:

1. builds the page (`tools/app`, with the engine and content inside);
2. writes the app name from the `app.label` slot;
3. compiles the resources and Java;
4. dexes;
5. runs the agent-server tests on the JVM;
6. signs with the v2 scheme and verifies.

The layout:

- `android/src/` — the native shell:
  - `MainActivity` — the WebView, keyboard and system bars, back, storage,
    sharing, file sync, backup and the agent bridge.
  - `Store` — the saved files.
  - `AgentServer` and `AgentProtocol` — the agent's two doors, MCP and plain
    HTTP.
- `android/res/`, `android/AndroidManifest.xml` — theme, icon, backup rules.
- `tools/app/index.html` — the whole interface. It is tested on an emulated
  phone by `tools/smoke/app.cjs`.

### Signing

Android only updates an app that is signed with the same key. Without one,
`build.sh` makes a key in `android/.keystore/` (git-ignored). CI uses the
repository secrets `ANDROID_KEYSTORE_B64` (a base64 PKCS12 keystore with
alias `scraped`) and `ANDROID_KEYSTORE_PASSWORD` when they are set; set them
once so every CI build can update the last.

The APK is signed with the v2 scheme only, which is what Android 8 and newer
check. The Play Store would need an app bundle instead; that's a later
step.
