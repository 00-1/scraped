//! D03 properties: every feature follows its cause, every scene traces to
//! history, towns of a role and size never share a layout, and the
//! world is varied enough.

use std::collections::BTreeSet;

use scraped_world::features::{self, Group};
use scraped_world::geology::Rock;
use scraped_world::scenes::SceneAt;
use scraped_world::structures::StructureKind;
use scraped_world::terrain::{Biome, SIZE};
use scraped_world::World;

const SEEDS: [u64; 6] = [1, 2, 3, 42, 777, 9001];

fn worlds() -> Vec<World> {
    SEEDS.iter().map(|&s| World::generate(s)).collect()
}

fn near<F: Fn(usize, usize) -> bool>(x: usize, y: usize, r: i64, f: F) -> bool {
    (-r..=r).any(|dy| {
        (-r..=r).any(|dx| {
            let (nx, ny) = (x as i64 + dx, y as i64 + dy);
            nx >= 0
                && ny >= 0
                && nx < SIZE as i64
                && ny < SIZE as i64
                && f(nx as usize, ny as usize)
        })
    })
}

#[test]
fn every_feature_follows_its_cause() {
    for w in worlds() {
        let t = &w.terrain;
        for f in &w.features {
            let (x, y) = (f.cell.ux(), f.cell.uy());
            let b = *t.biome.get(x, y);
            let here = *t.height.get(x, y);
            let rock = w.geology.at(x, y);
            let river = |x, y| w.water.is_river(t, x, y);
            let sea = |x: usize, y: usize| *t.biome.get(x, y) == Biome::Sea;
            let ok = match f.kind {
                "spring" => !river(x, y) && t.is_land(x, y),
                "warm spring" => rock == Rock::Basalt,
                "waterfall" | "rapids" | "gorge" => river(x, y),
                "delta" => river(x, y) && near(x, y, 1, sea),
                "sea stack" => b == Biome::Sea,
                "sea cave" | "tidal flats" => near(x, y, 1, sea),
                "oxbow lake" => near(x, y, 1, |a, c| *t.biome.get(a, c) == Biome::Lake),
                "cave mouth" | "sinkhole" => rock.soluble(),
                "glacier" | "snowfield" => b == Biome::Snow,
                "salt flat" | "petrified forest" => b == Biome::Desert,
                "rock pillar" => rock == Rock::Sandstone,
                "boulder field" => rock == Rock::Granite,
                "dead forest" => matches!(b, Biome::Forest | Biome::Pine),
                "ancient tree" | "grove" | "flower meadow" => b == Biome::Grassland,
                "quarry" => rock.quarried(),
                "cairn" => here > 300.0,
                "spoil heap" => w
                    .structures
                    .iter()
                    .any(|s| s.kind == StructureKind::Mine && s.cell == f.cell),
                "burial mound" | "standing stones" | "terraces" | "field walls" | "old road" => {
                    f.settlement.is_some() && f.era.is_some()
                }
                _ => true,
            };
            assert!(
                ok,
                "seed {}: {} at {:?} ({:?}, {:?}) breaks its cause {}",
                w.seed, f.kind, f.cell, b, rock, f.cause
            );
            // Caves are recorded for D04.
            assert_eq!(
                f.inside.is_some(),
                matches!(f.kind, "cave mouth" | "sea cave"),
                "{}",
                f.kind
            );
        }
    }
}

#[test]
fn every_scene_traces_to_history() {
    for w in worlds() {
        for s in &w.scenes {
            // An event, or the place's own history: a river's floods, the
            // road, devotion at a shrine.
            let place_history = matches!(s.cause, "flood" | "road" | "devotion");
            assert!(
                s.event.is_some() || place_history,
                "seed {}: scene {} ({})",
                w.seed,
                s.id,
                s.kind
            );
            if let Some(e) = s.event {
                assert!(e < w.history.events.len());
            }
            assert!(
                !scraped_world::scenes::parts(s.kind).is_empty(),
                "{}",
                s.kind
            );
            if let SceneAt::Room { structure, room } = s.at {
                assert!(room < w.structures[structure].interior.rooms.len());
            }
        }
        // At least three in every settlement, and some outside them.
        for st in &w.history.settlements {
            let n = w
                .scenes
                .iter()
                .filter(|s| s.settlement == Some(st.id))
                .count();
            assert!(
                n >= 3,
                "seed {}: settlement {} has {n} scenes",
                w.seed,
                st.id
            );
        }
        assert!(
            w.scenes.iter().any(|s| s.settlement.is_none()),
            "seed {}",
            w.seed
        );
    }
}

#[test]
fn towns_of_a_role_and_size_never_share_a_layout() {
    for w in worlds() {
        let mut seen = BTreeSet::new();
        for t in &w.towns {
            let size = w.history.settlements[t.settlement].size;
            assert!(
                seen.insert((t.role, size, t.signature())),
                "seed {}: town {} repeats a layout",
                w.seed,
                t.settlement
            );
            assert_eq!(t.districts[0].kind, "square");
            assert!(t
                .streets
                .iter()
                .all(|s| s.from < t.districts.len() && s.to < t.districts.len()));
        }
        // Every town building has a district of its town.
        for s in w.structures.iter().filter(|s| s.settlement.is_some()) {
            let t = &w.towns[s.settlement.unwrap()];
            assert!(
                s.district.is_some_and(|d| d < t.districts.len()),
                "seed {}: {}",
                w.seed,
                s.id
            );
        }
    }
}

#[test]
fn worlds_are_varied() {
    for w in worlds() {
        let kinds: BTreeSet<_> = w.structures.iter().map(|s| s.kind).collect();
        assert!(
            kinds.len() >= 25,
            "seed {}: {} building kinds",
            w.seed,
            kinds.len()
        );
        let natural: BTreeSet<_> = w
            .features
            .iter()
            .filter(|f| features::kind(f.kind).group != Group::Marks)
            .map(|f| f.kind)
            .collect();
        assert!(
            natural.len() >= 12,
            "seed {}: {} natural kinds {natural:?}",
            w.seed,
            natural.len()
        );
        let roles: BTreeSet<_> = w.towns.iter().map(|t| t.role).collect();
        assert!(roles.len() >= 4, "seed {}: roles {roles:?}", w.seed);
        assert!(w.underground.iter().any(|r| r.kind == "drain"));
    }
}

#[test]
fn every_kind_has_an_id_it_serialises_as() {
    let mut ids = BTreeSet::new();
    for k in StructureKind::ALL {
        let info = k.info();
        assert!(ids.insert(info.id), "{}", info.id);
        assert_eq!(serde_json::to_value(k).unwrap(), info.id);
    }
    assert!(ids.len() >= 35);
}

#[test]
fn ids_follow_the_kinds() {
    for (k, id) in StructureKind::ALL
        .iter()
        .zip(scraped_world::structures::IDS)
    {
        assert_eq!(k.id(), id);
    }
}
