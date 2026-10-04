//! The player as author: writing new text in the language, the
//! understanding gate, agreement with what lies beneath, and misfires.
//!
//! Player text is entered as glyphs (script numbers or the player's own
//! labels), never English. The game parses it to a meaning; text that does
//! not parse is still written, and is inert.

use scraped_lang::difficulty::Separation;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use scraped_content::Value;
use scraped_lang::concepts::{self, Pos};
use scraped_lang::corpus::Kind;
use scraped_lang::meaning::{Head, Mood, NounPhrase, Sentence};
use scraped_lang::parse::{glyph_sym, Mode, Parser, Tok};
use scraped_lang::script::GlyphKey;
use scraped_sim::body::Activity;
use scraped_sim::writing::live_of;
use scraped_world::texts::Text;

use crate::parser::{resolve, Resolution};
use crate::site::{ctx, label};
use crate::{Game, Output, Target};

/// A text the player wrote: where, what they entered, and when.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Written {
    pub thing: usize,
    /// Glyph indices in the era's script; `None` is a gap between words.
    pub glyphs: Vec<Option<usize>>,
    pub minutes: u32,
}

/// What a `write` did, for agents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WriteReport {
    pub accepted: bool,
    /// Why not, by id: "no_tool", "covered", "unknown_mark", "hesitate",
    /// "smudge", "nothing", "dark", "not_surface".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refused: Option<String>,
    /// The glyphs written, as script numbers (gaps as `null`).
    pub glyphs: Vec<Option<usize>>,
}

/// How many distinct texts a root must have been met in before the player
/// can write it.
// DESIGN-Q: two texts, as proposed; partly scraped readings count, and
// function words (the potent formulae, "and") need no encounters.
pub const THRESHOLD: usize = 2;

/// Features that take new writing.
const WRITABLE: &[&str] = &[
    "wall",
    "stele",
    "altar",
    "niche",
    "lintel",
    "door-slab",
    "gate",
    "parapet",
    "gravestone",
    "tablet",
    "statue",
    "basin",
    "beam",
    "table",
    "inscription",
    "milestone",
];

impl Game {
    /// The era the player writes in: the newest.
    // DESIGN-Q: player writing is always in the newest era's language and
    // script.
    pub(crate) fn writing_era(&self) -> usize {
        self.site.world.languages.len() - 1
    }

    /// Rebuilds the texts the player has written from the state.
    pub(crate) fn sync_written(&mut self) {
        if self.player_texts.len() == self.state.written.len() {
            return;
        }
        self.player_texts.clear();
        for i in 0..self.state.written.len() {
            let t = self.player_text(i);
            self.player_texts.push(t);
        }
    }

    /// The meaning of a player's text, and whether it was framed as potent
    /// without parsing (a misfire waiting to happen).
    fn parse_written(&self, glyphs: &[Option<usize>]) -> (Option<Sentence>, bool) {
        let era = self.writing_era();
        let lang = &self.site.world.languages[era];
        let r = self.site.world.renderer(era as u32);
        let p = Parser::without_names(&r, Mode::Glyphs);
        let toks: Vec<Tok> = glyphs
            .iter()
            .map(|g| match g {
                Some(i) => match &lang.script.glyphs[*i].0 {
                    GlyphKey::Divider => Tok::Gap,
                    k => Tok::Sym(glyph_sym(k)),
                },
                None => Tok::Gap,
            })
            .collect();
        let mut meaning = p.sentence(&toks);
        // Does it open with the potent formula?
        let open_syms: Vec<Tok> = p.tokens(&[r.plain_word("pot.open")]);
        let framed = toks
            .iter()
            .filter(|t| **t != Tok::Gap)
            .take(open_syms.len())
            .eq(open_syms.iter());
        // A misfire (D09): potent writing the grammar can't place whole
        // acts as what it can, without the one word that doesn't fit, so a
        // misplaced condition is dropped and the spell acts always, a
        // misplaced "greatly" leaves it plain. Writing with more than one
        // such fault turns on its writer instead.
        // DESIGN-Q: only a single dropped word is salvaged, the first that
        // makes the rest a spell.
        if meaning.is_none() && framed {
            meaning = salvage(&p, &toks);
        }
        (meaning, framed)
    }

    fn player_text(&self, i: usize) -> Text {
        let w = &self.state.written[i];
        let (meaning, _) = self.parse_written(&w.glyphs);
        let thing = self.thing(w.thing);
        let (structure, room) = match thing.home {
            crate::site::Place::Room { structure, room } => (structure, Some(room)),
            crate::site::Place::Outside => {
                // Outdoors: the nearest building stands for where it is.
                let s = self
                    .site
                    .land
                    .structure_pos
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, p)| p.dist2(thing.pos))
                    .map(|(s, _)| s)
                    .unwrap_or(0);
                (s, None)
            }
        };
        let potent = meaning
            .as_ref()
            .is_some_and(|m| matches!(m, Sentence::Clause(c) if c.mood == Mood::Potent));
        Text {
            id: self.site.writing.count(&self.site.world) + i,
            era: self.writing_era() as u32,
            year: self.site.world.history.eras.last().map_or(0, |e| e.end) + 1 + i as i32,
            kind: if potent { Kind::Potent } else { Kind::Label },
            genre: scraped_world::texts::Genre::of(if potent { Kind::Potent } else { Kind::Label }),
            arc: None,
            meaning: meaning.unwrap_or(Sentence::List(Vec::new())),
            author: None,
            event: None,
            structure,
            room,
            feature: None,
            material: thing.material,
        }
    }

    /// Whether the player's text `i` turns on its writer when scraped: it
    /// is framed as potent but says nothing the language can parse.
    pub(crate) fn backlash(&self, i: usize) -> bool {
        let (meaning, framed) = self.parse_written(&self.state.written[i].glyphs);
        framed && meaning.is_none()
    }

    /// Notes the roots of everything the player has just read.
    pub(crate) fn encounter(&mut self, texts: &[usize]) {
        for &t in texts {
            self.state.read.insert(t);
            let mut roots = BTreeSet::new();
            concepts_of(
                &self.text(t).meaning.clone(),
                &mut roots,
                &self.site.world.languages[self.text(t).era as usize].numerals,
            );
            for r in roots {
                self.state.encountered.entry(r).or_default().insert(t);
            }
        }
    }

    /// One written word: its signs (indexes in the script of `era`) and how
    /// many were traced in. Sounds are spelled as the script spells them,
    /// and every sign that has a sound must have been heard; `#N` is the
    /// Nth sign of the last text read, which must be at hand, in light,
    /// legible and of the same script.
    // DESIGN-Q: words are written by sound (the script spells them, adding
    // signs that have no sound, such as an abjad's vowel carrier), and
    // signs not heard are traced in by number from a text in view.
    fn spell_written(
        &self,
        era: usize,
        word: &str,
    ) -> Result<(Vec<usize>, u32), (&'static str, String)> {
        let lang = &self.site.world.languages[era];
        let script = &lang.script;
        let mut out = Vec::new();
        let mut traced = 0;
        let mut rest = word;
        while !rest.is_empty() {
            if let Some(r) = rest.strip_prefix('#') {
                let digits: String = r.chars().take_while(char::is_ascii_digit).collect();
                rest = &r[digits.len()..];
                let n: usize = digits
                    .parse()
                    .map_err(|_| ("write.unknown_mark", word.to_string()))?;
                let thing = self
                    .state
                    .last_read
                    .filter(|&t| self.at_hand(t) && !self.is_dark())
                    .ok_or(("write.unknown_mark", format!("#{n}")))?;
                match self.signs(thing).get(n.wrapping_sub(1)) {
                    Some(&(e, index, false)) if e as usize == era => {
                        out.push(index);
                        traced += 1;
                    }
                    _ => return Err(("write.unknown_mark", format!("#{n}"))),
                }
                continue;
            }
            let end = rest.find('#').unwrap_or(rest.len());
            let sounds = &rest[..end];
            rest = &rest[end..];
            let phonemes = lang
                .phonology
                .decode(sounds)
                .ok_or(("write.unknown_mark", sounds.to_string()))?;
            let ipa: Vec<&'static str> = phonemes
                .iter()
                .map(|&id| lang.phonology.inventory.get(id).ipa)
                .collect();
            for key in script.spell(&ipa) {
                let index = script.index(&key);
                if self.sign_sound(era as u32, index).is_some() && !self.heard(era as u32, index) {
                    return Err(("write.unheard", sounds.to_string()));
                }
                out.push(index);
            }
        }
        Ok((out, traced))
    }

    /// A meaning as a player would type it to write it: its words by
    /// their sounds, in the script of the day.
    pub fn sound_words(&self, m: &Sentence) -> String {
        let era = self.writing_era();
        let r = self.site.world.renderer(era as u32);
        let lang = &self.site.world.languages[era];
        r.render(m)
            .words
            .iter()
            .map(|w| lang.romanise(&w.phonemes()))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Roots of a meaning the player hasn't met often enough to write.
    fn unknown_roots(&self, m: &Sentence) -> Vec<String> {
        let era = self.writing_era();
        let mut roots = BTreeSet::new();
        concepts_of(m, &mut roots, &self.site.world.languages[era].numerals);
        roots
            .into_iter()
            .filter(|r| concepts::get(r).pos != Pos::Particle)
            .filter(|r| self.state.encountered.get(r).map_or(0, |s| s.len()) < self.threshold)
            .collect()
    }

    /// `write <words> on <thing>`: each word by its sounds, as the player
    /// has heard them (S01), spelled in the script of the day; `#4` copies
    /// the fourth sign of the last text read, traced from it.
    pub(crate) fn write(&mut self, words: &[String]) -> Output {
        let Some(on) = words.iter().rposition(|w| w == "on" || w == "onto") else {
            return self.write_refused("nothing", None, Vec::new());
        };
        let (marks, target) = (&words[..on], &words[on + 1..]);
        let cands = self.visible_targets();
        let Resolution::One(Target::Thing(thing)) = resolve(target, &cands, self.state.it.as_ref())
        else {
            let t = self.say(
                "say.not_here",
                ctx(&[("words", Value::from(target.join(" ")))]),
            );
            return self.output(vec![t], None);
        };
        self.state.it = Some(Target::Thing(thing));
        let name = self.thing_name(thing);
        if !self
            .state
            .carried
            .iter()
            .any(|&t| self.thing(t).kind == "stylus")
        {
            return self.write_refused("no_tool", Some(name), Vec::new());
        }
        if self.is_dark() {
            return self.write_refused("dark", Some(name), Vec::new());
        }
        let th = self.thing(thing);
        if th.texts.is_empty() && (th.portable || !WRITABLE.contains(&th.kind)) {
            return self.write_refused("not_surface", Some(name), Vec::new());
        }
        if self.power() < self.needs_power(thing) {
            return self.write_refused("too_great", Some(name), Vec::new());
        }
        let layers = self.layers(thing);
        if layers
            .last()
            .is_some_and(|t| !self.state.scraped.contains(t))
        {
            return self.write_refused("covered", Some(name), Vec::new());
        }
        // Words, by their sounds, spelled in the script of the day; signs
        // not heard can be traced in from the last text read ("#4").
        let era = self.writing_era();
        let mut glyphs = Vec::new();
        let mut traced = 0u32;
        for word in marks {
            let word = word.trim_matches(|c| c == '"' || c == ',');
            if word.is_empty() || matches!(word, "/" | "|" | "·") {
                continue;
            }
            match self.spell_written(era, word) {
                Ok((signs, t)) => {
                    // Words apart as the script sets them apart.
                    if !glyphs.is_empty() {
                        let lang = &self.site.world.languages[era];
                        match lang.difficulty.separation {
                            Separation::Spaces => glyphs.push(None),
                            Separation::Dots => {
                                glyphs.push(Some(lang.script.index(&GlyphKey::Divider)))
                            }
                            Separation::None => {}
                        }
                    }
                    glyphs.extend(signs.into_iter().map(Some));
                    traced += t;
                }
                Err((slot, mark)) => {
                    let t = self.say(slot, ctx(&[("mark", Value::from(mark))]));
                    self.last_write = Some(WriteReport {
                        accepted: false,
                        refused: Some(slot.trim_start_matches("write.").into()),
                        glyphs: Vec::new(),
                    });
                    return self.output(vec![t], None);
                }
            }
        }
        // Copying a sign from a text takes the time tracing takes.
        self.advance(traced * crate::reading::TRACE_MINUTES, Activity::Resting);
        if glyphs.iter().all(Option::is_none) {
            return self.write_refused("nothing", Some(name), glyphs);
        }
        let (meaning, _) = self.parse_written(&glyphs);
        // The understanding gate: only words met often enough.
        if let Some(m) = &meaning {
            let unknown = self.unknown_roots(m);
            if !unknown.is_empty() {
                self.advance(2, Activity::Resting);
                let t = self.say(
                    "write.hesitate",
                    ctx(&[("count", Value::Number(unknown.len() as i64))]),
                );
                self.last_write = Some(WriteReport {
                    accepted: false,
                    refused: Some("hesitate".into()),
                    glyphs,
                });
                return self.output(vec![t], None);
            }
        }
        // Agreement with the trace beneath.
        if let Some(ghost) = live_of(&layers, &self.state.scraped) {
            let ghost_meaning = self.text(ghost).meaning.clone();
            let ghost_potent = self.text(ghost).kind == Kind::Potent;
            if !agrees(&ghost_meaning, ghost_potent, meaning.as_ref()) {
                self.advance(5, Activity::Resting);
                let material = label(&self.thing(thing).material);
                let t = self.say(
                    "write.smudge",
                    ctx(&[
                        ("thing", Value::from(name)),
                        ("material", Value::from(material)),
                    ]),
                );
                self.last_write = Some(WriteReport {
                    accepted: false,
                    refused: Some("smudge".into()),
                    glyphs,
                });
                return self.output(vec![t], None);
            }
        }
        // It takes.
        self.advance(15, Activity::Resting);
        self.state.written.push(Written {
            thing,
            glyphs: glyphs.clone(),
            minutes: self.state.minutes,
        });
        self.sync_written();
        // Each sign as the player knows it: by its sound, else its look.
        let descriptions: Vec<Value> = glyphs
            .iter()
            .flatten()
            .map(|&i| {
                let e = era as u32;
                match self.sign_sound(e, i).filter(|_| self.heard(e, i)) {
                    Some(s) => Value::from(format!("«{s}»")),
                    None => Value::from(self.sign_impression(e, i)),
                }
            })
            .collect();
        let material = label(&self.thing(thing).material);
        let t = self.say(
            "write.done",
            ctx(&[
                ("thing", Value::from(name)),
                ("material", Value::from(material)),
                ("glyphs", Value::List(descriptions)),
            ]),
        );
        self.last_write = Some(WriteReport {
            accepted: true,
            refused: None,
            glyphs,
        });
        self.hook("first_write", "", "");
        self.output(vec![t], None)
    }

    fn write_refused(
        &mut self,
        why: &str,
        thing: Option<String>,
        glyphs: Vec<Option<usize>>,
    ) -> Output {
        self.last_write = Some(WriteReport {
            accepted: false,
            refused: Some(why.to_string()),
            glyphs,
        });
        let c = ctx(&[
            ("reason", Value::from(why)),
            ("thing", Value::from(thing.unwrap_or_default())),
        ]);
        let t = self.say("write.refused", c);
        self.output(vec![t], None)
    }

    /// Debug: what the player has encountered, by root.
    pub(crate) fn understanding(&self) -> serde_json::Value {
        serde_json::json!(self
            .state
            .encountered
            .iter()
            .map(|(k, v)| (k.clone(), v.len()))
            .collect::<std::collections::BTreeMap<_, _>>())
    }
}

/// Every root a meaning uses: predicates, nouns, modifiers, numerals,
/// adverbs, and the potent formulae.
pub fn concepts_of(
    s: &Sentence,
    out: &mut BTreeSet<String>,
    numerals: &scraped_lang::numerals::Numerals,
) {
    fn np(n: &NounPhrase, out: &mut BTreeSet<String>, numerals: &scraped_lang::numerals::Numerals) {
        if let Head::Concept(c) = &n.head {
            out.insert(c.clone());
        }
        out.extend(n.adjectives.iter().cloned());
        if let Some(d) = &n.determiner {
            out.insert(d.clone());
        }
        if let Some(q) = n.quantity {
            out.extend(numerals.words(q).into_iter().map(str::to_string));
        }
        if let Some(p) = &n.possessor {
            np(p, out, numerals);
        }
        for a in &n.apposition {
            np(a, out, numerals);
        }
    }
    for c in s.clauses() {
        out.insert(c.predicate.clone());
        out.extend(c.adverbs.iter().cloned());
        if c.mood == Mood::Potent {
            out.extend(["pot", "pot.open", "pot.close"].map(str::to_string));
        }
    }
    for n in s.noun_phrases() {
        np(n, out, numerals);
        if let Some(d) = &n.degree {
            out.insert(d.adjective.clone());
        }
    }
    out.extend(s.function_words().into_iter().map(str::to_string));
}

/// The agreement rule: new writing over a trace must keep its register and
/// fill the same slots, as if completing it: the same roles, each with the
/// same number and the same kind of noun. Text that doesn't parse agrees
/// with nothing but a blank surface.
// DESIGN-Q: agreement is checked against the whole trace beneath (not only
// its surviving words): register, roles, number and the domain of each head.
pub fn agrees(ghost: &Sentence, ghost_potent: bool, new: Option<&Sentence>) -> bool {
    let Some(new) = new else { return false };
    let potent = |s: &Sentence| matches!(s, Sentence::Clause(c) if c.mood == Mood::Potent);
    if potent(new) != ghost_potent {
        return false;
    }
    match (ghost, new) {
        (Sentence::Clause(g), Sentence::Clause(n)) => {
            let roles = |c: &scraped_lang::meaning::Clause| {
                c.args.iter().map(|a| a.role).collect::<Vec<_>>()
            };
            if roles(g) != roles(n) {
                return false;
            }
            g.args
                .iter()
                .zip(&n.args)
                .all(|(a, b)| a.np.number == b.np.number && domain(&a.np) == domain(&b.np))
        }
        (Sentence::List(g), Sentence::List(n)) => g.len() == n.len(),
        (Sentence::Text(g), Sentence::Text(n)) => g.len() == n.len(),
        _ => false,
    }
}

fn domain(n: &NounPhrase) -> String {
    match &n.head {
        Head::Concept(c) => label(&concepts::get(c).domain),
        Head::Name(_) => "people".to_string(),
    }
}

/// The spell left when one word of potent writing is dropped, if any.
fn salvage(p: &Parser, toks: &[Tok]) -> Option<Sentence> {
    let mut words: Vec<Vec<Tok>> = vec![Vec::new()];
    for t in toks {
        if *t == Tok::Gap {
            if !words.last().is_some_and(Vec::is_empty) {
                words.push(Vec::new());
            }
        } else if let Some(w) = words.last_mut() {
            w.push(t.clone());
        }
    }
    words.retain(|w| !w.is_empty());
    // The formula's opening and closing words stay.
    for k in 1..words.len().saturating_sub(1) {
        let mut rest: Vec<Tok> = Vec::new();
        for (i, w) in words.iter().enumerate() {
            if i == k {
                continue;
            }
            if !rest.is_empty() {
                rest.push(Tok::Gap);
            }
            rest.extend(w.iter().cloned());
        }
        if let Some(m) = p.sentence(&rest) {
            if matches!(&m, Sentence::Clause(c) if c.mood == Mood::Potent) {
                return Some(m);
            }
        }
    }
    None
}
