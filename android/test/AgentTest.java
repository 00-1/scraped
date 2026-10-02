// Off-device tests of the agent server and protocol: a real socket on
// localhost, MCP and plain HTTP, the key, and that only text goes in and out.
//   run by android/test-agent.sh
import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;

import org.json.JSONArray;
import org.json.JSONObject;
import org.scrapedagain.AgentProtocol;
import org.scrapedagain.AgentServer;

public class AgentTest {
    static final List<String> typed = new ArrayList<>();
    static int fails = 0;

    static void check(boolean ok, String what) {
        if (!ok) {
            System.err.println("FAIL: " + what);
            fails++;
        }
    }

    static String[] call(String method, String url, String key, String body, String type) throws Exception {
        HttpURLConnection c = (HttpURLConnection) new URL(url).openConnection();
        c.setRequestMethod(method);
        if (key != null) c.setRequestProperty("Authorization", "Bearer " + key);
        if (body != null) {
            c.setDoOutput(true);
            c.setRequestProperty("Content-Type", type);
            try (OutputStream o = c.getOutputStream()) { o.write(body.getBytes(StandardCharsets.UTF_8)); }
        }
        int status = c.getResponseCode();
        InputStream in = status < 400 ? c.getInputStream() : c.getErrorStream();
        ByteArrayOutputStream b = new ByteArrayOutputStream();
        if (in != null) { byte[] buf = new byte[4096]; int n; while ((n = in.read(buf)) > 0) b.write(buf, 0, n); }
        return new String[] { String.valueOf(status), b.toString("UTF-8") };
    }

    public static void main(String[] args) throws Exception {
        AgentProtocol p = new AgentProtocol(new AgentProtocol.Game() {
            public String act(String command) { typed.add(command); return "You see a stele. (" + command + ")"; }
            public String read(int n) { return "> look\n\nYou see a stele. (" + n + ")"; }
        }, "s3cret");
        AgentServer s = new AgentServer(p);
        int port = s.start();
        check(port > 0, "server started");
        String base = "http://127.0.0.1:" + port;
        // The key is required.
        check(call("GET", base + "/read", null, null, null)[0].equals("401"), "no key, no entry");
        check(call("GET", base + "/read", "wrong", null, null)[0].equals("401"), "wrong key");
        // Plain HTTP.
        String[] r = call("POST", base + "/act", "s3cret", "read stele\n", "text/plain");
        check(r[0].equals("200") && r[1].equals("You see a stele. (read stele)"), "act: " + r[1]);
        r = call("GET", base + "/read?n=3&key=s3cret", null, null, null);
        check(r[0].equals("200") && r[1].endsWith("(3)"), "read with key in query: " + r[1]);
        r = call("POST", base + "/act", "s3cret", "{\"command\": \"look\"}", "application/json");
        check(r[1].endsWith("(look)"), "act with JSON body");
        // MCP.
        r = call("POST", base + "/mcp", "s3cret", "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-06-18\",\"capabilities\":{},\"clientInfo\":{\"name\":\"t\",\"version\":\"0\"}}}", "application/json");
        JSONObject init = new JSONObject(r[1]);
        check(init.getJSONObject("result").getString("protocolVersion").equals("2025-06-18"), "initialize");
        r = call("POST", base + "/mcp", "s3cret", "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}", "application/json");
        check(r[0].equals("202"), "notification accepted: " + r[0]);
        r = call("POST", base + "/mcp", "s3cret", "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}", "application/json");
        JSONArray tools = new JSONObject(r[1]).getJSONObject("result").getJSONArray("tools");
        check(tools.length() == 2 && tools.getJSONObject(0).getString("name").equals("act") && tools.getJSONObject(1).getString("name").equals("read"), "only act and read");
        r = call("POST", base + "/mcp", "s3cret", "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"act\",\"arguments\":{\"command\":\"go temple\"}}}", "application/json");
        String text = new JSONObject(r[1]).getJSONObject("result").getJSONArray("content").getJSONObject(0).getString("text");
        check(text.endsWith("(go temple)"), "mcp act: " + text);
        r = call("POST", base + "/mcp", "s3cret", "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"tools/call\",\"params\":{\"name\":\"save\",\"arguments\":{}}}", "application/json");
        check(new JSONObject(r[1]).getJSONObject("error").getInt("code") == -32602, "no other tools");
        r = call("POST", base + "/mcp", "s3cret", "{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"resources/list\"}", "application/json");
        check(new JSONObject(r[1]).getJSONObject("error").getInt("code") == -32601, "no other methods");
        r = call("POST", base + "/mcp", "s3cret", "not json", "application/json");
        check(new JSONObject(r[1]).getJSONObject("error").getInt("code") == -32700, "parse error");
        // Long and multi-line commands are cut to one line.
        StringBuilder big = new StringBuilder();
        for (int i = 0; i < 100; i++) big.append("look\n");
        call("POST", base + "/act", "s3cret", big.toString(), "text/plain");
        String last = typed.get(typed.size() - 1);
        check(last.length() <= AgentProtocol.MAX_COMMAND && !last.contains("\n"), "commands are one line, bounded");
        check(call("GET", base + "/nothing", "s3cret", null, null)[0].equals("404"), "unknown path");
        s.stop();
        check(!s.running(), "stopped");
        if (fails > 0) System.exit(1);
        System.out.println("agent tests passed (" + typed.size() + " commands)");
    }
}
