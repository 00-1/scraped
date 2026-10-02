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
- **A backdrop**, which can be switched off: a faint reading room by
  lamplight (drifting warm light in the dark theme, mottled paper in the
  light ones). It is painted by the Rust engine from the clock and the
  theme only. It never shows anything about the game, which has no
  graphics. It stays still if the phone asks for reduced motion and stops
  when the app is out of sight.

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

This writes `android/dist/scraped-again.apk`. It needs:

- Rust with the Android targets (`aarch64-linux-android`,
  `armv7-linux-androideabi`, `x86_64-linux-android`) and `cargo-ndk`.
- The Android SDK and NDK (`ANDROID_HOME`, `ANDROID_NDK_HOME`).
- A JDK (17+) and node.

It does four things:

1. It builds the tools, then writes the app's name from the `app.label`
   slot.
2. It compiles the engine (`crates/android`, with Jb's content built in)
   for each phone architecture.
3. It runs Gradle.
4. It copies the APK out.

CI does all of this. It then plays the app on an emulator
(`./gradlew connectedDebugAndroidTest`, from `android/`).

The layout:

- `crates/android` — the engine as a native library. It holds the JNI
  bridge to the same JSON interface the browser uses, plus the backdrop
  painter.
- `android/app/src/main/java/org/scrapedagain/`:
  - `MainActivity` — the one screen.
  - `Ui.kt` — every screen, in Jetpack Compose with Material 3.
  - `Theme.kt` — colours, the reading face and the backdrop.
  - `AppModel.kt` — the game, saving, sync, backup and agent access.
  - `Engine.kt` — the engine, called on its own thread.
  - `Store.kt` — the saved files.
  - `AgentServer` and `AgentProtocol` — the agent's two doors, MCP and plain
    HTTP.
- `android/app/src/androidTest/`:
  - `AppTest` — plays a world on a device: it types, sends an agent move,
    connects over HTTP with the key and uses the notebook.
  - `EngineTest` — checks that the engine on the phone produces the same
    transcripts as every other platform.
- `android/test-agent.sh` — tests the agent server off-device on a plain JVM.

### Signing

Android only updates an app that is signed with the same key. To sign for
release, set `ANDROID_KEYSTORE` (a PKCS12 file), `ANDROID_KEYSTORE_PASSWORD`
and optionally `ANDROID_KEY_ALIAS` (default `scraped`). Without them, the
APK is signed with the machine's debug key.

CI signs with the repository secrets `ANDROID_KEYSTORE_B64` (the keystore in
base64) and `ANDROID_KEYSTORE_PASSWORD`. **Until those are set, each CI build
has a different debug key.** Then a new APK won't install over the last
one, and you have to uninstall first, which loses worlds that aren't synced.
To make a key once:

```sh
keytool -genkeypair -keystore scraped-again.p12 -storetype PKCS12 \
  -alias scraped -keyalg RSA -keysize 4096 -validity 10000
base64 -w0 scraped-again.p12   # paste as ANDROID_KEYSTORE_B64
```

The native APK can't be installed over the earlier WebView one, which was
signed with a different key. Sync or export your worlds first, uninstall,
then restore. The saved-file format is the same.

The Play Store would need an app bundle instead (`./gradlew bundleRelease`).
That's a later step.
