package org.scrapedagain

import kotlinx.coroutines.asCoroutineDispatcher
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.util.concurrent.Executors

/**
 * The Rust engine (crates/android): the same JSON interface every client
 * uses, so the app plays exactly as the terminal and the browser do. The
 * game's state lives on the engine's own thread, so every game call goes
 * through [thread]; long replays never block the screen.
 */
object Engine {
    init {
        System.loadLibrary("scraped_android")
    }

    /** One JSON request, one JSON reply. */
    @JvmStatic
    external fun call(request: String): String

    /** One frame of the decorative backdrop into [pixels] (ARGB). */
    @JvmStatic
    external fun atmosphere(pixels: IntArray, width: Int, height: Int, time: Float, dark: Boolean)

    val thread = Executors.newSingleThreadExecutor { r -> Thread(r, "engine") }.asCoroutineDispatcher()

    suspend fun ask(request: JSONObject): JSONObject =
        withContext(thread) { JSONObject(call(request.toString())) }

    /** For tests and the agent's thread. */
    fun askBlocking(request: JSONObject): JSONObject = runBlocking { ask(request) }

    fun req(cmd: String, vararg pairs: Pair<String, Any?>): JSONObject {
        val o = JSONObject().put("cmd", cmd)
        for ((k, v) in pairs) o.put(k, v)
        return o
    }
}
