//! Objects with histories in play (D05): examined, they show what they
//! are, what they're made of, how they've fared and whose emblem they
//! carry; looked at closer, a maker's mark, wear and mending, and the
//! border style of their era.

use scraped_content::{Context, Value};
use scraped_world::objects::{emblem, Emblem, Owner};

use crate::site::ctx;
use crate::{Game, Output};

impl Game {
    /// An emblem in words, through `emblem.describe`.
    pub(crate) fn emblem_words(&mut self, e: Emblem) -> String {
        let c = ctx(&[
            ("motif", Value::from(e.motif)),
            ("device", Value::from(e.device)),
            ("border", Value::from(e.border)),
        ]);
        let key = 30_000_000
            + scraped_world::objects::MOTIFS
                .iter()
                .position(|m| *m == e.motif)
                .unwrap_or(0) as u64
                * 64
            + scraped_world::objects::DEVICES
                .iter()
                .position(|d| *d == e.device)
                .unwrap_or(0) as u64
                * 8
            + scraped_world::objects::BORDERS
                .iter()
                .position(|b| *b == e.border)
                .unwrap_or(0) as u64;
        self.stable("emblem.describe", c, key)
    }

    /// `examine` an object: the first look, or (`closer`) a closer one.
    pub(crate) fn examine_object(&mut self, t: usize, closer: bool) -> Output {
        let o = self.site.world.objects[self.thing(t).object.expect("an object")].clone();
        let seed = self.site.world.seed;
        self.pass(if closer { 3 } else { 1 });
        let owner = match o.owner {
            Owner::Faction(_) => "people",
            Owner::Family(_) => "family",
            Owner::Temple(_) => "temple",
            Owner::Era(_) => "era",
        };
        let em = if o.marked {
            let e = emblem(seed, o.owner);
            self.emblem_words(e)
        } else {
            String::new()
        };
        let mut c: Context = ctx(&[
            ("kind", Value::from(o.kind)),
            ("family", Value::from(family_id(o.family))),
            ("material", Value::from(o.stuff)),
            ("condition", Value::from(o.condition)),
            ("emblem", Value::from(em)),
            ("owner", Value::from(if o.marked { owner } else { "" })),
            ("left", Value::Bool(o.event.is_some())),
        ]);
        if !closer {
            let tx = self.say("object.examine", c);
            return self.output(vec![tx], None);
        }
        let maker = match o.maker {
            Some(m) => {
                let e = emblem(seed, Owner::Family(m));
                self.emblem_words(e)
            }
            None => String::new(),
        };
        c.insert("maker".into(), Value::from(maker));
        c.insert(
            "border".into(),
            Value::from(emblem(seed, Owner::Era(o.era)).border),
        );
        let tx = self.say("object.closer", c);
        self.output(vec![tx], None)
    }
}

/// A family's id, as content sees it.
pub(crate) fn family_id(f: scraped_world::objects::Family) -> String {
    serde_json::to_value(f)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}
