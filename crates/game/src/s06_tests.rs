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

/// S06: tracing matches the impression. A mark the impression calls a
/// round, closed thing (an egg, an eye, a wheel) traces with something
/// round (a ring or an arc), on every script of seeds 1, 42 and 9001.
#[test]
fn traced_form_matches_the_impression() {
    use scraped_lang::script::Stroke;
    // Round things: a knot, shield or key may close with crossing bars.
    const CLOSED: [&str; 3] = ["eye", "wheel", "egg"];
    for seed in [1, 42, 9001] {
        let g = Game::new(seed, pack());
        for lang in &g.site.world.languages {
            let imps = scraped_lang::impression::impressions(&lang.script, false);
            for (i, (_, glyph)) in lang.script.glyphs.iter().enumerate() {
                let res = imps[i].shape.resembles;
                if CLOSED.contains(&res) {
                    let closes = glyph
                        .marks
                        .iter()
                        .any(|m| matches!(m.stroke, Stroke::Ring | Stroke::Arc));
                    assert!(
                        closes,
                        "seed {seed}: a {res} that traces as {}",
                        glyph.describe()
                    );
                }
            }
        }
    }
}
