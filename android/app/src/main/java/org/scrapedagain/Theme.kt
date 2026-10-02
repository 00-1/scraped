package org.scrapedagain

import android.graphics.Bitmap
import android.provider.Settings
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.FilterQuality
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.IntSize
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.repeatOnLifecycle
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext

/** Colours beyond Material's: the bubbles and quiet text. */
data class Ink(val bubble: Color, val agent: Color, val faint: Color, val dark: Boolean)

private val light = lightColorScheme(
    primary = Color(0xFF8A4F1D), onPrimary = Color(0xFFFFFAF2),
    background = Color(0xFFF7F5EF), onBackground = Color(0xFF22201B),
    surface = Color(0xFFF7F5EF), onSurface = Color(0xFF22201B),
    surfaceVariant = Color(0xFFEFECE4), onSurfaceVariant = Color(0xFF5F5A50),
    surfaceContainer = Color(0xFFFFFDF8), surfaceContainerLow = Color(0xFFFFFDF8),
    surfaceContainerHigh = Color(0xFFEFECE4), surfaceContainerHighest = Color(0xFFE9E5DB),
    outline = Color(0xFFD6D0C3), outlineVariant = Color(0xFFE2DDD1),
    secondaryContainer = Color(0xFFE9E3D6), onSecondaryContainer = Color(0xFF22201B),
)
private val sepia = lightColorScheme(
    primary = Color(0xFF8D4A17), onPrimary = Color(0xFFFBF3E4),
    background = Color(0xFFF1E7D3), onBackground = Color(0xFF3B2F20),
    surface = Color(0xFFF1E7D3), onSurface = Color(0xFF3B2F20),
    surfaceVariant = Color(0xFFE8DCC4), onSurfaceVariant = Color(0xFF6B5A43),
    surfaceContainer = Color(0xFFF7EFDF), surfaceContainerLow = Color(0xFFF7EFDF),
    surfaceContainerHigh = Color(0xFFE8DCC4), surfaceContainerHighest = Color(0xFFE1D3B7),
    outline = Color(0xFFCDBB98), outlineVariant = Color(0xFFDCCDB0),
    secondaryContainer = Color(0xFFE3D4B6), onSecondaryContainer = Color(0xFF3B2F20),
)
private val dark = darkColorScheme(
    primary = Color(0xFFE3A463), onPrimary = Color(0xFF1B1A17),
    background = Color(0xFF121110), onBackground = Color(0xFFEBE6DB),
    surface = Color(0xFF121110), onSurface = Color(0xFFEBE6DB),
    surfaceVariant = Color(0xFF24221E), onSurfaceVariant = Color(0xFFB3AC9F),
    surfaceContainer = Color(0xFF1B1A17), surfaceContainerLow = Color(0xFF1B1A17),
    surfaceContainerHigh = Color(0xFF24221E), surfaceContainerHighest = Color(0xFF2D2A24),
    outline = Color(0xFF45413A), outlineVariant = Color(0xFF2F2C27),
    secondaryContainer = Color(0xFF2D2A24), onSecondaryContainer = Color(0xFFEBE6DB),
)

/** The scheme and extra colours for a look. */
@Composable
fun schemeFor(look: Look): Pair<ColorScheme, Ink> {
    val night = isSystemInDarkTheme()
    return when {
        look.theme == "sepia" -> sepia to Ink(Color(0xFFE3D4B6), Color(0xFFD7DCC4), Color(0xFF93806A), false)
        look.theme == "dark" || (look.theme == "system" && night) ->
            dark to Ink(Color(0xFF2D2A24), Color(0xFF1F2C26), Color(0xFF857F74), true)
        else -> light to Ink(Color(0xFFE9E3D6), Color(0xFFDFE8E2), Color(0xFF8B857A), false)
    }
}

/** The reading face: Android's Noto Serif, or its plain sans. */
fun readingFont(look: Look): FontFamily = if (look.font == "sans") FontFamily.SansSerif else FontFamily.Serif

@Composable
fun ScrapedTheme(scheme: ColorScheme, content: @Composable () -> Unit) {
    MaterialTheme(colorScheme = scheme, content = content)
}

/**
 * The backdrop: a reading room by lamplight, painted by the Rust engine
 * (crates/android/src/atmosphere.rs) as a tiny image and drawn scaled up,
 * so filtering softens it. Decoration only: it depends on time and theme,
 * never on the game. Still when the phone asks for reduced motion, and
 * paused when the app is out of sight.
 */
@Composable
fun Atmosphere(dark: Boolean, modifier: Modifier = Modifier) {
    val w = 36
    val h = 78
    val context = LocalContext.current
    val still = remember {
        Settings.Global.getFloat(context.contentResolver, Settings.Global.ANIMATOR_DURATION_SCALE, 1f) == 0f
    }
    val bitmap = remember { Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888) }
    val image = remember(bitmap) { bitmap.asImageBitmap() }
    var frame by remember { mutableIntStateOf(0) }
    val lifecycle = LocalLifecycleOwner.current
    LaunchedEffect(dark, still) {
        val pixels = IntArray(w * h)
        val start = System.nanoTime()
        lifecycle.repeatOnLifecycle(Lifecycle.State.STARTED) {
            while (true) {
                val t = if (still) 600f else 600f + (System.nanoTime() - start) / 1e9f
                withContext(Dispatchers.Default) { Engine.atmosphere(pixels, w, h, t, dark) }
                bitmap.setPixels(pixels, 0, w, 0, 0, w, h)
                frame++
                if (still) awaitCancellation()
                delay(100)
            }
        }
    }
    Canvas(modifier.fillMaxSize()) {
        @Suppress("UNUSED_EXPRESSION")
        frame
        drawImage(
            image = image,
            dstSize = IntSize(size.width.toInt(), size.height.toInt()),
            filterQuality = FilterQuality.Low,
        )
    }
}
