//! C01: the app (through the engine calls it makes) and an agent (through
//! the player program's shared-world code) take turns on one world through
//! a shared folder standing in for the worlds repo.

use std::path::Path;

use scraped_android::call;
use scraped_play::world::Shared;
use serde_json::{json, Value};

fn ask(req: Value) -> Value {
    let r: Value = serde_json::from_str(&call(&req.to_string())).unwrap();
    assert!(r.get("error").is_none(), "{req}: {r}");
    r
}

fn pack() -> scraped_content::Pack {
    scraped_play::load_pack(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap()
}

fn read(p: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

#[test]
fn app_and_agent_take_turns_through_a_shared_repo() {
    let repo = std::env::temp_dir().join(format!("scraped-sync-{}", std::process::id()));
    std::fs::create_dir_all(&repo).unwrap();
    std::env::set_var("SCRAPED_HOME", repo.join(".agent"));
    let path = repo.join("w.world");

    // The app starts a world, shares it, moves, and pushes.
    ask(json!({ "cmd": "play_new", "seed": "42" }));
    let file = ask(json!({ "cmd": "world_share", "id": "w" }))["file"].clone();
    let mut ours =
        ask(json!({ "cmd": "world_play", "file": file, "line": "look", "who": "jb", "at": 1 }))
            ["file"]
            .clone();
    std::fs::write(&path, ours.to_string()).unwrap();

    // The agent pulls, reads the talk, plays, talks and pushes.
    let (mut ai, _) = Shared::open(&path, "ai", pack()).unwrap();
    ai.play("listen").unwrap();
    ai.talk("I listened at the door.").unwrap();

    // The app pulls: theirs is newer; it carries on from it.
    let m = ask(json!({ "cmd": "world_merge", "ours": ours, "theirs": read(&path) }));
    assert_eq!(m["merge"]["newest"], "theirs");
    ours = m["file"].clone();
    assert_eq!(ours["moves"].as_array().unwrap().len(), 2);
    assert_eq!(ours["talk"][0]["who"], "ai");
    ask(json!({ "cmd": "world_open", "file": ours }));
    ours = ask(json!({ "cmd": "world_play", "file": ours, "line": "wait", "who": "jb", "at": 3 }))
        ["file"]
        .clone();
    std::fs::write(&path, ours.to_string()).unwrap();

    // The agent sees the app's move.
    let (mut ai, _) = Shared::open(&path, "ai", pack()).unwrap();
    assert_eq!(ai.file.last_mover(), Some("jb"));

    // Both play on from the same point: the app finds a split, keeps its
    // own line and gets the agent's as a branch; both open.
    ai.play("drink").unwrap();
    let mine =
        ask(json!({ "cmd": "world_play", "file": ours, "line": "sleep", "who": "jb", "at": 4 }))
            ["file"]
            .clone();
    let m = ask(json!({ "cmd": "world_merge", "ours": mine, "theirs": read(&path) }));
    assert_eq!(m["merge"]["kind"], "split");
    assert_eq!(m["branch"]["branch"], "ai-3");
    ask(json!({ "cmd": "world_open", "file": m["file"] }));
    ask(json!({ "cmd": "world_open", "file": m["branch"] }));
    let _ = std::fs::remove_dir_all(&repo);
}
