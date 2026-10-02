package org.scrapedagain;

import java.io.BufferedInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.Inet4Address;
import java.net.InetAddress;
import java.net.NetworkInterface;
import java.net.ServerSocket;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.util.Collections;
import java.util.HashMap;
import java.util.Map;

/**
 * A tiny HTTP server on the local network, so an agent can play: it hands
 * each request to {@link AgentProtocol}. One request per connection; small
 * bodies only.
 */
public final class AgentServer {
    /** Ports tried, in order. */
    static final int FIRST_PORT = 8765;
    private static final int MAX_BODY = 64 * 1024;

    private final AgentProtocol protocol;
    private ServerSocket socket;
    private Thread thread;

    public AgentServer(AgentProtocol protocol) {
        this.protocol = protocol;
    }

    /** Starts listening; returns the port, or -1. */
    public synchronized int start() {
        if (socket != null) {
            return socket.getLocalPort();
        }
        for (int p = FIRST_PORT; p < FIRST_PORT + 10; p++) {
            try {
                socket = new ServerSocket(p);
                break;
            } catch (IOException e) {
                socket = null;
            }
        }
        if (socket == null) {
            return -1;
        }
        final ServerSocket s = socket;
        thread = new Thread(new Runnable() {
            @Override
            public void run() {
                while (!s.isClosed()) {
                    try {
                        final Socket c = s.accept();
                        Thread t = new Thread(new Runnable() {
                            @Override
                            public void run() {
                                serve(c);
                            }
                        }, "agent-request");
                        t.setDaemon(true);
                        t.start();
                    } catch (IOException e) {
                        // Closed, or a failed accept: carry on until closed.
                    }
                }
            }
        }, "agent-server");
        thread.setDaemon(true);
        thread.start();
        return s.getLocalPort();
    }

    public synchronized void stop() {
        if (socket != null) {
            try {
                socket.close();
            } catch (IOException ignored) {
                // Already closed.
            }
            socket = null;
        }
    }

    public synchronized boolean running() {
        return socket != null;
    }

    public synchronized int port() {
        return socket == null ? -1 : socket.getLocalPort();
    }

    /** This device's address on the local network, if it has one. */
    public static String lanAddress() {
        try {
            for (NetworkInterface n : Collections.list(NetworkInterface.getNetworkInterfaces())) {
                if (!n.isUp() || n.isLoopback()) {
                    continue;
                }
                for (InetAddress a : Collections.list(n.getInetAddresses())) {
                    if (a instanceof Inet4Address && a.isSiteLocalAddress()) {
                        return a.getHostAddress();
                    }
                }
            }
        } catch (IOException | RuntimeException ignored) {
            // No network.
        }
        return null;
    }

    private void serve(Socket c) {
        try {
            c.setSoTimeout(30000);
            InputStream in = new BufferedInputStream(c.getInputStream());
            String requestLine = line(in);
            if (requestLine == null) {
                return;
            }
            String[] parts = requestLine.split(" ");
            if (parts.length < 2) {
                return;
            }
            Map<String, String> headers = new HashMap<>();
            String h;
            while ((h = line(in)) != null && !h.isEmpty()) {
                int colon = h.indexOf(':');
                if (colon > 0) {
                    headers.put(h.substring(0, colon).trim().toLowerCase(java.util.Locale.ROOT), h.substring(colon + 1).trim());
                }
            }
            int length = 0;
            try {
                length = Integer.parseInt(headers.containsKey("content-length") ? headers.get("content-length") : "0");
            } catch (NumberFormatException ignored) {
                length = 0;
            }
            AgentProtocol.Response r;
            if (length < 0 || length > MAX_BODY) {
                r = new AgentProtocol.Response(413, "text/plain", "too large"); // DEBUG-TEXT
            } else {
                byte[] body = new byte[length];
                int read = 0;
                while (read < length) {
                    int n = in.read(body, read, length - read);
                    if (n < 0) {
                        break;
                    }
                    read += n;
                }
                r = protocol.handle(parts[0], parts[1], headers, new String(body, 0, read, StandardCharsets.UTF_8));
            }
            byte[] out = r.body.getBytes(StandardCharsets.UTF_8);
            String head = "HTTP/1.1 " + r.status + " " + reason(r.status) + "\r\n"
                    + "Content-Type: " + r.type + "\r\n"
                    + "Content-Length: " + out.length + "\r\n"
                    + "Cache-Control: no-store\r\n"
                    + "Connection: close\r\n\r\n";
            OutputStream o = c.getOutputStream();
            o.write(head.getBytes(StandardCharsets.UTF_8));
            o.write(out);
            o.flush();
        } catch (IOException ignored) {
            // The agent went away.
        } finally {
            try {
                c.close();
            } catch (IOException ignored) {
                // Already closed.
            }
        }
    }

    private static String line(InputStream in) throws IOException {
        ByteArrayOutputStream b = new ByteArrayOutputStream();
        int c;
        while ((c = in.read()) >= 0) {
            if (c == '\n') {
                break;
            }
            if (c != '\r') {
                b.write(c);
            }
            if (b.size() > 8192) {
                return null;
            }
        }
        if (c < 0 && b.size() == 0) {
            return null;
        }
        return new String(b.toByteArray(), StandardCharsets.UTF_8);
    }

    private static String reason(int status) {
        switch (status) {
            case 200: return "OK";
            case 202: return "Accepted";
            case 400: return "Bad Request";
            case 401: return "Unauthorized";
            case 404: return "Not Found";
            case 405: return "Method Not Allowed";
            case 413: return "Payload Too Large";
            default: return "Error";
        }
    }
}
