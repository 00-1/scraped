//! D03 in play: walking a town's districts, finding features and scenes
//! by looking closer, and storylets placed at scenes.

use scraped_content::PackFile;
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
