//! Debug views of a world (all spoilers): ASCII and PNG maps, the history
//! timeline, and a site view with its interior and writing.

use crate::history::{EventKind, Role};
use crate::structures::{Condition, PassageState, StructureKind};
use crate::terrain::{Biome, SIZE};
use crate::World;

fn biome_char(b: Biome) -> char {
    match b {
        Biome::Sea => ' ',
        Biome::Lake => 'o',
        Biome::Shore => '.',
        Biome::Marsh => ',',
        Biome::Grassland => '"',
        Biome::Scrub => ';',
        Biome::Desert => ':',
        Biome::Forest => 'T',
        Biome::Pine => 'A',
        Biome::Tundra => '-',
        Biome::Rock => '^',
        Biome::Snow => '*',
    }
}

/// An ASCII map, `step` cells per character.
pub fn ascii_map(w: &World, step: usize) -> String {
    let mut grid: Vec<Vec<char>> = (0..SIZE)
        .step_by(step)
        .map(|y| {
            (0..SIZE)
                .step_by(step)
                .map(|x| {
                    if w.water.is_river(&w.terrain, x, y) {
                        '~'
                    } else {
                        biome_char(*w.terrain.biome.get(x, y))
                    }
                })
                .collect()
        })
        .collect();
    let mut mark = |x: usize, y: usize, c: char| {
        if let Some(row) = grid.get_mut(y / step) {
            if let Some(cell) = row.get_mut(x / step) {
                *cell = c;
            }
        }
    };
    for r in &w.history.roads {
        for c in &r.path {
            mark(c.ux(), c.uy(), '+');
        }
    }
    for s in &w.structures {
        let c = match s.kind {
            StructureKind::Bridge => '=',
            StructureKind::Tomb => 't',
            StructureKind::Waystation => 'w',
            StructureKind::Mine => 'm',
            _ => continue,
        };
        mark(s.cell.ux(), s.cell.uy(), c);
    }
    for (i, s) in w.history.settlements.iter().enumerate() {
        let c = if s.abandoned.is_some() {
            '#'
        } else {
            char::from_digit((i % 36) as u32, 36)
                .unwrap_or('?')
                .to_ascii_uppercase()
        };
        mark(s.cell.ux(), s.cell.uy(), c);
    }
    let mut out: String = grid
        .into_iter()
        .map(|r| r.into_iter().collect::<String>() + "\n")
        .collect();
    out.push_str(
        "key: 0-9A-Z settlement  # abandoned  + road  = bridge  t tomb  w waystation  m mine  ~ river\n      \" grass ; scrub : desert T forest A pine - tundra ^ rock * snow , marsh . shore o lake\n",
    );
    out
}

/// Colour of a cell for the PNG map.
fn colour(w: &World, x: usize, y: usize) -> [u8; 3] {
    if w.water.is_river(&w.terrain, x, y) {
        return [70, 120, 200];
    }
    let shade = (w.terrain.height.get(x, y).clamp(0.0, 2000.0) / 2000.0 * 40.0) as u8;
    let base: [u8; 3] = match w.terrain.biome.get(x, y) {
        Biome::Sea => [30, 60, 110],
        Biome::Lake => [50, 95, 170],
        Biome::Shore => [210, 200, 160],
        Biome::Marsh => [90, 120, 90],
        Biome::Grassland => [140, 175, 90],
        Biome::Scrub => [170, 165, 100],
        Biome::Desert => [215, 195, 140],
        Biome::Forest => [60, 120, 60],
        Biome::Pine => [50, 95, 70],
        Biome::Tundra => [160, 165, 150],
        Biome::Rock => [130, 125, 120],
        Biome::Snow => [235, 238, 242],
    };
    base.map(|c: u8| c.saturating_add(shade))
}

/// The map as RGB pixels, `scale` pixels per cell.
pub fn pixels(w: &World, scale: usize) -> (usize, Vec<u8>) {
    let side = SIZE * scale;
    let mut px = vec![0u8; side * side * 3];
    let mut put = |x: usize, y: usize, c: [u8; 3]| {
        for dy in 0..scale {
            for dx in 0..scale {
                let i = ((y * scale + dy) * side + x * scale + dx) * 3;
                px[i..i + 3].copy_from_slice(&c);
            }
        }
    };
    for y in 0..SIZE {
        for x in 0..SIZE {
            put(x, y, colour(w, x, y));
        }
    }
    for r in &w.history.roads {
        for c in &r.path {
            put(c.ux(), c.uy(), [120, 80, 40]);
        }
    }
    for s in &w.structures {
        let c = match s.condition {
            Condition::Intact | Condition::Worn => [40, 30, 30],
            _ => [150, 60, 50],
        };
        put(s.cell.ux(), s.cell.uy(), c);
    }
    for s in &w.history.settlements {
        put(
            s.cell.ux(),
            s.cell.uy(),
            if s.abandoned.is_some() {
                [200, 40, 40]
            } else {
                [255, 255, 255]
            },
        );
    }
    (side, px)
}

/// A PNG file of the map. Uncompressed (stored deflate blocks), so it
/// needs no compression library.
pub fn png(w: &World, scale: usize) -> Vec<u8> {
    let (side, px) = pixels(w, scale);
    let mut raw = Vec::with_capacity(side * (side * 3 + 1));
    for row in px.chunks(side * 3) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut z = vec![0x78, 0x01];
    for (i, chunk) in raw.chunks(65_535).enumerate() {
        let last = (i + 1) * 65_535 >= raw.len();
        z.push(u8::from(last));
        let len = chunk.len() as u16;
        z.extend_from_slice(&len.to_le_bytes());
        z.extend_from_slice(&(!len).to_le_bytes());
        z.extend_from_slice(chunk);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + u32::from(x)) % 65_521;
        b = (b + a) % 65_521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());

    let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let mut chunk = |kind: &[u8; 4], data: &[u8]| {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut crc_input = kind.to_vec();
        crc_input.extend_from_slice(data);
        out.extend_from_slice(&crc_input);
        out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(side as u32).to_be_bytes());
    ihdr.extend_from_slice(&(side as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &z);
    chunk(b"IEND", &[]);
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xffff_ffffu32;
    for &b in data {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xedb8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
    }
    !c
}

/// The history as a timeline.
pub fn timeline(w: &World) -> String {
    let h = &w.history;
    let name = |era: u32, p: &scraped_lang::phonology::Phonemes| {
        scraped_lang::render::capitalise(&w.languages[era as usize].romanise(p))
    };
    let person = |i: usize| {
        let p = &h.people[i];
        let title = p
            .role
            .title()
            .map(|t| format!(" the {t}"))
            .unwrap_or_default();
        format!("{}{title}", name(p.era, &p.name))
    };
    let town = |i: usize| name(h.settlements[i].era, &h.settlements[i].name);
    let mut out = format!(
        "HISTORY — seed {} · present year {} · trajectory {:?}\n",
        w.seed, h.present, h.trajectory
    );
    for era in &h.eras {
        out.push_str(&format!(
            "\nERA {} (years {}–{})\n",
            era.index, era.start, era.end
        ));
        for e in h.events.iter().filter(|e| e.era == era.index) {
            let what = match &e.kind {
                EventKind::Founding { settlement } => format!("{} founded", town(*settlement)),
                EventKind::Abandonment { settlement, cause } => {
                    format!("{} abandoned ({cause:?})", town(*settlement))
                }
                EventKind::Succession { ruler, .. } => format!("{} becomes ruler", person(*ruler)),
                EventKind::Death { person: p, .. } if h.people[*p].role == Role::Ruler => {
                    format!("{} dies", person(*p))
                }
                EventKind::Death { .. } => continue,
                EventKind::War { settlement, .. } => format!("war at {}", town(*settlement)),
                EventKind::Plague { settlement } => format!("plague in {}", town(*settlement)),
                EventKind::Famine { settlement } => format!("famine in {}", town(*settlement)),
                EventKind::Schism { new_faction, .. } => {
                    format!(
                        "schism: the {} break away",
                        name(e.era, &h.factions[*new_faction].name)
                    )
                }
                EventKind::Migration { from, to } => {
                    format!("people move from {} to {}", town(*from), town(*to))
                }
                EventKind::Writing {
                    claim,
                    author,
                    root,
                    ..
                } => format!(
                    "{}{} writes: let the {} {}{}",
                    if *root { "ROOT EVENT — " } else { "" },
                    person(*author),
                    claim.subject,
                    if claim.negative { "not " } else { "" },
                    claim.verb
                ),
            };
            out.push_str(&format!("  {:>5}  {what}\n", e.year));
        }
    }
    out
}

/// One site: the structures at a settlement (or a single structure), their
/// rooms and their writing.
pub fn site(w: &World, settlement: usize) -> String {
    let h = &w.history;
    let Some(s) = h.settlements.get(settlement) else {
        return format!("no settlement {settlement}\n");
    };
    let lang = &w.languages[s.era as usize];
    let mut out = format!(
        "SITE {} — {} · era {} · founded {}{} · cell {},{}\n",
        settlement,
        scraped_lang::render::capitalise(&lang.romanise(&s.name)),
        s.era,
        s.founded,
        s.abandoned
            .map(|a| format!(" · abandoned {a}"))
            .unwrap_or_default(),
        s.cell.x,
        s.cell.y
    );
    for st in w
        .structures
        .iter()
        .filter(|st| st.settlement == Some(settlement))
    {
        out.push_str(&format!(
            "\n  #{} {:?} ({:?}, built {})\n",
            st.id, st.kind, st.condition, st.built
        ));
        for &t in &st.outside {
            out.push_str(&format!("      outside: {}\n", text_line(w, t)));
        }
        for (ri, room) in st.interior.rooms.iter().enumerate() {
            out.push_str(&format!(
                "    room {ri} {} (level {}){}\n",
                room.purpose,
                room.level,
                if room.collapsed { " COLLAPSED" } else { "" }
            ));
            for l in st.interior.links.iter().filter(|l| l.a == ri) {
                let state = match l.state {
                    PassageState::Open => "",
                    PassageState::Closed => " (closed)",
                    PassageState::Blocked => " (blocked)",
                };
                out.push_str(&format!(
                    "      {:?} {:?} to room {}{state}\n",
                    l.exit, l.passage, l.b
                ));
            }
            for f in &room.features {
                for &t in &f.texts {
                    out.push_str(&format!(
                        "      {} ({:?}): {}\n",
                        f.kind,
                        f.material,
                        text_line(w, t)
                    ));
                }
            }
        }
    }
    out
}

fn text_line(w: &World, t: usize) -> String {
    let text = &w.texts[t];
    let r = w.renderer(text.era);
    let english = scraped_lang::english::translate(&text.meaning, &|p| r.name(p));
    format!(
        "[era {} {}] {}  — {}",
        text.era,
        text.kind.label(),
        w.surface(text),
        english
    )
}

/// The places of a world as text (D03), for the bench: each town's role,
/// shape and districts, its scenes, and the natural features by kind.
pub fn places(w: &World) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for t in &w.towns {
        let s = &w.history.settlements[t.settlement];
        let kinds: std::collections::BTreeSet<&str> = w
            .structures
            .iter()
            .filter(|st| st.settlement == Some(s.id))
            .map(|st| st.kind.id())
            .collect();
        let scenes: Vec<&str> = w
            .scenes
            .iter()
            .filter(|sc| sc.settlement == Some(s.id))
            .map(|sc| sc.kind)
            .collect();
        let districts: Vec<&str> = t.districts.iter().map(|d| d.kind).collect();
        // DEBUG-TEXT
        let _ = writeln!(
            out,
            "settlement {} (size {}): {}, by {}, {}{}\n  districts: {}\n  buildings: {}\n  scenes: {}",
            s.id,
            s.size,
            t.role.id(),
            t.reason,
            t.plan,
            if t.walled { format!(", walled with {} gates", t.gates) } else { String::new() },
            districts.join(", "),
            kinds.into_iter().collect::<Vec<_>>().join(", "),
            scenes.join(", ")
        );
    }
    let mut by: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for f in &w.features {
        *by.entry(f.kind).or_default() += 1;
    }
    let _ = writeln!(
        out,
        "\nfeatures: {}",
        by.iter()
            .map(|(k, n)| format!("{k} {n}"))
            .collect::<Vec<_>>()
            .join(", ")
    ); // DEBUG-TEXT
    let outside: Vec<&str> = w
        .scenes
        .iter()
        .filter(|s| s.settlement.is_none())
        .map(|s| s.kind)
        .collect();
    let _ = writeln!(out, "scenes outside towns: {}", outside.join(", ")); // DEBUG-TEXT
    let mut ug: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for r in &w.underground {
        *ug.entry(r.kind).or_default() += 1;
    }
    let _ = writeln!(
        out,
        "underground, for D04: {}",
        ug.iter()
            .map(|(k, n)| format!("{k} {n}"))
            .collect::<Vec<_>>()
            .join(", ")
    ); // DEBUG-TEXT
    out
}
