//! D06 living things: signs where species live, life that follows the
//! region, and the catch of fishing and snares.

use scraped_sim::outdoors::Pos;
use scraped_sim::region::LIFE;
use scraped_world::life::Kingdom;

use crate::attention::Response;
use crate::composing_tests::pack;
use crate::site::Place;
use crate::Game;

fn game(seed: u64) -> Game {
    let mut g = Game::new(seed, pack());
    g.start();
    g.state.place = Place::Outside;
    g
}

/// Points spread over the land.
fn spots(g: &Game) -> Vec<Pos> {
    let mut out = Vec::new();
    for y in (5..150).step_by(13) {
        for x in (5..150).step_by(13) {
            if g.site.world.terrain.is_land(x, y) {
                out.push(Pos::of_cell(x, y));
            }
        }
    }
    out
}

#[test]
fn signs_only_appear_where_the_species_lives() {
    let mut g = game(42);
    let mut signs = 0;
    for p in spots(&g) {
        g.state.pos = p;
        let habitats = g.habitats_here();
        let living: Vec<usize> = g.life_here().iter().map(|l| l.species).collect();
        for l in &living {
            assert!(habitats.contains(&g.site.world.life.species[*l].habitat));
        }
        let mut facts = Vec::new();
        g.life_facts(Response::Closer, &mut facts);
        for f in facts.iter().filter(|f| f.slot == "life.sign") {
            let sp: usize = f.key.split(':').nth(1).unwrap().parse().unwrap();
            assert!(
                habitats.contains(&g.site.world.life.species[sp].habitat),
                "sign of {sp} at {p:?}"
            );
            signs += 1;
        }
    }
    assert!(signs > 10, "too few signs: {signs}");
}

#[test]
fn a_dying_region_loses_species_and_a_recovering_one_gains_them() {
    let mut g = game(7);
    let mut fewer = 0;
    let mut more = 0;
    for p in spots(&g) {
        g.state.pos = p;
        let Some(r) = g.site.regions.at(p) else {
            continue;
        };
        let start = g.state.regions.vars[r][LIFE];
        let now = g.life_here().len();
        g.state.regions.vars[r][LIFE] = start / 3;
        let dying = g.life_here().len();
        g.state.regions.vars[r][LIFE] = start * 3 / 2;
        let recovered = g.life_here().len();
        g.state.regions.vars[r][LIFE] = start;
        assert!(dying <= now && recovered >= now);
        fewer += usize::from(dying < now);
        more += usize::from(recovered > now);
    }
    assert!(fewer > 5 && more > 5, "fewer {fewer}, more {more}");
}

#[test]
fn plants_and_animals_are_both_met() {
    let mut g = game(1);
    let mut plants = 0;
    let mut animals = 0;
    for p in spots(&g) {
        g.state.pos = p;
        for l in g.life_here() {
            match g.site.world.life.species[l.species].kingdom {
                Kingdom::Plant => plants += 1,
                Kingdom::Animal => animals += 1,
            }
        }
    }
    assert!(plants > 50 && animals > 50, "{plants} {animals}");
}

#[test]
fn fishing_and_snares_catch_what_lives_there() {
    let mut caught = 0;
    for seed in [1u64, 42, 9001] {
        let mut g = game(seed);
        // Stand by water and fish a few times.
        let by_water = spots(&g)
            .into_iter()
            .find(|&p| {
                g.state.pos = p;
                g.water_near(1).iter().any(|(_, d, _)| *d < 300.0)
            })
            .unwrap();
        g.state.pos = by_water;
        for _ in 0..6 {
            g.fish();
        }
        // A snare, checked after a night.
        g.snare();
        g.state.minutes += 12 * 60;
        g.snare();
        caught += g
            .state
            .carried
            .iter()
            .filter(|&&t| matches!(g.thing(t).kind, "fish" | "game"))
            .count();
    }
    assert!(caught >= 3, "caught {caught}");
}

#[test]
fn a_clear_night_shows_the_still_star_and_steers_the_walker() {
    let mut g = game(3);
    let p = spots(&g)
        .into_iter()
        .find(|p| {
            let (x, y) = p.cell();
            !matches!(
                g.site.world.terrain.biome.get(x, y),
                scraped_world::terrain::Biome::Forest | scraped_world::terrain::Biome::Pine
            )
        })
        .unwrap();
    g.state.pos = p;
    // The first clear night hour.
    let night = (2..40)
        .flat_map(|d| [d * 1440 + 23 * 60, d * 1440 + 60])
        .find(|&t| {
            g.state.minutes = t;
            g.conditions().0 == "clear"
        })
        .expect("a clear night");
    g.state.minutes = night;
    let mut facts = Vec::new();
    g.sky_facts(&mut facts);
    assert!(facts.iter().any(|f| f.slot == "sky.figure"
        && f.vars.get("still") == Some(&scraped_content::Value::Bool(true))));
    let (w, l, _) = g.conditions();
    assert!(g.star_to_steer_by(w, l, p));
}

#[test]
fn weather_closes_ways_and_shows_it() {
    // Somewhere in the first months, the weather closes a cell that a
    // walker could otherwise cross, and the land shows what happened.
    let mut g = game(2);
    let mut shown = false;
    'outer: for d in (0..360).step_by(5) {
        g.state.minutes = d * 1440 + 12 * 60;
        for p in spots(&g) {
            let (x, y) = p.cell();
            if g.env().closed(x, y).is_some() {
                g.state.pos = p;
                let mut facts = Vec::new();
                g.weather_facts(Response::Look, &mut facts);
                assert!(facts.iter().any(|f| f.slot == "weather.mark"));
                assert!(!g.env().passable(x, y));
                shown = true;
                break 'outer;
            }
        }
    }
    assert!(shown);
}
