package org.scrapedagain;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.Map;

import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

/**
 * What an AI agent may do: read the game's text and type commands. Nothing
 * else (no saves, no state, no spoilers, no other worlds). Two doors to the
 * same two abilities:
 *
 * <ul>
 *   <li>MCP over HTTP: POST /mcp with JSON-RPC (initialize, tools/list,
 *       tools/call with the tools "act" and "read");</li>
 *   <li>plain HTTP: GET /read and POST /act with the command as the body.</li>
 * </ul>
 *
 * Every request needs the key, as "Authorization: Bearer KEY" or "?key=KEY".
 * Pure Java (plus org.json), so it is tested off the device.
 */
public final class AgentProtocol {
    /** The game, as the agent sees it. */
    public interface Game {
        /** Types a command; returns what the game said back. */
        String act(String command);

        /** The last few exchanges, as text. */
        String read(int entries);
    }

    /** An HTTP response. */
    public static final class Response {
        public final int status;
        public final String type;
        public final String body;

        Response(int status, String type, String body) {
            this.status = status;
            this.type = type;
            this.body = body;
        }
    }

    public static final String MCP_VERSION = "2025-06-18";
    /** Longest command accepted. */
    public static final int MAX_COMMAND = 400;

    private final Game game;
    private final String key;

    public AgentProtocol(Game game, String key) {
        this.game = game;
        this.key = key;
    }

    /** Handles one request. Header names are lower case. */
    public Response handle(String method, String target, Map<String, String> headers, String body) {
        String path = target;
        String query = "";
        int q = target.indexOf('?');
        if (q >= 0) {
            path = target.substring(0, q);
            query = target.substring(q + 1);
        }
        if (!authorised(headers, query)) {
            return text(401, "key required"); // DEBUG-TEXT: for agents
        }
        try {
            if (path.equals("/mcp")) {
                if (!method.equals("POST")) {
                    return text(405, "POST JSON-RPC here"); // DEBUG-TEXT
                }
                return mcp(body);
            }
            if (path.equals("/act") && method.equals("POST")) {
                String cmd = command(body);
                if (cmd.isEmpty()) {
                    return text(400, "send a command as the body"); // DEBUG-TEXT
                }
                return text(200, game.act(cmd));
            }
            if (path.equals("/read") && method.equals("GET")) {
                return text(200, game.read(count(param(query, "n"), 12)));
            }
            return text(404, "use POST /mcp, POST /act or GET /read"); // DEBUG-TEXT
        } catch (JSONException e) {
            return json(200, error(null, -32700, "parse error"));
        }
    }

    private boolean authorised(Map<String, String> headers, String query) {
        String given = null;
        String auth = headers.get("authorization");
        if (auth != null && auth.regionMatches(true, 0, "Bearer ", 0, 7)) {
            given = auth.substring(7).trim();
        }
        if (given == null) {
            given = param(query, "key");
        }
        return given != null
                && MessageDigest.isEqual(
                        given.getBytes(StandardCharsets.UTF_8), key.getBytes(StandardCharsets.UTF_8));
    }

    private static String param(String query, String name) {
        for (String part : query.split("&")) {
            int eq = part.indexOf('=');
            if (eq > 0 && part.substring(0, eq).equals(name)) {
                try {
                    return java.net.URLDecoder.decode(part.substring(eq + 1), "UTF-8");
                } catch (java.io.UnsupportedEncodingException e) {
                    return null;
                }
            }
        }
        return null;
    }

    private static int count(String s, int fallback) {
        try {
            return Math.max(1, Math.min(100, Integer.parseInt(s)));
        } catch (RuntimeException e) {
            return fallback;
        }
    }

    /** The command in a body: plain text, or {"command": "..."}. */
    private static String command(String body) throws JSONException {
        String b = body == null ? "" : body.trim();
        if (b.startsWith("{")) {
            b = new JSONObject(b).optString("command", "");
        }
        b = b.replace('\n', ' ').replace('\r', ' ').trim();
        return b.length() > MAX_COMMAND ? b.substring(0, MAX_COMMAND) : b;
    }

    private Response mcp(String body) throws JSONException {
        JSONObject msg = new JSONObject(body == null ? "" : body);
        Object id = msg.opt("id");
        String method = msg.optString("method", "");
        if (id == null || id == JSONObject.NULL) {
            // A notification (e.g. notifications/initialized): no reply.
            return new Response(202, "text/plain", "");
        }
        JSONObject params = msg.optJSONObject("params");
        if (params == null) {
            params = new JSONObject();
        }
        switch (method) {
            case "initialize": {
                JSONObject r = new JSONObject();
                r.put("protocolVersion", params.optString("protocolVersion", MCP_VERSION));
                r.put("capabilities", new JSONObject().put("tools", new JSONObject()));
                r.put("serverInfo", new JSONObject().put("name", "scraped-again-phone").put("version", "1"));
                // DEBUG-TEXT: instructions for agents, not player text.
                r.put("instructions", "You are playing Scraped Again, a text game about deciphering a lost language, on someone's phone. "
                        + "Use read to see the latest text and act to type a command, exactly as a player would (look, read stele, go temple, help). "
                        + "You see only what the player sees. Never claim to know what writing means without evidence.");
                return json(200, result(id, r));
            }
            case "ping":
                return json(200, result(id, new JSONObject()));
            case "tools/list":
                return json(200, result(id, new JSONObject().put("tools", tools())));
            case "tools/call": {
                String name = params.optString("name", "");
                JSONObject args = params.optJSONObject("arguments");
                if (args == null) {
                    args = new JSONObject();
                }
                String out;
                if (name.equals("act")) {
                    String cmd = command(new JSONObject().put("command", args.optString("command", "")).toString());
                    if (cmd.isEmpty()) {
                        return json(200, error(id, -32602, "act needs a command"));
                    }
                    out = game.act(cmd);
                } else if (name.equals("read")) {
                    out = game.read(count(args.optString("entries", ""), 12));
                } else {
                    return json(200, error(id, -32602, "no tool " + name));
                }
                JSONObject content = new JSONObject().put("type", "text").put("text", out == null ? "" : out);
                return json(200, result(id, new JSONObject().put("content", new JSONArray().put(content))));
            }
            default:
                return json(200, error(id, -32601, "unknown method " + method));
        }
    }

    // DEBUG-TEXT: tool descriptions are for agents.
    private static JSONArray tools() throws JSONException {
        JSONObject act = new JSONObject()
                .put("name", "act")
                .put("description", "Type one command into the game, as a player would (look, read stele, go temple, take scraper, help). Returns the game's reply.")
                .put("inputSchema", new JSONObject()
                        .put("type", "object")
                        .put("properties", new JSONObject().put("command", new JSONObject().put("type", "string")))
                        .put("required", new JSONArray().put("command")));
        JSONObject read = new JSONObject()
                .put("name", "read")
                .put("description", "The latest exchanges of the game on screen: the game's text, and commands as lines starting with '> '.")
                .put("inputSchema", new JSONObject()
                        .put("type", "object")
                        .put("properties", new JSONObject().put("entries", new JSONObject().put("type", "integer").put("minimum", 1).put("maximum", 100))));
        return new JSONArray().put(act).put(read);
    }

    private static JSONObject result(Object id, JSONObject r) throws JSONException {
        return new JSONObject().put("jsonrpc", "2.0").put("id", id).put("result", r);
    }

    private static JSONObject error(Object id, int code, String message) {
        try {
            return new JSONObject().put("jsonrpc", "2.0").put("id", id == null ? JSONObject.NULL : id)
                    .put("error", new JSONObject().put("code", code).put("message", message));
        } catch (JSONException e) {
            throw new IllegalStateException(e);
        }
    }

    private static Response text(int status, String body) {
        return new Response(status, "text/plain; charset=utf-8", body == null ? "" : body);
    }

    private static Response json(int status, JSONObject o) {
        return new Response(status, "application/json", o.toString());
    }
}
