package org.scrapedagain

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject
import java.io.File

/** One exchange in a world's transcript: the game ('g'), you ('y') or an agent ('a'). */
data class Entry(val kind: Char, val text: String)

/**
 * A world as the app keeps it: its save (seed, difficulty, commands: the
 * engine replays it), the transcript as shown, and the player's notebook.
 * The JSON shape is the one exports and synced files use.
 */
class World(
    val id: String,
    val seed: String,
    val difficulty: String,
    val code: String,
    var save: JSONObject,
    val transcript: MutableList<Entry>,
    var notebook: String,
    val created: Long,
    var updated: Long,
    var day: Int,
    var ended: Boolean,
) {
    fun toJson(): JSONObject {
        val t = JSONArray()
        for (e in transcript) t.put(JSONObject().put("k", e.kind.toString()).put("t", e.text))
        return JSONObject()
            .put("id", id).put("seed", seed).put("difficulty", difficulty).put("code", code)
            .put("save", save).put("transcript", t).put("notebook", notebook)
            .put("created", created).put("updated", updated).put("day", day).put("ended", ended)
    }

    /** The last thing the game said. */
    fun lastGame(): String = transcript.lastOrNull { it.kind == 'g' }?.text ?: ""

    companion object {
        fun fromJson(o: JSONObject): World? {
            val save = o.optJSONObject("save") ?: return null
            val id = o.optString("id").ifEmpty { return null }
            val t = o.optJSONArray("transcript") ?: JSONArray()
            val entries = ArrayList<Entry>(t.length())
            for (i in 0 until t.length()) {
                val e = t.optJSONObject(i) ?: continue
                entries += Entry(e.optString("k", "g").firstOrNull() ?: 'g', e.optString("t"))
            }
            return World(
                id = id,
                seed = o.optString("seed"),
                difficulty = o.optString("difficulty", "standard"),
                code = o.optString("code"),
                save = save,
                transcript = entries,
                notebook = o.optString("notebook"),
                created = o.optLong("created"),
                updated = o.optLong("updated"),
                day = o.optInt("day", 1),
                ended = o.optBoolean("ended"),
            )
        }
    }
}

/**
 * Files under files/data, one per key. Android backs this folder up to the
 * player's Google account (res/xml). Writes go through a temporary file so
 * a crash never leaves half a world.
 */
class Store(context: Context) {
    private val dir = File(context.filesDir, "data").apply { mkdirs() }

    private fun file(key: String): File {
        var safe = key.replace(Regex("[^A-Za-z0-9_.-]"), "_")
        if (safe.isEmpty() || safe.startsWith(".")) safe = "_$safe"
        return File(dir, "$safe.json")
    }

    @Synchronized
    fun get(key: String): String? = file(key).takeIf { it.isFile }?.readText()

    @Synchronized
    fun put(key: String, value: String) {
        val f = file(key)
        val tmp = File(dir, f.name + ".tmp")
        tmp.writeText(value)
        if (!tmp.renameTo(f)) tmp.delete()
    }

    @Synchronized
    fun del(key: String) {
        file(key).delete()
    }

    @Synchronized
    fun keys(): List<String> =
        dir.listFiles()?.map { it.name }?.filter { it.endsWith(".json") }?.map { it.removeSuffix(".json") } ?: emptyList()

    fun worlds(): List<World> = keys().filter { it.startsWith("w_") }
        .mapNotNull { k -> get(k)?.let { runCatching { World.fromJson(JSONObject(it)) }.getOrNull() } }
        .sortedByDescending { it.updated }

    fun putWorld(w: World) = put("w_" + w.id, w.toJson().toString())
}
