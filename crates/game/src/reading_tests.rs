//! Reading in layers (S01): a glance, a close reading by impressions (or
//! sounds once heard), one sign examined, and tracing as a task.

use std::collections::BTreeSet;

use crate::composing_tests::pack;
use crate::site::Place;
use crate::{Game, Target};

/// A game standing by a long piece of writing in a building, in good light.
fn by_writing(seed: u64) -> (Game, usize) {
    let mut g = Game::new(seed, pack());
    g.trace = true;
    g.forced_light = true;
    g.forced = Some(("clear", "day"));
    g.start();
    let thing = (0..g.site.things.len())
        .filter(|&t| matches!(g.site.things[t].home, Place::Room { .. }))
        .find(|&t| g.signs(t).len() >= 24 && g.layers_seen(t).iter().all(|l| !l.1))
        .expect("a long text indoors");
    g.state.place = g.site.things[thing].home;
    g.state.pos = g.site.things[thing].pos;
    (g, thing)
}

fn slots(o: &crate::Output) -> Vec<&str> {
    o.renders.iter().map(|r| r.trace.slot.as_str()).collect()
}

#[test]
fn reading_is_a_glance_and_reading_closely_goes_sign_by_sign() {
    for seed in [1, 42] {
        let (mut g, thing) = by_writing(seed);
        let o = g.act("read", Target::Thing(thing));
        let s = slots(&o);
        assert!(s.contains(&"read.whole"), "{s:?}");
        assert!(
            !s.contains(&"read.glyph"),
            "a glance lists no signs: {}",
            o.text
        );
        assert!(!s.contains(&"glyph.describe"), "{s:?}");
        let o = g.step("read closely");
        let s = slots(&o);
        assert!(s.contains(&"read.glyph"), "{}", o.text);
        assert!(s.contains(&"glyph.impression"), "{s:?}");
        assert!(!s.contains(&"glyph.describe"), "no exact strokes: {s:?}");
        // Looking closer while reading reads on.
        let o = g.step("look closer");
        assert!(slots(&o).contains(&"read.frame"), "{}", o.text);
    }
}

#[test]
fn impressions_are_stable_and_tell_signs_apart() {
    let (mut g, _) = by_writing(42);
    for era in 0..g.site.world.languages.len() as u32 {
        let n = g.site.world.languages[era as usize].script.glyphs.len();
        let texts: Vec<String> = (0..n).map(|i| g.sign_impression(era, i)).collect();
        let distinct: BTreeSet<&String> = texts.iter().collect();
        assert_eq!(distinct.len(), n, "era {era}: {texts:?}");
        assert_eq!(texts[3], g.sign_impression(era, 3));
    }
}

#[test]
fn one_sign_examined_gives_a_fuller_impression() {
    let (mut g, thing) = by_writing(42);
    g.act("read", Target::Thing(thing));
    let o = g.step("examine the fourth sign");
    assert!(slots(&o).contains(&"glyph.closer"), "{}", o.text);
    let o = g.step("look at sign 2");
    assert!(slots(&o).contains(&"glyph.closer"), "{}", o.text);
}

#[test]
fn tracing_takes_time_and_light_and_gives_the_strokes() {
    let (mut g, thing) = by_writing(42);
    g.act("read", Target::Thing(thing));
    let before = g.state.minutes;
    let o = g.step("trace the fourth sign");
    assert!(slots(&o).contains(&"trace.sign"), "{}", o.text);
    assert!(slots(&o).contains(&"glyph.describe"), "{:?}", slots(&o));
    assert!(g.state.minutes >= before + crate::reading::TRACE_MINUTES);
    // A whole text, a few signs at a go, carrying on.
    let name = g.site.things[thing].kind;
    let o = g.step(&format!("trace the {name}"));
    assert!(slots(&o).contains(&"trace.more"), "{}", o.text);
    let first = g.state.tracing;
    g.step(&format!("trace the {name}"));
    assert_ne!(g.state.tracing, first);
    // In the dark, no tracing.
    g.forced_light = false;
    g.forced = Some(("clear", "night"));
    let dark = g.is_dark();
    if dark {
        let o = g.step("trace sign 2");
        assert!(!slots(&o).contains(&"trace.sign"), "{}", o.text);
    }
}

#[test]
fn scraping_where_it_is_quiet_lets_signs_be_heard() {
    let (mut g, thing) = by_writing(42);
    g.make_item("scraper", true);
    // Listening first, so the place's sounds don't drown them.
    g.step("listen");
    let o = g.act("scrape", Target::Thing(thing));
    if g.state.dead.is_some() {
        return;
    }
    assert!(slots(&o).contains(&"glyph.heard"), "{}", o.text);
    assert!(!g.state.heard.is_empty());
    // A heard sign now reads by its sound.
    let (era, index) = *g.state.heard.iter().next().unwrap();
    assert!(g.sign_sound(era, index).is_some());
}

#[test]
fn unheard_sounds_cannot_be_written() {
    let Some((mut g, thing)) = crate::composing_tests::at_blank_surface(42) else {
        return;
    };
    g.trace = true;
    let words = g.sound_words(&crate::composing_tests::claim("burn", "house", false));
    let name = g.site.things[thing].kind;
    let o = g.step(&format!("write {words} on {name}"));
    assert!(slots(&o).contains(&"write.unheard"), "{}", o.text);
}

#[test]
fn old_saves_with_labels_still_load() {
    let json = serde_json::to_value(Game::new(5, pack()).state).unwrap();
    let mut old = json.clone();
    old.as_object_mut()
        .unwrap()
        .insert("labels".into(), serde_json::json!({"0:3": "ka"}));
    old.as_object_mut().unwrap().remove("heard");
    old.as_object_mut().unwrap().remove("tracing");
    let state: crate::State = serde_json::from_value(old).expect("an old state loads");
    assert!(state.heard.is_empty());
    // And an old command is answered, not a crash.
    let mut g = Game::new(5, pack());
    g.start();
    g.step("define 3 as ka");
}
