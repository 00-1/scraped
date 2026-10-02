package org.scrapedagain;

import android.app.Activity;
import android.app.backup.BackupManager;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.content.res.Configuration;
import android.database.Cursor;
import android.graphics.Color;
import android.graphics.Insets;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.provider.OpenableColumns;
import android.view.HapticFeedbackConstants;
import android.view.View;
import android.view.Window;
import android.view.WindowInsets;
import android.view.WindowInsetsController;
import android.webkit.JavascriptInterface;
import android.webkit.ValueCallback;
import android.webkit.WebResourceRequest;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.FrameLayout;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.charset.StandardCharsets;
import java.security.SecureRandom;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;

import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;
import org.json.JSONTokener;

/**
 * The whole app is one screen: a WebView showing the game's page (the
 * engine runs inside it as WebAssembly). This shell gives the page what a
 * browser can't: files that Android backs up to the player's Google
 * account, a synced file of their choosing, sharing, haptics, the keyboard
 * and system bars handled properly, and the agent server.
 */
public class MainActivity extends Activity {
    private static final int REQ_SYNC = 1;
    private static final int REQ_RESTORE = 2;
    private static final int REQ_EXPORT = 3;
    private static final int REQ_IMPORT = 4;

    private final Handler main = new Handler(Looper.getMainLooper());
    private WebView web;
    private FrameLayout root;
    private Store store;
    private SharedPreferences prefs;
    private AgentServer agent;
    private boolean pageReady;
    private String pendingExport;
    private int insetTop;
    private int insetBottom;
    private boolean imeOpen;

    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        store = new Store(this);
        prefs = getSharedPreferences("app", MODE_PRIVATE);
        Window w = getWindow();
        boolean edge = Build.VERSION.SDK_INT >= 30;
        if (edge) {
            w.setDecorFitsSystemWindows(false);
            w.setStatusBarColor(Color.TRANSPARENT);
            w.setNavigationBarColor(Color.TRANSPARENT);
        }
        root = new FrameLayout(this);
        root.setBackgroundColor(background(night()));
        web = new WebView(this);
        web.setBackgroundColor(background(night()));
        web.setOverScrollMode(View.OVER_SCROLL_NEVER);
        web.setVerticalScrollBarEnabled(false);
        WebSettings s = web.getSettings();
        s.setJavaScriptEnabled(true);
        s.setDomStorageEnabled(true);
        s.setAllowFileAccess(false);
        s.setAllowContentAccess(false);
        s.setSupportZoom(false);
        s.setBuiltInZoomControls(false);
        s.setTextZoom(Math.round(getResources().getConfiguration().fontScale * 100));
        web.setWebViewClient(new WebViewClient() {
            @Override
            public boolean shouldOverrideUrlLoading(WebView view, WebResourceRequest request) {
                // The page never navigates; links leave for the browser.
                Uri u = request.getUrl();
                if ("http".equals(u.getScheme()) || "https".equals(u.getScheme())) {
                    startActivity(new Intent(Intent.ACTION_VIEW, u));
                }
                return true;
            }
        });
        web.addJavascriptInterface(new Bridge(), "Android");
        root.addView(web, new FrameLayout.LayoutParams(-1, -1));
        setContentView(root);
        if (edge) {
            root.setOnApplyWindowInsetsListener(new View.OnApplyWindowInsetsListener() {
                @Override
                public WindowInsets onApplyWindowInsets(View v, WindowInsets insets) {
                    Insets sys = insets.getInsets(WindowInsets.Type.systemBars() | WindowInsets.Type.displayCutout());
                    Insets ime = insets.getInsets(WindowInsets.Type.ime());
                    boolean open = insets.isVisible(WindowInsets.Type.ime());
                    // The keyboard shrinks the page (the composer rides on top
                    // of it); the bars are drawn over, and the page pads itself.
                    FrameLayout.LayoutParams lp = (FrameLayout.LayoutParams) web.getLayoutParams();
                    lp.leftMargin = sys.left;
                    lp.rightMargin = sys.right;
                    lp.bottomMargin = open ? Math.max(ime.bottom, sys.bottom) : 0;
                    web.setLayoutParams(lp);
                    insetTop = sys.top;
                    insetBottom = open ? 0 : sys.bottom;
                    boolean was = imeOpen;
                    imeOpen = open;
                    sendInsets(open && !was);
                    return WindowInsets.CONSUMED;
                }
            });
        }
        applyBars(night());
        web.loadUrl("file:///android_asset/index.html");
    }

    @Override
    protected void onDestroy() {
        if (agent != null) {
            agent.stop();
        }
        web.destroy();
        super.onDestroy();
    }

    @Override
    @SuppressWarnings("deprecation")
    public void onBackPressed() {
        if (!pageReady) {
            super.onBackPressed();
            return;
        }
        web.evaluateJavascript("window.__back && window.__back()", new ValueCallback<String>() {
            @Override
            public void onReceiveValue(String handled) {
                if (!"true".equals(handled)) {
                    moveTaskToBack(true);
                }
            }
        });
    }

    @Override
    public void onConfigurationChanged(Configuration c) {
        super.onConfigurationChanged(c);
        web.getSettings().setTextZoom(Math.round(c.fontScale * 100));
        boolean dark = night();
        root.setBackgroundColor(background(dark));
        emit("{\"type\":\"night\",\"dark\":" + dark + "}");
    }

    private boolean night() {
        return (getResources().getConfiguration().uiMode & Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES;
    }

    private static int background(boolean dark) {
        return dark ? Color.rgb(0x12, 0x11, 0x10) : Color.rgb(0xf7, 0xf5, 0xef);
    }

    /** Dark icons on light pages and the reverse. */
    @SuppressWarnings("deprecation")
    private void applyBars(boolean darkPage) {
        if (Build.VERSION.SDK_INT >= 30) {
            WindowInsetsController c = getWindow().getInsetsController();
            if (c != null) {
                int light = WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS | WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS;
                c.setSystemBarsAppearance(darkPage ? 0 : light, light);
            }
        } else {
            Window w = getWindow();
            w.setStatusBarColor(background(darkPage));
            w.setNavigationBarColor(background(darkPage));
            int flags = View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR | View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR;
            View d = w.getDecorView();
            d.setSystemUiVisibility(darkPage ? d.getSystemUiVisibility() & ~flags : d.getSystemUiVisibility() | flags);
        }
    }

    private void sendInsets(boolean imeJustOpened) {
        float density = getResources().getDisplayMetrics().density;
        emit("{\"type\":\"insets\",\"top\":" + Math.round(insetTop / density)
                + ",\"bottom\":" + Math.round(insetBottom / density)
                + ",\"ime\":" + imeJustOpened + "}");
    }

    /** Sends an event to the page (on the main thread). */
    void emit(final String json) {
        main.post(new Runnable() {
            @Override
            public void run() {
                if (pageReady) {
                    web.evaluateJavascript("window.hostEvent && window.hostEvent(" + JSONObject.quote(json) + ")", null);
                }
            }
        });
    }

    // ---------- files the player chooses ----------

    private void pick(String action, int request, String name) {
        Intent i = new Intent(action);
        i.addCategory(Intent.CATEGORY_OPENABLE);
        i.setType(action.equals(Intent.ACTION_CREATE_DOCUMENT) ? "application/json" : "*/*");
        if (name != null) {
            i.putExtra(Intent.EXTRA_TITLE, name);
        }
        try {
            startActivityForResult(i, request);
        } catch (RuntimeException e) {
            emit("{\"type\":\"" + (request == REQ_SYNC ? "sync" : "restore") + "\",\"ok\":false}");
        }
    }

    @Override
    protected void onActivityResult(int request, int result, Intent data) {
        super.onActivityResult(request, result, data);
        Uri uri = result == RESULT_OK && data != null ? data.getData() : null;
        try {
            switch (request) {
                case REQ_SYNC:
                    if (uri != null) {
                        getContentResolver().takePersistableUriPermission(uri,
                                Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION);
                        prefs.edit().putString("sync", uri.toString()).apply();
                        emit("{\"type\":\"sync\",\"ok\":true}");
                    }
                    break;
                case REQ_RESTORE:
                case REQ_IMPORT:
                    if (uri != null) {
                        readInto(uri, request == REQ_RESTORE ? "restore" : "import");
                    }
                    break;
                case REQ_EXPORT:
                    if (uri != null && pendingExport != null) {
                        write(uri, pendingExport);
                    }
                    pendingExport = null;
                    break;
                default:
                    break;
            }
        } catch (RuntimeException | IOException e) {
            emit("{\"type\":\"sync\",\"ok\":false}");
        }
    }

    private void readInto(final Uri uri, final String type) {
        new Thread(new Runnable() {
            @Override
            public void run() {
                try (InputStream in = getContentResolver().openInputStream(uri)) {
                    ByteArrayOutputStream b = new ByteArrayOutputStream();
                    byte[] buf = new byte[16384];
                    int n;
                    while (in != null && (n = in.read(buf)) > 0) {
                        b.write(buf, 0, n);
                    }
                    String text = new String(b.toByteArray(), StandardCharsets.UTF_8);
                    emit(new JSONObject().put("type", type).put("ok", true).put("data", text).toString());
                } catch (IOException | JSONException | RuntimeException e) {
                    emit("{\"type\":\"" + type + "\",\"ok\":false}");
                }
            }
        }).start();
    }

    private void write(Uri uri, String text) throws IOException {
        try (OutputStream o = getContentResolver().openOutputStream(uri, "wt")) {
            if (o == null) {
                throw new IOException("no stream");
            }
            o.write(text.getBytes(StandardCharsets.UTF_8));
        }
    }

    private String displayName(Uri uri) {
        try (Cursor c = getContentResolver().query(uri, new String[] {OpenableColumns.DISPLAY_NAME}, null, null, null)) {
            if (c != null && c.moveToFirst()) {
                return c.getString(0);
            }
        } catch (RuntimeException ignored) {
            // Permission gone.
        }
        return uri.getLastPathSegment();
    }

    // ---------- the agent ----------

    private String agentKey() {
        String k = prefs.getString("agentKey", null);
        if (k == null) {
            k = newKey();
        }
        return k;
    }

    private String newKey() {
        byte[] b = new byte[18];
        new SecureRandom().nextBytes(b);
        StringBuilder s = new StringBuilder();
        for (byte x : b) {
            s.append(String.format("%02x", x & 0xff));
        }
        String k = s.toString();
        prefs.edit().putString("agentKey", k).apply();
        return k;
    }

    private synchronized void startAgent() {
        if (agent != null && agent.running()) {
            return;
        }
        agent = new AgentServer(new AgentProtocol(new AgentProtocol.Game() {
            @Override
            public String act(String command) {
                return callPage("window.__agentAct(" + JSONObject.quote(command) + ")");
            }

            @Override
            public String read(int entries) {
                return callPage("window.__agentRead(" + entries + ")");
            }
        }, agentKey()));
        agent.start();
    }

    private synchronized void stopAgent() {
        if (agent != null) {
            agent.stop();
            agent = null;
        }
    }

    /** Runs a page function on the main thread and waits for its text. */
    private String callPage(final String js) {
        final AtomicReference<String> out = new AtomicReference<>("");
        final CountDownLatch done = new CountDownLatch(1);
        main.post(new Runnable() {
            @Override
            public void run() {
                if (!pageReady) {
                    done.countDown();
                    return;
                }
                web.evaluateJavascript(js, new ValueCallback<String>() {
                    @Override
                    public void onReceiveValue(String v) {
                        try {
                            Object o = new JSONTokener(v).nextValue();
                            out.set(o instanceof String ? (String) o : "");
                        } catch (JSONException | RuntimeException e) {
                            out.set("");
                        }
                        done.countDown();
                    }
                });
            }
        });
        try {
            done.await(30, TimeUnit.SECONDS);
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
        }
        return out.get();
    }

    private JSONObject agentInfo() throws JSONException {
        JSONObject a = new JSONObject();
        boolean on = agent != null && agent.running();
        a.put("on", on);
        if (on) {
            String ip = AgentServer.lanAddress();
            if (ip != null) {
                a.put("url", "http://" + ip + ":" + agent.port());
            }
            a.put("key", agentKey());
        }
        return a;
    }

    /** What the page may ask of the phone. Called on a background thread. */
    final class Bridge {
        @JavascriptInterface
        public String get(String key) {
            return store.get(key);
        }

        @JavascriptInterface
        public void put(String key, String value) {
            store.put(key, value);
            new BackupManager(MainActivity.this).dataChanged();
        }

        @JavascriptInterface
        public void del(String key) {
            store.del(key);
            new BackupManager(MainActivity.this).dataChanged();
        }

        @JavascriptInterface
        public String keys() {
            JSONArray a = new JSONArray();
            for (String k : store.keys()) {
                a.put(k);
            }
            return a.toString();
        }

        @JavascriptInterface
        public void ready() {
            main.post(new Runnable() {
                @Override
                public void run() {
                    pageReady = true;
                    sendInsets(false);
                    if (prefs.getBoolean("agent", false)) {
                        startAgent();
                        emit("{\"type\":\"agent\"}");
                    }
                }
            });
        }

        @JavascriptInterface
        public void haptic() {
            main.post(new Runnable() {
                @Override
                public void run() {
                    web.performHapticFeedback(Build.VERSION.SDK_INT >= 30
                            ? HapticFeedbackConstants.CONFIRM : HapticFeedbackConstants.KEYBOARD_TAP);
                }
            });
        }

        @JavascriptInterface
        public void share(String text) {
            final Intent i = new Intent(Intent.ACTION_SEND);
            i.setType("text/plain");
            i.putExtra(Intent.EXTRA_TEXT, text);
            main.post(new Runnable() {
                @Override
                public void run() {
                    startActivity(Intent.createChooser(i, null));
                }
            });
        }

        @JavascriptInterface
        public void copy(String text) {
            ClipboardManager c = (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
            if (c != null) {
                c.setPrimaryClip(ClipData.newPlainText("text", text));
            }
        }

        @JavascriptInterface
        public void setDark(final boolean dark) {
            main.post(new Runnable() {
                @Override
                public void run() {
                    applyBars(dark);
                    root.setBackgroundColor(background(dark));
                    web.setBackgroundColor(background(dark));
                }
            });
        }

        @JavascriptInterface
        public String info() {
            try {
                JSONObject o = new JSONObject();
                o.put("night", night());
                o.put("backup", true);
                String sync = prefs.getString("sync", null);
                o.put("sync", sync == null ? JSONObject.NULL : displayName(Uri.parse(sync)));
                o.put("agent", agentInfo());
                return o.toString();
            } catch (JSONException e) {
                return "{}";
            }
        }

        @JavascriptInterface
        public void backupNow() {
            new BackupManager(MainActivity.this).dataChanged();
        }

        @JavascriptInterface
        public void syncChoose() {
            main.post(new Runnable() {
                @Override
                public void run() {
                    pick(Intent.ACTION_CREATE_DOCUMENT, REQ_SYNC, "scraped-again-sync.json");
                }
            });
        }

        @JavascriptInterface
        public void syncStop() {
            String s = prefs.getString("sync", null);
            if (s != null) {
                try {
                    getContentResolver().releasePersistableUriPermission(Uri.parse(s),
                            Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION);
                } catch (RuntimeException ignored) {
                    // Already gone.
                }
            }
            prefs.edit().remove("sync").apply();
        }

        @JavascriptInterface
        public void syncWrite(final String data) {
            final String s = prefs.getString("sync", null);
            if (s == null) {
                return;
            }
            new Thread(new Runnable() {
                @Override
                public void run() {
                    try {
                        write(Uri.parse(s), data);
                    } catch (IOException | RuntimeException e) {
                        emit("{\"type\":\"sync\",\"ok\":false}");
                    }
                }
            }).start();
        }

        @JavascriptInterface
        public void restore() {
            String s = prefs.getString("sync", null);
            if (s != null) {
                readInto(Uri.parse(s), "restore");
            } else {
                main.post(new Runnable() {
                    @Override
                    public void run() {
                        pick(Intent.ACTION_OPEN_DOCUMENT, REQ_RESTORE, null);
                    }
                });
            }
        }

        @JavascriptInterface
        public void exportFile(final String name, String data) {
            pendingExport = data;
            main.post(new Runnable() {
                @Override
                public void run() {
                    pick(Intent.ACTION_CREATE_DOCUMENT, REQ_EXPORT, name);
                }
            });
        }

        @JavascriptInterface
        public void importFile() {
            main.post(new Runnable() {
                @Override
                public void run() {
                    pick(Intent.ACTION_OPEN_DOCUMENT, REQ_IMPORT, null);
                }
            });
        }

        @JavascriptInterface
        public String agent(boolean on) {
            prefs.edit().putBoolean("agent", on).apply();
            if (on) {
                startAgent();
            } else {
                stopAgent();
            }
            try {
                return agentInfo().toString();
            } catch (JSONException e) {
                return "{}";
            }
        }

        @JavascriptInterface
        public String agentKey() {
            newKey();
            boolean on = agent != null && agent.running();
            stopAgent();
            if (on) {
                startAgent();
            }
            try {
                return agentInfo().toString();
            } catch (JSONException e) {
                return "{}";
            }
        }
    }
}
