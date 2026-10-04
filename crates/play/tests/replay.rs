//! C01: a world played across three builds replays stretch by stretch, each
//! on its own build, to the same final state. The two older "builds" are
//! this package's player program standing in for older releases.

use std::path::PathBuf;

use scraped_game::saves::{seal, Stretch, ENGINE};
use scraped_game::Game;
use scraped_play::baked_pack;
use scraped_play::replay::replay;

#[test]
fn a_world_played_on_three_builds_replays_to_its_saved_state() {
    let mut g = Game::create(42, baked_pack(), "standard", None);
    g.start();
    let moves = [
        "look",
        "take all",
        "go in",
        "look closer",
        "out",
        "listen",
        "wait",
        "head north",
        "look",
    ];
    for m in moves {
        g.step(m);
    }
    let mut save = g.save();
    let stretch = |engine: &str, from, to| Stretch {
        engine: engine.into(),
        pack: save.pack_version.clone(),
        from,
        to,
    };
    save.history = vec![
        stretch("0.0.1", 0, 3),
        stretch("0.0.2", 3, 6),
        stretch(ENGINE, 6, 9),
    ];
    let text = seal(&serde_json::to_string(&save).unwrap());
    let player = PathBuf::from(env!("CARGO_BIN_EXE_scraped-player"));
    let builds = vec![
        ("0.0.1".to_string(), player.clone()),
        ("0.0.2".to_string(), player),
    ];
    let report = replay(&text, &builds, baked_pack()).unwrap();
    assert!(report.contains("moves 3–6 on 0.0.2"), "{report}");
    // A missing build is named.
    let err = replay(&text, &builds[..1], baked_pack()).unwrap_err();
    assert!(err.contains("0.0.2"), "{err}");
}
