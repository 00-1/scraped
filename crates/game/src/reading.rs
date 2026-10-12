//! Reading in layers (S01, `docs/DESIGN.md`, "How the script is
//! perceived"): `read` gives the whole text at a glance; `read closely`
//! gives each sign as it looks (or by its sound, once heard); examining one
//! sign gives a fuller impression; only tracing gives the exact strokes,
//! and it takes time and light. The game keeps no list of signs: keeping
//! track is the player's job (`docs/DESIGN.md`, "The game is not your
//! notebook"). The one thing it keeps is the sounds heard (`writing.rs`).

use std::collections::BTreeMap;

use scraped_content::{Context, Renderer, Value};
use scraped_lang::script::GlyphKey;
use scraped_lang::slots::{impression_texts, LangHooks};
use scraped_sim::outdoors::label;
use serde_json::json;

use crate::parser::Candidate;
use crate::site::ctx;
use crate::{Game, Mark, Output, Reading, Rendered, Target, PAGE};

/// About how many signs a line holds, to tell how many lines a text runs
/// to.
const LINE: usize = 12;

/// Texts this short are read closely at once.
const SHORT: usize = 6;

/// Minutes to trace one sign.
// DESIGN-Q: tracing takes 4 minutes a sign (a whole stele is hours).
pub const TRACE_MINUTES: u32 = 4;

/// Signs traced at one go before the player is asked to carry on.
const TRACE_PAGE: usize = 8;

/// Ordinal words the player may use for signs ("the fourth sign").
const ORDINALS: [&str; 20] = [
    "first",
    "second",
    "third",
    "fourth",
    "fifth",
    "sixth",
    "seventh",
    "eighth",
    "ninth",
    "tenth",
    "eleventh",
    "twelfth",
    "thirteenth",
    "fourteenth",
    "fifteenth",
    "sixteenth",
    "seventeenth",
    "eighteenth",
    "nineteenth",
    "twentieth",
];

impl Game {
    /// A sign as it looks at a glance, through `glyph.impression`. Stable:
    /// the same sign always gives the same impression.
    pub(crate) fn sign_impression(&mut self, era: u32, index: usize) -> String {
        // What changes the impressions: sounds heard, and the word for
        // marks (S06).
        let heard_here = self.state.heard.iter().filter(|h| h.0 == era).count()
            + if self.state.studied.len() >= crate::STUDIED_FOR_SIGNS {
                1 << 20
            } else {
                0
            };
        if let Some(t) = self
            .impression_cache
            .get(&era)
            .filter(|(n, _)| *n == heard_here)
            .and_then(|(_, v)| v.get(index))
        {
            return t.clone();
        }
        let confusable = self.preset == "archaeologist";
        let lang = &self.site.world.languages[era as usize];
        let hooks = LangHooks { lang };
        let seed = self.seed() ^ (u64::from(era) << 40 | 0x1e55);
        let ambient = self.ambient();
        let mut r = Renderer::new(&self.registry, &self.pack, seed, &hooks);
        r.ambient = ambient;
        let heard = |j: usize| {
            self.state
                .heard
                .contains(&(era, j))
                .then(|| self.sign_sound(era, j))
                .flatten()
        };
        let texts = impression_texts(&mut r, &lang.script, confusable, &heard);
        let traces = std::mem::take(&mut r.trace);
        drop(r);
        let minutes = self.state.minutes;
        self.renders
            .extend(traces.into_iter().map(|trace| Rendered {
                trace,
                vars: Context::new(),
                minutes,
                seed,
            }));
        let t = texts.get(index).cloned().unwrap_or_default();
        self.impression_cache.insert(era, (heard_here, texts));
        t
    }

    /// What a sign sounds like, romanised, if it has a sound at all
    /// (numerals, dividers and the abjad's vowel carrier don't).
    pub(crate) fn sign_sound(&self, era: u32, index: usize) -> Option<String> {
        let lang = &self.site.world.languages[era as usize];
        let roman = |ipa: &str| {
            lang.phonology
                .inventory
                .by_ipa(ipa)
                .map(|id| lang.phonology.inventory.get(id).roman.clone())
        };
        match &lang.script.glyphs.get(index)?.0 {
            GlyphKey::Sound(s) => roman(s),
            GlyphKey::Syllable(c, v) => Some(format!(
                "{}{}",
                c.and_then(roman).unwrap_or_default(),
                roman(v)?
            )),
            GlyphKey::Dead(c) => roman(c),
            _ => None,
        }
    }

    /// Whether the player has heard this sign's sound.
    pub(crate) fn heard(&self, era: u32, index: usize) -> bool {
        self.state.heard.contains(&(era, index))
    }

    /// The signs of a thing in reading order: era, index in its script,
    /// and whether it is lost.
    pub fn signs(&self, thing: usize) -> Vec<(u32, usize, bool)> {
        self.marks(thing)
            .into_iter()
            .filter_map(|m| match m {
                Mark::Glyph { era, index, lost } => Some((era, index, lost)),
                _ => None,
            })
            .collect()
    }

    /// What the truth behind a reading is, for the JSON protocol.
    fn reading_truth(&self, thing: usize) -> serde_json::Value {
        json!(self
            .thing(thing)
            .texts
            .iter()
            .map(|&tid| {
                let text = self.text(tid);
                let r = self.site.world.renderer(text.era);
                let state = if self.state.scraped.contains(&tid) {
                    "scraped"
                } else {
                    "unscraped"
                };
                json!({
                    "era": text.era,
                    "kind": text.kind,
                    "state": state,
                    "text": self.site.world.surface(text),
                    "translation": scraped_lang::english::translate(&text.meaning, &|p| r.name(p)),
                })
            })
            .collect::<Vec<_>>())
    }

    /// Notes that these texts were read, for the record and the hooks.
    fn note_read(&mut self, thing: usize) {
        let seen = self.layers_seen(thing);
        let read: Vec<usize> = seen.iter().map(|x| x.0).collect();
        self.encounter(&read);
        if seen.iter().any(|x| x.1) {
            self.hook("first_scraped_seen", "", "");
        }
        if read.iter().any(|t| self.site.writing.deep.contains(t)) {
            self.hook("deepest_found", "", "");
        }
    }

    /// `read the stele`: the whole text at a glance. How much there is,
    /// how it is made, how it lies, whether it was scraped and what shows
    /// beneath; never sign by sign. Short texts go on to a close reading.
    pub(crate) fn read_whole(&mut self, thing: usize) -> Output {
        self.state.reading = Some(Reading { thing, page: 0 });
        self.state.last_read = Some(thing);
        self.reading_now = true;
        self.pass(1);
        let marks = self.marks(thing);
        let signs = self.signs(thing);
        let words = marks
            .split(|m| !matches!(m, Mark::Glyph { .. }))
            .filter(|w| !w.is_empty())
            .count();
        // Some shapes keep coming back: the commonest sign is a good share.
        let mut counts: BTreeMap<(u32, usize), usize> = BTreeMap::new();
        for &(era, index, lost) in &signs {
            if !lost {
                *counts.entry((era, index)).or_default() += 1;
            }
        }
        let recurring = signs.len() >= 12 && counts.values().any(|&n| n * 8 >= signs.len());
        let seen = self.layers_seen(thing);
        // The piece being studied (S07), and how the others look beside it.
        let k = self.piece(thing);
        let top = self
            .piece_layer(thing, k)
            .map_or(self.thing(thing).texts[0], |l| l.0);
        let deep = self.deep_layers(thing);
        let others: Vec<Value> = (0..seen.len())
            .filter(|&j| j != k)
            .filter_map(|j| self.piece_layer(thing, j))
            .map(|(tid, partial)| {
                let mut d = Vec::new();
                d.push(if partial || deep.contains(&tid) {
                    "fainter"
                } else {
                    "as clear"
                });
                if self.hand(tid) != self.hand(top) {
                    d.push("another hand");
                }
                if self.text(tid).era != self.text(top).era {
                    d.push("cut differently");
                }
                Value::from(d.join(", "))
            })
            .collect();
        let hand = self.hand(top);
        let era = self.text(top).era as usize;
        let name = self.thing_name(thing);
        let t = self.thing(thing);
        let c = ctx(&[
            ("thing", Value::from(name)),
            ("material", Value::from(label(&t.material))),
            ("hand", Value::from(hand)),
            ("glyphs", Value::Number(signs.len() as i64)),
            ("words", Value::Number(words as i64)),
            ("lines", Value::Number(signs.len().div_ceil(LINE) as i64)),
            ("texts", Value::Number(t.texts.len() as i64)),
            (
                "direction",
                Value::from(label(&self.site.world.languages[era].script.direction)),
            ),
            ("recurring", Value::Bool(recurring)),
            ("scraped", Value::Bool(seen.first().is_some_and(|l| l.1))),
            ("form", Value::from(form(self.text(top).genre))),
            ("pieces", Value::Number(seen.len() as i64)),
            ("piece", Value::Number(k as i64)),
            ("others", Value::List(others)),
        ]);
        let mut parts = vec![self.say("read.whole", c)];
        if let Some(cover) = self.grime(thing) {
            parts.push(self.say("read.grime", ctx(&[("cover", Value::from(cover))])));
        }
        if let Some(&(_, true)) = seen.first() {
            let lost = signs.iter().filter(|s| s.2).count();
            // S03: an impression, not a count ('read closely' shows each
            // lost sign).
            let share = lost * 100 / signs.len().max(1);
            let amount = match share {
                0..=15 => "a few",
                16..=40 => "some",
                41..=60 => "about half",
                61..=85 => "most",
                _ => "nearly all",
            };
            let c = ctx(&[
                ("material", Value::from(label(&self.thing(thing).material))),
                ("lost", Value::from(amount)),
            ]);
            parts.push(self.say("read.scraped", c));
        }
        let deep = self.deep_layers(thing);
        if !deep.is_empty() {
            let c = ctx(&[("count", Value::Number(deep.len() as i64))]);
            parts.push(self.say("read.deep", c));
        }
        if self.site.writing.legacy.is_some_and(|l| deep.contains(&l)) {
            parts.push(self.say("read.legacy", Context::new()));
        }
        let ghosts = self.ghost_count(thing);
        if ghosts > 0 {
            parts.push(self.say(
                "read.ghosts",
                ctx(&[("count", Value::Number(ghosts as i64))]),
            ));
        }
        self.note_read(thing);
        if signs.len() <= SHORT {
            let more = self.page();
            parts.push(more.text);
        }
        let truth = self.reading_truth(thing);
        self.output(parts, Some(truth))
    }

    /// `read closely` (`read on`, `study`, `more`, or `look closer` while
    /// reading): the next page of signs, each as it looks, or by its sound
    /// once heard. With a thing named, starts on that thing.
    pub(crate) fn read_closely(&mut self, words: &[String]) -> Output {
        if !words.is_empty() {
            let cands: Vec<Candidate<Target>> = self
                .visible_targets()
                .into_iter()
                .filter(|c| matches!(c.target, Target::Thing(_)))
                .collect();
            if let crate::parser::Resolution::One(Target::Thing(i)) =
                crate::parser::resolve(words, &cands, self.state.it.as_ref())
            {
                if self.thing(i).texts.is_empty() {
                    let named = ctx(&[("thing", Value::from(self.thing_name(i)))]);
                    let t = self.say("read.nothing", named);
                    return self.output(vec![t], None);
                }
                if self.is_dark() {
                    let named = ctx(&[("thing", Value::from(self.thing_name(i)))]);
                    let t = self.say("read.dark", named);
                    return self.output(vec![t], None);
                }
                self.state.reading = Some(Reading { thing: i, page: 0 });
                self.state.last_read = Some(i);
            }
        }
        if self.state.reading.is_none() {
            // Back over the last thing read, if it is still in view.
            if let Some(t) = self.state.last_read.filter(|&t| self.at_hand(t)) {
                self.state.reading = Some(Reading { thing: t, page: 0 });
            }
        }
        if self.state.reading.is_some() && self.is_dark() {
            let t = self.say("read.dark", ctx(&[("thing", Value::from(""))]));
            return self.output(vec![t], None);
        }
        self.page()
    }

    /// Whether a thing is here to be read (in view or carried).
    pub(crate) fn at_hand(&self, thing: usize) -> bool {
        self.state.carried.contains(&thing) || self.here().contains(&thing)
    }

    /// One page of a close reading.
    pub(crate) fn page(&mut self) -> Output {
        let Some(Reading { thing, page }) = self.state.reading.clone() else {
            let t = self.say("read.no_more", Context::new());
            return self.output(vec![t], None);
        };
        self.reading_now = true;
        // Studied closely (S06): enough of these, and marks are signs.
        self.state.studied.insert(thing);
        let marks = self.marks(thing);
        let count = marks
            .iter()
            .filter(|m| matches!(m, Mark::Glyph { .. }))
            .count();
        let pages = count.div_ceil(PAGE).max(1);
        let first = page * PAGE + 1;
        let last = ((page + 1) * PAGE).min(count);
        self.pass(5);
        let thing_name = self.thing_name(thing);
        let seen = self.layers_seen(thing);
        let top = seen.last().map_or(self.thing(thing).texts[0], |l| l.0);
        let hand = self.hand(top);
        let t = self.thing(thing);
        let era = self.text(top).era as usize;
        let frame_ctx = ctx(&[
            ("hand", Value::from(hand)),
            ("thing", Value::from(thing_name)),
            ("material", Value::from(label(&t.material))),
            ("glyphs", Value::Number(count as i64)),
            ("page", Value::Number(page as i64 + 1)),
            ("pages", Value::Number(pages as i64)),
            ("texts", Value::Number(t.texts.len() as i64)),
            (
                "direction",
                Value::from(label(&self.site.world.languages[era].script.direction)),
            ),
        ]);
        let frame = self.say("read.frame", frame_ctx);
        // One sign per line, a blank line between words, a rule between
        // separate pieces of writing.
        let mut lines: Vec<String> = Vec::new();
        let mut n = 0;
        for m in &marks {
            match *m {
                Mark::Glyph { lost: true, .. } => {
                    n += 1;
                    if n < first || n > last {
                        continue;
                    }
                    let line = self.stable(
                        "read.lost",
                        ctx(&[("number", Value::Number(n as i64))]),
                        3_000_000 + n as u64,
                    );
                    lines.push(line);
                }
                Mark::Glyph { era, index, .. } => {
                    n += 1;
                    if n < first || n > last {
                        continue;
                    }
                    let impression = self.sign_impression(era, index);
                    let heard = self.heard(era, index);
                    let sound = if heard {
                        self.sign_sound(era, index).unwrap_or_default()
                    } else {
                        String::new()
                    };
                    let c = ctx(&[
                        ("number", Value::Number(n as i64)),
                        ("impression", Value::from(impression)),
                        ("heard", Value::Bool(heard && !sound.is_empty())),
                        ("sound", Value::from(sound)),
                    ]);
                    let line = self.stable("read.glyph", c, 2_000_000 + n as u64);
                    lines.push(line);
                }
                Mark::Gap
                    if n >= first && n < last && lines.last().is_some_and(|l| !l.is_empty()) =>
                {
                    lines.push(String::new())
                }
                _ => {}
            }
        }
        let body = lines.join("\n");
        self.note_read(thing);
        let after = if page + 1 < pages {
            self.state.reading = Some(Reading {
                thing,
                page: page + 1,
            });
            self.say(
                "read.more",
                ctx(&[("remaining", Value::Number((pages - page - 1) as i64))]),
            )
        } else {
            self.state.reading = None;
            self.say("read.end", Context::new())
        };
        let truth = self.reading_truth(thing);
        self.output(vec![frame, body, after], Some(truth))
    }

    /// The signs of the last thing read, while it is in view, as things to
    /// examine or trace: "the fourth sign", "sign 12", "the last sign".
    pub(crate) fn sign_targets(&self) -> Vec<Candidate<Target>> {
        let Some(thing) = self.state.last_read.filter(|&t| self.at_hand(t)) else {
            return Vec::new();
        };
        let count = self.signs(thing).len();
        (1..=count)
            .map(|n| {
                let num = n.to_string();
                let th = format!("{n}th");
                let mut extra: Vec<&str> =
                    vec![&num, &th, "mark", "shape", "glyph", "character", "letter"];
                if let Some(o) = ORDINALS.get(n - 1) {
                    extra.push(o);
                }
                if n == count {
                    extra.push("last");
                }
                Candidate::new(Target::Sign(n), &format!("sign {n}"), &extra)
            })
            .collect()
    }

    /// `examine the fourth sign`: one mark, exactly, is tracing it (S06):
    /// there is no separate fuller impression.
    pub(crate) fn examine_sign(&mut self, n: usize) -> Output {
        self.trace(Target::Sign(n))
    }

    /// What the player can take marks with (S06): soft clay pressed into
    /// cut marks (no light needed), or charcoal rubbed on bark or cloth.
    // DESIGN-Q: a cloak is the cloth; every mark counts as cut or
    // scratched, so clay always takes it.
    pub(crate) fn tracing_tool(&self) -> Option<&'static str> {
        let has = |k: &str| self.state.carried.iter().any(|&t| self.kind_of(t) == k);
        if has("clay") {
            Some("pressing")
        } else if has("charcoal") && (has("bark") || has("cloak")) {
            Some("rubbing")
        } else {
            None
        }
    }

    /// `trace the fourth sign` / `trace the stele` (`copy`, `make a
    /// rubbing of`): the exact strokes, the only way to get them. A task
    /// in itself: minutes a sign, good light, a sign still legible. A whole
    /// text is traced a few signs at a go, carrying on where it left off.
    /// What is traced is told once; the game doesn't keep it. It needs a
    /// tracing tool (S06), and light unless it is a pressing.
    pub(crate) fn trace(&mut self, target: Target) -> Output {
        let Some(with) = self.tracing_tool() else {
            let t = self.say("trace.no_tool", Context::new());
            return self.output(vec![t], None);
        };
        let (thing, from, to) = match target {
            Target::Sign(n) => match self.state.last_read {
                Some(t) => (t, n, n),
                None => {
                    let t = self.say("read.no_more", Context::new());
                    return self.output(vec![t], None);
                }
            },
            Target::Thing(i) => {
                if self.thing(i).texts.is_empty() {
                    let named = ctx(&[("thing", Value::from(self.thing_name(i)))]);
                    let t = self.say("read.nothing", named);
                    return self.output(vec![t], None);
                }
                let count = self.signs(i).len();
                let next = match self.state.tracing {
                    Some((t, k)) if t == i && k <= count => k,
                    _ => 1,
                };
                self.state.last_read = Some(i);
                (i, next, (next + TRACE_PAGE - 1).min(count))
            }
            _ => {
                let t = self.say("say.not_here", ctx(&[("words", Value::from(""))]));
                return self.output(vec![t], None);
            }
        };
        let name = self.thing_name(thing);
        if with != "pressing" && (self.is_dark() || !self.light_to_trace()) {
            let t = self.say("trace.dark", ctx(&[("thing", Value::from(name))]));
            return self.output(vec![t], None);
        }
        let signs = self.signs(thing);
        let mut lines = Vec::new();
        for n in from..=to {
            let Some(&(era, index, lost)) = signs.get(n - 1) else {
                break;
            };
            if lost {
                lines.push(self.say("trace.lost", ctx(&[("number", Value::Number(n as i64))])));
                continue;
            }
            let description = self.glyph_description(era, index);
            lines.push(self.say(
                "trace.sign",
                ctx(&[
                    ("number", Value::Number(n as i64)),
                    ("description", Value::from(description)),
                ]),
            ));
        }
        let done = lines.len();
        self.pass(TRACE_MINUTES * done as u32);
        let frame = self.say(
            "trace.frame",
            ctx(&[
                ("thing", Value::from(name)),
                ("with", Value::from(with)),
                ("from", Value::Number(from as i64)),
                ("to", Value::Number(to as i64)),
                ("glyphs", Value::Number(signs.len() as i64)),
                (
                    "how_long",
                    Value::from(crate::measures::how_long(TRACE_MINUTES * done as u32)),
                ),
            ]),
        );
        let mut parts = vec![frame, lines.join("\n")];
        if matches!(target, Target::Thing(_)) {
            if to < signs.len() {
                self.state.tracing = Some((thing, to + 1));
                parts.push(self.say(
                    "trace.more",
                    ctx(&[("remaining", Value::Number((signs.len() - to) as i64))]),
                ));
            } else {
                self.state.tracing = None;
            }
        }
        self.output(parts, None)
    }

    /// Tracing needs better light than reading: a good lamp or daylight.
    fn light_to_trace(&self) -> bool {
        // DESIGN-Q: tracing needs the light reading needs; dimmer light
        // (dusk, a guttering candle) could slow it or make it fail.
        !self.is_dark()
    }
}

/// Reading measures for `scraped-lang depth` (S01): words a glance and a
/// page of close reading take, with no sounds heard and with the
/// commonest signs heard; and how many signs of each era's script have an
/// impression of their own.
pub fn measures(pack: &scraped_content::Pack, seed: u64) -> Vec<(String, f64)> {
    let mut g = Game::new(seed, pack.clone());
    g.forced_light = true;
    g.start();
    let things: Vec<usize> = (0..g.site.things.len())
        .filter(|&t| g.signs(t).len() >= 12)
        .take(12)
        .collect();
    let words = |s: &str| s.split_whitespace().count() as f64;
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len().max(1) as f64;
    let mut out = Vec::new();
    let read_close = |g: &mut Game| -> (Vec<f64>, Vec<f64>) {
        let (mut glance, mut page) = (Vec::new(), Vec::new());
        for &t in &things {
            g.state.place = g.site.things[t].home;
            g.state.pos = g.site.things[t].pos;
            glance.push(words(&g.act("read", Target::Thing(t)).text));
            page.push(words(&g.step("read closely").text));
        }
        (glance, page)
    };
    let (glance, page) = read_close(&mut g);
    out.push(("reading.words_per_read".to_string(), mean(&glance)));
    out.push(("reading.words_per_page".to_string(), mean(&page)));
    // The commonest signs heard, as after some scraping.
    let mut counts: BTreeMap<(u32, usize), usize> = BTreeMap::new();
    for &t in &things {
        for (era, index, _) in g.signs(t) {
            if g.sign_sound(era, index).is_some() {
                *counts.entry((era, index)).or_default() += 1;
            }
        }
    }
    let mut common: Vec<((u32, usize), usize)> = counts.into_iter().collect();
    common.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    for (k, _) in common.into_iter().take(12) {
        g.state.heard.insert(k);
    }
    let (_, page) = read_close(&mut g);
    out.push(("reading.words_per_page_heard".to_string(), mean(&page)));
    // Signs whose impression no other sign of the script shares.
    let mut shares = Vec::new();
    for era in 0..g.site.world.languages.len() as u32 {
        let n = g.site.world.languages[era as usize].script.glyphs.len();
        let texts: Vec<String> = (0..n).map(|i| g.sign_impression(era, i)).collect();
        let unique = texts
            .iter()
            .filter(|t| texts.iter().filter(|o| o == t).count() == 1)
            .count();
        shares.push(unique as f64 / n.max(1) as f64);
    }
    out.push((
        "reading.unique_impression_share".to_string(),
        shares.iter().copied().fold(1.0, f64::min),
    ));
    out
}

/// How a text is laid out on its surface, as the eye takes it in before
/// reading a sign (D08): numbers in columns, entries one under another, a
/// seal at the foot, a list, short verses, or running lines.
pub(crate) fn form(g: scraped_world::texts::Genre) -> &'static str {
    use scraped_world::texts::Genre as G;
    match g {
        G::Ledger | G::Inventory | G::Receipt => "columns",
        G::Annal | G::KingList | G::Court | G::Calendar | G::Lesson => "entries",
        G::Letter | G::Contract | G::Decree => "sealed",
        G::WordList | G::Label | G::Milestone => "list",
        G::Hymn | G::Prayer | G::Blessing | G::Curse => "verses",
        _ => "running",
    }
}

/// Every layout `form` gives.
pub(crate) const FORMS: [&str; 6] = ["columns", "entries", "sealed", "list", "verses", "running"];
