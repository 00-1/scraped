// Loads the engine library through JNI, as the app does, and plays a little.
//   java -Djava.library.path=target/release -cp out EngineTest
import org.scrapedagain.Engine;

public class EngineTest {
    public static void main(String[] a) {
        System.loadLibrary("scraped_android");
        String labels = Engine.call("{\"cmd\":\"app_labels\"}");
        if (!labels.contains("app_name")) throw new AssertionError(labels);
        String first = Engine.call("{\"cmd\":\"play_new\",\"seed\":\"7\"}");
        if (!first.contains("\"text\"")) throw new AssertionError(first);
        String look = Engine.call("{\"cmd\":\"play\",\"line\":\"look\"}");
        if (!look.contains("\"things\"")) throw new AssertionError(look);
        String save = Engine.call("{\"cmd\":\"play_save\"}");
        if (!save.contains("\"commands\":[\"look\"]")) throw new AssertionError(save);
        int[] px = new int[24 * 48];
        Engine.atmosphere(px, 24, 48, 3.5f, true);
        int lit = 0;
        for (int p : px) if ((p >>> 24) > 0) lit++;
        if (lit == 0) throw new AssertionError("blank backdrop");
        // Unicode survives the trip both ways.
        String echo = Engine.call("{\"cmd\":\"play\",\"line\":\"read ≈ stële\"}");
        if (!echo.contains("\"text\"")) throw new AssertionError(echo);
        System.out.println("JNI engine test passed (" + lit + " lit pixels)");
    }
}
