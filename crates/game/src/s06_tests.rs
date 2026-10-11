//! S06: what the game shows. The guide lists what the player has found,
//! the first hour says nothing of writing, cues can be followed up.

use crate::composing_tests::pack;
use crate::Game;

/// `help` lists the basics and the verbs the player has used, never a
/// verb they haven't found; every verb is understood all the same.
#[test]
fn help_lists_the_basics_and_what_was_used() {
    let mut g = Game::new(42, pack());
    g.start();
    let first = g.step("help").text;
    for hidden in [
        "clean", "read", "trace", "scrape", "write", "fish", "manual",
    ] {
        assert!(
            !first.contains(hidden),
            "help names {hidden} before use: {first}"
        );
    }
    assert!(
        first.contains("look") && first.contains("go back"),
        "{first}"
    );
    g.step("go in");
    g.step("listen");
    g.step("look closer");
    let later = g.step("help").text;
    assert!(later.contains("look closer"), "{later}");
    // Not followed, not added.
    g.step("frobnicate the moon");
    assert!(!g.step("help").text.contains("frobnicate"));
}
