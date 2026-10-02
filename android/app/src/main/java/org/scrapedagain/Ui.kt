@file:OptIn(ExperimentalMaterial3Api::class, ExperimentalFoundationApi::class)

package org.scrapedagain

import android.content.Intent
import android.os.Build
import android.text.format.DateUtils
import android.view.HapticFeedbackConstants
import androidx.activity.ComponentActivity
import androidx.activity.SystemBarStyle
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.union
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Build
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.FilledIconButton
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LargeTopAppBar
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Slider
import androidx.compose.material3.SmallFloatingActionButton
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.SuggestionChip
import androidx.compose.material3.SuggestionChipDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.material3.rememberTopAppBarState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/**
 * The whole app on screen. Shaped like a chat app, because the game is a
 * conversation: worlds listed like conversations; in a world, the game's
 * text set as prose and your commands as bubbles, with a composer that
 * rides on the keyboard. Every word around the game comes from Jb's
 * app.label slot ([AppModel.label]).
 */
@Composable
fun App(model: AppModel) {
    val (scheme, ink) = schemeFor(model.look)
    val activity = LocalContext.current as ComponentActivity
    LaunchedEffect(ink.dark) {
        val style = if (ink.dark) SystemBarStyle.dark(android.graphics.Color.TRANSPARENT)
        else SystemBarStyle.light(android.graphics.Color.TRANSPARENT, android.graphics.Color.TRANSPARENT)
        activity.enableEdgeToEdge(statusBarStyle = style, navigationBarStyle = style)
    }
    ScrapedTheme(scheme) {
        val snackbar = remember { SnackbarHostState() }
        LaunchedEffect(model) {
            model.notices.collect { snackbar.showSnackbar(model.label(it)) }
        }
        BackHandler(enabled = model.screen != Screen.Worlds) {
            when (model.screen) {
                Screen.Game -> model.leave()
                Screen.Notebook -> model.screen = Screen.Game
                Screen.Integrations -> model.screen = Screen.Worlds
                Screen.Worlds -> {}
            }
        }
        Box(Modifier.fillMaxSize().background(scheme.background)) {
            if (model.look.atmosphere) Atmosphere(ink.dark)
            if (model.ready) {
                when (model.screen) {
                    Screen.Worlds -> WorldsScreen(model)
                    Screen.Game -> GameScreen(model, ink)
                    Screen.Notebook -> NotebookScreen(model)
                    Screen.Integrations -> IntegrationsScreen(model)
                }
            }
            if (!model.ready || model.busy) {
                Box(
                    Modifier.fillMaxSize().background(scheme.background.copy(alpha = 0.9f)),
                    contentAlignment = Alignment.Center,
                ) {
                    Text(
                        model.label("loading"),
                        fontFamily = FontFamily.Serif,
                        fontStyle = FontStyle.Italic,
                        color = ink.faint,
                    )
                }
            }
            SnackbarHost(
                snackbar,
                Modifier.align(Alignment.BottomCenter).navigationBarsPadding().imePadding().padding(bottom = 96.dp),
            )
        }
    }
}

private fun haptic(view: android.view.View) {
    view.performHapticFeedback(
        if (Build.VERSION.SDK_INT >= 30) HapticFeedbackConstants.CONFIRM else HapticFeedbackConstants.KEYBOARD_TAP,
    )
}

// ---------------------------------------------------------------- worlds

@Composable
private fun WorldsScreen(model: AppModel) {
    val scroll = TopAppBarDefaults.exitUntilCollapsedScrollBehavior(rememberTopAppBarState())
    var sheet by remember { mutableStateOf<String?>(null) }
    var menuFor by remember { mutableStateOf<World?>(null) }
    val view = LocalView.current
    Scaffold(
        modifier = Modifier.nestedScroll(scroll.nestedScrollConnection),
        containerColor = Color.Transparent,
        topBar = {
            LargeTopAppBar(
                title = { Text(model.label("worlds"), fontFamily = FontFamily.Serif, fontWeight = FontWeight.SemiBold) },
                actions = {
                    IconButton(onClick = { sheet = "appearance" }) {
                        Icon(Icons.Filled.Settings, contentDescription = model.label("appearance"))
                    }
                    IconButton(onClick = { model.screen = Screen.Integrations }) {
                        Icon(Icons.Filled.Build, contentDescription = model.label("integrations"))
                    }
                },
                colors = TopAppBarDefaults.largeTopAppBarColors(
                    containerColor = Color.Transparent,
                    scrolledContainerColor = MaterialTheme.colorScheme.background,
                ),
                scrollBehavior = scroll,
            )
        },
        floatingActionButton = {
            ExtendedFloatingActionButton(
                text = { Text(model.label("new_world")) },
                icon = { Icon(Icons.Filled.Add, contentDescription = null) },
                onClick = { sheet = "new" },
                modifier = Modifier.testTag("newWorld"),
            )
        },
    ) { pad ->
        if (model.worlds.isEmpty()) {
            Box(Modifier.fillMaxSize().padding(pad), contentAlignment = Alignment.Center) {
                Text(
                    model.label("no_worlds"),
                    fontFamily = FontFamily.Serif,
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        LazyColumn(
            contentPadding = PaddingValues(top = pad.calculateTopPadding(), bottom = pad.calculateBottomPadding() + 96.dp),
            modifier = Modifier.fillMaxSize().testTag("worlds"),
        ) {
            items(model.worlds, key = { it.id }) { w ->
                WorldRow(model, w, onClick = { model.open(w.id) }, onLong = { haptic(view); menuFor = w })
            }
        }
    }
    when (sheet) {
        "new" -> NewWorldSheet(model) { sheet = null }
        "appearance" -> AppearanceSheet(model) { sheet = null }
    }
    menuFor?.let { w -> WorldMenuSheet(model, w, inGame = false) { menuFor = null } }
}

@Composable
private fun Avatar(w: World) {
    val hue = (w.seed.fold(0L) { h, c -> (h * 31 + c.code) and 0xffffffffL } % 360).toFloat()
    // A code's first characters are its header, the same for most worlds;
    // the middle tells them apart.
    val letters = w.code.replace("-", "").drop(3).take(2)
    Box(
        Modifier.size(46.dp).clip(CircleShape).background(Color.hsl(hue, 0.34f, 0.42f)),
        contentAlignment = Alignment.Center,
    ) {
        Text(letters, color = Color(0xFFFFFAF0), fontFamily = FontFamily.Monospace, fontWeight = FontWeight.SemiBold)
    }
}

@Composable
private fun WorldRow(model: AppModel, w: World, onClick: () -> Unit, onLong: () -> Unit) {
    val preview = w.lastGame().lineSequence().firstOrNull { it.isNotBlank() }?.trim() ?: ""
    val tag = if (w.ended) model.label("ended") else "${model.label("day")} ${w.day}"
    ListItem(
        modifier = Modifier.combinedClickable(onClick = onClick, onLongClick = onLong).testTag("world"),
        colors = ListItemDefaults.colors(containerColor = Color.Transparent),
        leadingContent = { Avatar(w) },
        headlineContent = {
            Text("${model.label("world")} ${w.code}", maxLines = 1, overflow = TextOverflow.Ellipsis, fontWeight = FontWeight.SemiBold)
        },
        supportingContent = { Text("$tag · $preview", maxLines = 1, overflow = TextOverflow.Ellipsis) },
        trailingContent = {
            Text(
                DateUtils.getRelativeTimeSpanString(w.updated, System.currentTimeMillis(), DateUtils.MINUTE_IN_MILLIS).toString(),
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        },
    )
}

@Composable
private fun Segments(options: List<String>, selected: String, label: (String) -> String, onPick: (String) -> Unit) {
    SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth()) {
        options.forEachIndexed { i, o ->
            SegmentedButton(
                selected = o == selected,
                onClick = { onPick(o) },
                shape = SegmentedButtonDefaults.itemShape(index = i, count = options.size),
            ) { Text(label(o), maxLines = 1, overflow = TextOverflow.Ellipsis) }
        }
    }
}

@Composable
private fun NewWorldSheet(model: AppModel, onClose: () -> Unit) {
    val scope = rememberCoroutineScope()
    var difficulty by remember { mutableStateOf("standard") }
    var code by remember { mutableStateOf("") }
    var bad by remember { mutableStateOf(false) }
    ModalBottomSheet(onDismissRequest = onClose, sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)) {
        Column(Modifier.padding(horizontal = 20.dp).padding(bottom = 24.dp).imePadding()) {
            Text(model.label("new_world"), style = MaterialTheme.typography.titleLarge)
            Spacer(Modifier.height(16.dp))
            Text(model.label("difficulty"), style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Spacer(Modifier.height(6.dp))
            Segments(listOf("gentle", "standard", "archaeologist"), difficulty, { model.label(it) }) { difficulty = it }
            Spacer(Modifier.height(16.dp))
            OutlinedTextField(
                value = code,
                onValueChange = { code = it.uppercase(); bad = false },
                label = { Text(model.label("code_hint")) },
                singleLine = true,
                isError = bad,
                supportingText = if (bad) ({ Text(model.label("code_bad")) }) else null,
                keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Characters, autoCorrectEnabled = false),
                textStyle = TextStyle(fontFamily = FontFamily.Monospace),
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(16.dp))
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                TextButton(onClick = onClose) { Text(model.label("cancel")) }
                Spacer(Modifier.width(8.dp))
                Button(
                    onClick = {
                        scope.launch {
                            if (model.newWorld(difficulty, code)) onClose() else bad = true
                        }
                    },
                    modifier = Modifier.testTag("begin"),
                ) { Text(model.label("begin")) }
            }
        }
    }
}

@Composable
private fun AppearanceSheet(model: AppModel, onClose: () -> Unit) {
    val look = model.look
    ModalBottomSheet(onDismissRequest = onClose) {
        Column(Modifier.padding(horizontal = 20.dp).padding(bottom = 28.dp)) {
            Text(model.label("appearance"), style = MaterialTheme.typography.titleLarge)
            Spacer(Modifier.height(16.dp))
            Text(model.label("text_size"), style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Slider(
                value = look.size.toFloat(),
                onValueChange = { model.setLook(look.copy(size = it.toInt())) },
                valueRange = 14f..28f,
                steps = 13,
            )
            val sample = model.world?.lastGame()?.lineSequence()?.firstOrNull { it.isNotBlank() } ?: model.label("no_worlds")
            Text(sample, fontFamily = readingFont(look), fontSize = look.size.sp, lineHeight = (look.size * 1.55).sp, maxLines = 2, overflow = TextOverflow.Ellipsis)
            Spacer(Modifier.height(16.dp))
            Text(model.label("theme"), style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Spacer(Modifier.height(6.dp))
            Segments(listOf("system", "light", "dark", "sepia"), look.theme, { model.label(it) }) { model.setLook(look.copy(theme = it)) }
            Spacer(Modifier.height(16.dp))
            Text(model.label("font"), style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Spacer(Modifier.height(6.dp))
            Segments(listOf("serif", "sans"), look.font, { model.label(it) }) { model.setLook(look.copy(font = it)) }
            Spacer(Modifier.height(12.dp))
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Text(model.label("atmosphere"), Modifier.weight(1f))
                Switch(checked = look.atmosphere, onCheckedChange = { model.setLook(look.copy(atmosphere = it)) })
            }
        }
    }
}

@Composable
private fun WorldMenuSheet(model: AppModel, w: World, inGame: Boolean, onClose: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var confirm by remember { mutableStateOf(false) }
    val export = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("application/json")) {
        model.exportTo(it, w)
        onClose()
    }
    ModalBottomSheet(onDismissRequest = onClose) {
        Column(Modifier.padding(horizontal = 12.dp).padding(bottom = 24.dp)) {
            Text("${model.label("world")} ${w.code}", style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(8.dp))
            MenuItem(model.label("share_code")) {
                val i = Intent(Intent.ACTION_SEND).setType("text/plain")
                    .putExtra(Intent.EXTRA_TEXT, "${model.label("share_text")} ${w.code}")
                context.startActivity(Intent.createChooser(i, null))
                onClose()
            }
            if (inGame) {
                MenuItem(model.label("manual")) {
                    onClose()
                    scope.launch { model.send("manual", 'y') }
                }
            }
            MenuItem(model.label("export")) { export.launch("scraped-${w.code}.json") }
            MenuItem(model.label("delete"), MaterialTheme.colorScheme.error) { confirm = true }
        }
    }
    if (confirm) {
        AlertDialog(
            onDismissRequest = { confirm = false },
            title = { Text(model.label("delete_confirm")) },
            confirmButton = {
                TextButton(onClick = { confirm = false; onClose(); model.delete(w.id) }) { Text(model.label("delete")) }
            },
            dismissButton = { TextButton(onClick = { confirm = false }) { Text(model.label("cancel")) } },
        )
    }
}

@Composable
private fun MenuItem(text: String, color: Color = Color.Unspecified, onClick: () -> Unit) {
    TextButton(onClick = onClick, modifier = Modifier.fillMaxWidth()) {
        Text(text, Modifier.fillMaxWidth(), color = color, style = MaterialTheme.typography.bodyLarge)
    }
}

// ---------------------------------------------------------------- a world

@Composable
private fun GameScreen(model: AppModel, ink: Ink) {
    val w = model.world ?: return
    val look = model.look
    val scope = rememberCoroutineScope()
    val list = rememberLazyListState()
    val view = LocalView.current
    val clipboard = LocalClipboardManager.current
    var menu by remember { mutableStateOf(false) }
    var open by remember { mutableStateOf(-1) }
    var input by remember { mutableStateOf(TextFieldValue("")) }
    val focus = remember { FocusRequester() }
    val n = model.transcript.size
    // Follow the newest text as chat apps do, unless the player has
    // scrolled back to read; then offer a way down.
    LaunchedEffect(n) {
        if (list.firstVisibleItemIndex <= 2) list.animateScrollToItem(0)
    }
    val reading by remember { androidx.compose.runtime.derivedStateOf { list.firstVisibleItemIndex > 2 } }

    fun submit() {
        val cmd = input.text.trim()
        if (cmd.isEmpty()) return
        haptic(view)
        input = TextFieldValue("")
        scope.launch {
            model.send(cmd, 'y')
            list.animateScrollToItem(0)
        }
    }

    Column(Modifier.fillMaxSize()) {
        TopAppBar(
            navigationIcon = {
                IconButton(onClick = { model.leave() }) {
                    Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = model.label("back"))
                }
            },
            title = {
                Column {
                    Text("${model.label("world")} ${w.code}", maxLines = 1, overflow = TextOverflow.Ellipsis, style = MaterialTheme.typography.titleMedium)
                    Text(
                        "${model.label("day")} ${w.day} · ${model.label(w.difficulty)}",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            },
            actions = {
                IconButton(onClick = { model.screen = Screen.Notebook }) {
                    Icon(Icons.Filled.Edit, contentDescription = model.label("notebook"))
                }
                IconButton(onClick = { menu = true }) {
                    Icon(Icons.Filled.MoreVert, contentDescription = model.label("menu"))
                }
            },
            colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.background.copy(alpha = 0.92f)),
        )
        Box(Modifier.weight(1f)) {
            LazyColumn(
                state = list,
                reverseLayout = true,
                contentPadding = PaddingValues(horizontal = 20.dp, vertical = 12.dp),
                modifier = Modifier.fillMaxSize().testTag("transcript"),
            ) {
                items(n, key = { n - 1 - it }) { i ->
                    val index = n - 1 - i
                    val e = model.transcript[index]
                    if (e.kind == 'g') {
                        GameText(
                            e.text, look, newest = index == n - 1, open = open == index,
                            onTap = { open = if (open == index) -1 else index },
                            onCopy = { clipboard.setText(AnnotatedString(e.text)); model.notices.tryEmit("copied"); open = -1 },
                            onNote = { model.addToNotebook(e.text); model.notices.tryEmit("added"); open = -1 },
                            model = model,
                        )
                    } else {
                        Bubble(e, look, ink, model)
                    }
                }
            }
            if (reading) {
                SmallFloatingActionButton(
                    onClick = { scope.launch { list.animateScrollToItem(0) } },
                    modifier = Modifier.align(Alignment.BottomCenter).padding(8.dp),
                    containerColor = MaterialTheme.colorScheme.surfaceContainer,
                ) { Icon(Icons.Filled.KeyboardArrowDown, contentDescription = model.label("latest")) }
            }
        }
        if (w.ended) {
            Surface(
                color = MaterialTheme.colorScheme.surfaceContainer,
                shape = RoundedCornerShape(16.dp),
                modifier = Modifier.fillMaxWidth().padding(horizontal = 14.dp, vertical = 4.dp),
            ) {
                Row(Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text(model.label("ended"), Modifier.weight(1f))
                    FilledTonalButton(onClick = { model.leave() }) { Text(model.label("new_world")) }
                }
            }
        }
        Composer(
            model = model,
            input = input,
            onInput = { input = it },
            onSend = { submit() },
            onRun = { cmd -> haptic(view); scope.launch { model.send(cmd, 'y'); list.animateScrollToItem(0) } },
            focus = focus,
            modifier = Modifier.windowInsetsPadding(WindowInsets.navigationBars.union(WindowInsets.ime)),
        )
    }
    if (menu) WorldMenuSheet(model, w, inGame = true) { menu = false }
}

@Composable
private fun GameText(
    text: String,
    look: Look,
    newest: Boolean,
    open: Boolean,
    onTap: () -> Unit,
    onCopy: () -> Unit,
    onNote: () -> Unit,
    model: AppModel,
) {
    val clean = remember(text) { text.replace(Regex("(?m)^[ \\t]+"), "").trim() }
    Column(Modifier.fillMaxWidth().padding(bottom = 18.dp)) {
        Text(
            clean,
            fontFamily = readingFont(look),
            fontSize = look.size.sp,
            lineHeight = (look.size * 1.6).sp,
            color = MaterialTheme.colorScheme.onBackground,
            modifier = Modifier
                .widthIn(max = 640.dp)
                .clip(RoundedCornerShape(10.dp))
                .background(if (open) MaterialTheme.colorScheme.surfaceContainer else Color.Transparent)
                .combinedClickable(onClick = onTap, onLongClick = onTap)
                .padding(vertical = 2.dp)
                .semantics { if (newest) liveRegion = LiveRegionMode.Polite }
                .testTag("game"),
        )
        if (open) {
            Row(Modifier.padding(top = 6.dp)) {
                FilledTonalButton(onClick = onCopy) { Text(model.label("copy")) }
                Spacer(Modifier.width(8.dp))
                FilledTonalButton(onClick = onNote, modifier = Modifier.testTag("toNotebook")) { Text(model.label("to_notebook")) }
            }
        }
    }
}

@Composable
private fun Bubble(e: Entry, look: Look, ink: Ink, model: AppModel) {
    val agent = e.kind == 'a'
    val who = if (agent) model.label("agent") else model.label("you")
    Row(Modifier.fillMaxWidth().padding(top = 4.dp, bottom = 14.dp), horizontalArrangement = Arrangement.End) {
        Surface(
            color = if (agent) ink.agent else ink.bubble,
            shape = RoundedCornerShape(topStart = 20.dp, topEnd = 20.dp, bottomEnd = 6.dp, bottomStart = 20.dp),
            modifier = Modifier
                .widthIn(max = 300.dp)
                .semantics(mergeDescendants = true) { contentDescription = "$who: ${e.text}" }
                .testTag(if (agent) "agentBubble" else "bubble"),
        ) {
            Column(Modifier.padding(horizontal = 15.dp, vertical = 9.dp)) {
                if (agent) {
                    Text(who.uppercase(), style = MaterialTheme.typography.labelSmall, color = ink.faint)
                }
                Text(e.text, fontFamily = FontFamily.Monospace, fontSize = (look.size * 0.82).sp)
            }
        }
    }
}

@Composable
private fun Composer(
    model: AppModel,
    input: TextFieldValue,
    onInput: (TextFieldValue) -> Unit,
    onSend: () -> Unit,
    onRun: (String) -> Unit,
    focus: FocusRequester,
    modifier: Modifier = Modifier,
) {
    val chips = model.chips
    fun put(text: String) = onInput(TextFieldValue(text, TextRange(text.length)))
    Column(modifier.fillMaxWidth().background(MaterialTheme.colorScheme.background.copy(alpha = 0.94f)).padding(top = 6.dp, bottom = 8.dp)) {
        LazyRow(
            contentPadding = PaddingValues(horizontal = 12.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            modifier = Modifier.testTag("chips"),
        ) {
            items(chips.commands) { (label, cmd) ->
                Chip(label, tonal = true) { onRun(cmd) }
            }
            items(chips.verbs) { v ->
                Chip(v, accent = true) {
                    val rest = input.text.replace(Regex("^\\s*(read|take|examine)\\b\\s*", RegexOption.IGNORE_CASE), "")
                    put("$v $rest")
                    focus.requestFocus()
                }
            }
            items(chips.things) { (label, word) ->
                Chip(label) {
                    val v = input.text.trimEnd()
                    put(if (v.isEmpty()) "examine $word" else "$v $word")
                    focus.requestFocus()
                }
            }
            items(chips.after) { c -> Chip(c, tonal = true) { onRun(c) } }
        }
        Spacer(Modifier.height(8.dp))
        Row(Modifier.padding(horizontal = 10.dp), verticalAlignment = Alignment.Bottom) {
            TextField(
                value = input,
                onValueChange = onInput,
                modifier = Modifier.weight(1f).focusRequester(focus).testTag("composer"),
                placeholder = {
                    Text(model.label("composer"), fontFamily = readingFont(model.look), fontStyle = FontStyle.Italic)
                },
                leadingIcon = if (input.text.isEmpty() && model.hasHistory()) {
                    {
                        IconButton(onClick = { put(model.recall()) }) {
                            Icon(Icons.Filled.Refresh, contentDescription = model.label("recall"))
                        }
                    }
                } else {
                    null
                },
                shape = RoundedCornerShape(26.dp),
                colors = TextFieldDefaults.colors(
                    focusedContainerColor = MaterialTheme.colorScheme.surfaceContainer,
                    unfocusedContainerColor = MaterialTheme.colorScheme.surfaceContainer,
                    focusedIndicatorColor = Color.Transparent,
                    unfocusedIndicatorColor = Color.Transparent,
                ),
                textStyle = TextStyle(fontFamily = FontFamily.Monospace, fontSize = 16.sp),
                keyboardOptions = KeyboardOptions(
                    capitalization = KeyboardCapitalization.None,
                    autoCorrectEnabled = false,
                    imeAction = ImeAction.Send,
                ),
                keyboardActions = KeyboardActions(onSend = { onSend() }),
                maxLines = 5,
            )
            Spacer(Modifier.width(8.dp))
            FilledIconButton(
                onClick = onSend,
                enabled = input.text.isNotBlank(),
                modifier = Modifier.size(54.dp).testTag("send"),
            ) {
                Icon(Icons.AutoMirrored.Filled.Send, contentDescription = model.label("send"))
            }
        }
    }
}

@Composable
private fun Chip(text: String, tonal: Boolean = false, accent: Boolean = false, onClick: () -> Unit) {
    SuggestionChip(
        onClick = onClick,
        label = { Text(text, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.widthIn(max = 220.dp)) },
        shape = RoundedCornerShape(50),
        colors = SuggestionChipDefaults.suggestionChipColors(
            containerColor = if (tonal) MaterialTheme.colorScheme.surfaceVariant else MaterialTheme.colorScheme.surfaceContainer,
            labelColor = if (accent) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface,
        ),
        border = if (tonal || accent) null else SuggestionChipDefaults.suggestionChipBorder(enabled = true),
    )
}

// ---------------------------------------------------------------- notebook

@Composable
private fun NotebookScreen(model: AppModel) {
    val w = model.world ?: return
    var text by remember { mutableStateOf(TextFieldValue(w.notebook)) }
    LaunchedEffect(text.text) {
        delay(500)
        model.setNotebook(text.text)
    }
    DisposableEffect(Unit) { onDispose { model.setNotebook(text.text) } }
    Column(Modifier.fillMaxSize().imePadding()) {
        TopAppBar(
            navigationIcon = {
                IconButton(onClick = { model.screen = Screen.Game }) {
                    Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = model.label("back"))
                }
            },
            title = {
                Column {
                    Text(model.label("notebook"), style = MaterialTheme.typography.titleMedium)
                    Text("${model.label("world")} ${w.code}", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            },
            colors = TopAppBarDefaults.topAppBarColors(containerColor = Color.Transparent),
        )
        TextField(
            value = text,
            onValueChange = { text = it },
            modifier = Modifier.fillMaxSize().navigationBarsPadding().testTag("notebookText"),
            placeholder = { Text(model.label("notebook_hint"), fontStyle = FontStyle.Italic, fontFamily = readingFont(model.look)) },
            textStyle = TextStyle(fontFamily = readingFont(model.look), fontSize = model.look.size.sp, lineHeight = (model.look.size * 1.6).sp),
            colors = TextFieldDefaults.colors(
                focusedContainerColor = MaterialTheme.colorScheme.surfaceContainer,
                unfocusedContainerColor = MaterialTheme.colorScheme.surfaceContainer,
                focusedIndicatorColor = Color.Transparent,
                unfocusedIndicatorColor = Color.Transparent,
            ),
        )
    }
}

// ---------------------------------------------------------------- integrations

@Composable
private fun IntegrationsScreen(model: AppModel) {
    val clipboard = LocalClipboardManager.current
    val chooseSync = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("application/json")) { model.setSync(it) }
    val openFile = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { model.importFrom(it) }
    LaunchedEffect(Unit) { model.refreshAgent() }
    Column(Modifier.fillMaxSize()) {
        TopAppBar(
            navigationIcon = {
                IconButton(onClick = { model.screen = Screen.Worlds }) {
                    Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = model.label("back"))
                }
            },
            title = { Text(model.label("integrations")) },
            colors = TopAppBarDefaults.topAppBarColors(containerColor = Color.Transparent),
        )
        Column(
            Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 16.dp).navigationBarsPadding(),
            verticalArrangement = Arrangement.spacedBy(14.dp),
        ) {
            Section(model.label("backup"), model.label("backup_hint")) {
                FilledTonalButton(onClick = { model.backupNow() }) { Text(model.label("backup_now")) }
            }
            Section(model.label("sync"), model.label("sync_hint")) {
                model.syncName?.let { Field(model.label("sync_on"), it) }
                if (model.syncName == null) {
                    Button(onClick = { chooseSync.launch("scraped-again-sync.json") }) { Text(model.label("sync_choose")) }
                } else {
                    FilledTonalButton(onClick = { model.stopSync() }) { Text(model.label("sync_stop")) }
                }
                FilledTonalButton(onClick = {
                    val u = model.syncUri()
                    if (u != null) model.importFrom(u) else openFile.launch(arrayOf("application/json", "*/*"))
                }) { Text(model.label("sync_restore")) }
                FilledTonalButton(onClick = { openFile.launch(arrayOf("application/json", "*/*")) }) { Text(model.label("import")) }
            }
            Section(model.label("agent_access"), model.label("agent_hint"), toggle = {
                Switch(
                    checked = model.agentOn,
                    onCheckedChange = { model.setAgent(it) },
                    modifier = Modifier.semantics { contentDescription = model.label("agent_on") }.testTag("agentSwitch"),
                )
            }) {
                if (model.agentOn) {
                    val url = model.agentUrl
                    if (url == null) {
                        Text(model.label("agent_offline"), color = MaterialTheme.colorScheme.error)
                    } else {
                        Field(model.label("agent_address"), "$url/mcp")
                        Field(model.label("agent_key"), model.agentKey)
                        Button(onClick = {
                            val config = org.json.JSONObject().put(
                                "mcpServers",
                                org.json.JSONObject().put(
                                    "scraped-again",
                                    org.json.JSONObject().put("type", "http").put("url", "$url/mcp")
                                        .put("headers", org.json.JSONObject().put("Authorization", "Bearer ${model.agentKey}")),
                                ),
                            ).toString(2)
                            clipboard.setText(AnnotatedString(config))
                            model.notices.tryEmit("copied")
                        }) { Text(model.label("agent_copy")) }
                        FilledTonalButton(onClick = { model.renewKey() }) { Text(model.label("agent_new_key")) }
                    }
                }
            }
            Spacer(Modifier.height(24.dp))
        }
    }
}

@Composable
private fun Section(title: String, hint: String, toggle: (@Composable () -> Unit)? = null, content: @Composable () -> Unit) {
    Card(
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
        shape = RoundedCornerShape(20.dp),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(Modifier.padding(18.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(title, style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
                toggle?.invoke()
            }
            Text(hint, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            content()
        }
    }
}

@Composable
private fun Field(label: String, value: String) {
    Column {
        Text(label, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        SelectionContainer {
            Surface(color = MaterialTheme.colorScheme.surfaceVariant, shape = RoundedCornerShape(10.dp)) {
                Text(value, fontFamily = FontFamily.Monospace, fontSize = 14.sp, modifier = Modifier.padding(10.dp))
            }
        }
    }
}
