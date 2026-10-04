//! D03 in play: walking a town's districts, finding features and scenes
//! by looking closer, and storylets placed at scenes.

use scraped_content::{PackFile, Value};
use scraped_world::scenes::SceneAt;

use crate::composing_tests::pack;
use crate::site::Place;
use crate::storylets::{candidates, scene_room};
use crate::{Game, Target};

const SEEDS: [u64; 4] = [1, 42, 7, 9001];

fn slots(o: &crate::Output) -> Vec<String> {
    o.renders.iter().map(|r| r.trace.slot.clone()).collect()
}

#[test]
fn every_district_of_the_starting_town_can_be_walked_to() {
    for seed in SEEDS {
        let mut g = Game::new(seed, pack());
        g.trace = true;
        g.start();
        let (t, _) = g.district_here().expect("play starts in a town");
        let n = g.site.world.towns[t].districts.len();
        for d in (0..n).rev() {
            let o = g.act("go", Target::District(d));
            assert_eq!(g.district_here(), Some((t, d)), "seed {seed}: district {d}");
            assert!(!o.text.is_empty());
        }
        // Looking around in town tells its layout.
        let o = g.step("look around");
        assert!(
            slots(&o).iter().any(|s| s == "place.layout"),
            "seed {seed}: {}",
            o.text
        );
    }
}

#[test]
fn the_parser_knows_districts_by_name() {
    for seed in SEEDS {
        let mut g = Game::new(seed, pack());
        g.start();
        let (t, _) = g.district_here().unwrap();
        let kind = g.site.world.towns[t].districts.last().unwrap().kind;
        let d = g.site.world.towns[t].districts.len() - 1;
        g.step(&format!("go to the {kind}"));
        assert_eq!(
            g.district_here().map(|x| x.1),
            Some(d),
            "seed {seed}: go to the {kind}"
        );
    }
}

#[test]
fn features_close_by_are_noticed_and_can_be_examined() {
    let mut found = 0;
    for seed in SEEDS {
        let mut g = Game::new(seed, pack());
        g.trace = true;
        g.start();
        // Stand by each of the first few features that can be stood on.
        let spots: Vec<(usize, scraped_sim::outdoors::Pos)> = g
            .site
            .world
            .features
            .iter()
            .map(|f| {
                (
                    f.id,
                    scraped_sim::outdoors::Pos::of_cell(f.cell.ux(), f.cell.uy()),
                )
            })
            .filter(|(_, p)| g.site.land.passable(&g.site.world, p.cell().0, p.cell().1))
            .take(5)
            .collect();
        for (f, p) in spots {
            g.state.pos = p;
            g.state.place = Place::Outside;
            let o = g.step("look closer");
            assert!(
                o.renders.iter().any(|r| r.trace.slot == "land.feature"),
                "seed {seed}: feature {f} not noticed: {}",
                o.text
            );
            let o = g.act("examine", Target::Feature(f));
            assert!(slots(&o).iter().any(|s| s == "feature.closer"));
            found += 1;
        }
    }
    assert!(found >= 12);
}

#[test]
fn scenes_are_found_by_looking_closer() {
    for seed in SEEDS {
        let mut g = Game::new(seed, pack());
        g.trace = true;
        g.start();
        let mut seen = 0;
        let scenes: Vec<SceneAt> = g.site.world.scenes.iter().map(|s| s.at).take(30).collect();
        for at in scenes {
            match at {
                SceneAt::Room { structure, room } => {
                    if g.site.world.structures[structure].interior.rooms[room].collapsed {
                        continue;
                    }
                    g.state.place = Place::Room { structure, room };
                    g.state.pos = g.site.land.structure_pos[structure];
                }
                SceneAt::Outside { x, y } => {
                    g.state.place = Place::Outside;
                    g.state.pos =
                        scraped_sim::outdoors::Pos::of_cell(usize::from(x), usize::from(y));
                }
            }
            // Light enough to see (a lit lamp would do as well).
            g.forced = Some(("clear", "daylight"));
            let o = g.step("look closer");
            if g.is_dark() {
                continue;
            }
            assert!(
                slots(&o).iter().any(|s| s == "place.scene"),
                "seed {seed}: scene at {at:?} not found: {}",
                o.text
            );
            seen += 1;
        }
        assert!(seen >= 5, "seed {seed}: {seen}");
    }
}

#[test]
fn storylets_can_be_placed_at_scenes() {
    const SRC: &str = r#"
[[storylet]]
id = "last_meal"
about = "x"
at = "structure"

[storylet.place]
scene = ["meal left", "barricade", "bones with belongings"]
"#;
    let s = PackFile::parse("x.toml", SRC).unwrap().storylets.remove(0);
    let mut placed = 0;
    for seed in SEEDS {
        let mut p = pack();
        p.files.push(PackFile::parse("extra.toml", SRC).unwrap());
        let g = Game::new(seed, p);
        let w = &g.site.world;
        for c in candidates(w, g.site.settlement, &s) {
            assert!(scene_room(w, c, &s.place.scene).is_some());
        }
        if let Some(pl) = g.placed.iter().find(|pl| pl.id == "last_meal") {
            assert!(scene_room(w, pl.structure, &s.place.scene).is_some());
            placed += 1;
        }
    }
    assert!(placed >= 3, "placed in {placed} worlds");
}

/// S01: arriving where one set out for leads with it.
#[test]
fn arriving_somewhere_describes_it_first() {
    let mut arrived = 0;
    for seed in [1u64, 42, 9001, 7] {
        let probe = {
            let mut g = Game::new(seed, crate::composing_tests::pack());
            g.start();
            g.in_view()
                .into_iter()
                .map(|v| v.landmark)
                .take(6)
                .collect::<Vec<_>>()
        };
        for i in probe {
            let mut g = Game::new(seed, crate::composing_tests::pack());
            g.trace = true;
            g.sustain = true;
            g.start();
            // On again after anything that stops the journey.
            let mut o = g.act("go", crate::Target::Landmark(i));
            for _ in 0..10 {
                if o.renders.iter().any(|r| r.trace.slot == "travel.arrive") {
                    break;
                }
                o = g.act("go", crate::Target::Landmark(i));
            }
            let Some(at) = o
                .renders
                .iter()
                .position(|r| r.trace.slot == "travel.arrive")
            else {
                continue;
            };
            arrived += 1;
            let l = &g.site.land.landmarks[i];
            // The first fact said after the arrival, by where its words
            // stand in what the player read.
            let lower = o.text.to_lowercase();
            let after = lower
                .find(&o.renders[at].trace.text.trim().to_lowercase())
                .unwrap_or(0);
            let first = o.renders[at + 1..]
                .iter()
                // Names are parts of facts, not facts.
                .filter(|r| {
                    r.trace.depth == 0
                        && !r.trace.slot.starts_with("travel.")
                        && !r.trace.slot.ends_with(".name")
                        && !r.trace.slot.ends_with("_name")
                })
                .filter_map(|r| {
                    let t = r.trace.text.trim().to_lowercase();
                    let t = t.trim_end_matches('.');
                    (!t.is_empty()).then(|| lower[after..].find(t).map(|p| (p, r)))?
                })
                .min_by_key(|(p, _)| *p)
                .map(|(_, r)| r)
                .expect("a description follows");
            // A bare hill or mountain is told by the arrival itself ("you
            // reach the high mountain"); what follows is what is around.
            let bare = l.feature.is_none() && l.settlement.is_none() && l.structure.is_none();
            let about = bare
                || match first.trace.slot.as_str() {
                    "land.feature" => l.feature.is_some_and(|f| {
                        first.vars.get("kind") == Some(&Value::from(g.site.world.features[f].kind))
                    }),
                    "place.whole" => l.settlement.is_some() || l.structure.is_some(),
                    "place.standout" | "place.structure" => l.structure.is_some(),
                    "land.landmark" => true,
                    _ => false,
                };
            assert!(
                about,
                "seed {seed} landmark {} ({}): {} {:?}: {}",
                i, l.kind, first.trace.slot, first.vars, o.text
            );
        }
    }
    assert!(arrived >= 6, "too few arrivals to judge: {arrived}");
}

/// S03: every creature, plant, thing and object name has a plural that
/// differs from it (unless listed as unchanging), and none doubles its
/// ending.
#[test]
fn every_name_has_a_plural() {
    use scraped_content::english::{plural_if, unchanging};
    for seed in [1u64, 42, 9001] {
        let g = crate::Game::new(seed, scraped_content::Pack::default());
        let mut names: Vec<String> = g
            .site
            .world
            .life
            .species
            .iter()
            .map(|s| s.form.to_string())
            .collect();
        names.extend(g.site.things.iter().map(|t| t.kind.to_string()));
        names.extend(g.site.world.objects.iter().map(|o| o.kind.to_string()));
        names.extend(scraped_sim::items::ids().iter().map(|s| s.to_string()));
        names.sort();
        names.dedup();
        for n in names {
            let n = n.replace('_', " ");
            let p = plural_if(&n, 2);
            assert!(!p.ends_with("seses"), "{n} → {p}");
            assert!(p != n || unchanging(&n), "{n} has no plural");
        }
    }
}

/// S03: a feature whose name is plural takes no article.
#[test]
fn plural_features_take_no_article() {
    let mut g = crate::Game::new(1, crate::composing_tests::pack());
    for k in scraped_world::features::KINDS {
        let c = crate::site::ctx(&[
            ("kind", scraped_content::Value::from(k.id)),
            (
                "plural",
                scraped_content::Value::Bool(scraped_world::features::plural_name(k.id)),
            ),
            (
                "group",
                scraped_content::Value::from(scraped_sim::outdoors::label(&k.group)),
            ),
            ("bearing", scraped_content::Value::from("west")),
            ("distance", scraped_content::Value::from("near")),
            ("rock", scraped_content::Value::from("granite")),
        ]);
        let t = g.say("land.feature", c);
        if scraped_world::features::plural_name(k.id) {
            assert!(!t.starts_with("A ") && !t.starts_with("An "), "{t}");
        }
    }
}
