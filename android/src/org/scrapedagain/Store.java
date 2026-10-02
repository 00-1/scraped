package org.scrapedagain;

import android.content.Context;

import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;

/**
 * The page's saved data: one file per key under files/data, which Android
 * backs up to the player's Google account (see res/xml). Writes go to a
 * temporary file first, so a crash never leaves half a world.
 */
final class Store {
    private final File dir;

    Store(Context c) {
        dir = new File(c.getFilesDir(), "data");
        if (!dir.isDirectory() && !dir.mkdirs()) {
            throw new IllegalStateException("cannot make " + dir);
        }
    }

    private File file(String key) {
        String safe = key.replaceAll("[^A-Za-z0-9_.-]", "_");
        if (safe.isEmpty() || safe.startsWith(".")) {
            safe = "_" + safe;
        }
        return new File(dir, safe + ".json");
    }

    synchronized String get(String key) {
        File f = file(key);
        if (!f.isFile()) {
            return null;
        }
        try (FileInputStream in = new FileInputStream(f)) {
            byte[] b = new byte[(int) f.length()];
            int read = 0;
            while (read < b.length) {
                int n = in.read(b, read, b.length - read);
                if (n < 0) {
                    break;
                }
                read += n;
            }
            return new String(b, 0, read, StandardCharsets.UTF_8);
        } catch (IOException e) {
            return null;
        }
    }

    synchronized void put(String key, String value) {
        File f = file(key);
        File tmp = new File(dir, f.getName() + ".tmp");
        try (FileOutputStream o = new FileOutputStream(tmp)) {
            o.write(value.getBytes(StandardCharsets.UTF_8));
            o.getFD().sync();
        } catch (IOException e) {
            return;
        }
        if (!tmp.renameTo(f)) {
            tmp.delete();
        }
    }

    synchronized void del(String key) {
        File f = file(key);
        if (f.isFile()) {
            f.delete();
        }
    }

    synchronized List<String> keys() {
        List<String> out = new ArrayList<>();
        File[] files = dir.listFiles();
        if (files != null) {
            for (File f : files) {
                String n = f.getName();
                if (n.endsWith(".json")) {
                    out.add(n.substring(0, n.length() - 5));
                }
            }
        }
        return out;
    }
}
