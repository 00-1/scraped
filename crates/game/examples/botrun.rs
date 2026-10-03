//! Runs the depth bots on a few seeds and prints how they fared.
//!   cargo run --release -p scraped-game --example botrun -- explorer 1 2 3
use scraped_content::Pack;

fn pack() -> Pack {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content");
    let mut files = Vec::new();
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "toml") {
            files.push((
                p.file_name().unwrap().to_string_lossy().to_string(),
                std::fs::read_to_string(&p).unwrap(),
            ));
        }
    }
    Pack::load(&files).0
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let kind: &'static str = if args.first().map(String::as_str) == Some("scholar") {
        "scholar"
    } else {
        "explorer"
    };
    let hours: f64 = std::env::var("HOURS")
        .ok()
        .and_then(|h| h.parse().ok())
        .unwrap_or(72.0);
    let steps: usize = std::env::var("STEPS")
        .ok()
        .and_then(|h| h.parse().ok())
        .unwrap_or(5000);
    let pack = pack();
    if args.first().map(String::as_str) == Some("where") {
        for s in args.iter().skip(1) {
            where_things_are(&pack, s.parse().unwrap());
        }
        return;
    }
    for s in args.iter().skip(1) {
        let seed: u64 = s.parse().unwrap();
        let t = std::time::Instant::now();
        let r = scraped_game::bots::play(&pack, seed, kind, hours, steps);
        let kinds = r.novelty.iter().filter(|n| n.new_kind).count();
        println!(
            "seed {seed}: {} steps, {:.1} h, died {:?}, buildings {}, read {}, novel {} ({} kinds), deepest {}, great {} [{:.1}s]",
            r.commands.len(), r.hours, r.died, r.buildings, r.texts_read, r.novelty.len(), kinds,
            r.read_deepest, r.read_great, t.elapsed().as_secs_f64()
        );
        println!("  carried: {:?}", r.carried);
        println!("  words: {:?}", r.words);
        {
            let g = scraped_game::Game::new(seed, pack.clone());
            let home = |t: &scraped_game::site::Thing| match t.home {
                scraped_game::site::Place::Room { structure, .. } => structure,
                _ => usize::MAX,
            };
            let lens = g
                .site
                .things
                .iter()
                .find(|t| t.kind == "first_lens")
                .map(home);
            let root = g
                .site
                .writing
                .deep
                .first()
                .and_then(|r| g.site.things.iter().find(|t| t.texts.contains(r)))
                .map(home);
            let room_of = |t: &scraped_game::site::Thing| match t.home {
                scraped_game::site::Place::Room { structure, room } => {
                    format!("structure {structure} room {room}")
                }
                _ => String::new(),
            };
            {
                let rr = g
                    .site
                    .writing
                    .deep
                    .first()
                    .and_then(|r| g.site.things.iter().find(|t| t.texts.contains(r)))
                    .map(room_of)
                    .unwrap_or_default();
                let lr = g
                    .site
                    .things
                    .iter()
                    .find(|t| t.kind == "first_lens")
                    .map(room_of)
                    .unwrap_or_default();
                println!(
                    "  lens room seen {} (lit {}), has lens {}; root room seen {} (lit {})",
                    r.rooms.iter().any(|p| p.starts_with(&lr)
                        && p.len() >= lr.len()
                        && (p == &lr || p.starts_with(&format!("{lr} ")))),
                    r.rooms.contains(&lr),
                    r.carried.iter().any(|c| c.contains("first_lens")),
                    r.rooms
                        .iter()
                        .any(|p| p == &rr || p.starts_with(&format!("{rr} "))),
                    r.rooms.contains(&rr),
                );
            }
            if std::env::var("UNSEEN").is_ok() {
                let mut far = Vec::new();
                for st in &g.site.world.structures {
                    if r.entered.contains(&st.id) || st.interior.rooms.is_empty() {
                        continue;
                    }
                    let p = g.site.land.structure_pos[st.id];
                    let sq = (p.x.div_euclid(1000), p.y.div_euclid(1000));
                    let d = r
                        .walked
                        .iter()
                        .map(|w| (w.0 - sq.0).abs().max((w.1 - sq.1).abs()))
                        .min()
                        .unwrap_or(99);
                    far.push((d, st.id, format!("{:?}", st.kind), st.settlement.is_some()));
                }
                far.sort();
                println!("  unvisited (km from walked, id, kind, in town): {:?}", far);
            }
            if std::env::var("WHERE").is_ok() {
                let rr = g
                    .site
                    .writing
                    .deep
                    .first()
                    .and_then(|r| g.site.things.iter().find(|t| t.texts.contains(r)))
                    .map(room_of);
                let lr = g
                    .site
                    .things
                    .iter()
                    .find(|t| t.kind == "first_lens")
                    .map(room_of);
                println!("  root room {rr:?}, lens room {lr:?}");
                let lb = lr
                    .as_deref()
                    .and_then(|r| r.rsplit_once(" room ").map(|(b, _)| format!("{b} room ")))
                    .unwrap_or_default();
                let lseen: Vec<&String> = r.rooms.iter().filter(|p| p.starts_with(&lb)).collect();
                println!("  lens building rooms seen: {lseen:?}");
                if let Some(sid) = lr
                    .as_deref()
                    .and_then(|r| r.strip_prefix("structure "))
                    .and_then(|r| r.split(' ').next())
                    .and_then(|n| n.parse::<usize>().ok())
                {
                    for l in &g.site.world.structures[sid].interior.links {
                        println!("    link {} -> {} {:?} {:?}", l.a, l.b, l.passage, l.state);
                    }
                }
                let b = rr
                    .as_deref()
                    .and_then(|r| r.rsplit_once(" room ").map(|(b, _)| format!("{b} room ")))
                    .unwrap_or_default();
                let seen: Vec<&String> = r.rooms.iter().filter(|p| p.starts_with(&b)).collect();
                println!("  rooms seen there: {seen:?}");
            }
            println!(
                "  lens building {:?} entered {}; root building {:?} entered {}",
                lens,
                lens.is_some_and(|l| r.entered.contains(&l)),
                root,
                root.is_some_and(|l| r.entered.contains(&l))
            );
        }
        if std::env::var("NOTES").is_ok() {
            for n in &r.notes {
                println!("  {n}");
            }
        }
        if let Ok(n) = std::env::var("REPLAY") {
            // Replay the run to just before command n, then show what n
            // rendered, slot by slot.
            let n: usize = n.parse().unwrap();
            let mut g = scraped_game::Game::new(seed, pack.clone());
            g.sustain = kind == "scholar";
            g.start();
            for c in &r.commands[..n] {
                g.step(c);
            }
            g.keep_renders = true;
            let out = g.step(&r.commands[n]);
            println!("> {}\n{}", r.commands[n], out.text);
            for x in g.renders() {
                println!(
                    "  [{}] {:?}",
                    x.trace.slot,
                    x.vars.keys().collect::<Vec<_>>()
                );
            }
        }
        if std::env::var("SHOW").is_ok() {
            for (c, t) in r.commands.iter().zip(r.texts.iter().skip(1)) {
                println!("> {c}\n{t}\n");
            }
        }
    }
}

#[allow(dead_code)]
fn where_things_are(pack: &Pack, seed: u64) {
    let g = scraped_game::Game::new(seed, pack.clone());
    let start = g.site.start();
    let structures = g.site.world.structures.len();
    let mut d: Vec<(f64, usize)> = g
        .site
        .land
        .structure_pos
        .iter()
        .enumerate()
        .map(|(i, p)| (p.dist(start), i))
        .collect();
    d.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let rank = |sid: usize| d.iter().position(|x| x.1 == sid).unwrap_or(999);
    let find = |k: &str| {
        g.site
            .things
            .iter()
            .find(|t| t.kind == k)
            .map(|t| match t.home {
                scraped_game::site::Place::Room { structure, .. } => (
                    structure,
                    rank(structure),
                    g.site.land.structure_pos[structure].dist(start) as i64,
                ),
                _ => (0, 0, 0),
            })
    };
    let root = g
        .site
        .writing
        .deep
        .first()
        .and_then(|r| g.site.things.iter().find(|t| t.texts.contains(r)))
        .map(|t| match t.home {
            scraped_game::site::Place::Room { structure, .. } => (
                structure,
                rank(structure),
                g.site.land.structure_pos[structure].dist(start) as i64,
            ),
            _ => (0, 0, 0),
        });
    println!(
        "seed {seed}: {structures} structures; first_lens {:?}; first_scraper {:?}; root {:?}",
        find("first_lens"),
        find("first_scraper"),
        root
    );
}
