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
    private val history = ArrayList<String>()
    private var recallAt = 0
    private var agent: AgentServer? = null
    private var syncJob: Job? = null

    /** Brief notices for the snackbar, as label ids. */
    val notices = MutableSharedFlow<String>(extraBufferCapacity = 8)

    init {
        viewModelScope.launch {
            val l = Engine.ask(Engine.req("app_labels"))
            labels = l.keys().asSequence().associateWith { l.optString(it) }
            refreshWorlds()
            syncName = prefs.getString("sync", null)?.let { displayName(Uri.parse(it)) }
            if (prefs.getBoolean("agent", false)) setAgent(true)
            ready = true
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
            playing.withLock {
                val r = Engine.ask(Engine.req("play_load", "save" to w.save))
                show(w, r.optJSONObject("state"))
            }
            busy = false
            screen = Screen.Game
        }
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
            val r = Engine.ask(Engine.req("play", "line" to cmd))
            val text = r.optString("text")
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
        return Chips(commands, listOf("read", "take", "examine"), things, listOf("inventory", "status", "wait"))
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

    // ---------- appearance ----------

    private fun loadLook() = Look(
        size = prefs.getInt("size", 18),
        theme = prefs.getString("theme", "system") ?: "system",
        font = prefs.getString("font", "serif") ?: "serif",
        atmosphere = prefs.getBoolean("atmosphere", true),
    )

    fun setLook(l: Look) {
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
                val r = Engine.ask(Engine.req("play_load", "save" to recent.save))
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
