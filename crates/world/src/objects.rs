//! Objects with histories (D05): the things people made, used, owned and
//! left behind, placed where their life put them, each with a reason to be
//! there: an owner, a maker, an era and, sometimes, the event that left it.
//!
//! Emblems mark who owned or made things: every faction, family line,
//! temple and era has one, and the same owner always has the same emblem,
//! so a player can learn who built or owned what without reading a word.

use serde::Serialize;

use crate::history::{History, Role};
use crate::structures::{Condition, Structure, StructureKind};

/// A family of objects, which decides where they are found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Family {
    Vessel,
    Tool,
    Coin,
    Seal,
    Measure,
    Jewellery,
    Figurine,
    Game,
    Instrument,
    Weapon,
    Armour,
    Clothing,
    Light,
    Box,
    Sky,
    Medical,
    Writing,
    Everyday,
}

/// One kind of object: its id, family, what it may be made of, and its
/// weight in rough kilograms.
#[derive(Debug, Clone, Copy)]
pub struct Kind {
    pub id: &'static str,
    pub family: Family,
    pub stuffs: &'static [&'static str],
    pub weight: u32,
}

const fn k(id: &'static str, family: Family, stuffs: &'static [&'static str], weight: u32) -> Kind {
    Kind {
        id,
        family,
        stuffs,
        weight,
    }
}

const POT: &[&str] = &["clay", "bronze", "stone"];
const FINE: &[&str] = &["gold", "silver", "bronze"];
const METAL: &[&str] = &["bronze", "iron", "copper"];
const TOOL: &[&str] = &["iron", "bronze", "wood"];
const SMALL: &[&str] = &["bone", "wood", "clay", "stone"];
const CLOTH: &[&str] = &["cloth", "leather"];
const BOX: &[&str] = &["wood", "bronze", "bone"];

/// Every object kind. Ids double as content variable values.
// DESIGN-Q: the object kinds, their families, materials and weights.
pub const KINDS: &[Kind] = &[
    // Vessels.
    k("cup", Family::Vessel, POT, 1),
    k("bowl", Family::Vessel, POT, 1),
    k("jug", Family::Vessel, POT, 2),
    k("amphora", Family::Vessel, &["clay"], 6),
    k("flask", Family::Vessel, &["clay", "glass", "bronze"], 1),
    k("beaker", Family::Vessel, &["clay", "glass"], 1),
    k("platter", Family::Vessel, POT, 2),
    k("ladle", Family::Vessel, &["bronze", "wood"], 1),
    k("cauldron", Family::Vessel, &["bronze", "iron"], 8),
    k("urn", Family::Vessel, &["clay", "stone"], 5),
    k("pitcher", Family::Vessel, POT, 2),
    // Tools of each trade.
    k("hammer", Family::Tool, TOOL, 2),
    k("chisel", Family::Tool, METAL, 1),
    k("adze", Family::Tool, TOOL, 2),
    k("sickle", Family::Tool, METAL, 1),
    k("hoe", Family::Tool, TOOL, 2),
    k("spindle", Family::Tool, &["wood", "bone"], 1),
    k("loom weight", Family::Tool, &["clay", "stone"], 1),
    k("awl", Family::Tool, &["bone", "bronze"], 1),
    k("saw", Family::Tool, METAL, 2),
    k("tongs", Family::Tool, METAL, 2),
    k("plumb bob", Family::Tool, &["stone", "bronze"], 1),
    k("fishhook", Family::Tool, &["bone", "bronze"], 1),
    k("net", Family::Tool, &["cord"], 2),
    k("trowel", Family::Tool, METAL, 1),
    k("mortar", Family::Tool, &["stone"], 4),
    k("pestle", Family::Tool, &["stone", "wood"], 1),
    k("needle", Family::Tool, &["bone", "bronze"], 1),
    // Coins and metal.
    k(
        "coin",
        Family::Coin,
        &["silver", "bronze", "gold", "copper"],
        1,
    ),
    k("ingot", Family::Coin, &["copper", "bronze", "silver"], 3),
    k("token", Family::Coin, &["clay", "bone", "bronze"], 1),
    // Seals.
    k("seal", Family::Seal, &["stone", "bone", "bronze"], 1),
    k("signet", Family::Seal, FINE, 1),
    k("seal stamp", Family::Seal, &["clay", "stone"], 1),
    // Weights and measures.
    k("weight", Family::Measure, &["stone", "bronze"], 1),
    k("balance", Family::Measure, &["bronze", "wood"], 3),
    k("measuring cup", Family::Measure, &["clay", "bronze"], 1),
    k("measuring rod", Family::Measure, &["wood", "bronze"], 2),
    // Jewellery.
    k("ring", Family::Jewellery, FINE, 1),
    k("bracelet", Family::Jewellery, FINE, 1),
    k(
        "necklace",
        Family::Jewellery,
        &["gold", "silver", "shell", "glass"],
        1,
    ),
    k("brooch", Family::Jewellery, FINE, 1),
    k("earring", Family::Jewellery, FINE, 1),
    k("diadem", Family::Jewellery, &["gold", "silver"], 1),
    k("pendant", Family::Jewellery, &["gold", "stone", "shell"], 1),
    // Figurines and sacred things.
    k(
        "figurine",
        Family::Figurine,
        &["clay", "bronze", "stone", "wood"],
        1,
    ),
    k("idol", Family::Figurine, &["stone", "wood", "gold"], 3),
    k("votive", Family::Figurine, &["clay", "bronze"], 1),
    k("mask", Family::Figurine, &["wood", "gold", "bronze"], 2),
    k("amulet", Family::Figurine, &["stone", "bone", "gold"], 1),
    // Games and toys.
    k("dice", Family::Game, SMALL, 1),
    k("game board", Family::Game, &["wood", "stone"], 2),
    k("game pieces", Family::Game, SMALL, 1),
    k("doll", Family::Game, &["wood", "cloth", "clay"], 1),
    k("spinning top", Family::Game, &["wood", "clay"], 1),
    k("rattle", Family::Game, &["clay", "wood"], 1),
    // Instruments.
    k("flute", Family::Instrument, &["bone", "reed", "wood"], 1),
    k("lyre", Family::Instrument, &["wood"], 3),
    k("drum", Family::Instrument, &["wood", "leather"], 3),
    k("horn", Family::Instrument, &["horn", "bronze"], 2),
    k("bell", Family::Instrument, &["bronze"], 2),
    k("sistrum", Family::Instrument, &["bronze"], 1),
    k("panpipes", Family::Instrument, &["reed", "clay"], 1),
    // Weapons and armour, as relics.
    k("sword", Family::Weapon, METAL, 3),
    k("spearhead", Family::Weapon, METAL, 1),
    k("arrowhead", Family::Weapon, &["bronze", "iron", "stone"], 1),
    k("axe head", Family::Weapon, METAL, 2),
    k("dagger", Family::Weapon, METAL, 1),
    k("shield boss", Family::Weapon, METAL, 2),
    k("sling", Family::Weapon, &["cord", "leather"], 1),
    k("helmet", Family::Armour, METAL, 3),
    k("breastplate", Family::Armour, &["bronze", "leather"], 6),
    k("greave", Family::Armour, &["bronze"], 2),
    // Clothing.
    k("sandal", Family::Clothing, &["leather", "reed"], 1),
    k("belt", Family::Clothing, CLOTH, 1),
    k("buckle", Family::Clothing, METAL, 1),
    k("hairpin", Family::Clothing, &["bone", "bronze", "gold"], 1),
    k("comb", Family::Clothing, &["bone", "wood", "horn"], 1),
    k("cap", Family::Clothing, CLOTH, 1),
    // Light.
    k("wick dish", Family::Light, &["clay", "bronze"], 1),
    k("candelabrum", Family::Light, &["bronze", "iron"], 4),
    k("candlestick", Family::Light, &["bronze", "clay"], 1),
    k("mirror", Family::Light, &["bronze", "silver"], 1),
    // Boxes and bags.
    k("casket", Family::Box, BOX, 2),
    k("coffer", Family::Box, &["wood", "iron"], 5),
    k("basket", Family::Box, &["reed"], 1),
    k("pouch", Family::Box, &["leather", "cloth"], 1),
    // The sky.
    k("sighting tube", Family::Sky, &["bronze", "wood"], 1),
    k("star disc", Family::Sky, &["bronze", "stone"], 2),
    k("sundial", Family::Sky, &["stone", "bronze"], 3),
    k("water clock", Family::Sky, &["clay", "bronze"], 4),
    k("calendar stone", Family::Sky, &["stone"], 8),
    // Medicine.
    k("scalpel", Family::Medical, &["bronze", "iron"], 1),
    k("probe", Family::Medical, &["bronze", "bone"], 1),
    k("salve pot", Family::Medical, &["clay", "stone"], 1),
    k("bandage roll", Family::Medical, &["cloth"], 1),
    // Writing.
    k("ink pot", Family::Writing, &["clay", "stone", "bronze"], 1),
    k("reed pen", Family::Writing, &["reed"], 1),
    k("wax tablet", Family::Writing, &["wood"], 1),
    k("ruler", Family::Writing, &["wood", "bone"], 1),
    k("inkstone", Family::Writing, &["stone"], 1),
    // Everyday things with ordinary uses.
    k("rope", Family::Everyday, &["cord"], 2),
    k("pole", Family::Everyday, &["wood"], 3),
    k("hook", Family::Everyday, &["iron", "bronze"], 1),
    k("whistle", Family::Everyday, &["bone", "clay"], 1),
    k("broom", Family::Everyday, &["reed", "wood"], 1),
    k("key", Family::Everyday, &["bronze", "iron"], 1),
    k("map", Family::Writing, &["vellum", "cloth"], 1),
];

/// The kind with this id.
pub fn kind(id: &str) -> Option<&'static Kind> {
    KINDS.iter().find(|k| k.id == id)
}

/// Every material an object can be made of.
pub fn stuffs() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = Vec::new();
    for k in KINDS {
        for s in k.stuffs {
            if !v.contains(s) {
                v.push(s);
            }
        }
    }
    v
}

/// Who owned or made something, for emblems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "of", content = "id", rename_all = "lowercase")]
pub enum Owner {
    /// A people (a faction).
    Faction(usize),
    /// A family line, by its eldest known member.
    Family(usize),
    /// A temple, by its structure.
    Temple(usize),
    /// An era's makers, by the era.
    Era(u32),
}

/// An emblem: a motif, how it's set, and its border. The same owner always
/// has the same emblem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Emblem {
    pub motif: &'static str,
    pub device: &'static str,
    pub border: &'static str,
}

/// Motifs, devices and borders of emblems. Ids double as content values.
// DESIGN-Q: the emblem vocabulary.
pub const MOTIFS: &[&str] = &[
    "bull", "lion", "falcon", "serpent", "fish", "stag", "boar", "owl", "horse", "bee", "sun",
    "moon", "star", "wave", "tree", "wheat", "lotus", "flame", "mountain", "eye", "hand", "key",
    "tower", "ship",
];
pub const DEVICES: &[&str] = &[
    "single",
    "twin",
    "crowned",
    "in a ring",
    "on a bar",
    "crossed",
];
pub const BORDERS: &[&str] = &["plain", "beaded", "rayed", "square", "none"];

fn mix(parts: &[u64]) -> u64 {
    let mut v: u64 = 0x9e37_79b9_7f4a_7c15;
    for &p in parts {
        v ^= p;
        v = v.wrapping_mul(0xbf58_476d_1ce4_e5b9);
        v ^= v >> 31;
    }
    v
}

/// The emblem of an owner in a world: fixed by the seed and the owner, so
/// it is the same wherever it appears. Factions differ in motif; family
/// lines and temples carry their own; eras differ in border, so a player
/// can date a thing by it.
pub fn emblem(seed: u64, owner: Owner) -> Emblem {
    let (tag, id) = match owner {
        Owner::Faction(i) => (1, i as u64),
        Owner::Family(i) => (2, i as u64),
        Owner::Temple(i) => (3, i as u64),
        Owner::Era(e) => (4, u64::from(e)),
    };
    let v = mix(&[seed, 0xe3b1, tag, id]);
    let pick = |n: usize, s: u32| ((v >> s) % n as u64) as usize;
    match owner {
        Owner::Era(e) => Emblem {
            motif: "",
            device: "",
            border: BORDERS[e as usize % BORDERS.len()],
        },
        _ => Emblem {
            motif: MOTIFS[pick(MOTIFS.len(), 3)],
            device: DEVICES[pick(DEVICES.len(), 17)],
            border: BORDERS[pick(BORDERS.len() - 1, 29)],
        },
    }
}

/// One object in the world.
#[derive(Debug, Clone, Serialize)]
pub struct Object {
    pub id: usize,
    pub kind: &'static str,
    pub family: Family,
    pub stuff: &'static str,
    /// Where it lies: a room of a structure.
    pub structure: usize,
    pub room: usize,
    /// Who owned it (emblem on it, if marked), and who made it (a maker's
    /// mark, by family line).
    pub owner: Owner,
    pub marked: bool,
    pub maker: Option<usize>,
    pub era: u32,
    /// How it has fared: "whole", "worn", "chipped", "broken", "mended".
    pub condition: &'static str,
    /// The event that left it here, if any (a war, a plague, a flight).
    pub event: Option<usize>,
    /// The container it lies in, unseen until that is opened.
    pub inside: Option<usize>,
    /// For a locked container: the key that opens it.
    pub key: Option<usize>,
    /// For a key: what it opens.
    pub opens: Option<Opens>,
    /// Hidden: "floor" or "wall" in a room (found by looking closer), or
    /// "buried" at a feature out on the land (found by digging); "" if not.
    pub cache: &'static str,
    /// For something buried: the feature it lies by (index into
    /// `World::features`).
    pub feature: Option<usize>,
    /// For an old map: what it shows.
    pub map: Option<OldMap>,
}

/// What a key opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "lock", rename_all = "lowercase")]
pub enum Opens {
    Object { id: usize },
    Door { structure: usize, link: usize },
}

/// An old map (D05): drawn around a settlement as the land was in its
/// era, with a cross where something was buried.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct OldMap {
    pub centre: usize,
    pub era: u32,
    /// The buried object the cross marks.
    pub cross: Option<usize>,
}

/// A locked door and its key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DoorLock {
    pub structure: usize,
    pub link: usize,
    pub key: usize,
}

/// Conditions an object may be in.
pub const CONDITIONS: &[&str] = &["whole", "worn", "chipped", "broken", "mended"];

/// The families found in a room, by what the room was for and what the
/// building was: kitchens hold vessels, treasuries coins, shrines
/// figurines, and so on.
fn families_for(purpose: &str, kind: StructureKind) -> &'static [Family] {
    use Family::*;
    let p = purpose;
    let has = |w: &[&str]| w.iter().any(|x| p.contains(x));
    if has(&[
        "kitchen",
        "refectory",
        "pantry",
        "larder",
        "hearth",
        "bakery",
    ]) {
        &[Vessel, Vessel, Tool, Light]
    } else if has(&[
        "forge", "smithy", "workshop", "kiln", "mill", "loom", "dye", "tannery",
    ]) {
        &[Tool, Tool, Vessel, Measure]
    } else if has(&["treasury", "vault", "strongroom", "counting"]) {
        &[Coin, Coin, Jewellery, Seal, Measure, Box]
    } else if has(&[
        "shrine",
        "sanctum",
        "chapel",
        "altar",
        "sanctuary",
        "nave",
        "cella",
        "oracle",
    ]) {
        &[Figurine, Figurine, Light, Instrument, Vessel]
    } else if has(&["tomb", "crypt", "burial", "ossuary", "grave", "catacomb"]) {
        &[Jewellery, Figurine, Vessel, Weapon, Clothing, Game]
    } else if has(&["barracks", "guard", "armoury", "arsenal", "gate", "tower"]) {
        &[Weapon, Weapon, Armour, Clothing, Game]
    } else if has(&[
        "library",
        "scriptorium",
        "archive",
        "writing",
        "stacks",
        "reading",
    ]) {
        &[Writing, Writing, Sky, Box, Light]
    } else if has(&["observatory", "roof"]) {
        &[Sky, Sky, Writing]
    } else if has(&["infirmary", "sick", "physician"]) {
        &[Medical, Medical, Vessel]
    } else if has(&["market", "store", "granary", "tally", "toll", "warehouse"]) {
        &[Measure, Measure, Coin, Vessel, Box, Everyday]
    } else if has(&["throne", "audience", "court"]) {
        &[Jewellery, Seal, Instrument, Light]
    } else if has(&["school", "classroom"]) {
        &[Writing, Game, Sky]
    } else if matches!(kind, StructureKind::House)
        || has(&["bed", "residence", "dormitor", "hall", "room"])
    {
        &[
            Clothing, Vessel, Game, Jewellery, Box, Everyday, Light, Instrument,
        ]
    } else if matches!(kind, StructureKind::Mine) {
        &[Tool, Everyday, Light]
    } else {
        &[Everyday, Vessel, Light]
    }
}

/// Places objects in every structure's rooms: by the room's purpose, more
/// in wealthier and holier places, fewer in ruins. Each has an owner (the
/// household's family line, the temple, or the town's people), sometimes a
/// maker, its era's style, a condition, and the event that left it, if
/// the building is evidence of one.
pub fn place(seed: u64, h: &History, structures: &[Structure]) -> Vec<Object> {
    let mut out = Vec::new();
    for st in structures {
        if matches!(st.kind, StructureKind::Cave | StructureKind::SeaCave) {
            continue;
        }
        let faction = st.settlement.map_or(0, |s| h.settlements[s].faction);
        // A household's family line: its eldest known member here.
        let family = st.settlement.and_then(|s| {
            h.people
                .iter()
                .filter(|p| p.settlement == s && p.role != Role::Ruler)
                .nth((mix(&[seed, st.id as u64, 0xfa]) % 7) as usize)
                .map(|p| eldest(h, p.id))
        });
        let makers: Vec<usize> = st
            .settlement
            .map(|s| {
                h.people
                    .iter()
                    .filter(|p| p.settlement == s && p.role == Role::Smith)
                    .map(|p| p.id)
                    .collect()
            })
            .unwrap_or_default();
        let rich = match st.condition {
            Condition::Intact => 3,
            Condition::Worn => 2,
            Condition::Damaged => 1,
            _ => 0,
        };
        for (ri, room) in st.interior.rooms.iter().enumerate() {
            if room.collapsed || room.hidden {
                continue;
            }
            let fams = families_for(room.purpose, st.kind);
            let v = mix(&[seed, st.id as u64, ri as u64, 0x0b1]);
            // Up to three things a room, fewer in poor or ruined places.
            let n = (v % (2 + rich) as u64) as usize;
            for j in 0..n.min(3) {
                let w = mix(&[v, j as u64]);
                let fam = fams[(w % fams.len() as u64) as usize];
                // Keys and maps come only with their locks and crosses.
                let kinds: Vec<&Kind> = KINDS
                    .iter()
                    .filter(|k| k.family == fam && !matches!(k.id, "key" | "map"))
                    .collect();
                let kd = kinds[((w >> 8) % kinds.len() as u64) as usize];
                let stuff = kd.stuffs[((w >> 16) % kd.stuffs.len() as u64) as usize];
                let owner = match (st.kind, family) {
                    (StructureKind::Temple | StructureKind::WaysideShrine, _) => {
                        Owner::Temple(st.id)
                    }
                    (StructureKind::House, Some(f)) => Owner::Family(f),
                    (_, Some(f)) if (w >> 24).is_multiple_of(3) => Owner::Family(f),
                    _ => Owner::Faction(faction),
                };
                let condition = match (st.condition, (w >> 28) % 6) {
                    (Condition::Ruined | Condition::Buried, x) if x < 3 => "broken",
                    (_, 0) => "worn",
                    (_, 1) => "chipped",
                    (_, 2) => "mended",
                    (Condition::Damaged, 3) => "broken",
                    _ => "whole",
                };
                out.push(Object {
                    id: out.len(),
                    kind: kd.id,
                    family: fam,
                    stuff,
                    structure: st.id,
                    room: ri,
                    owner,
                    marked: (w >> 34).is_multiple_of(2),
                    maker: (!makers.is_empty() && (w >> 36).is_multiple_of(3))
                        .then(|| makers[((w >> 40) % makers.len() as u64) as usize]),
                    era: st.era,
                    condition,
                    event: st.event,
                    inside: None,
                    key: None,
                    opens: None,
                    cache: "",
                    feature: None,
                    map: None,
                });
            }
        }
    }
    // D08: a merchant's seal lies where he traded, marked with his
    // family's emblem, for the letters that tell of the journey.
    for arc in &h.arcs {
        let Some(&first) = arc.events.first() else {
            continue;
        };
        let crate::history::EventKind::Venture { merchant, to, .. } = h.events[first].kind else {
            continue;
        };
        let store = structures
            .iter()
            .filter(|st| st.settlement == Some(to) && st.condition != Condition::Buried)
            .find(|st| {
                matches!(
                    st.kind,
                    StructureKind::Warehouse
                        | StructureKind::MarketHall
                        | StructureKind::Storehouse
                )
            })
            .or_else(|| structures.iter().find(|st| st.settlement == Some(to)));
        let Some(st) = store else { continue };
        let Some(room) = st
            .interior
            .rooms
            .iter()
            .position(|r| !r.collapsed && !r.hidden)
        else {
            continue;
        };
        out.push(Object {
            id: out.len(),
            kind: "seal",
            family: Family::Seal,
            stuff: "bronze",
            structure: st.id,
            room,
            owner: Owner::Family(eldest(h, merchant)),
            marked: true,
            maker: None,
            era: h.people[merchant].era,
            condition: "worn",
            event: Some(first),
            inside: None,
            key: None,
            opens: None,
            cache: "",
            feature: None,
            map: None,
        });
    }
    out
}

/// The eldest of a person's known line: follow kinship back.
fn eldest(h: &History, mut p: usize) -> usize {
    for _ in 0..8 {
        match &h.people[p].relation {
            Some((_, q)) if *q < p => p = *q,
            _ => break,
        }
    }
    p
}

/// A new object like `o` (same owner, era, place) of another kind.
fn like(o: &Object, id: usize, kind: &'static str, stuff: &'static str) -> Object {
    Object {
        id,
        kind,
        family: self::kind(kind).map_or(Family::Everyday, |k| k.family),
        stuff,
        marked: true,
        maker: None,
        condition: "worn",
        inside: None,
        key: None,
        opens: None,
        cache: "",
        feature: None,
        map: None,
        ..o.clone()
    }
}

/// Containers, locks, caches and old maps (D05), laid over the objects:
/// small things put into boxes and jars; some boxes and strongroom doors
/// locked, their keys (with the owner's emblem) left in another room or
/// house of the same owner; valuables hidden under floors and in walls;
/// a few things buried by features out on the land, and old maps, kept in
/// libraries, archives and palaces, with a cross where one lies.
// DESIGN-Q: how many locks, caches and maps (about one locked box in
// three, a strongroom door per palace or temple, a hidden cache per town,
// two to four buried caches and at least two maps a world).
pub fn hide_and_lock(
    seed: u64,
    h: &History,
    structures: &[Structure],
    features: &[crate::features::Feature],
    objects: &mut Vec<Object>,
) -> Vec<DoorLock> {
    let n0 = objects.len();
    // Small things into boxes and jars in the same room.
    for c in 0..n0 {
        let holds = objects[c].family == Family::Box
            || matches!(objects[c].kind, "urn" | "amphora" | "jug" | "pitcher");
        if !holds {
            continue;
        }
        let (st, room) = (objects[c].structure, objects[c].room);
        let mut put = 0;
        for o in 0..n0 {
            if put >= 2 || o == c {
                continue;
            }
            // Small valuables and personal things go into boxes and jars.
            let small = kind(objects[o].kind).is_some_and(|k| k.weight <= 1)
                && matches!(
                    objects[o].family,
                    Family::Coin
                        | Family::Seal
                        | Family::Jewellery
                        | Family::Figurine
                        | Family::Game
                        | Family::Clothing
                        | Family::Measure
                        | Family::Writing
                        | Family::Medical
                        | Family::Light
                );
            if objects[o].structure == st
                && objects[o].room == room
                && objects[o].inside.is_none()
                && small
                && objects.iter().all(|x| x.inside != Some(o))
            {
                objects[o].inside = Some(c);
                put += 1;
            }
        }
    }
    // Rooms of the same owner, for keys: another room of the building, or
    // a room of another building of the same family or town.
    let elsewhere = |objects: &[Object], o: &Object, salt: u64| -> (usize, usize) {
        let st = &structures[o.structure];
        let v = mix(&[seed, o.id as u64, salt]);
        let others: Vec<(usize, usize)> = objects
            .iter()
            .filter(|x| x.owner == o.owner && x.structure != o.structure && x.feature.is_none())
            .filter(|x| {
                !["treasury", "vault", "strongroom"].iter().any(|w| {
                    structures[x.structure].interior.rooms[x.room]
                        .purpose
                        .contains(w)
                })
            })
            .map(|x| (x.structure, x.room))
            .collect();
        if !others.is_empty() && v.is_multiple_of(2) {
            return others[((v >> 8) % others.len() as u64) as usize];
        }
        // Never in a strongroom (it may be the one the key opens).
        let strong = |r: usize| {
            ["treasury", "vault", "strongroom"]
                .iter()
                .any(|w| st.interior.rooms[r].purpose.contains(w))
        };
        let rooms: Vec<usize> = (0..st.interior.rooms.len())
            .filter(|&r| {
                r != o.room
                    && !st.interior.rooms[r].collapsed
                    && !st.interior.rooms[r].hidden
                    && !strong(r)
            })
            .collect();
        match rooms.is_empty() {
            true => (o.structure, o.room),
            false => (
                o.structure,
                rooms[((v >> 16) % rooms.len() as u64) as usize],
            ),
        }
    };
    // Locked boxes.
    for c in 0..n0 {
        if !matches!(objects[c].kind, "coffer" | "casket")
            || !mix(&[seed, c as u64, 0x10c]).is_multiple_of(3)
        {
            continue;
        }
        let id = objects.len();
        let (ks, kr) = elsewhere(objects, &objects[c], 0x4e1);
        let mut key = like(&objects[c], id, "key", "bronze");
        key.structure = ks;
        key.room = kr;
        key.opens = Some(Opens::Object { id: c });
        objects[c].key = Some(id);
        objects[c].marked = true;
        objects.push(key);
    }
    // Strongroom doors: the door into a treasury, vault or strongroom.
    let mut locks = Vec::new();
    for st in structures {
        // Not where old writing is kept: its vaults stay open to readers.
        if matches!(st.kind, StructureKind::Archive | StructureKind::Library) {
            continue;
        }
        let Some(link) = st.interior.links.iter().position(|l| {
            l.passage == crate::structures::Passage::Door
                && ["treasury", "vault", "strongroom"]
                    .iter()
                    .any(|w| st.interior.rooms[l.b].purpose.contains(w))
        }) else {
            continue;
        };
        let room = st.interior.links[link].a;
        let owner = match st.kind {
            StructureKind::Temple => Owner::Temple(st.id),
            _ => Owner::Faction(st.settlement.map_or(0, |s| h.settlements[s].faction)),
        };
        let id = objects.len();
        let base = Object {
            id,
            kind: "key",
            family: Family::Everyday,
            stuff: "iron",
            structure: st.id,
            room,
            owner,
            marked: true,
            maker: None,
            era: st.era,
            condition: "worn",
            event: None,
            inside: None,
            key: None,
            opens: Some(Opens::Door {
                structure: st.id,
                link,
            }),
            cache: "",
            feature: None,
            map: None,
        };
        let (ks, kr) = elsewhere(objects, &base, 0xd00);
        objects.push(Object {
            structure: ks,
            room: kr,
            ..base
        });
        locks.push(DoorLock {
            structure: st.id,
            link,
            key: id,
        });
    }
    // A cache in one house or temple room of each settlement.
    for s in &h.settlements {
        let homes: Vec<&Structure> = structures
            .iter()
            .filter(|st| {
                st.settlement == Some(s.id)
                    && matches!(st.kind, StructureKind::House | StructureKind::Temple)
                    && st.condition != Condition::Buried
            })
            .collect();
        if homes.is_empty() {
            continue;
        }
        let v = mix(&[seed, s.id as u64, 0xcac]);
        let st = homes[(v % homes.len() as u64) as usize];
        let room = ((v >> 8) % st.interior.rooms.len() as u64) as usize;
        if st.interior.rooms[room].collapsed || st.interior.rooms[room].hidden {
            continue;
        }
        let owner = match st.kind {
            StructureKind::Temple => Owner::Temple(st.id),
            _ => Owner::Faction(s.faction),
        };
        let hoard = ["coin", "ring", "bracelet", "ingot", "necklace"][((v >> 16) % 5) as usize];
        let id = objects.len();
        objects.push(Object {
            id,
            kind: hoard,
            family: kind(hoard).map_or(Family::Coin, |k| k.family),
            stuff: if (v >> 20).is_multiple_of(2) {
                "silver"
            } else {
                "gold"
            },
            structure: st.id,
            room,
            owner,
            marked: true,
            maker: None,
            era: st.era,
            condition: "whole",
            event: st.event,
            inside: None,
            key: None,
            opens: None,
            cache: if (v >> 24).is_multiple_of(2) {
                "floor"
            } else {
                "wall"
            },
            feature: None,
            map: None,
        });
    }
    // Things buried by features near towns, and old maps with a cross.
    let near = |f: &crate::features::Feature, s: &crate::history::Settlement| {
        let dx = f.cell.x as i64 - s.cell.x as i64;
        let dy = f.cell.y as i64 - s.cell.y as i64;
        dx * dx + dy * dy <= 40 * 40
    };
    let markable = [
        "cairn",
        "standing stones",
        "ancient tree",
        "boulder field",
        "burial mound",
        "rock pillar",
        "natural arch",
        "spring",
    ];
    let keepers: Vec<&Structure> = structures
        .iter()
        .filter(|st| {
            matches!(
                st.kind,
                StructureKind::Library
                    | StructureKind::Archive
                    | StructureKind::Palace
                    | StructureKind::Temple
            ) && st.settlement.is_some()
                && st.condition != Condition::Buried
        })
        .collect();
    let mut maps = 0;
    for (n, st) in keepers.iter().enumerate() {
        if maps >= 4 {
            break;
        }
        let s = &h.settlements[st.settlement.expect("a town")];
        let Some(f) = features
            .iter()
            .filter(|f| markable.contains(&f.kind) && near(f, s))
            .find(|f| !objects.iter().any(|o| o.feature == Some(f.id)))
        else {
            continue;
        };
        let v = mix(&[seed, n as u64, 0xb0b]);
        let hoard = ["coin", "ingot", "diadem", "amulet", "signet"][(v % 5) as usize];
        let buried = objects.len();
        objects.push(Object {
            id: buried,
            kind: hoard,
            family: kind(hoard).map_or(Family::Coin, |k| k.family),
            stuff: "gold",
            structure: st.id,
            room: 0,
            owner: Owner::Faction(s.faction),
            marked: true,
            maker: None,
            era: st.era,
            condition: "whole",
            event: None,
            inside: None,
            key: None,
            opens: None,
            cache: "buried",
            feature: Some(f.id),
            map: None,
        });
        let rooms: Vec<usize> = (0..st.interior.rooms.len())
            .filter(|&r| !st.interior.rooms[r].collapsed && !st.interior.rooms[r].hidden)
            .collect();
        let room = rooms[((v >> 8) % rooms.len().max(1) as u64) as usize % rooms.len().max(1)];
        let id = objects.len();
        objects.push(Object {
            id,
            kind: "map",
            family: Family::Writing,
            stuff: "vellum",
            structure: st.id,
            room,
            owner: Owner::Faction(s.faction),
            marked: true,
            maker: None,
            era: st.era,
            condition: "worn",
            event: None,
            inside: None,
            key: None,
            opens: None,
            cache: "",
            feature: None,
            map: Some(OldMap {
                centre: s.id,
                era: st.era,
                cross: Some(buried),
            }),
        });
        maps += 1;
    }
    locks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_are_many_and_unique() {
        assert!(KINDS.len() >= 100, "{}", KINDS.len());
        let mut ids: Vec<&str> = KINDS.iter().map(|k| k.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), KINDS.len());
    }

    #[test]
    fn the_same_owner_has_the_same_emblem() {
        for i in 0..20 {
            assert_eq!(emblem(7, Owner::Faction(i)), emblem(7, Owner::Faction(i)));
        }
        let all: std::collections::BTreeSet<(&str, &str)> = (0..20)
            .map(|i| {
                let e = emblem(7, Owner::Family(i));
                (e.motif, e.device)
            })
            .collect();
        assert!(all.len() >= 12, "family emblems too alike: {}", all.len());
    }
}
