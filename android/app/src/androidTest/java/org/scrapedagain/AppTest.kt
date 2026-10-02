package org.scrapedagain

import androidx.compose.ui.test.assertIsFocused
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performImeAction
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.printToLog
import androidx.test.ext.junit.runners.AndroidJUnit4
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.net.Socket
import java.net.URL

/** Plays through the real interface on a device: a world, typing, the agent, back. */
@RunWith(AndroidJUnit4::class)
class AppTest {
    @get:Rule
    val rule = createAndroidComposeRule<MainActivity>()

    private val model get() = rule.activity.model

    @Test
    fun playAWorldByTypingAndByAgent() {
        waitFor(60_000) { model.ready }
        val before = model.worlds.size
        rule.onNodeWithTag("newWorld").performClick()
        rule.onNodeWithTag("begin").performClick()
        waitFor(60_000) { model.screen == Screen.Game && !model.busy }
        waitFor(10_000) { rule.onAllNodesWithTag("game").fetchSemanticsNodes().isNotEmpty() }

        // Type a command and send it from the keyboard; the box keeps focus.
        rule.onNodeWithTag("composer").performClick()
        rule.onNodeWithTag("composer").performTextInput("look")
        rule.onNodeWithTag("composer").performImeAction()
        waitFor(30_000) { model.transcript.size == 3 }
        // The command stays in view above its reply, however long the reply.
        waitFor(5_000) { rule.onAllNodesWithTag("bubble").fetchSemanticsNodes().size == 1 }
        rule.onNodeWithTag("composer").assertIsFocused()
        assertEquals(3, model.transcript.size)

        // An agent's move shows, tagged, and only text comes back.
        val reply = runBlocking(Dispatchers.Main) { model.agentAct("inventory") }
        assertTrue(reply.isNotBlank())
        waitFor(10_000) { model.transcript.size == 5 }
        waitFor(5_000) { rule.onAllNodesWithTag("agentBubble").fetchSemanticsNodes().size == 1 }

        // Over the network too: agent access on, read and act with the key.
        rule.runOnUiThread { model.setAgent(true) }
        val key = model.agentKey
        val read = http("GET", "http://127.0.0.1:8765/read?n=4", key, null)
        assertTrue(read, read.contains("> inventory"))
        val acted = http("POST", "http://127.0.0.1:8765/act", key, "status")
        assertTrue(acted.isNotBlank())
        waitFor(10_000) { model.transcript.size == 7 && model.transcript[5].kind == 'a' }
        assertTrue(http("GET", "http://127.0.0.1:8765/read", "wrong", null).isEmpty())
        rule.runOnUiThread { model.setAgent(false) }

        // A passage to the notebook.
        rule.onAllNodesWithTag("game")[0].performClick()
        rule.onNodeWithTag("toNotebook").performClick()
        waitFor(5_000) { model.world?.notebook?.startsWith("> ") == true }

        // Back to the list: the world is there, kept.
        rule.runOnUiThread { rule.activity.onBackPressedDispatcher.onBackPressed() }
        waitFor(10_000) { model.screen == Screen.Worlds }
        waitFor(10_000) { model.worlds.size == before + 1 }
        // look, inventory and status, each with the game's reply, after the opening.
        assertEquals(7, model.worlds.first().transcript.size)
    }

    /** Like waitUntil, but a timeout says which step and what the app was doing. */
    private fun waitFor(ms: Long, cond: () -> Boolean) {
        val step = Throwable().stackTrace.getOrNull(1)?.lineNumber
        try {
            rule.waitUntil(ms) { cond() }
        } catch (e: Throwable) {
            val m = model
            val state = "line $step; screen=${m.screen} busy=${m.busy} ready=${m.ready} " +
                "worlds=${m.worlds.size} transcript=${m.transcript.map { it.kind.toString() + ":" + it.text.take(60) }}"
            android.util.Log.e("AppTest", state)
            runCatching { rule.onRoot().printToLog("AppTest") }
            throw AssertionError(state, e)
        }
    }

    /**
     * A bare HTTP/1.1 request over a socket: the agent's view from outside.
     * (Inside the app, Android refuses plain HttpURLConnection to http://.)
     * Returns the body, or "" for an error status.
     */
    private fun http(method: String, url: String, key: String, body: String?): String {
        val u = URL(url)
        Socket(u.host, u.port).use { s ->
            s.soTimeout = 30_000
            val data = (body ?: "").toByteArray()
            val head = "$method ${u.file} HTTP/1.1\r\nHost: ${u.host}\r\nAuthorization: Bearer $key\r\n" +
                "Content-Type: text/plain\r\nContent-Length: ${data.size}\r\nConnection: close\r\n\r\n"
            s.getOutputStream().apply { write(head.toByteArray()); write(data); flush() }
            val reply = s.getInputStream().readBytes().toString(Charsets.UTF_8)
            val status = reply.substringAfter(' ').take(3).toIntOrNull() ?: 0
            return if (status in 200..399) reply.substringAfter("\r\n\r\n") else ""
        }
    }
}
