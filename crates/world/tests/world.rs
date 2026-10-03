//! Milestone 4 properties, checked on several seeds.

use std::collections::{BTreeMap, VecDeque};

use scraped_world::history::{EventKind, Evidence};
use scraped_world::terrain::SIZE;
use scraped_world::World;

const SEEDS: [u64; 6] = [1, 2, 3, 42, 777, 9001];

fn worlds() -> Vec<World> {
    SEEDS.iter().map(|&s| World::generate(s)).collect()
}

/// FNV-1a over the world's JSON: a cheap fingerprint for determinism.
fn fingerprint(w: &World) -> u64 {
    let json = serde_json::to_string(w).unwrap();
    json.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[test]
fn same_seed_same_world() {
    for seed in [1, 42] {
        let (a, b) = (World::generate(seed), World::generate(seed));
        assert_eq!(fingerprint(&a), fingerprint(&b));
        assert_eq!(
            scraped_world::debug::png(&a, 1),
            scraped_world::debug::png(&b, 1)
        );
    }
}

/// Pinned fingerprints: any change to generation shows up here. Update the
/// numbers (printed on failure) when a change is intended.
#[test]
fn snapshot_fingerprints() {
    let got: Vec<(u64, u64)> = [1u64, 42, 9001]
        .iter()
        .map(|&s| (s, fingerprint(&World::generate(s))))
        .collect();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fingerprints.txt");
    let text: String = got.iter().map(|(s, f)| format!("{s} {f:016x}\n")).collect();
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(path, &text).unwrap();
    }
    let want = std::fs::read_to_string(path).unwrap_or_default();
    assert_eq!(
        text, want,
        "world fingerprints changed; rerun with UPDATE_SNAPSHOTS=1 if intended"
    );
}

#[test]
fn every_structure_is_reachable_from_another() {
    for w in worlds() {
        // Passable land: rivers only at fords and bridges.
        let bridges: Vec<_> = w
            .structures
            .iter()
            .filter(|s| s.kind == scraped_world::structures::StructureKind::Bridge)
            .map(|s| s.cell)
            .collect();
        let passable = |x: usize, y: usize| {
            w.terrain.is_land(x, y)
                && (!w.water.needs_crossing(&w.terrain, x, y)
                    || bridges.iter().any(|c| c.ux() == x && c.uy() == y))
        };
        let mut comp = vec![usize::MAX; SIZE * SIZE];
        let mut n = 0;
        for start in 0..SIZE * SIZE {
            if comp[start] != usize::MAX || !passable(start % SIZE, start / SIZE) {
                continue;
            }
            let mut q = VecDeque::from([start]);
            comp[start] = n;
            while let Some(c) = q.pop_front() {
                for (nx, ny) in w.terrain.height.neighbours(c % SIZE, c / SIZE) {
                    let i = ny * SIZE + nx;
                    if comp[i] == usize::MAX && passable(nx, ny) {
                        comp[i] = n;
                        q.push_back(i);
                    }
                }
            }
            n += 1;
        }
        let mut per: BTreeMap<usize, usize> = BTreeMap::new();
        for s in &w.structures {
            *per.entry(comp[s.cell.uy() * SIZE + s.cell.ux()])
                .or_default() += 1;
        }
        // Caves are found, not built: one on an islet may be cut off.
        for s in w.structures.iter().filter(|s| {
            !matches!(
                s.kind,
                scraped_world::structures::StructureKind::Cave
                    | scraped_world::structures::StructureKind::SeaCave
            )
        }) {
            let c = comp[s.cell.uy() * SIZE + s.cell.ux()];
            assert!(
                c != usize::MAX && per[&c] > 1,
                "seed {}: structure {} ({:?}) is cut off",
                w.seed,
                s.id,
                s.kind
            );
        }
    }
}

#[test]
fn texts_are_in_their_eras_language() {
    for w in worlds() {
        for t in &w.texts {
            let lang = &w.languages[t.era as usize];
            let rendered = w.render(t);
            let names: Vec<_> = w
                .history
                .people
                .iter()
                .filter(|p| p.era == t.era)
                .map(|p| p.name.clone())
                .collect();
            let analyses = lang.analyses(&names);
            for word in &rendered.words {
                let ph = word.phonemes();
                assert!(
                    lang.phonology.is_valid(&ph),
                    "seed {}: text {} era {}",
                    w.seed,
                    t.id,
                    t.era
                );
                assert!(
                    analyses.contains_key(&ph),
                    "seed {}: text {} has a word outside era {}",
                    w.seed,
                    t.id,
                    t.era
                );
            }
        }
        for p in &w.history.people {
            assert!(
                w.languages[p.era as usize].phonology.is_valid(&p.name),
                "seed {}: name of {}",
                w.seed,
                p.id
            );
        }
    }
}

#[test]
fn evidence_exists_for_every_event() {
    for w in worlds() {
        for e in &w.history.events {
            if e.evidence.is_empty() {
                continue;
            }
            let texts = w.texts.iter().any(|t| t.event == Some(e.id));
            let traces = w.traces.iter().any(|t| t.event == e.id)
                || w.structures.iter().any(|s| s.event == Some(e.id));
            let physical_only = e
                .evidence
                .iter()
                .all(|x| matches!(x, Evidence::Ruin | Evidence::Wall));
            assert!(
                texts || traces || (physical_only && traces),
                "seed {}: event {} ({:?}) left no evidence",
                w.seed,
                e.id,
                e.kind
            );
        }
    }
}

#[test]
fn every_world_has_a_root_writing_event() {
    for w in worlds() {
        let root = &w.history.events[w.history.root];
        assert!(
            matches!(root.kind, EventKind::Writing { root: true, .. }),
            "seed {}",
            w.seed
        );
        assert_eq!(root.era, 0);
        assert!(
            w.texts.iter().any(|t| t.event == Some(root.id)),
            "seed {}: the deepest text is missing",
            w.seed
        );
    }
}

#[test]
fn sizes_stay_in_bounds() {
    for w in worlds() {
        let s = w.history.settlements.len();
        assert!((4..=40).contains(&s), "seed {}: {s} settlements", w.seed);
        assert!(
            (30..=600).contains(&w.structures.len()),
            "seed {}: {} structures",
            w.seed,
            w.structures.len()
        );
        assert!(
            (40..=1500).contains(&w.texts.len()),
            "seed {}: {} texts",
            w.seed,
            w.texts.len()
        );
        assert_eq!(w.history.eras.len(), 3);
        // Interiors are entered at room 0, which always survives.
        for st in &w.structures {
            assert!(!st.interior.rooms.is_empty() && !st.interior.rooms[0].collapsed);
        }
    }
}

#[test]
fn png_is_a_valid_png() {
    let w = World::generate(1);
    let png = scraped_world::debug::png(&w, 2);
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    assert!(png.len() > (SIZE * 2) * (SIZE * 2) * 3);
}
