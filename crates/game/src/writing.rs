//! Writing in play: what can be read of each surface, scraping, and the
//! physical cues of claims taking effect.

use scraped_content::{Context, Value};
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

    /// The layers a reader can see: (text, partial), oldest first. With the
    /// deep-reading lens, the ghost layer beneath shows too, fainter.
    pub(crate) fn layers_seen(&self, thing: usize) -> Vec<(usize, bool)> {
        let layers = self.layers(thing);
        let mut out = visible_of(&layers, &self.state.scraped);
        if let Some(deep) = self.deep_layer(thing) {
            out.insert(0, (deep, true));
        }
        out
    }

    /// The layer the lens shows, if the player carries it.
    pub(crate) fn deep_layer(&self, thing: usize) -> Option<usize> {
        if !self
            .state
            .carried
            .iter()
            .any(|&t| self.thing(t).kind == "lens")
        {
            return None;
        }
        beneath_of(&self.layers(thing), &self.state.scraped)
    }

    pub(crate) fn ghost_count(&self, thing: usize) -> usize {
        let layers = self.layers(thing);
        let ghosts = ghosts_of(&layers, &self.state.scraped);
        ghosts - usize::from(self.deep_layer(thing).is_some())
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
        // glyphs in daylight, 20% in dim light.
        let shown = match (deep, light == "daylight") {
            (false, true) => 60,
            (false, false) => 40,
            (true, true) => 35,
            (true, false) => 20,
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
                if let Some(c) = claim_of(&self.site.world, &self.site.land, self.text(live), live)
                {
                    claims.push(c);
                }
            }
        }
        self.claims = claims;
    }

    /// Scrapes the whole top unscraped text from a surface. Needs the
    /// scraping tool. The text stays, as a scraped layer; if it was potent,
    /// its claim now acts.
    pub(crate) fn scrape(&mut self, thing: usize) -> Output {
        let name = self.thing_name(thing);
        let named = ctx(&[("thing", Value::from(name.as_str()))]);
        let has_tool = self
            .state
            .carried
            .iter()
            .any(|&t| self.thing(t).kind == "scraper");
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
        self.state.scraped.insert(text);
        self.recompute_claims();
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
        let felt = self.claim_changes(&before);
        self.scrape_felt = !felt.is_empty();
        parts.extend(felt);
        if backlash {
            // A malformed claim in the potent frame turns on its writer.
            parts.push(self.say("write.backlash", Context::new()));
            self.hurt(1, "writing");
        }
        self.output(parts, None)
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
        if !matches!(kind, "scraper" | "stylus" | "lens") || self.state.found.contains(kind) {
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
