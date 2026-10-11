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

/// S06: a cue can always be dug into. Replaying the explorer's and the
/// hand player's runs, each time a cue names something, examining,
/// touching, looking closer at, listening at and smelling it each get a
/// response about that thing.
#[test]
#[ignore = "slow in debug; CI runs it in release"]
fn every_cue_can_be_followed_up() {
    let p = pack();
    let mut followed = 0;
    for seed in [1, 42, 9001] {
        for kind in ["explorer", "hand"] {
            let run = crate::bots::play(&p, seed, kind, 12.0, 20_000);
            let mut g = Game::new(seed, p.clone());
            g.keep_renders = true;
            g.start();
            let mut checked = 0;
            for cmd in &run.commands {
                let mark = g.renders().len();
                g.step(cmd);
                let things: Vec<String> = g.renders()[mark..]
                    .iter()
                    .filter(|r| r.trace.slot == "spell.cue")
                    .filter_map(|r| r.vars.get("thing").map(|v| v.text()))
                    .collect();
                for thing in things.into_iter().take(1) {
                    if checked >= 4 || g.state.dead.is_some() {
                        break;
                    }
                    checked += 1;
                    let save = g.save();
                    let word = thing.rsplit(' ').next().unwrap_or(&thing).to_string();
                    for follow in [
                        format!("examine {word}"),
                        format!("touch {word}"),
                        format!("look closer at {word}"),
                        format!("listen at {word}"),
                        format!("smell {word}"),
                    ] {
                        let (mut f, _) = Game::open(&save, p.clone()).expect("reopens");
                        f.keep_renders = true;
                        let m = f.renders().len();
                        let o = f.step(&follow);
                        let about = f.renders()[m..]
                            .iter()
                            .any(|r| r.trace.slot.starts_with("cue."));
                        assert!(
                            about,
                            "seed {seed} {kind}: `{follow}` after a cue on {thing}: {}",
                            o.text
                        );
                        followed += 1;
                    }
                }
            }
        }
    }
    assert!(followed > 0, "no cue was met to follow up");
}
