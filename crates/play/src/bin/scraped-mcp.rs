//! `scraped-mcp`: the game as an MCP server over stdio, so an agent can
//! play. Add it to an MCP client as a stdio server:
//!
//! ```text
//! scraped-mcp --content path/to/content
//! ```

use std::io::{BufRead, Write};
use std::path::PathBuf;

use scraped_play::{load_pack, mcp::Server};

fn main() {
    let mut content = PathBuf::from("content");
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--content" {
            if let Some(d) = args.next() {
                content = PathBuf::from(d);
            }
        }
    }
    let pack = match load_pack(&content) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}"); // DEBUG-TEXT
            std::process::exit(1);
        }
    };
    let mut server = Server::new(pack);
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<serde_json::Value>(&line) {
            Ok(msg) => server.handle(&msg),
            Err(e) => Some(serde_json::json!({
                "jsonrpc": "2.0", "id": null,
                "error": { "code": -32700, "message": format!("parse error: {e}") }
            })),
        };
        if let Some(r) = reply {
            let _ = writeln!(out, "{r}");
            let _ = out.flush();
        }
    }
}
