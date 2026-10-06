package org.scrapedagain

import android.app.Application
import android.app.backup.BackupManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.OpenableColumns
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import java.security.SecureRandom

/** What the screen is showing. */
enum class Screen { Worlds, Game, Notebook, Integrations }

/** How the app looks; kept in preferences. */
data class Look(
    val size: Int = 18,
    val theme: String = "system",
    val font: String = "serif",
    val atmosphere: Boolean = true,
)

/** One-tap chips: whole commands, verbs to start one, and things to name. */
data class Chips(
    val commands: List<Pair<String, String>> = emptyList(),
    val verbs: List<String> = emptyList(),
    val things: List<Pair<String, String>> = emptyList(),
    val after: List<String> = emptyList(),
)

/**
 * The app's state and everything it does: worlds, playing, the notebook,
 * backup, file sync and agent access. Game calls go to the engine's thread;
 * state here is read by Compose on the main thread.
 */
class AppModel(app: Application) : AndroidViewModel(app) {
    private val store = Store(app)
    private val prefs = app.getSharedPreferences("app", Context.MODE_PRIVATE)
    private val playing = Mutex()

    var ready by mutableStateOf(false)
        private set
    var busy by mutableStateOf(false)
        private set
    var labels by mutableStateOf<Map<String, String>>(emptyMap())
        private set
    var screen by mutableStateOf(Screen.Worlds)
    val worlds = mutableStateListOf<World>()
    var world by mutableStateOf<World?>(null)
        private set
    val transcript = mutableStateListOf<Entry>()
    var chips by mutableStateOf(Chips())
        private set
    var look by mutableStateOf(loadLook())
        private set
    var syncName by mutableStateOf<String?>(null)
        private set
    var agentOn by mutableStateOf(false)
        private set
    var agentUrl by mutableStateOf<String?>(null)
        private set
    var agentKey by mutableStateOf("")
        private set
    /** Shared worlds (C01): the sync's state, as a label id ("" when idle). */
    var worldsStatus by mutableStateOf("")
        private set
    var worldsRepo by mutableStateOf("")
        private set
    var worldsMe by mutableStateOf("jb")
        private set
    /** Bumped when a shared world's file changes, so its talk redraws. */
    var talkVersion by mutableStateOf(0)
        private set
    private val history = ArrayList<String>()
    private var recallAt = 0
    private var agent: AgentServer? = null
    private var syncJob: Job? = null
    private var worldsJob: Job? = null
    private val syncing = Mutex()

    /** Brief notices for the snackbar, as label ids. */
    val notices = MutableSharedFlow<String>(extraBufferCapacity = 8)

    init {
        viewModelScope.launch {
            val l = Engine.ask(Engine.req("app_labels"))
            labels = l.keys().asSequence().associateWith { l.optString(it) }
            refreshWorlds()
            syncName = prefs.getString("sync", null)?.let { displayName(Uri.parse(it)) }
            if (prefs.getBoolean("agent", false)) setAgent(true)
            worldsRepo = prefs.getString("worldsRepo", "") ?: ""
            worldsMe = prefs.getString("worldsMe", "jb") ?: "jb"
            ready = true
            syncWorlds()
        }
    }

    fun label(id: String): String = labels[id] ?: ""

    private fun refreshWorlds() {
        val list = store.worlds()
        worlds.clear()
        worlds.addAll(list)
    }

    // ---------- worlds ----------

    /** Starts a new world: from a code someone shared, or a fair new seed. Returns false if the code doesn't read. */
    suspend fun newWorld(difficulty: String, code: String): Boolean {
        val (seed, diff) = if (code.isNotBlank()) {
            val d = Engine.ask(Engine.req("seed_decode", "code" to code.trim()))
            if (d.has("error")) return false
            d.getString("seed") to d.getString("difficulty")
        } else {
            val random = (SecureRandom().nextLong() and Long.MAX_VALUE) % 1_000_000_000L
            Engine.ask(Engine.req("fair_seed", "seed" to random.toString(), "difficulty" to difficulty))
                .getString("seed") to difficulty
        }
        busy = true
        playing.withLock {
            val first = Engine.ask(Engine.req("play_new", "seed" to seed, "difficulty" to diff))
            val worldCode = Engine.ask(Engine.req("play_code")).optString("code")
            val save = Engine.ask(Engine.req("play_save"))
            val now = System.currentTimeMillis()
            val w = World(
                id = now.toString(36) + (SecureRandom().nextInt(1 shl 20)).toString(36),
                seed = seed, difficulty = diff, code = worldCode, save = save,
                transcript = mutableListOf(Entry('g', first.optString("text"))),
                notebook = "", created = now, updated = now, day = 1, ended = false,
            )
            persist(w)
            show(w, first.optJSONObject("state"))
        }
        busy = false
        screen = Screen.Game
        return true
    }

    fun open(id: String) {
        val w = store.worlds().firstOrNull { it.id == id } ?: return
        busy = true
        viewModelScope.launch {
            val ok = playing.withLock {
                val r = load(w)
                if (r.has("error")) false else { show(w, r.optJSONObject("state")); true }
            }
            busy = false
            if (ok) screen = Screen.Game else notices.tryEmit("world_refused")
            if (ok && w.shared != null) syncWorlds()
        }
    }

    /** Loads a world into the engine: a shared one from its world file. */
    private suspend fun load(w: World): JSONObject {
        val f = w.shared
        return if (f != null) Engine.ask(Engine.req("world_open", "file" to f))
        else Engine.ask(Engine.req("play_load", "save" to w.save))
    }

    private fun show(w: World, state: JSONObject?) {
        world = w
        transcript.clear()
        transcript.addAll(w.transcript)
        history.clear()
        history.addAll(w.transcript.filter { it.kind != 'g' }.map { it.text })
        recallAt = 0
        chips = chipsFor(state)
    }

    fun leave() {
        screen = Screen.Worlds
        refreshWorlds()
    }

    fun delete(id: String) {
        store.del("w_$id")
        if (world?.id == id) {
            world = null
            screen = Screen.Worlds
        }
        refreshWorlds()
        changed()
    }

    // ---------- playing ----------

    /** One command from the player ('y') or an agent ('a'); returns the game's reply. */
    suspend fun send(command: String, who: Char): String {
        val cmd = command.replace('\n', ' ').trim().take(AgentProtocol.MAX_COMMAND)
        if (cmd.isEmpty()) return ""
        return playing.withLock {
            val w = world ?: return@withLock ""
            val file = w.shared
            val r = if (file != null) {
                val name = if (who == 'y') worldsMe else "agent"
                Engine.ask(Engine.req("world_play", "file" to file, "line" to cmd, "who" to name, "at" to System.currentTimeMillis() / 1000))
            } else {
                Engine.ask(Engine.req("play", "line" to cmd))
            }
            val text = r.optString("text")
            r.optJSONObject("file")?.let { w.shared = it }
            val save = Engine.ask(Engine.req("play_save"))
            withContext(Dispatchers.Main) {
                w.transcript += Entry(who, cmd)
                w.transcript += Entry('g', text)
                transcript += Entry(who, cmd)
                transcript += Entry('g', text)
                w.save = save
                val state = r.optJSONObject("state")
                if (state != null) {
                    w.day = state.optInt("minutes") / 1440 + 1
                    if (state.has("dead")) w.ended = true
                }
                chips = chipsFor(state)
                if (who == 'y') history += cmd
                recallAt = 0
            }
            persist(w)
            if (w.shared != null) pushSoon()
            text
        }
    }

    /** Walks back through the commands typed so far. */
    fun recall(): String {
        if (history.isEmpty()) return ""
        recallAt = (recallAt + 1).coerceAtMost(history.size)
        return history[history.size - recallAt]
    }

    fun hasHistory() = history.isNotEmpty()

    // Command words on chips are the parser's own verbs (crates/game/data/
    // verbs.toml): they are what a player types, not prose.
    private fun chipsFor(state: JSONObject?): Chips {
        if (state == null) return Chips(commands = listOf("look" to "look"))
        val outside = state.optString("place") == "outside"
        val commands = ArrayList<Pair<String, String>>()
        commands += "look" to "look"
        val exits = state.optJSONArray("exits") ?: JSONArray()
        for (i in 0 until minOf(exits.length(), 10)) {
            val e = exits.optString(i)
            commands += e to (if (outside) "go " + lastWord(e) else e)
        }
        if (!outside) commands += "out" to "out"
        val things = ArrayList<Pair<String, String>>()
        val t = state.optJSONArray("things") ?: JSONArray()
        for (i in 0 until minOf(t.length(), 12)) {
            val name = t.optString(i)
            things += name to lastWord(name)
        }
        return Chips(commands, listOf("read", "take", "examine", "clean"), things, listOf("inventory", "status", "listen", "wait"))
    }

    private fun lastWord(name: String): String =
        name.trim().split(Regex("\\s+")).last().filter { it.isLetterOrDigit() || it == '_' || it == '-' }

    // ---------- notebook ----------

    fun setNotebook(text: String) {
        val w = world ?: return
        if (w.notebook == text) return
        w.notebook = text
        persist(w)
    }

    fun addToNotebook(passage: String) {
        val w = world ?: return
        val quoted = passage.trim().lines().joinToString("\n") { "> $it" }
        setNotebook((if (w.notebook.isBlank()) "" else w.notebook.trimEnd() + "\n\n") + quoted + "\n")
    }

    // ---------- saving, backup and sync ----------

    private fun persist(w: World) {
        w.updated = System.currentTimeMillis()
        val json = w.toJson().toString()
        viewModelScope.launch(Dispatchers.IO) {
            store.put("w_" + w.id, json)
            withContext(Dispatchers.Main) { changed() }
        }
    }

    /** Something to back up and sync. */
    private fun changed() {
        BackupManager(getApplication()).dataChanged()
        val uri = prefs.getString("sync", null) ?: return
        syncJob?.cancel()
        syncJob = viewModelScope.launch(Dispatchers.IO) {
            delay(1500)
            try {
                getApplication<Application>().contentResolver.openOutputStream(Uri.parse(uri), "wt")?.use {
                    it.write(bundle().toString().toByteArray())
                }
            } catch (e: Exception) {
                notices.tryEmit("sync_failed")
            }
        }
    }

    fun backupNow() {
        BackupManager(getApplication()).dataChanged()
        notices.tryEmit("backup_asked")
    }

    /** Every world, as the synced and exported files hold them. */
    fun bundle(only: World? = null): JSONObject {
        val list = JSONArray()
        for (w in (only?.let { listOf(it) } ?: store.worlds())) list.put(w.toJson())
        return JSONObject().put("app", "scraped-again").put("version", 1).put("worlds", list)
    }

    fun setSync(uri: Uri?) {
        if (uri == null) return
        val flags = Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION
        try {
            getApplication<Application>().contentResolver.takePersistableUriPermission(uri, flags)
        } catch (e: Exception) {
            // Not every provider offers lasting access; writing may still work this session.
        }
        prefs.edit().putString("sync", uri.toString()).apply()
        syncName = displayName(uri)
        changed()
    }

    fun stopSync() {
        prefs.getString("sync", null)?.let {
            runCatching {
                getApplication<Application>().contentResolver.releasePersistableUriPermission(
                    Uri.parse(it), Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION,
                )
            }
        }
        prefs.edit().remove("sync").apply()
        syncName = null
    }

    fun syncUri(): Uri? = prefs.getString("sync", null)?.let { Uri.parse(it) }

    /** Brings in worlds from a synced or exported file; newer copies win. */
    fun importFrom(uri: Uri?) {
        if (uri == null) return
        viewModelScope.launch(Dispatchers.IO) {
            try {
                val text = getApplication<Application>().contentResolver.openInputStream(uri)?.use {
                    it.readBytes().toString(Charsets.UTF_8)
                } ?: throw IllegalStateException()
                val list = JSONObject(text).optJSONArray("worlds") ?: JSONArray()
                for (i in 0 until list.length()) {
                    val w = World.fromJson(list.optJSONObject(i) ?: continue) ?: continue
                    val have = store.get("w_" + w.id)?.let { runCatching { World.fromJson(JSONObject(it)) }.getOrNull() }
                    if (have == null || w.updated >= have.updated) store.putWorld(w)
                }
                withContext(Dispatchers.Main) {
                    refreshWorlds()
                    notices.tryEmit("restored")
                }
            } catch (e: Exception) {
                notices.tryEmit("sync_failed")
            }
        }
    }

    fun exportTo(uri: Uri?, w: World) {
        if (uri == null) return
        viewModelScope.launch(Dispatchers.IO) {
            try {
                getApplication<Application>().contentResolver.openOutputStream(uri, "wt")?.use {
                    it.write(bundle(w).toString().toByteArray())
                }
            } catch (e: Exception) {
                notices.tryEmit("sync_failed")
            }
        }
    }

    private fun displayName(uri: Uri): String {
        return try {
            getApplication<Application>().contentResolver
                .query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)
                ?.use { c -> if (c.moveToFirst()) c.getString(0) else null }
                ?: uri.lastPathSegment ?: ""
        } catch (e: Exception) {
            uri.lastPathSegment ?: ""
        }
    }

    // ---------- shared worlds (C01) ----------

    /** Where shared worlds sync, if it's set up. */
    private fun worldSync(): WorldSync? {
        val repo = prefs.getString("worldsRepo", "") ?: ""
        val token = prefs.getString("worldsToken", "") ?: ""
        if (repo.isBlank() || token.isBlank()) return null
        return GitHubSync(repo.trim(), token.trim())
    }

    fun worldsReady() = worldSync() != null

    fun setWorlds(repo: String, token: String?, me: String) {
        val e = prefs.edit().putString("worldsRepo", repo.trim()).putString("worldsMe", me.trim().ifEmpty { "jb" })
        if (token != null) e.putString("worldsToken", token.trim())
        e.apply()
        worldsRepo = repo.trim()
        worldsMe = me.trim().ifEmpty { "jb" }
    }

    fun hasToken() = !(prefs.getString("worldsToken", "") ?: "").isBlank()

    /** Shares the open world: from now on its moves and talk sync. */
    fun share() {
        val w = world ?: return
        if (w.shared != null) return
        viewModelScope.launch {
            playing.withLock {
                val r = Engine.ask(Engine.req("world_share", "id" to w.id))
                val f = r.optJSONObject("file") ?: return@withLock
                w.shared = f
                w.remote = "${w.code.ifEmpty { w.id }}.world"
                persist(w)
            }
            talkVersion++
            syncWorlds()
        }
    }

    /** Leaves a message across the table in the open shared world. */
    fun talk(text: String) {
        val w = world ?: return
        val f = w.shared ?: return
        val t = text.trim()
        if (t.isEmpty()) return
        val talk = f.optJSONArray("talk") ?: JSONArray().also { f.put("talk", it) }
        talk.put(
            JSONObject().put("who", worldsMe).put("at", System.currentTimeMillis() / 1000)
                .put("turn", f.optInt("turn")).put("text", t),
        )
        w.talkRead = talk.length()
        talkVersion++
        persist(w)
        pushSoon()
    }

    /** The open world's talk has been read. */
    fun talkSeen() {
        val w = world ?: return
        val n = w.talk().length()
        if (w.talkRead != n) {
            w.talkRead = n
            persist(w)
        }
    }

    private fun pushSoon() {
        worldsJob?.cancel()
        worldsJob = viewModelScope.launch {
            delay(2000)
            syncWorlds()
        }
    }

    /**
     * Brings shared worlds up to date both ways: each local one is merged
     * with the copy where worlds sync (the newer wins, talk is joined, a
     * split keeps the other line as a branch) and written back; worlds
     * shared from elsewhere are brought in.
     */
    fun syncWorlds() {
        val sync = worldSync() ?: return
        viewModelScope.launch {
            syncing.withLock {
                worldsStatus = "syncing"
                worldsStatus = try {
                    syncOnce(sync)
                    "synced"
                } catch (e: Exception) {
                    "sync_failed"
                }
            }
        }
    }

    private suspend fun syncOnce(sync: WorldSync) {
        val there = withContext(Dispatchers.IO) { sync.list() }
        val locals = store.worlds().filter { it.shared != null }
        for (local in locals) {
            // The open world's copy is the live one.
            val w = if (world?.id == local.id) world!! else local
            var tag = there[w.remote]
            if (tag != null && tag != w.tag) {
                val (text, t) = withContext(Dispatchers.IO) { sync.get(w.remote) } ?: continue
                merge(w, JSONObject(text))
                tag = t
            }
            if (tag != null && tag == w.tag && w.mark() == w.synced) continue
            val text = w.shared.toString()
            val wrote = withContext(Dispatchers.IO) {
                sync.put(w.remote, text, tag, "${worldsMe}: ${w.shared?.optJSONArray("moves")?.length() ?: 0} moves")
            }
            if (wrote != null) {
                w.tag = wrote
                w.synced = w.mark()
                persist(w)
            }
        }
        val known = store.worlds().map { it.remote }.toSet()
        for (name in there.keys) {
            if (name in known) continue
            val (text, t) = withContext(Dispatchers.IO) { sync.get(name) } ?: continue
            adopt(name, JSONObject(text), t)
        }
        withContext(Dispatchers.Main) { refreshWorlds() }
    }

    /** Merges the synced copy of a world into ours. */
    private suspend fun merge(w: World, theirs: JSONObject) {
        val ours = w.shared ?: return
        val r = Engine.ask(Engine.req("world_merge", "ours" to ours, "theirs" to theirs))
        val file = r.optJSONObject("file") ?: return
        val before = ours.optJSONArray("moves")?.length() ?: 0
        val moves = file.optJSONArray("moves") ?: JSONArray()
        val kept = r.optJSONObject("merge")?.optString("newest")
        withContext(Dispatchers.Main) {
            w.shared = file
            if (kept == "theirs") {
                // The other player's moves, into the transcript as they saw them.
                for (i in before until moves.length()) {
                    val m = moves.optJSONObject(i) ?: continue
                    val e = listOf(Entry(if (m.optString("who") == worldsMe) 'y' else 'a', m.optString("command")), Entry('g', m.optString("text")))
                    w.transcript += e
                    if (world?.id == w.id) transcript += e
                }
                w.save = JSONObject().put("sealed", file.optString("save")).put("turn", file.optInt("turn"))
            }
            talkVersion++
        }
        if (kept == "theirs" && world?.id == w.id) {
            playing.withLock {
                val o = Engine.ask(Engine.req("world_open", "file" to file))
                withContext(Dispatchers.Main) { chips = chipsFor(o.optJSONObject("state")) }
            }
        }
        persist(w)
        r.optJSONObject("branch")?.let { b ->
            val name = w.remote.removeSuffix(".world") + "." + b.optString("branch") + ".world"
            adopt(name, b, null)
            notices.tryEmit("world_split")
        }
    }

    /** Brings in a world shared from elsewhere (or a branch of one). */
    private suspend fun adopt(name: String, file: JSONObject, tag: String?) {
        val opened = playing.withLock {
            val o = Engine.ask(Engine.req("world_open", "file" to file))
            // Put the open world back in the engine.
            world?.let { load(it) }
            o
        }
        if (opened.has("error")) {
            notices.tryEmit("world_refused")
            return
        }
        val entries = mutableListOf<Entry>()
        val moves = file.optJSONArray("moves") ?: JSONArray()
        for (i in 0 until moves.length()) {
            val m = moves.optJSONObject(i) ?: continue
            entries += Entry(if (m.optString("who") == worldsMe) 'y' else 'a', m.optString("command"))
            entries += Entry('g', m.optString("text"))
        }
        if (entries.isEmpty()) entries += Entry('g', opened.optString("text"))
        val now = System.currentTimeMillis()
        val state = opened.optJSONObject("state")
        val w = World(
            id = now.toString(36) + (SecureRandom().nextInt(1 shl 20)).toString(36),
            seed = "", difficulty = "standard", code = opened.optString("code"),
            save = JSONObject().put("sealed", file.optString("save")).put("turn", file.optInt("turn")),
            transcript = entries, notebook = "", created = now, updated = now,
            day = (state?.optInt("minutes") ?: 0) / 1440 + 1, ended = false,
            shared = file, remote = name, tag = tag,
        )
        if (tag != null) w.synced = w.mark()
        store.putWorld(w)
    }

    // ---------- appearance ----------

    private fun loadLook() = Look(
        size = prefs.getInt("size", 18),
        theme = prefs.getString("theme", "system") ?: "system",
        font = prefs.getString("font", "serif") ?: "serif",
        atmosphere = prefs.getBoolean("atmosphere", true),
    )

    fun changeLook(l: Look) {
        look = l
        prefs.edit().putInt("size", l.size).putString("theme", l.theme).putString("font", l.font)
            .putBoolean("atmosphere", l.atmosphere).apply()
    }

    // ---------- agent access ----------

    private fun key(): String = prefs.getString("agentKey", null) ?: newKey()

    private fun newKey(): String {
        val b = ByteArray(18)
        SecureRandom().nextBytes(b)
        val k = b.joinToString("") { "%02x".format(it.toInt() and 0xff) }
        prefs.edit().putString("agentKey", k).apply()
        return k
    }

    fun setAgent(on: Boolean) {
        prefs.edit().putBoolean("agent", on).apply()
        agent?.stop()
        agent = null
        if (on) {
            val server = AgentServer(AgentProtocol(object : AgentProtocol.Game {
                override fun act(command: String): String = runBlocking(Dispatchers.Main) { agentAct(command) }
                override fun read(entries: Int): String = runBlocking(Dispatchers.Main) { agentRead(entries) }
            }, key()))
            server.start()
            agent = server
        }
        refreshAgent()
    }

    fun renewKey() {
        newKey()
        if (agentOn) setAgent(true) else refreshAgent()
    }

    fun refreshAgent() {
        val a = agent
        agentOn = a != null && a.running()
        agentKey = key()
        agentUrl = if (agentOn) AgentServer.lanAddress()?.let { "http://$it:${a!!.port()}" } else null
    }

    /** The agent's move: in the open world, or the most recent one. */
    suspend fun agentAct(command: String): String {
        if (world == null) {
            val recent = store.worlds().firstOrNull() ?: return ""
            playing.withLock {
                val r = load(recent)
                if (r.has("error")) return ""
                show(recent, r.optJSONObject("state"))
            }
        }
        return send(command, 'a')
    }

    fun agentRead(entries: Int): String {
        val list = world?.transcript ?: store.worlds().firstOrNull()?.transcript ?: return ""
        return list.takeLast(entries).joinToString("\n\n") { if (it.kind == 'g') it.text else "> " + it.text }
    }

    override fun onCleared() {
        agent?.stop()
        super.onCleared()
    }
}
