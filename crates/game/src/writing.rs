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
        if id >= scraped_sim::writing::SEALED_BASE {
            self.site.writing.text(&self.site.world, id)
        } else if id >= base {
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
                "loupe" => 2,
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

    /// What covers a written surface, if anything (D10): moss or lichen on
    /// stone out of doors, soot or dust within. Gone once cleaned.
    // DESIGN-Q: two in five surfaces out of doors or in worn buildings are
    // grimed, one in eight elsewhere, four in five where a spell waits;
    // grime hides two signs in five.
    pub(crate) fn grime(&self, thing: usize) -> Option<&'static str> {
        if self.state.cleaned.contains(&thing) {
            return None;
        }
        let t = self.thing(thing);
        if t.texts.is_empty() {
            return None;
        }
        let (outside, worn) = match t.home {
            Place::Outside => (true, true),
            Place::Room { structure, .. } => (
                false,
                self.site.world.structures[structure].condition
                    != scraped_world::structures::Condition::Intact,
            ),
        };
        // Writing never cast was never tended either: a spell left waiting
        // mostly lies under moss or soot.
        let waiting = top_unscraped_of(&self.layers(thing), &self.state.scraped)
            .is_some_and(|t| self.text(t).kind == scraped_lang::corpus::Kind::Potent);
        let h = hash(&[self.seed(), 0x6e1e, thing as u64]);
        let chance = match (waiting, worn) {
            (true, _) => 80,
            (false, true) => 40,
            (false, false) => 12,
        };
        if h % 100 >= chance {
            return None;
        }
        let covers: &[&'static str] = if outside {
            &["moss", "lichen", "grime"]
        } else {
            &["soot", "dust", "grime"]
        };
        Some(covers[((h >> 12) % covers.len() as u64) as usize])
    }

    /// Whether glyph `g` of a text on `thing` is hidden by grime.
    pub(crate) fn grimed(&self, thing: usize, text: usize, g: usize) -> bool {
        self.grime(thing).is_some() && hash(&[self.seed(), 0x6e1f, text as u64, g as u64]) % 5 < 2
    }

    /// Writing taken off by the world itself (D10): falling stone scours a
    /// room's surfaces; floodwater washes the soft ones (clay, plaster,
    /// wood, vellum). The top layer comes away whole, as any scrape, and
    /// whatever it held is loose. Returns what the player feels of it.
    pub(crate) fn world_strips(
        &mut self,
        structure: usize,
        room: usize,
        soft: bool,
    ) -> Vec<String> {
        let here = Place::Room { structure, room };
        let mut stripped = false;
        let before = self.claims.clone();
        for t in 0..self.thing_count() {
            if self.thing(t).home != here || self.thing(t).texts.is_empty() {
                continue;
            }
            if soft
                && matches!(
                    self.thing(t).material,
                    scraped_world::structures::Material::Stone
                        | scraped_world::structures::Material::Metal
                )
            {
                continue;
            }
            let layers = self.layers(t);
            if let Some(top) = top_unscraped_of(&layers, &self.state.scraped) {
                // Only what was meant to act is set loose: plain writing
                // simply wears away unread.
                if self.text(top).kind == scraped_lang::corpus::Kind::Potent {
                    self.state.scraped.insert(top);
                    self.state.cleaned.insert(t);
                    stripped = true;
                }
            }
        }
        if !stripped {
            return Vec::new();
        }
        self.recompute_claims();
        self.hook("first_accident", "", "");
        self.claim_changes(&before)
    }

    /// `clean the stele` (D10): with an edged or abrasive tool in hand,
    /// the grime comes off and the whole top layer with it, exactly as a
    /// scrape; by hand, only the grime.
    pub(crate) fn clean(&mut self, thing: usize) -> Output {
        let name = self.thing_name(thing);
        if self.power() > 0 && !self.layers(thing).is_empty() {
            self.state.cleaned.insert(thing);
            return self.scrape(thing);
        }
        match self.grime(thing) {
            Some(cover) => {
                self.pass(15);
                self.state.cleaned.insert(thing);
                let t = self.say(
                    "clean.done",
                    ctx(&[("thing", Value::from(name)), ("cover", Value::from(cover))]),
                );
                self.output(vec![t], None)
            }
            None => {
                let t = self.say("clean.nothing", ctx(&[("thing", Value::from(name))]));
                self.output(vec![t], None)
            }
        }
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
        // The player's own text (the sealed places' accounts lie above it).
        let mine = text >= base && text < scraped_sim::writing::SEALED_BASE;
        if mine && self.state.minutes < self.state.written[text - base].minutes + DRYING {
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
        if mine {
            self.state.released.insert(text, power);
        }
        self.recompute_claims();
        self.record_acts(text, &drivers_before);
        self.last_scrape = Some(text);
        let backlash = mine && self.backlash(text - base);
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
        if self.claims.iter().any(|c| c.text == text) && mine {
            felt.extend(self.release_feeling(power));
            self.hook("first_release", "", "");
        } else if self.claims.iter().any(|c| c.text == text) {
            // Old writing set loose, most likely by accident (D10).
            self.hook("first_accident", "", "");
        }
        self.scrape_felt = !felt.is_empty();
        parts.extend(felt);
        if backlash {
            // A malformed claim in the potent frame turns on its writer.
            parts.push(self.say("write.backlash", Context::new()));
            self.hurt(1, "writing");
        } else if mine {
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
            let property = c.property.name();
            let class = c.class.name();
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

    /// Evidence of the spells acting where the player is (D09), one fact
    /// for each kind of thing and quality pushed here: in a building, the
    /// spells on it; out of doors, those on the land, water, plants, beasts
    /// and air that reach this spot. Heat, doors and loose stone have
    /// their own facts.
    pub(crate) fn spell_cues(&self, out: &mut Vec<crate::attention::Fact>) {
        use scraped_sim::writing::Property as Q;
        let env = self.env();
        let (at, inside, place_key) = match self.state.place {
            Place::Room { structure, .. } => (
                self.site.land.structure_pos[structure],
                true,
                format!("s{structure}"),
            ),
            Place::Outside => {
                let (x, y) = self.state.pos.cell();
                (self.state.pos, false, format!("c{x}:{y}"))
            }
        };
        let here: Vec<&Claim> = self
            .claims
            .iter()
            .filter(|c| {
                !matches!(c.property, Q::Heat | Q::Openness)
                    && !(c.property == Q::Stability && c.class == Class::Structure)
            })
            .filter(|c| c.class.indoors() == inside && c.class != Class::Person)
            .filter(|c| c.pos.dist(at) <= c.range && c.acts(&env.when))
            .collect();
        let mut pairs: Vec<(Class, Q)> = here.iter().map(|c| (c.class, c.property)).collect();
        pairs.sort_unstable();
        pairs.dedup();
        let resolved: Vec<(Class, Q, i32, &str)> = pairs
            .iter()
            .filter_map(|&(class, q)| {
                let amount = env.claimed(q, &[class], at)?;
                let thing = here
                    .iter()
                    .filter(|c| c.class == class && c.property == q)
                    .min_by(|a, b| a.pos.dist2(at).cmp(&b.pos.dist2(at)))?
                    .subject
                    .as_str();
                (amount != 0).then_some((class, q, amount, thing))
            })
            .collect();
        for &(class, q, amount, thing) in &resolved {
            // Two qualities on one kind of thing make one sight.
            let with = resolved
                .iter()
                .find(|o| o.0 == class && o.1 != q)
                .map(|o| (o.1.name(), o.2 > 0));
            let strength = amount.abs().min(3);
            out.push(crate::attention::Fact::new(
                "spell.cue",
                format!(
                    "cue:{place_key}:{}:{}:{}",
                    class.name(),
                    q.name(),
                    amount.signum()
                ),
                26.0 + 6.0 * f64::from(strength),
                ctx(&[
                    ("quality", Value::from(q.name())),
                    ("rising", Value::Bool(amount > 0)),
                    ("class", Value::from(class.name())),
                    ("thing", Value::from(thing.replace('_', " "))),
                    ("strength", Value::Number(i64::from(strength))),
                    ("with", Value::from(with.map_or("", |w| w.0))),
                    ("with_rising", Value::Bool(with.is_some_and(|w| w.1))),
                    ("indoors", Value::Bool(inside)),
                ]),
            ));
        }
    }

    /// Whether writing holds a thing where it lies (D09): bound fast, or
    /// made too heavy to lift, by a spell on things of its kind in this
    /// building.
    pub(crate) fn held_by_writing(&self, thing: usize) -> Option<&'static str> {
        use scraped_sim::writing::Property as Q;
        let Place::Room { structure, .. } = self.state.place else {
            return None;
        };
        let kind = self.thing(thing).kind;
        let env = self.env();
        let at = self.site.land.structure_pos[structure];
        let on_it = |q: Q| {
            let mine: Vec<&Claim> = self
                .claims
                .iter()
                .filter(|c| c.class == Class::Thing && c.property == q && c.subject == kind)
                .filter(|c| c.acts(&env.when))
                .collect();
            scraped_sim::writing::resolve(&mine, at).is_some_and(|a| a > 0)
        };
        if on_it(Q::Binding) {
            Some("bound")
        } else if on_it(Q::Weight) {
            Some("heavy")
        } else {
            None
        }
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
    /// The ward stone of a sealed place (D11): the thing out of doors that
    /// carries its ward.
    pub(crate) fn ward_stone(&self, structure: usize) -> Option<usize> {
        let sealed = self
            .site
            .writing
            .sealed
            .iter()
            .find(|x| x.structure == structure)?;
        (0..self.site.things.len()).find(|&t| self.site.things[t].texts.contains(&sealed.ward))
    }

    /// Whether a sealed place's way in is shut (D11): it stays shut until
    /// what is live on its ward stone is a spell that opens, acting now.
    /// Nothing written anywhere else moves it.
    pub(crate) fn sealed_shut(&self, structure: usize) -> bool {
        let Some(stone) = self.ward_stone(structure) else {
            return false;
        };
        let Some(live) = live_of(&self.layers(stone), &self.state.scraped) else {
            return true;
        };
        let now = self.spell_now();
        !self.claims.iter().any(|c| {
            c.text == live
                && c.property == scraped_sim::writing::Property::Openness
                && c.amount > 0
                && c.acts(&now)
        })
    }

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
            "knife"
                | "pumice"
                | "stylus"
                | "lens"
                | "penknife"
                | "mason_chisel"
                | "graver"
                | "loupe"
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
