package org.scrapedagain

import android.util.Base64
import org.json.JSONArray
import org.json.JSONObject
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URL
import java.net.URLEncoder

/**
 * Where shared worlds live between turns (C01): a place holding one file
 * per world, which the app and an agent both read and write. Kept behind
 * this interface so the place can change (a private GitHub repo now; a
 * synced folder later, if wanted).
 */
interface WorldSync {
    /** Every world file there: its name and a tag that changes with it. */
    fun list(): Map<String, String>

    /** One file's text and tag, or null if it isn't there. */
    fun get(name: String): Pair<String, String>?

    /**
     * Writes a file, if it is still at [tag] (null: if it doesn't exist
     * yet). Returns the new tag, or null if someone else wrote it first.
     */
    fun put(name: String, text: String, tag: String?, message: String): String?
}

/**
 * A private GitHub repository used only for worlds, reached with a token
 * scoped to that one repository (contents: read and write). World files
 * sit at its top level as NAME.world.
 */
class GitHubSync(private val repo: String, private val token: String) : WorldSync {
    private val api = "https://api.github.com/repos/$repo/contents"

    private fun open(url: String, method: String, raw: Boolean = false): HttpURLConnection {
        val c = URL(url).openConnection() as HttpURLConnection
        c.requestMethod = method
        c.connectTimeout = 15_000
        c.readTimeout = 30_000
        c.setRequestProperty("Authorization", "Bearer $token")
        c.setRequestProperty("Accept", if (raw) "application/vnd.github.raw+json" else "application/vnd.github+json")
        c.setRequestProperty("X-GitHub-Api-Version", "2022-11-28")
        return c
    }

    private fun body(c: HttpURLConnection): String =
        (if (c.responseCode < 400) c.inputStream else c.errorStream)?.use { it.readBytes().toString(Charsets.UTF_8) } ?: ""

    private fun path(name: String) = URLEncoder.encode(name, "UTF-8").replace("+", "%20")

    override fun list(): Map<String, String> {
        val c = open(api, "GET")
        val code = c.responseCode
        val text = body(c)
        if (code == 404) return emptyMap()
        if (code >= 400) throw IOException("list: $code")
        val arr = JSONArray(text)
        val out = LinkedHashMap<String, String>()
        for (i in 0 until arr.length()) {
            val o = arr.optJSONObject(i) ?: continue
            val name = o.optString("name")
            if (o.optString("type") == "file" && name.endsWith(".world")) out[name] = o.optString("sha")
        }
        return out
    }

    override fun get(name: String): Pair<String, String>? {
        val meta = open("$api/${path(name)}", "GET")
        val code = meta.responseCode
        val text = body(meta)
        if (code == 404) return null
        if (code >= 400) throw IOException("get: $code")
        val sha = JSONObject(text).optString("sha")
        // The raw form, which works for files of any size the API serves.
        val raw = open("$api/${path(name)}", "GET", raw = true)
        if (raw.responseCode >= 400) throw IOException("get: ${raw.responseCode}")
        return body(raw) to sha
    }

    override fun put(name: String, text: String, tag: String?, message: String): String? {
        val c = open("$api/${path(name)}", "PUT")
        c.doOutput = true
        c.setRequestProperty("Content-Type", "application/json")
        val req = JSONObject()
            .put("message", message)
            .put("content", Base64.encodeToString(text.toByteArray(Charsets.UTF_8), Base64.NO_WRAP))
        if (tag != null) req.put("sha", tag)
        c.outputStream.use { it.write(req.toString().toByteArray(Charsets.UTF_8)) }
        val code = c.responseCode
        val out = body(c)
        if (code == 409 || code == 422) return null
        if (code >= 400) throw IOException("put: $code")
        return JSONObject(out).optJSONObject("content")?.optString("sha")
    }
}
