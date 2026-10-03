//! D04 properties: interiors are consistent geometry, every space is
//! reachable or deliberately sealed, great interiors are vast, deep, loopy
//! and hold hidden spaces, caves follow the rock, and growth by era shows.

use std::collections::BTreeSet;

use scraped_world::interiors::{shape, GreatKind};
use scraped_world::structures::{Exit, Passage, StructureKind};
use scraped_world::World;

const SEEDS: [u64; 4] = [1, 3, 42, 9001];

fn worlds() -> Vec<World> {
    SEEDS.iter().map(|&s| World::generate(s)).collect()
}

#[test]
fn geometry_is_consistent() {
    for w in worlds() {
        for st in &w.structures {
            let rooms = &st.interior.rooms;
            for (a, ra) in rooms.iter().enumerate() {
                assert!(
                    ra.w > 0 && ra.d > 0,
                    "seed {} structure {} room {a}: no footprint",
                    w.seed,
                    st.id
                );
                for (b, rb) in rooms.iter().enumerate().skip(a + 1) {
                    assert!(
                        !ra.overlaps(rb),
                        "seed {} structure {} ({:?}): rooms {a} and {b} overlap",
                        w.seed,
                        st.id,
                        st.kind
                    );
                }
            }
            for (n, l) in st.interior.links.iter().enumerate() {
                let (ra, rb) = (&rooms[l.a], &rooms[l.b]);
                if ra.level == rb.level {
                    assert!(
                        !matches!(l.passage, Passage::Hole | Passage::Shaft | Passage::Ladder),
                        "seed {} structure {} link {n}: a level way that climbs",
                        w.seed,
                        st.id
                    );
                    assert_eq!(
                        ra.touches(rb),
                        Some(l.exit),
                        "seed {} structure {} ({:?}) link {n}: rooms {} and {} don't touch on its side",
                        w.seed,
                        st.id,
                        st.kind,
                        l.a,
                        l.b
                    );
                } else {
                    assert!(
                        ra.stacked(rb),
                        "seed {} structure {} link {n}: stair between rooms not over each other",
                        w.seed,
                        st.id
                    );
                    let up = rb.level > ra.level;
                    assert_eq!(l.exit, if up { Exit::Up } else { Exit::Down });
                }
            }
        }
    }
}

#[test]
fn every_space_is_reachable_or_sealed() {
    for w in worlds() {
        for st in &w.structures {
            // Sealed ways (blocked by collapse or decay) are recorded ways
            // in: count through them.
            let mut unsealed = st.interior.clone();
            for l in &mut unsealed.links {
                l.state = scraped_world::structures::PassageState::Open;
            }
            for r in &mut unsealed.rooms {
                r.collapsed = false;
            }
            let seen = unsealed.reachable_all();
            for (r, room) in st.interior.rooms.iter().enumerate() {
                assert!(
                    seen[r] || room.collapsed,
                    "seed {} structure {} ({:?}): room {r} is cut off",
                    w.seed,
                    st.id,
                    st.kind
                );
            }
        }
    }
}

#[test]
fn great_interiors_are_vast_deep_and_intricate() {
    for w in worlds() {
        let shapes: Vec<(GreatKind, _)> = w
            .greats
            .iter()
            .map(|&(i, k)| (k, shape(&w.structures[i].interior)))
            .collect();
        let big: Vec<_> = shapes.iter().filter(|(_, s)| s.spaces >= 200).collect();
        assert!(
            big.len() >= 4,
            "seed {}: {} great interiors",
            w.seed,
            big.len()
        );
        assert!(
            big.iter().any(|(k, _)| *k == GreatKind::Cave),
            "seed {}: no great cave",
            w.seed
        );
        let largest = shapes.iter().map(|(_, s)| s.spaces).max().unwrap();
        assert!(
            largest >= 400,
            "seed {}: largest has {largest} spaces",
            w.seed
        );
        let deepest = shapes.iter().map(|(_, s)| s.levels).max().unwrap();
        assert!(
            deepest >= 6,
            "seed {}: deepest has {deepest} levels",
            w.seed
        );
        for (k, s) in &big {
            assert!(
                s.loops >= 15,
                "seed {}: {:?} has {} loops",
                w.seed,
                k,
                s.loops
            );
            assert!(
                s.hidden >= 5,
                "seed {}: {:?} has {} hidden spaces",
                w.seed,
                k,
                s.hidden
            );
            assert!(
                s.one_way >= 1,
                "seed {}: {:?} has no one-way ways",
                w.seed,
                k
            );
        }
    }
}

#[test]
fn hidden_spaces_are_reached_only_by_hidden_ways() {
    for w in worlds() {
        for &(i, _) in &w.greats {
            let int = &w.structures[i].interior;
            for (r, room) in int.rooms.iter().enumerate() {
                if !room.hidden {
                    continue;
                }
                // Every way into the hidden part from the visible part is
                // itself hidden.
                for l in &int.links {
                    let (a, b) = (&int.rooms[l.a], &int.rooms[l.b]);
                    if (l.a == r || l.b == r) && a.hidden != b.hidden {
                        assert!(
                            l.hidden,
                            "seed {} structure {i}: hidden room {r} has an open way in",
                            w.seed
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn caves_follow_the_rock_and_water() {
    for w in worlds() {
        for st in w
            .structures
            .iter()
            .filter(|s| matches!(s.kind, StructureKind::Cave | StructureKind::SeaCave))
        {
            let f = w
                .features
                .iter()
                .find(|f| f.cell == st.cell && f.inside.is_some());
            assert!(f.is_some(), "seed {}: a cave with no mouth", w.seed);
            let rock = w.geology.at(st.cell.ux(), st.cell.uy());
            let mouths = st.interior.rooms.iter().filter(|r| r.outside).count();
            assert!(mouths >= 1);
            for room in &st.interior.rooms {
                assert_eq!(room.style, "natural");
                // Dripstone only in rock that water dissolves.
                if room
                    .features
                    .iter()
                    .any(|f| matches!(f.kind, "stalactites" | "flowstone" | "column"))
                {
                    assert!(rock.soluble(), "seed {}: dripstone in {:?}", w.seed, rock);
                }
                // Water gathers at the bottom.
                if matches!(room.water, "stream" | "sump") {
                    let low = st.interior.rooms.iter().map(|r| r.level).min().unwrap();
                    assert!(
                        room.level <= low + 1,
                        "seed {}: a stream high in a cave",
                        w.seed
                    );
                }
            }
            for l in &st.interior.links {
                assert!(
                    !matches!(l.passage, Passage::Door | Passage::Arch | Passage::Window),
                    "a door in a cave"
                );
            }
        }
    }
}

#[test]
fn growth_by_era_shows_in_styles() {
    for w in worlds() {
        let last = w.history.eras.len() as u32 - 1;
        for &(i, k) in &w.greats {
            let st = &w.structures[i];
            if k == GreatKind::Cave || st.era == last {
                continue;
            }
            let styles: BTreeSet<&str> = st
                .interior
                .rooms
                .iter()
                .map(|r| r.style)
                .filter(|s| !s.is_empty())
                .collect();
            assert!(
                styles.len() >= 2,
                "seed {}: {:?} {i} built in one manner only: {styles:?}",
                w.seed,
                k
            );
        }
    }
}

#[test]
fn generation_is_the_same_whenever() {
    let a = World::generate(42);
    let b = World::generate(42);
    for (&(i, _), &(j, _)) in a.greats.iter().zip(&b.greats) {
        assert_eq!(i, j);
        let ja = serde_json::to_string(&a.structures[i].interior).unwrap();
        let jb = serde_json::to_string(&b.structures[j].interior).unwrap();
        assert_eq!(ja, jb);
    }
}

#[test]
fn no_space_is_a_trap() {
    for w in worlds() {
        for st in &w.structures {
            assert!(
                st.interior.trapped().is_none(),
                "seed {} structure {} ({:?}): room {:?} has no way back out",
                w.seed,
                st.id,
                st.kind,
                st.interior.trapped()
            );
        }
    }
}
