package org.scrapedagain

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/** The engine on Android gives the same transcripts as every other platform. */
@RunWith(AndroidJUnit4::class)
class EngineTest {
    @Test
    fun transcriptsMatchEveryPlatform() {
        val ctx = InstrumentationRegistry.getInstrumentation().context
        val lines = ctx.assets.open("transcripts.txt").bufferedReader().readLines().filter { it.isNotBlank() }
        assertTrue(lines.isNotEmpty())
        for (line in lines) {
            val (seed, difficulty, hash) = line.trim().split(" ")
            val r = Engine.askBlocking(Engine.req("transcript_hash", "seed" to seed, "difficulty" to difficulty, "steps" to 40))
            assertEquals("seed $seed $difficulty", hash, r.optString("hash"))
        }
    }

    @Test
    fun backdropIsFaintAndMoves() {
        val a = IntArray(36 * 78)
        val b = IntArray(36 * 78)
        Engine.atmosphere(a, 36, 78, 10f, true)
        Engine.atmosphere(b, 36, 78, 40f, true)
        assertTrue(a.all { (it ushr 24) <= 40 })
        assertTrue(!a.contentEquals(b))
    }
}
