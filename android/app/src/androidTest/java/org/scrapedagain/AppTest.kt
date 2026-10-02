package org.scrapedagain

import androidx.compose.ui.test.assertIsFocused
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performImeAction
import androidx.compose.ui.test.performTextInput
import androidx.test.ext.junit.runners.AndroidJUnit4
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.net.HttpURLConnection
import java.net.URL

/** Plays through the real interface on a device: a world, typing, the agent, back. */
@RunWith(AndroidJUnit4::class)
class AppTest {
    @get:Rule
    val rule = createAndroidComposeRule<MainActivity>()

    private val model get() = rule.activity.model

    @Test
    fun playAWorldByTypingAndByAgent() {
        rule.waitUntil(60_000) { model.ready }
        val before = model.worlds.size
        rule.onNodeWithTag("newWorld").performClick()
        rule.onNodeWithTag("begin").performClick()
        rule.waitUntil(60_000) { model.screen == Screen.Game && !model.busy }
        rule.waitUntil(10_000) { rule.onAllNodesWithTag("game").fetchSemanticsNodes().isNotEmpty() }

        // Type a command and send it from the keyboard; the box keeps focus.
        rule.onNodeWithTag("composer").performClick()
        rule.onNodeWithTag("composer").performTextInput("look")
        rule.onNodeWithTag("composer").performImeAction()
        rule.waitUntil(30_000) { rule.onAllNodesWithTag("bubble").fetchSemanticsNodes().size == 1 }
        rule.onNodeWithTag("composer").assertIsFocused()
        assertEquals(3, model.transcript.size)

        // An agent's move shows, tagged, and only text comes back.
        val reply = runBlocking(Dispatchers.Main) { model.agentAct("inventory") }
        assertTrue(reply.isNotBlank())
        rule.waitUntil(10_000) { rule.onAllNodesWithTag("agentBubble").fetchSemanticsNodes().size == 1 }

        // Over the network too: agent access on, read and act with the key.
        rule.runOnUiThread { model.setAgent(true) }
        val key = model.agentKey
        val read = http("GET", "http://127.0.0.1:8765/read?n=4", key, null)
        assertTrue(read, read.contains("> inventory"))
        val acted = http("POST", "http://127.0.0.1:8765/act", key, "status")
        assertTrue(acted.isNotBlank())
        rule.waitUntil(10_000) { rule.onAllNodesWithTag("agentBubble").fetchSemanticsNodes().size == 2 }
        assertTrue(http("GET", "http://127.0.0.1:8765/read", "wrong", null).isEmpty())
        rule.runOnUiThread { model.setAgent(false) }

        // A passage to the notebook.
        rule.onAllNodesWithTag("game")[0].performClick()
        rule.onNodeWithTag("toNotebook").performClick()
        rule.waitUntil(5_000) { model.world?.notebook?.startsWith("> ") == true }

        // Back to the list: the world is there, kept.
        rule.runOnUiThread { rule.activity.onBackPressedDispatcher.onBackPressed() }
        rule.waitUntil(10_000) { model.screen == Screen.Worlds }
        rule.waitUntil(10_000) { model.worlds.size == before + 1 }
        // look, inventory and status, each with the game's reply, after the opening.
        assertEquals(7, model.worlds.first().transcript.size)
    }

    private fun http(method: String, url: String, key: String, body: String?): String {
        val c = URL(url).openConnection() as HttpURLConnection
        c.requestMethod = method
        c.setRequestProperty("Authorization", "Bearer $key")
        if (body != null) {
            c.doOutput = true
            c.outputStream.use { it.write(body.toByteArray()) }
        }
        return if (c.responseCode < 400) c.inputStream.bufferedReader().readText() else ""
    }
}
