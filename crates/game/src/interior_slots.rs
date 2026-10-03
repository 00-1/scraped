//! The slots of great interiors (D04): moving inside with honest
//! directions and distances, long passages told in a line, windows onto
//! places not yet reached, hidden ways found, marks the player makes, and
//! spaces named so they can be found again.

use scraped_content::{Context, SlotDef, Value, VarType};

use crate::quiet_slots::{styles, ROOM_SIZES};
use crate::site::ctx;
use crate::slots::{PASSAGES, PURPOSES};

fn e(values: &[&str]) -> VarType {
    VarType::Enum {
        values: values.iter().map(|s| s.to_string()).collect(),
    }
}

/// Directions inside: the four sides, up and down.
pub const INSIDE_DIRECTIONS: &[&str] = &["north", "south", "east", "west", "up", "down"];

fn s_report(_: u64) -> Vec<Context> {
    INSIDE_DIRECTIONS
        .iter()
        .enumerate()
        .map(|(i, d)| {
            ctx(&[
                ("direction", Value::from(*d)),
                ("passage", Value::from(PASSAGES[i % PASSAGES.len()])),
                ("metres", Value::Number(4 + 6 * i as i64)),
                ("dark", Value::Bool(i % 3 == 2)),
                ("north", Value::Number(if i == 0 { 4 } else { -(i as i64) })),
                ("east", Value::Number(2 * i as i64)),
            ])
        })
        .collect()
}

fn s_thing(_: u64) -> Vec<Context> {
    vec![
        ctx(&[("thing", Value::from("a window east"))]),
        ctx(&[("thing", Value::from("a door north"))]),
    ]
}

fn s_run(_: u64) -> Vec<Context> {
    (0..4)
        .map(|i| {
            ctx(&[
                ("spaces", Value::Number(2 + i)),
                ("metres", Value::Number(10 + 12 * i)),
                ("turns", Value::Number(i % 3)),
                ("branches", Value::Number(i / 2)),
                ("direction", Value::from(INSIDE_DIRECTIONS[i as usize % 4])),
            ])
        })
        .collect()
}

fn s_walk(_: u64) -> Vec<Context> {
    (0..3)
        .map(|i| {
            ctx(&[
                (
                    "name",
                    Value::from(["the great hall", "the stair", "the dry pool"][i]),
                ),
                ("spaces", Value::Number(1 + 3 * i as i64)),
                ("metres", Value::Number(12 + 20 * i as i64)),
            ])
        })
        .collect()
}

fn s_window(_: u64) -> Vec<Context> {
    PURPOSES
        .iter()
        .take(8)
        .enumerate()
        .map(|(i, p)| {
            ctx(&[
                ("direction", Value::from(INSIDE_DIRECTIONS[i % 4])),
                ("purpose", Value::from(*p)),
                (
                    "space",
                    Value::from(scraped_world::interiors::SPACES[i % 5]),
                ),
                ("lit", Value::Bool(i % 2 == 0)),
            ])
        })
        .collect()
}

fn s_found(_: u64) -> Vec<Context> {
    ["panel", "crawlway"]
        .iter()
        .enumerate()
        .map(|(i, p)| {
            ctx(&[
                ("passage", Value::from(*p)),
                ("direction", Value::from(INSIDE_DIRECTIONS[i])),
            ])
        })
        .collect()
}

fn s_name(_: u64) -> Vec<Context> {
    PURPOSES
        .iter()
        .enumerate()
        .map(|(i, p)| {
            ctx(&[
                ("purpose", Value::from(*p)),
                (
                    "space",
                    Value::from(
                        scraped_world::interiors::SPACES
                            [i % scraped_world::interiors::SPACES.len()],
                    ),
                ),
                ("size", Value::from(ROOM_SIZES[i % ROOM_SIZES.len()])),
                ("style", Value::from(styles()[i % styles().len()])),
                ("level", Value::Number((i % 5) as i64 - 2)),
                ("marked", Value::Bool(i % 4 == 0)),
                (
                    "landmark",
                    Value::from(crate::quiet_slots::LANDMARKS[i % 4]),
                ),
            ])
        })
        .collect()
}

fn s_none(_: u64) -> Vec<Context> {
    vec![Context::new()]
}

/// Every slot of great interiors.
pub fn slots() -> Vec<SlotDef> {
    vec![
        SlotDef::new("move.report", "Which way the player went inside and how far, honestly, so they can draw a map ('North through the door, some 12 metres.'). Said before the next room. In the dark, only the direction is felt: no distance.")
            .var("direction", e(INSIDE_DIRECTIONS), "Which way.")
            .var("passage", e(PASSAGES), "What they went through.")
            .var("metres", VarType::Number, "From the middle of one space to the middle of the next, to the nearest 2 m.")
            .var("dark", VarType::Bool, "Whether they moved by feel.")
            .var("north", VarType::Number, "How far north the move went, in metres (negative: south), so a careful player's map comes out true.")
            .var("east", VarType::Number, "How far east (negative: west).")
            .max_len(100)
            .sampler(s_report),
        SlotDef::new("move.window", "The player tries to go through a window, grating or gallery rail: it can be seen through, never passed.")
            .var("thing", VarType::Text, "The way, as named by place.exit.")
            .sampler(s_thing),
        SlotDef::new("move.against", "A door won't open from this side: barred or latched on the far side. Someone on the other side could open it.")
            .var("thing", VarType::Text, "The way, as named by place.exit.")
            .sampler(s_thing),
        SlotDef::new("move.run", "Following a long passage, told in a line rather than space by space ('The passage winds on, branching twice, some 40 metres.').")
            .var("spaces", VarType::Number, "How many stretches of passage were passed through.")
            .var("metres", VarType::Number, "How far, to the nearest 2 m.")
            .var("turns", VarType::Number, "How many times it turned.")
            .var("branches", VarType::Number, "How many side ways were passed.")
            .var("direction", e(INSIDE_DIRECTIONS), "The way it was heading at the end.")
            .max_len(140)
            .sampler(s_run),
        SlotDef::new("move.walk", "The player walks back to a space they know, by the ways they know ('You make your way back to the great hall.').")
            .var("name", VarType::Text, "Where to, as named by room.name.")
            .var("spaces", VarType::Number, "How many spaces they passed through.")
            .var("metres", VarType::Number, "How far, to the nearest 2 m.")
            .max_len(120)
            .sampler(s_walk),
        SlotDef::new("move.no_way", "The player asks to go back to a space they know, but no way they know leads there from here.")
            .var("name", VarType::Text, "Where they wanted to go.")
            .sampler(s_walk),
        SlotDef::new("room.window", "Through a window, grating or gallery here, a space the player can see but not (yet) reach. Show what is visible; it invites finding the way round.")
            .var("direction", e(INSIDE_DIRECTIONS), "Which way it lies.")
            .var("purpose", e(PURPOSES), "What the space beyond was for.")
            .var("space", e(scraped_world::interiors::SPACES), "Its shape.")
            .var("lit", VarType::Bool, "Whether light reaches it.")
            .max_len(140)
            .sampler(s_window),
        SlotDef::new("room.found", "Looking closely, the player finds a hidden way: a loose panel, a crawlway behind rubble, a stone that turns. It is a way from now on.")
            .var("passage", e(PASSAGES), "What kind of way (panel, crawlway…).")
            .var("direction", e(INSIDE_DIRECTIONS), "Where it leads.")
            .max_len(140)
            .sampler(s_found),
        SlotDef::new("hazard.air", "The air here is bad (deep in a mine, cave or catacomb): a flame gutters, breath comes short, a headache. Staying hurts; going back up helps. Physical cues, no explanation.")
            .max_len(120)
            .sampler(s_none),
        SlotDef::new("hazard.air_hurt", "The player has stayed too long in bad air and it hurts them (dizziness, a pounding head).")
            .max_len(100)
            .sampler(s_none),
        SlotDef::new("move.rope", "The player climbs back up a drop (a hole or shaft) by the rope they carry: slow, hand over hand.")
            .max_len(100)
            .sampler(s_none),
        SlotDef::new("move.squeeze", "The player squeezes through a tight crawl with a heavy load, slowly, scraping and pushing the load ahead.")
            .max_len(100)
            .sampler(s_none),
        SlotDef::new("room.mark", "A mark the player made here earlier (chalk, scratches) is on the wall: they have been here.")
            .max_len(80)
            .sampler(s_none),
        SlotDef::new("mark.done", "The player marks the wall of this space, to know it again.")
            .max_len(80)
            .sampler(s_none),
        SlotDef::new("room.name", "A space the player has been in, as they name it to go back ('the vaulted hall', 'the stair down', 'the marked passage'). Include a word the player would type; it must tell alike spaces apart where it can (by style, size, level or a mark).")
            .var("purpose", e(PURPOSES), "What it was for.")
            .var("space", e(scraped_world::interiors::SPACES), "Its shape.")
            .var("size", e(ROOM_SIZES), "How big.")
            .var("style", e(&styles()), "How it was built.")
            .var("level", VarType::Number, "Floor: 0 ground, below 0 underground.")
            .var("marked", VarType::Bool, "Whether the player marked it.")
            .var("landmark", e(&crate::quiet_slots::LANDMARKS), "What makes it one to know again: vast, lofty, lone; empty for most.")
            .max_len(60)
            .sampler(s_name),
    ]
}
