package org.scrapedagain;

/** The same native methods the app's Kotlin Engine object declares. */
public final class Engine {
    public static native String call(String request);
    public static native void atmosphere(int[] pixels, int width, int height, float time, boolean dark);
}
