//! An MCP server for the game (M14): an agent such as Claude plays through
//! tools (`new_game`, `act`, `save`, `load`, `seed_code`), alongside a human
//! or alone. Messages are JSON-RPC 2.0, one per line on stdin and stdout.
//!
//! The tool results carry the same text and state as the JSON-lines
//! protocol (docs/PROTOCOL.md).

use serde_json::{json, Value};

use scraped_content::Pack;
use scraped_game::{Game, Save};

use crate::{protocol_line, PROTOCOL};

/// The MCP protocol revision this server speaks.
pub const MCP_VERSION: &str = "2025-06-18";

/// A running server: the pack, and the game if one has begun.
pub struct Server {
    pack: Pack,
    game: Option<Game>,
}

impl Server {
    pub fn new(pack: Pack) -> Self {
        Server { pack, game: None }
    }

    /// Handles one message; `None` for notifications, which get no reply.
    pub fn handle(&mut self, msg: &Value) -> Option<Value> {
        let id = msg.get("id").cloned();
        let method = msg["method"].as_str().unwrap_or("");
        let id = id?;
        let result = match method {
            "initialize" => Ok(json!({
                "protocolVersion": MCP_VERSION,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "scraped-again", "version": env!("CARGO_PKG_VERSION") },
                // DEBUG-TEXT: instructions for agents, not player text.
                "instructions": "Scraped Again: a text game about deciphering a lost language. Start with new_game, then send commands with act (look, read stele, go temple, help). Text in the game is what the player sees; never invent what writing means.",
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tools() })),
            "tools/call" => self.call(&msg["params"]),
            _ => Err((-32601, format!("unknown method {method}"))), // DEBUG-TEXT
        };
        Some(match result {
            Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
            Err((code, message)) => {
                json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
            }
        })
    }

    fn call(&mut self, params: &Value) -> Result<Value, (i64, String)> {
        let args = &params["arguments"];
        let text = |t: String, structured: Value| json!({ "content": [{ "type": "text", "text": t }], "structuredContent": structured });
        let output = |o: scraped_game::Output| {
            let line: Value = serde_json::from_str(&protocol_line(&o)).expect("protocol JSON");
            text(o.text.clone(), line)
        };
        // DEBUG-TEXT: tool errors are for agents.
        let no_game = || (-32000, "no game yet: call new_game first".to_string());
        match params["name"].as_str().unwrap_or("") {
            "new_game" => {
                let preset = args["difficulty"].as_str().unwrap_or("standard");
                let (seed, preset) = match args["code"].as_str() {
                    Some(c) => {
                        let sc = scraped_game::seedcode::decode(c)
                            .ok_or((-32602, "that seed code doesn't read".to_string()))?;
                        (sc.seed, sc.preset)
                    }
                    None => {
                        let seed = args["seed"].as_u64().unwrap_or(1);
                        (
                            scraped_game::fairness::fair_seed(seed, preset),
                            preset.to_string(),
                        )
                    }
                };
                let mut g = Game::create(seed, self.pack.clone(), &preset, None);
                let o = g.start();
                self.game = Some(g);
                Ok(output(o))
            }
            "act" => {
                let cmd = args["command"]
                    .as_str()
                    .ok_or((-32602, "act needs a command".to_string()))?;
                let g = self.game.as_mut().ok_or_else(no_game)?;
                Ok(output(g.step(cmd)))
            }
            "save" => {
                let g = self.game.as_ref().ok_or_else(no_game)?;
                let save = g.save();
                Ok(text(
                    serde_json::to_string(&save).expect("save"),
                    json!(save),
                ))
            }
            "load" => {
                let save: Save = serde_json::from_value(args["save"].clone())
                    .map_err(|e| (-32602, format!("not a save: {e}")))?;
                let (mut g, _) = Game::load(&save, self.pack.clone());
                let o = g.step("look");
                self.game = Some(g);
                Ok(output(o))
            }
            "seed_code" => {
                let g = self.game.as_mut().ok_or_else(no_game)?;
                let code = g.seed_code();
                Ok(text(code.clone(), json!({ "code": code })))
            }
            other => Err((-32602, format!("no tool {other}"))),
        }
    }
}

/// The tools an agent sees.
// DEBUG-TEXT: tool descriptions are for agents.
fn tools() -> Value {
    json!([
        {
            "name": "new_game",
            "description": "Start a new game. Give a seed (a number) or a seed code another player shared, and optionally a difficulty: gentle, standard or archaeologist.",
            "inputSchema": { "type": "object", "properties": {
                "seed": { "type": "integer", "minimum": 0 },
                "code": { "type": "string" },
                "difficulty": { "type": "string", "enum": ["gentle", "standard", "archaeologist"] }
            } }
        },
        {
            "name": "act",
            "description": "Send one command, exactly as a player would type it (look, read stele, go temple, take scraper, write 4 12 / 3 on wall, help). Returns what the player sees and a state summary.",
            "inputSchema": { "type": "object", "properties": { "command": { "type": "string" } }, "required": ["command"] }
        },
        {
            "name": "save",
            "description": "The current game as a save (seed, difficulty and commands), to load later.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "load",
            "description": "Load a save returned by the save tool.",
            "inputSchema": { "type": "object", "properties": { "save": { "type": "object" } }, "required": ["save"] }
        },
        {
            "name": "seed_code",
            "description": "This world's shareable code, so a human can play the same world.",
            "inputSchema": { "type": "object", "properties": {} }
        }
    ])
}

/// The protocol version agents see in every game response.
pub fn protocol() -> u32 {
    PROTOCOL
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn server() -> Server {
        let pack = crate::load_pack(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"))
            .expect("content");
        Server::new(pack)
    }

    fn call(s: &mut Server, id: u64, method: &str, params: Value) -> Value {
        s.handle(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))
            .expect("a reply")
    }

    #[test]
    fn a_session_over_mcp() {
        let mut s = server();
        let init = call(
            &mut s,
            1,
            "initialize",
            json!({ "protocolVersion": MCP_VERSION, "capabilities": {}, "clientInfo": { "name": "test", "version": "0" } }),
        );
        assert_eq!(init["result"]["protocolVersion"], MCP_VERSION);
        assert!(s
            .handle(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
            .is_none());
        let tools = call(&mut s, 2, "tools/list", json!({}));
        let names: Vec<&str> = tools["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["new_game", "act", "save", "load", "seed_code"]);
        let early = call(
            &mut s,
            3,
            "tools/call",
            json!({ "name": "act", "arguments": { "command": "look" } }),
        );
        assert!(early["error"]["message"]
            .as_str()
            .unwrap()
            .contains("new_game"));
        let start = call(
            &mut s,
            4,
            "tools/call",
            json!({ "name": "new_game", "arguments": { "seed": 42 } }),
        );
        assert_eq!(start["result"]["structuredContent"]["protocol"], protocol());
        let look = call(
            &mut s,
            5,
            "tools/call",
            json!({ "name": "act", "arguments": { "command": "look" } }),
        );
        let text = look["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(!text.is_empty());
        let save = call(
            &mut s,
            6,
            "tools/call",
            json!({ "name": "save", "arguments": {} }),
        );
        let code = call(
            &mut s,
            7,
            "tools/call",
            json!({ "name": "seed_code", "arguments": {} }),
        );
        let code = code["result"]["structuredContent"]["code"]
            .as_str()
            .unwrap()
            .to_string();
        // The same world from its code, and from the save.
        let again = call(
            &mut s,
            8,
            "tools/call",
            json!({ "name": "new_game", "arguments": { "code": code } }),
        );
        assert_eq!(
            again["result"]["content"][0]["text"],
            start["result"]["content"][0]["text"]
        );
        let loaded = call(
            &mut s,
            9,
            "tools/call",
            json!({ "name": "load", "arguments": { "save": save["result"]["structuredContent"] } }),
        );
        assert!(loaded["result"]["content"][0]["text"].is_string());
        assert_eq!(
            call(&mut s, 10, "nonsense", json!({}))["error"]["code"],
            -32601
        );
    }
}
