//! The slots of reading in layers (S01): the whole text at a glance, a
//! sign examined, a sign or a text traced stroke by stroke, and a sign's
//! name as the player refers to it.

use scraped_content::{Context, SlotDef, Value, VarType};
use scraped_lang::impression::RESEMBLANCES;

use crate::site::ctx;
use crate::slots::{MATERIALS, WRITING};
use crate::writing::HANDS;

fn e(values: &[&str]) -> VarType {
    VarType::Enum {
        values: values.iter().map(|s| s.to_string()).collect(),
    }
}

const OUTLINES: [&str; 6] = ["round", "tall", "wide", "angular", "slight", "plain"];
const STROKES: [&str; 9] = [
    "bar", "hook", "dot", "ring", "arc", "wedge", "tail", "cross", "zigzag",
];

fn s_whole(_: u64) -> Vec<Context> {
    (0..6)
        .map(|i| {
            let glyphs = [4, 12, 30, 64, 140, 9][i];
            ctx(&[
                (
                    "thing",
                    Value::from(["the stele", "a jar", "the lintel"][i % 3]),
                ),
                ("material", Value::from(MATERIALS[i % MATERIALS.len()])),
                ("hand", Value::from(HANDS[i % HANDS.len()])),
                ("glyphs", Value::Number(glyphs)),
                ("words", Value::Number(glyphs / 4 + 1)),
                ("lines", Value::Number(glyphs / 12 + 1)),
                ("texts", Value::Number(1 + (i as i64 % 3))),
                ("direction", Value::from(WRITING[i % WRITING.len()])),
                ("recurring", Value::Bool(glyphs >= 30)),
                ("scraped", Value::Bool(i == 4)),
            ])
        })
        .collect()
}

fn s_closer(_: u64) -> Vec<Context> {
    (0..6)
        .map(|i| {
            let heard = i % 3 == 1;
            ctx(&[
                ("number", Value::Number(1 + 3 * i as i64)),
                (
                    "impression",
                    Value::from(
                        [
                            "a tall sign, a hook",
                            "a round sign, a ring like an eye",
                            "like a tall sign, a hook, with a dot at the bottom",
                        ][i % 3],
                    ),
                ),
                ("outline", Value::from(OUTLINES[i % OUTLINES.len()])),
                ("main", Value::from(STROKES[i % STROKES.len()])),
                (
                    "resembles",
                    Value::from(RESEMBLANCES[i % RESEMBLANCES.len()]),
                ),
                ("count", Value::Number(1 + i as i64 % 4)),
                (
                    "distinctive",
                    Value::from(["a hook turned left, at the top", "a dot, in the middle"][i % 2]),
                ),
                ("hand", Value::from(HANDS[i % HANDS.len()])),
                ("heard", Value::Bool(heard)),
                ("sound", Value::from(if heard { "ka" } else { "" })),
            ])
        })
        .collect()
}

fn s_number(_: u64) -> Vec<Context> {
    [1, 4, 12, 40]
        .iter()
        .map(|&n| ctx(&[("number", Value::Number(n))]))
        .collect()
}

fn s_trace_sign(_: u64) -> Vec<Context> {
    [
        "A hook turned left, at the top; a bar upright, in the middle.",
        "A ring, in the middle; a dot, at the right.",
    ]
    .iter()
    .enumerate()
    .map(|(i, d)| {
        ctx(&[
            ("number", Value::Number(1 + i as i64)),
            ("description", Value::from(*d)),
        ])
    })
    .collect()
}

fn s_trace_frame(_: u64) -> Vec<Context> {
    [(1, 1, 30), (1, 8, 30), (9, 16, 30), (4, 4, 4)]
        .iter()
        .map(|&(from, to, glyphs)| {
            ctx(&[
                ("thing", Value::from("the stele")),
                ("from", Value::Number(from)),
                ("to", Value::Number(to)),
                ("glyphs", Value::Number(glyphs)),
                ("minutes", Value::Number(4 * (to - from + 1))),
            ])
        })
        .collect()
}

fn s_remaining(_: u64) -> Vec<Context> {
    [1, 8, 40]
        .iter()
        .map(|&n| ctx(&[("remaining", Value::Number(n))]))
        .collect()
}

fn s_heard(_: u64) -> Vec<Context> {
    [
        (vec!["ka", "ti", "mo"], vec!["stop", "nasal", "vowel"], true),
        (vec!["s", "e"], vec!["fricative", "vowel"], false),
        (vec!["n"], vec!["nasal"], true),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (sounds, manners, quiet))| {
        ctx(&[
            ("first", Value::from(sounds[0])),
            ("count", Value::Number(sounds.len() as i64)),
            (
                "sounds",
                Value::List(sounds.into_iter().map(Value::from).collect()),
            ),
            (
                "manners",
                Value::List(manners.into_iter().map(Value::from).collect()),
            ),
            ("new", Value::Number([3, 0, 1][i])),
            ("quiet", Value::Bool(quiet)),
        ])
    })
    .collect()
}

fn s_thing(_: u64) -> Vec<Context> {
    vec![ctx(&[("thing", Value::from("the stele"))])]
}

/// Every slot of reading in layers.
pub fn slots() -> Vec<SlotDef> {
    vec![
        SlotDef::new("read.whole", "The player reads a thing: the whole text at a glance, never sign by sign ('Six short lines cut deep into the stone, in a heavy hand; a few shapes keep coming back.'). How much writing, in how many lines, how it was made, its state. Reading closely ('read closely') goes sign by sign. Never reveals meaning.")
            .var("thing", VarType::Text, "The thing, as named by thing.name.")
            .var("material", e(MATERIALS), "The surface.")
            .var("hand", e(HANDS), "The look of the hand (the most recent visible layer). The same scribe always has the same hand.")
            .var("glyphs", VarType::Number, "How many signs.")
            .var("words", VarType::Number, "How many groups of signs (words, as far as the eye can tell).")
            .var("lines", VarType::Number, "About how many lines or bands.")
            .var("texts", VarType::Number, "How many separate pieces of writing are on the thing.")
            .var("direction", e(WRITING), "Which way the writing runs (a reader may not know yet).")
            .var("recurring", VarType::Bool, "Whether a few shapes keep coming back (a long text where one sign is common).")
            .var("scraped", VarType::Bool, "Whether the writing seen was scraped (only part of it survives; read.scraped follows).")
            .max_len(220)
            .sampler(s_whole),
        SlotDef::new("glyph.closer", "The player examines one sign of a text closely: a fuller impression than a close reading gives (its proportions, its most distinctive part, what it resembles, how it is cut), still how it looks, never how it is built stroke by stroke (tracing gives that). If its sound has been heard, it can say so.")
            .var("number", VarType::Number, "The sign's position in the text, from 1.")
            .var("impression", VarType::Text, "The sign at a glance, from glyph.impression.")
            .var("outline", e(&OUTLINES), "Its overall outline.")
            .var("main", e(&STROKES), "The stroke that carries it.")
            .var("resembles", e(RESEMBLANCES), "What it looks like ('' if nothing).")
            .var("count", VarType::Number, "How many strokes.")
            .var("distinctive", VarType::Text, "Its most distinctive part, from glyph.stroke.")
            .var("hand", e(HANDS), "The hand it is written in.")
            .var("heard", VarType::Bool, "Whether the player has heard its sound.")
            .var("sound", VarType::Text, "Its sound, romanised, if heard.")
            .max_len(260)
            .sampler(s_closer),
        SlotDef::new("sign.name", "A sign of the text being read, as the player refers to it ('sign 4', 'the fourth sign'). Must contain the number.")
            .var("number", VarType::Number, "Its position in the text, from 1.")
            .max_len(30)
            .sampler(s_number),
        SlotDef::new("trace.frame", "The player traces signs (or makes a rubbing): a slow, careful task, minutes a sign. Before the strokes of the signs traced, one per line.")
            .var("thing", VarType::Text, "What they trace from, as named by thing.name.")
            .var("from", VarType::Number, "The first sign traced (from 1).")
            .var("to", VarType::Number, "The last sign traced.")
            .var("glyphs", VarType::Number, "How many signs the text has in all.")
            .var("minutes", VarType::Number, "How long it took.")
            .max_len(160)
            .sampler(s_trace_frame),
        SlotDef::new("trace.sign", "One traced sign: its number and its exact strokes, precise enough to draw. Told once; the game doesn't keep it.")
            .var("number", VarType::Number, "The sign's position in the text.")
            .var("description", VarType::Text, "Its strokes, from glyph.describe.")
            .max_len(400)
            .sampler(s_trace_sign),
        SlotDef::new("trace.lost", "A sign that can't be traced: scraped or worn past making out. Keep the number visible.")
            .var("number", VarType::Number, "The sign's position.")
            .max_len(100)
            .sampler(s_number),
        SlotDef::new("trace.more", "After tracing part of a text: more remains, and tracing it again carries on where they left off.")
            .var("remaining", VarType::Number, "Signs left to trace.")
            .max_len(100)
            .sampler(s_remaining),
        SlotDef::new("glyph.heard", "Scraping writing away, the player hears each sign give its sound, faintly, as its strokes come away (S01): one of the first uncanny hints that writing is more than marks. Faint and easy to miss; it doesn't announce itself. From now on these signs show by their sounds in a close reading. Write the sounds as the romanisation gives them; how hearing them feels is yours.")
            .var("sounds", VarType::List, "The sounds heard, romanised, in the order they went (each once).")
            .var("first", VarType::Text, "The first sound heard.")
            .var("count", VarType::Number, "How many different sounds.")
            .var("manners", VarType::List, "What kinds of sounds they are, each once: stop, affricate, fricative, nasal, lateral, rhotic, glide, vowel.")
            .var("new", VarType::Number, "How many signs' sounds the player hadn't heard before.")
            .var("quiet", VarType::Bool, "Whether it was quiet here (else the player was listening hard).")
            .max_len(200)
            .sampler(s_heard),
        SlotDef::new("trace.dark", "The player tries to trace in too little light.")
            .var("thing", VarType::Text, "What they would trace from (may be empty).")
            .max_len(100)
            .sampler(s_thing),
    ]
}
