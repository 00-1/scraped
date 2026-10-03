//! Writing in play: what can be read of each surface, scraping, and the
//! physical cues of claims taking effect.

use scraped_content::{Context, Value};
use scraped_lang::script::GlyphKey;
use scraped_sim::body::Activity;
use scraped_sim::outdoors::hash;
use scraped_sim::writing::{
    beneath_of, claim_of, ghosts_of, live_of, top_unscraped_of, visible_of, Claim, Class, Property,
};
use scraped_world::texts::Text;

use crate::site::{ctx, label, Place};
use crate::{Game, Output};

/// Minutes fresh writing takes to dry.
// DESIGN-Q: half an hour.
pub const DRYING: u32 = 30;

/// Ids for the look of a scribe's hand. Content describes them.
// DESIGN-Q: six recognisable hands, assigned by author (anonymous latent
// inscriptions get one by era and building).
pub const HANDS: &[&str] = &["cramped", "broad", "slanted", "careful", "heavy", "fine"];

impl Game {
    pub(crate) fn text(&self, id: usize) -> &Text {
        let base = self.site.writing.count(&self.site.world);
        if id >= base {
            &self.player_texts[id - base]
        } else {
            self.site.writing.text(&self.site.world, id)
        }
    }

    /// A thing's layers of writing, oldest first: history's and the player's.
    pub(crate) fn layers(&self, thing: usize) -> Vec<usize> {
        let base = self.site.writing.count(&self.site.world);
        let mut out = self.thing(thing).texts.clone();
        out.extend(
            self.state
                .written
                .iter()
                .enumerate()
                .filter(|(_, w)| w.thing == thing)
                .map(|(i, _)| base + i),
        );
        out
    }

    /// The layers a reader can see: (text, partial), oldest first. With a
    /// deep-reading lens, faint layers beneath show too.
    pub(crate) fn layers_seen(&self, thing: usize) -> Vec<(usize, bool)> {
        let layers = self.layers(thing);
        let mut out = visible_of(&layers, &self.state.scraped);
        let deep = self.deep_layers(thing);
        out.splice(0..0, deep.into_iter().map(|d| (d, true)));
        out
    }

    /// The strongest lens carried: 0 none, 1 the lens, 2 the first lens.
    pub(crate) fn lens_power(&self) -> u8 {
        self.state
            .carried
            .iter()
            .map(|&t| match self.thing(t).kind {
                "lens" => 1,
                "first_lens" => 2,
                _ => 0,
            })
            .max()
            .unwrap_or(0)
    }

    /// The faint layers a lens shows, oldest first: the lens shows the one
    /// just beneath the live layer, unless it is one of the deepest
    /// accounts; the first lens shows every layer beneath.
    // DESIGN-Q: the first lens reads all ghost layers at once.
    pub(crate) fn deep_layers(&self, thing: usize) -> Vec<usize> {
        let layers = self.layers(thing);
        match self.lens_power() {
            0 => Vec::new(),
            1 => beneath_of(&layers, &self.state.scraped)
                .filter(|t| !self.site.writing.deep.contains(t))
                .into_iter()
                .collect(),
            _ => {
                let ghosts = ghosts_of(&layers, &self.state.scraped);
                layers[..ghosts].to_vec()
            }
        }
    }

    pub(crate) fn ghost_count(&self, thing: usize) -> usize {
        let layers = self.layers(thing);
        let ghosts = ghosts_of(&layers, &self.state.scraped);
        ghosts - self.deep_layers(thing).len()
    }

    /// Whether glyph `g` of a scraped text is lost to the eye here: more
    /// survive in better light.
    // DESIGN-Q: by eye, 60% of a scraped layer's glyphs show in daylight,
    // 40% in dim light.
    pub(crate) fn lost(&self, text: usize, g: usize, deep: bool) -> bool {
        let light = self
            .env()
            .local(self.spot(), self.state.minutes, self.carried_light())
            .light;
        // DESIGN-Q: through the lens, the layer beneath shows 35% of its
        // glyphs in daylight, 20% in dim light; through the first lens, 80%
        // and 60%.
        let first = self.lens_power() >= 2;
        let shown = match (deep, first, light == "daylight") {
            (false, _, true) => 60,
            (false, _, false) => 40,
            (true, false, true) => 35,
            (true, false, false) => 20,
            (true, true, true) => 80,
            (true, true, false) => 60,
        };
        hash(&[self.seed(), 0x5c4a, text as u64, g as u64]) % 100 >= shown
    }

    /// The hand a text was written in.
    pub(crate) fn hand(&self, text: usize) -> &'static str {
        let t = self.text(text);
        let key = match t.author {
            Some(a) => hash(&[self.seed(), 0x4a4d, a as u64]),
            None => hash(&[self.seed(), 0x4a4e, u64::from(t.era), t.structure as u64]),
        };
        HANDS[(key % HANDS.len() as u64) as usize]
    }

    pub(crate) fn recompute_claims(&mut self) {
        let mut claims = Vec::new();
        for t in 0..self.site.things.len() {
            if self.thing(t).texts.is_empty() && !self.state.written.iter().any(|w| w.thing == t) {
                continue;
            }
            let layers = self.layers(t);
            if let Some(live) = live_of(&layers, &self.state.scraped) {
                if let Some(mut c) =
                    claim_of(&self.site.world, &self.site.land, self.text(live), live)
                {
                    // A stronger scraper carries a release farther.
                    // DESIGN-Q: a fine scraper or better triples a release's
                    // local reach; old and first scrapers also act on regions.
                    if self.state.released.get(&live).is_some_and(|&p| p >= 2) {
                        c.range *= 3.0;
                    }
                    claims.push(c);
                }
            }
        }
        self.claims = claims;
        self.recompute_drivers();
    }

    /// Scrapes the whole top unscraped text from a surface. Needs the
    /// scraping tool. The text stays, as a scraped layer; if it was potent,
    /// its claim now acts.
    pub(crate) fn scrape(&mut self, thing: usize) -> Output {
        let name = self.thing_name(thing);
        let named = ctx(&[("thing", Value::from(name.as_str()))]);
        let has_tool = self.power() > 0;
        if !has_tool {
            let t = self.say("scrape.no_tool", named);
            return self.output(vec![t], None);
        }
        let layers = self.layers(thing);
        if layers.is_empty() {
            let t = self.say("read.nothing", named);
            return self.output(vec![t], None);
        }
        if self.is_dark() {
            let t = self.say("read.dark", named);
            return self.output(vec![t], None);
        }
        if self.power() < self.needs_power(thing) {
            return self.too_weak(thing);
        }
        let Some(text) = top_unscraped_of(&layers, &self.state.scraped) else {
            let t = self.say("scrape.bare", named);
            return self.output(vec![t], None);
        };
        // Fresh ink must dry before it can be scraped off cleanly.
        let base = self.site.writing.count(&self.site.world);
        if text >= base && self.state.minutes < self.state.written[text - base].minutes + DRYING {
            let t = self.say("scrape.wet", named);
            return self.output(vec![t], None);
        }
        self.advance(20, Activity::Resting);
        if self.state.dead.is_some() {
            return self.output(Vec::new(), None);
        }
        let before = self.claims.clone();
        let drivers_before = self.drivers.clone();
        self.state.scraped.insert(text);
        let power = self.power();
        if text >= base {
            self.state.released.insert(text, power);
        }
        self.recompute_claims();
        self.record_acts(text, &drivers_before);
        self.last_scrape = Some(text);
        let backlash = text >= base && self.backlash(text - base);
        let material = label(&self.thing(thing).material);
        let mut parts = vec![self.say(
            "scrape.done",
            ctx(&[
                ("thing", Value::from(name)),
                ("material", Value::from(material)),
            ]),
        )];
        parts.extend(self.hear_signs(text));
        let mut felt = self.claim_changes(&before);
        if self.claims.iter().any(|c| c.text == text) && text >= base {
            felt.extend(self.release_feeling(power));
            self.hook("first_release", "", "");
        }
        self.scrape_felt = !felt.is_empty();
        parts.extend(felt);
        if backlash {
            // A malformed claim in the potent frame turns on its writer.
            parts.push(self.say("write.backlash", Context::new()));
            self.hurt(1, "writing");
        } else if text >= base {
            self.check_self_claim(text);
        }
        self.output(parts, None)
    }

    /// As a text's strokes come away under the blade, each sign gives its
    /// sound, faintly (S01). Heard where it is quiet, or anywhere by a
    /// player who is listening; the sounds then attach to their signs.
    fn hear_signs(&mut self, text: usize) -> Option<String> {
        let strengths: Vec<f64> = self.sounds().iter().map(|s| s.strength).collect();
        let quiet = crate::senses::quiet(&strengths);
        if !quiet && !self.attentive {
            return None;
        }
        let t = self.text(text);
        let era = t.era;
        let r = self.site.world.renderer(era);
        let lang = &self.site.world.languages[era as usize];
        let keys = r.glyphs(&r.render(&t.meaning));
        let mut sounds: Vec<Value> = Vec::new();
        let mut manners: Vec<Value> = Vec::new();
        let mut new = 0;
        for k in keys.into_iter().flatten() {
            let index = lang.script.index(&k);
            let Some(sound) = self.sign_sound(era, index) else {
                continue;
            };
            if self.state.heard.insert((era, index)) {
                new += 1;
            }
            let v = Value::from(sound);
            if !sounds.contains(&v) {
                sounds.push(v);
            }
            let ipa: Vec<&str> = match &k {
                GlyphKey::Sound(s) | GlyphKey::Dead(s) => vec![*s],
                GlyphKey::Syllable(c, v) => c.iter().copied().chain([*v]).collect(),
                _ => Vec::new(),
            };
            for p in ipa {
                let ph = scraped_lang::phonology::phoneme(p);
                let m = match ph.manner() {
                    Some(m) => serde_json::to_value(m)
                        .ok()
                        .and_then(|v| v.as_str().map(str::to_string))
                        .unwrap_or_default(),
                    None => "vowel".to_string(),
                };
                let m = Value::from(m);
                if !manners.contains(&m) {
                    manners.push(m);
                }
            }
        }
        if sounds.is_empty() {
            return None;
        }
        let c = ctx(&[
            ("first", sounds[0].clone()),
            ("count", Value::Number(sounds.len() as i64)),
            ("sounds", Value::List(sounds)),
            ("manners", Value::List(manners)),
            ("new", Value::Number(new)),
            ("quiet", Value::Bool(quiet)),
        ]);
        Some(self.say("glyph.heard", c))
    }

    /// What the player feels of claims starting or stopping: physical cues
    /// only, never what the writing says.
    fn claim_changes(&mut self, before: &[Claim]) -> Vec<String> {
        let after = self.claims.clone();
        let here = self.state.pos;
        let mut out = Vec::new();
        let mut changes: Vec<(&Claim, bool)> = Vec::new();
        for c in &after {
            if !before.iter().any(|b| b.text == c.text) {
                changes.push((c, true));
            }
        }
        for c in before {
            if !after.iter().any(|a| a.text == c.text) {
                changes.push((c, false));
            }
        }
        for (c, starts) in changes {
            if c.pos.dist(here) > c.range {
                continue;
            }
            let up = (c.amount > 0) == starts;
            let property = match c.property {
                Property::Heat => "heat",
                Property::Openness => "openness",
                Property::Stability => "stability",
            };
            let class = match c.class {
                Class::Passage => "passage",
                Class::Room => "room",
                Class::Land => "land",
                Class::Structure => "structure",
            };
            let ctx = ctx(&[
                ("property", Value::from(property)),
                ("rising", Value::Bool(up)),
                ("class", Value::from(class)),
                ("indoors", Value::Bool(self.state.place != Place::Outside)),
            ]);
            out.push(self.say("effect.change", ctx));
        }
        out
    }

    /// Heat writing gives the air here, for cues ("frost against the
    /// season", "an unnatural warmth").
    pub(crate) fn uncanny(&self) -> &'static str {
        let env = self.env();
        let heat = match self.state.place {
            Place::Room { structure, .. } => env.room_heat(structure),
            Place::Outside => env
                .claimed(Property::Heat, &[Class::Land], self.state.pos)
                .unwrap_or(0),
        };
        match heat {
            h if h > 0 => "warmth",
            h if h < 0 => "frost",
            _ => "none",
        }
    }

    /// Whether writing holds the current building's doors: +1 open, -1 shut.
    pub(crate) fn doors_held(&self) -> Option<i32> {
        match self.state.place {
            Place::Room { structure, .. } => self.env().held(structure),
            Place::Outside => None,
        }
    }

    /// Said when a writing tool is first picked up.
    pub(crate) fn tool_found(&mut self, kind: &str) -> Option<String> {
        if !matches!(
            kind,
            "scraper"
                | "stylus"
                | "lens"
                | "fine_scraper"
                | "old_scraper"
                | "first_scraper"
                | "first_lens"
        ) || self.state.found.contains(kind)
        {
            return None;
        }
        self.state.found.insert(kind.to_string());
        Some(self.say("tool.found", ctx(&[("kind", Value::from(kind))])))
    }

    /// Debug: what live writing does around here.
    pub(crate) fn claims_here(&self) -> Vec<serde_json::Value> {
        self.claims
            .iter()
            .filter(|c| c.pos.dist(self.state.pos) <= c.range)
            .map(|c| {
                serde_json::json!({
                    "text": c.text,
                    "claim": format!("{}{} {}", if c.negative { "not " } else { "" }, c.verb, c.subject), // DEBUG-TEXT: spoiler
                    "property": c.property,
                    "amount": c.amount,
                    "range": c.range,
                })
            })
            .collect()
    }
}
