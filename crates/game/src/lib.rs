//! The game core for *Scraped Again*: state, commands, and what the player
//! is told.
//!
//! Pure logic, no I/O. A game is fully defined by its seed, the content
//! pack and the commands typed, so a save is just those and loading replays
//! them. Every sentence comes from a content slot; nothing in this crate is
//! prose.

pub mod parser;
pub mod site;
pub mod slots;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value as Json};

use scraped_content::{Context, Pack, Registry, Renderer, Value};
use scraped_lang::slots::{describe_glyph, LangHooks};
use scraped_world::structures::{Exit, PassageState};

use parser::{resolve, Candidate, Command, ParseError, Resolution};
use site::{ctx, label, light, time_of_day, Place, Site, Way};

/// Glyphs shown per page of reading.
pub const PAGE: usize = 16;

/// Something a command can refer to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "id")]
pub enum Target {
    Thing(usize),
    Structure(usize),
    /// A way out of the current room, by direction.
    Way(Exit),
}

/// A question the game is waiting for an answer to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pending {
    pub verb: String,
    pub options: Vec<Target>,
}

/// What the player is reading.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reading {
    pub thing: usize,
    pub page: usize,
}

/// Everything that changes during play.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub place: Place,
    pub carried: Vec<usize>,
    /// Things put down somewhere other than where they started.
    pub moved: BTreeMap<usize, Place>,
    /// Minutes since midnight of the first day.
    pub minutes: u32,
    /// The last thing referred to, for "it".
    pub it: Option<Target>,
    pub reading: Option<Reading>,
    /// The last thing read, for `define`.
    pub last_read: Option<usize>,
    /// The player's own glyph labels, keyed "era:glyph".
    pub labels: BTreeMap<String, String>,
    /// Doors the player has opened or closed: "structure:link" → open.
    pub doors: BTreeMap<String, bool>,
    pub pending: Option<Pending>,
}

/// A brief, machine-readable summary of what the player can perceive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Summary {
    pub place: String,
    pub things: Vec<String>,
    pub carried: Vec<String>,
    pub exits: Vec<String>,
    pub minutes: u32,
}

/// The response to one command.
#[derive(Debug, Clone, Serialize)]
pub struct Output {
    /// Exactly what a human player sees.
    pub text: String,
    pub state: Summary,
    /// Ground truth (only filled when asked for: spoilers).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truth: Option<Json>,
}

/// A saved game: everything needed to replay it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Save {
    pub seed: u64,
    pub pack_version: String,
    pub commands: Vec<String>,
}

/// A game in progress.
pub struct Game {
    pub site: Site,
    registry: Registry,
    pack: Pack,
    pub state: State,
    memory: BTreeMap<String, String>,
    log: Vec<String>,
    /// Whether outputs include ground truth.
    pub spoil: bool,
    glyph_cache: BTreeMap<(u32, usize), String>,
}

/// One glyph of a reading, or a gap between words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mark {
    Glyph {
        era: u32,
        index: usize,
    },
    Gap,
    /// Between separate pieces of writing on one thing.
    Break,
}

impl Game {
    /// A new game for `seed`, with Jb's content.
    pub fn new(seed: u64, pack: Pack) -> Self {
        let site = Site::new(seed);
        let state = State {
            place: Place::Outside,
            carried: Vec::new(),
            moved: BTreeMap::new(),
            minutes: 8 * 60,
            it: None,
            reading: None,
            last_read: None,
            labels: BTreeMap::new(),
            doors: BTreeMap::new(),
            pending: None,
        };
        Game {
            site,
            registry: slots::registry(),
            pack,
            state,
            memory: BTreeMap::new(),
            log: Vec::new(),
            spoil: false,
            glyph_cache: BTreeMap::new(),
        }
    }

    pub fn seed(&self) -> u64 {
        self.site.world.seed
    }

    /// The opening text.
    pub fn start(&mut self) -> Output {
        let intro = self.say(
            "say.intro",
            ctx(&[("biome", Value::from(self.site.biome()))]),
        );
        let look = self.look();
        self.output(vec![intro, look], None)
    }

    /// Saves the game as its seed, pack version and commands.
    pub fn save(&self) -> Save {
        Save {
            seed: self.seed(),
            pack_version: self.pack.version(),
            commands: self.log.clone(),
        }
    }

    /// Loads a save by replaying it. Also returns whether the content pack
    /// differs from the one the save was made with.
    pub fn load(save: &Save, pack: Pack) -> (Self, bool) {
        let changed = save.pack_version != pack.version();
        let mut g = Game::new(save.seed, pack);
        g.start();
        for c in &save.commands {
            g.step(c);
        }
        (g, changed)
    }

    /// The current state summary with no text, for session-level replies.
    pub fn step_quiet(&mut self) -> Output {
        self.output(Vec::new(), None)
    }

    /// Text for a game-level event the client handles (saved, loaded…).
    pub fn message(&mut self, slot: &str) -> String {
        self.say(slot, Context::new())
    }

    // ---------- rendering ----------

    fn renderer_say(&mut self, slot: &str, c: &Context, seed: u64, keep_memory: bool) -> String {
        let lang = &self.site.world.languages[0];
        let hooks = LangHooks { lang };
        let mut r = Renderer::new(&self.registry, &self.pack, seed, &hooks);
        if keep_memory {
            r.set_memory(std::mem::take(&mut self.memory));
        }
        let out = r.render(slot, c);
        if keep_memory {
            self.memory = r.memory();
        }
        out
    }

    /// Renders a description; variants may vary from turn to turn.
    fn say(&mut self, slot: &str, c: Context) -> String {
        let seed = self.seed() ^ u64::from(self.state.minutes);
        self.renderer_say(slot, &c, seed, true)
    }

    /// Renders a name or other text that must never change, so the player
    /// (and the parser) can rely on it.
    fn stable(&mut self, slot: &str, c: Context, key: u64) -> String {
        let seed = self.seed() ^ key.wrapping_mul(0x9e37_79b9_7f4a_7c15);
        self.renderer_say(slot, &c, seed, false)
    }

    fn thing_name(&mut self, id: usize) -> String {
        let c = self.site.thing_vars(&self.site.things[id]);
        self.stable("thing.name", c, 1_000 + id as u64)
    }

    fn structure_name(&mut self, id: usize) -> String {
        let c = self.site.structure_vars(id);
        self.stable("place.structure", c, 500_000 + id as u64)
    }

    fn way_name(&mut self, w: &Way) -> String {
        let c = self.site.way_vars(w);
        let key = match w.to {
            Place::Room { structure, room } => 900_000 + structure as u64 * 64 + room as u64,
            Place::Outside => 0,
        };
        self.stable("place.exit", c, key)
    }

    fn glyph_description(&mut self, era: u32, index: usize) -> String {
        if let Some(d) = self.glyph_cache.get(&(era, index)) {
            return d.clone();
        }
        let lang = &self.site.world.languages[era as usize];
        let glyph = lang.script.glyphs[index].1.clone();
        let hooks = LangHooks { lang };
        let seed = self.seed() ^ (u64::from(era) << 32 | index as u64);
        let mut r = Renderer::new(&self.registry, &self.pack, seed, &hooks);
        let d = describe_glyph(&mut r, &glyph);
        self.glyph_cache.insert((era, index), d.clone());
        d
    }

    // ---------- the world as the player sees it ----------

    /// Ways out of the current place, with doors the player has moved.
    fn ways(&self) -> Vec<Way> {
        let Place::Room { structure, .. } = self.state.place else {
            return Vec::new();
        };
        self.site
            .ways(self.state.place)
            .into_iter()
            .map(|mut w| {
                match self.state.doors.get(&format!("{structure}:{}", w.link)) {
                    Some(true) if w.state == PassageState::Closed => w.state = PassageState::Open,
                    Some(false) if w.state == PassageState::Open => w.state = PassageState::Closed,
                    _ => {}
                }
                w
            })
            .collect()
    }

    fn where_is(&self, thing: usize) -> Option<Place> {
        if self.state.carried.contains(&thing) {
            return None;
        }
        Some(
            *self
                .state
                .moved
                .get(&thing)
                .unwrap_or(&self.site.things[thing].home),
        )
    }

    fn here(&self) -> Vec<usize> {
        (0..self.site.things.len())
            .filter(|&t| self.where_is(t) == Some(self.state.place))
            .collect()
    }

    fn visible_targets(&mut self) -> Vec<Candidate<Target>> {
        let mut out = Vec::new();
        for t in self.here().into_iter().chain(self.state.carried.clone()) {
            let name = self.thing_name(t);
            out.push(Candidate::new(
                Target::Thing(t),
                &name,
                &[self.site.things[t].kind],
            ));
        }
        for w in self.ways() {
            let name = self.way_name(&w);
            let dir = label(&w.exit);
            out.push(Candidate::new(Target::Way(w.exit), &name, &[&dir]));
        }
        if self.state.place == Place::Outside {
            for s in self.site.structures.clone() {
                let name = self.structure_name(s);
                let kind = label(&self.site.structure(s).kind);
                out.push(Candidate::new(Target::Structure(s), &name, &[&kind]));
            }
        }
        out
    }

    fn look(&mut self) -> String {
        let time = time_of_day(self.state.minutes);
        match self.state.place {
            Place::Outside => {
                let names: Vec<Value> = self
                    .site
                    .structures
                    .clone()
                    .into_iter()
                    .map(|s| Value::from(self.structure_name(s)))
                    .collect();
                let mut things: Vec<Value> = Vec::new();
                for t in self.here() {
                    things.push(Value::from(self.thing_name(t)));
                }
                let mut all = names.clone();
                all.extend(things);
                let c = ctx(&[
                    ("biome", Value::from(self.site.biome())),
                    ("structures", Value::List(all)),
                    ("count", Value::Number(names.len() as i64)),
                    (
                        "abandoned",
                        Value::Bool(
                            self.site.world.history.settlements[self.site.settlement]
                                .abandoned
                                .is_some(),
                        ),
                    ),
                    ("time", Value::from(time)),
                ]);
                self.say("place.site", c)
            }
            Place::Room { structure, room } => {
                let st = self.site.structure(structure);
                let r = &st.interior.rooms[room];
                let (purpose, level, kind, condition) =
                    (r.purpose, r.level, label(&st.kind), label(&st.condition));
                let things: Vec<Value> = self
                    .here()
                    .into_iter()
                    .map(|t| Value::from(self.thing_name(t)))
                    .collect();
                let mut exits: Vec<Value> = self
                    .ways()
                    .iter()
                    .map(|w| Value::from(self.way_name(w)))
                    .collect();
                if room == 0 {
                    exits.push(Value::from(self.stable("place.out", Context::new(), 7)));
                }
                let c = ctx(&[
                    ("purpose", Value::from(purpose)),
                    ("structure", Value::from(kind)),
                    ("condition", Value::from(condition)),
                    ("level", Value::Number(i64::from(level))),
                    (
                        "light",
                        Value::from(light(self.state.place, &self.site, self.state.minutes)),
                    ),
                    ("things", Value::List(things)),
                    ("exits", Value::List(exits)),
                    ("time", Value::from(time)),
                ]);
                self.say("place.room", c)
            }
        }
    }

    fn summary(&mut self) -> Summary {
        let place = match self.state.place {
            Place::Outside => "outside".to_string(),
            Place::Room { structure, room } => format!("structure {structure} room {room}"), // DEBUG-TEXT: machine-readable id
        };
        let things = self
            .here()
            .into_iter()
            .map(|t| self.thing_name(t))
            .collect();
        let carried = self
            .state
            .carried
            .clone()
            .into_iter()
            .map(|t| self.thing_name(t))
            .collect();
        let exits = match self.state.place {
            Place::Outside => self
                .site
                .structures
                .clone()
                .into_iter()
                .map(|s| self.structure_name(s))
                .collect(),
            _ => self.ways().iter().map(|w| label(&w.exit)).collect(),
        };
        Summary {
            place,
            things,
            carried,
            exits,
            minutes: self.state.minutes,
        }
    }

    fn output(&mut self, parts: Vec<String>, truth: Option<Json>) -> Output {
        let text = parts
            .into_iter()
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
        let state = self.summary();
        Output {
            text,
            state,
            truth: if self.spoil { truth } else { None },
        }
    }

    // ---------- commands ----------

    /// Runs one command and describes the result.
    pub fn step(&mut self, input: &str) -> Output {
        self.log.push(input.to_string());
        let parsed = parser::parse(input);
        // While a "which one?" question is open, anything that is not a
        // command is taken as the answer.
        if let Some(pending) = self.state.pending.take() {
            if matches!(parsed, Err(ParseError::UnknownVerb(_))) {
                let words: Vec<String> = input
                    .to_lowercase()
                    .split_whitespace()
                    .map(str::to_string)
                    .collect();
                let cands: Vec<Candidate<Target>> = self
                    .visible_targets()
                    .into_iter()
                    .filter(|c| pending.options.contains(&c.target))
                    .collect();
                if let Resolution::One(t) = resolve(&words, &cands, None) {
                    return self.act(&pending.verb, t);
                }
            }
        }
        let cmd = match parsed {
            Ok(c) => c,
            Err(ParseError::Empty) => {
                let t = self.say("say.empty", Context::new());
                return self.output(vec![t], None);
            }
            Err(ParseError::UnknownVerb(w)) => {
                let t = self.say("say.unknown_verb", ctx(&[("word", Value::from(w))]));
                return self.output(vec![t], None);
            }
        };
        self.command(cmd)
    }

    fn pass(&mut self, minutes: u32) {
        self.state.minutes += minutes;
    }

    fn command(&mut self, cmd: Command) -> Output {
        let rule = parser::verbs()
            .iter()
            .find(|v| v.id == cmd.verb)
            .map(|v| v.object);
        if rule == Some(parser::ObjectRule::Required) && cmd.words.is_empty() {
            let t = self.say(
                "say.need_object",
                ctx(&[("verb", Value::from(cmd.verb.as_str()))]),
            );
            return self.output(vec![t], None);
        }
        match cmd.verb.as_str() {
            "look" => {
                self.pass(1);
                let t = self.look();
                self.output(vec![t], None)
            }
            "inventory" => {
                let items: Vec<Value> = self
                    .state
                    .carried
                    .clone()
                    .into_iter()
                    .map(|t| Value::from(self.thing_name(t)))
                    .collect();
                let n = items.len() as i64;
                let t = self.say(
                    "say.inventory",
                    ctx(&[("items", Value::List(items)), ("count", Value::Number(n))]),
                );
                self.output(vec![t], None)
            }
            "wait" => {
                self.pass(30);
                let t = self.say("say.wait", ctx(&[("minutes", Value::Number(30))]));
                self.output(vec![t], None)
            }
            "help" => {
                let t = self.say("say.help", Context::new());
                self.output(vec![t], None)
            }
            "more" => self.more(),
            "define" => self.define(&cmd.words),
            "out" => self.go_out(),
            "north" | "south" | "east" | "west" | "up" | "down" => self.go_dir(&cmd.verb),
            "go" => {
                // "go north" is a direction; anything else names a place.
                if let [d] = cmd.words.as_slice() {
                    if let Some(v) = parser::parse(d).ok().filter(|c| {
                        c.words.is_empty()
                            && ["north", "south", "east", "west", "up", "down"]
                                .contains(&c.verb.as_str())
                    }) {
                        return self.go_dir(&v.verb);
                    }
                }
                if matches!(cmd.words.as_slice(), [w] if w == "out" || w == "outside") {
                    return self.go_out();
                }
                self.with_target("go", &cmd.words)
            }
            verb => self.with_target(verb, &cmd.words),
        }
    }

    fn with_target(&mut self, verb: &str, words: &[String]) -> Output {
        let cands = self.visible_targets();
        match resolve(words, &cands, self.state.it.as_ref()) {
            Resolution::One(t) => self.act(verb, t),
            Resolution::None => {
                let t = self.say(
                    "say.not_here",
                    ctx(&[("words", Value::from(words.join(" ")))]),
                );
                self.output(vec![t], None)
            }
            Resolution::Many(options) => {
                let names: Vec<Value> = options
                    .iter()
                    .map(|o| Value::from(self.target_name(*o)))
                    .collect();
                self.state.pending = Some(Pending {
                    verb: verb.to_string(),
                    options,
                });
                let t = self.say("say.which", ctx(&[("options", Value::List(names))]));
                self.output(vec![t], None)
            }
        }
    }

    fn target_name(&mut self, t: Target) -> String {
        match t {
            Target::Thing(i) => self.thing_name(i),
            Target::Structure(i) => self.structure_name(i),
            Target::Way(e) => match self.ways().into_iter().find(|w| w.exit == e) {
                Some(w) => self.way_name(&w),
                None => label(&e),
            },
        }
    }

    fn act(&mut self, verb: &str, target: Target) -> Output {
        self.state.it = Some(target);
        let name = self.target_name(target);
        let named = ctx(&[("thing", Value::from(name.as_str()))]);
        match (verb, target) {
            ("examine", Target::Thing(i)) => {
                self.pass(2);
                let c = self.site.thing_vars(&self.site.things[i]);
                let t = self.say("thing.examine", c);
                self.output(vec![t], None)
            }
            ("examine", Target::Structure(s)) | ("go", Target::Structure(s)) => {
                if verb == "examine" {
                    self.pass(1);
                }
                if !self.site.enterable(s) {
                    let t = self.say("say.buried", named);
                    return self.output(vec![t], None);
                }
                self.pass(1);
                self.state.place = Place::Room {
                    structure: s,
                    room: 0,
                };
                let t = self.look();
                self.output(vec![t], None)
            }
            ("take", Target::Thing(i)) => {
                if self.state.carried.contains(&i) {
                    let t = self.say("say.take_held", named);
                    return self.output(vec![t], None);
                }
                if !self.site.things[i].portable {
                    let t = self.say("say.take_fixed", named);
                    return self.output(vec![t], None);
                }
                self.pass(1);
                self.state.carried.push(i);
                self.state.moved.remove(&i);
                let t = self.say("say.take", named);
                self.output(vec![t], None)
            }
            ("drop", Target::Thing(i)) => {
                if !self.state.carried.contains(&i) {
                    let t = self.say("say.drop_unheld", named);
                    return self.output(vec![t], None);
                }
                self.pass(1);
                self.state.carried.retain(|&c| c != i);
                if self.site.things[i].home != self.state.place {
                    self.state.moved.insert(i, self.state.place);
                } else {
                    self.state.moved.remove(&i);
                }
                let t = self.say("say.drop", named);
                self.output(vec![t], None)
            }
            ("read", Target::Thing(i)) => {
                if self.site.things[i].texts.is_empty() {
                    let t = self.say("read.nothing", named);
                    return self.output(vec![t], None);
                }
                self.state.reading = Some(Reading { thing: i, page: 0 });
                self.state.last_read = Some(i);
                self.page()
            }
            ("go", Target::Way(e)) => self.go_dir(&label(&e)),
            ("open", Target::Way(e)) | ("close", Target::Way(e)) => self.door(verb == "open", e),
            ("examine", Target::Way(e)) => {
                let w = self
                    .ways()
                    .into_iter()
                    .find(|w| w.exit == e)
                    .expect("resolved from ways");
                let t = self.way_name(&w);
                self.output(vec![t], None)
            }
            ("go", Target::Thing(_)) | (_, Target::Structure(_)) => {
                let t = self.say("say.not_here", ctx(&[("words", Value::from(name))]));
                self.output(vec![t], None)
            }
            _ => {
                let t = self.say("say.not_here", ctx(&[("words", Value::from(name))]));
                self.output(vec![t], None)
            }
        }
    }

    /// Opens or closes a door. Blocked and collapsed ways stay as they are.
    fn door(&mut self, open: bool, exit: Exit) -> Output {
        let Place::Room { structure, .. } = self.state.place else {
            unreachable!("ways exist only in rooms")
        };
        let w = self
            .ways()
            .into_iter()
            .find(|w| w.exit == exit)
            .expect("resolved from ways");
        let named = ctx(&[("thing", Value::from(self.way_name(&w)))]);
        let movable = !w.collapsed
            && w.state != PassageState::Blocked
            && w.passage == Some(scraped_world::structures::Passage::Door);
        let already = (w.state == PassageState::Open) == open;
        let slot = match (movable, already, open) {
            (false, _, _) => "say.door_stuck",
            (true, true, true) => "say.door_already_open",
            (true, true, false) => "say.door_already_closed",
            (true, false, true) => "say.door_open",
            (true, false, false) => "say.door_close",
        };
        if movable && !already {
            self.pass(1);
            self.state
                .doors
                .insert(format!("{structure}:{}", w.link), open);
        }
        let t = self.say(slot, named);
        self.output(vec![t], None)
    }

    fn go_out(&mut self) -> Output {
        match self.state.place {
            Place::Outside => {
                let t = self.say("say.outside_already", Context::new());
                self.output(vec![t], None)
            }
            Place::Room { room: 0, .. } => {
                self.pass(1);
                self.state.place = Place::Outside;
                let t = self.look();
                self.output(vec![t], None)
            }
            Place::Room { .. } => {
                let t = self.say("say.not_entrance", Context::new());
                self.output(vec![t], None)
            }
        }
    }

    fn go_dir(&mut self, dir: &str) -> Output {
        let exit = match dir {
            "north" => Exit::North,
            "south" => Exit::South,
            "east" => Exit::East,
            "west" => Exit::West,
            "up" => Exit::Up,
            _ => Exit::Down,
        };
        let way = self.ways().into_iter().find(|w| w.exit == exit);
        let Some(way) = way else {
            let t = self.say("say.no_exit", ctx(&[("direction", Value::from(dir))]));
            return self.output(vec![t], None);
        };
        if way.collapsed || way.state != PassageState::Open {
            let c = self.site.way_vars(&way);
            let t = self.say("say.blocked", c);
            return self.output(vec![t], None);
        }
        self.pass(1);
        self.state.place = way.to;
        let t = self.look();
        self.output(vec![t], None)
    }

    // ---------- reading ----------

    /// The glyphs of everything written on a thing, in reading order.
    fn marks(&self, thing: usize) -> Vec<Mark> {
        let mut out = Vec::new();
        for (n, &tid) in self.site.things[thing].texts.iter().enumerate() {
            if n > 0 {
                out.push(Mark::Break);
            }
            let text = &self.site.world.texts[tid];
            let r = self.site.world.renderer(text.era);
            let rendered = r.render(&text.meaning);
            let script = &self.site.world.languages[text.era as usize].script;
            for k in r.glyphs(&rendered) {
                out.push(match k {
                    Some(k) => Mark::Glyph {
                        era: text.era,
                        index: script.index(&k),
                    },
                    None => Mark::Gap,
                });
            }
        }
        out
    }

    fn page(&mut self) -> Output {
        let Some(Reading { thing, page }) = self.state.reading.clone() else {
            let t = self.say("read.no_more", Context::new());
            return self.output(vec![t], None);
        };
        let marks = self.marks(thing);
        let glyphs: Vec<(usize, Mark)> = marks
            .iter()
            .filter(|m| matches!(m, Mark::Glyph { .. }))
            .enumerate()
            .map(|(i, m)| (i + 1, *m))
            .collect();
        let pages = glyphs.len().div_ceil(PAGE).max(1);
        let first = page * PAGE + 1;
        let last = ((page + 1) * PAGE).min(glyphs.len());
        self.pass(5);
        let thing_name = self.thing_name(thing);
        let t = &self.site.things[thing];
        let era = self.site.world.texts[t.texts[0]].era as usize;
        let frame_ctx = ctx(&[
            ("thing", Value::from(thing_name)),
            ("material", Value::from(label(&t.material))),
            ("glyphs", Value::Number(glyphs.len() as i64)),
            ("page", Value::Number(page as i64 + 1)),
            ("pages", Value::Number(pages as i64)),
            ("texts", Value::Number(t.texts.len() as i64)),
            (
                "direction",
                Value::from(label(&self.site.world.languages[era].script.direction)),
            ),
        ]);
        let frame = self.say("read.frame", frame_ctx);
        // Lay the page out: one glyph per line, a blank line between words,
        // a rule between separate pieces of writing.
        let mut lines: Vec<String> = Vec::new();
        let mut n = 0;
        for m in &marks {
            match *m {
                Mark::Glyph { era, index } => {
                    n += 1;
                    if n < first || n > last {
                        continue;
                    }
                    let description = self.glyph_description(era, index);
                    let label = self
                        .state
                        .labels
                        .get(&format!("{era}:{index}"))
                        .cloned()
                        .unwrap_or_default();
                    let c = ctx(&[
                        ("number", Value::Number(n as i64)),
                        ("description", Value::from(description)),
                        ("known", Value::Bool(!label.is_empty())),
                        ("label", Value::from(label)),
                    ]);
                    let line = self.stable("read.glyph", c, 2_000_000 + n as u64);
                    lines.push(line);
                }
                Mark::Gap
                    if n >= first && n < last && lines.last().is_some_and(|l| !l.is_empty()) =>
                {
                    lines.push(String::new())
                }
                Mark::Break if n >= first && n < last => lines.push("—".to_string()),
                _ => {}
            }
        }
        let body = lines.join("\n");
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
        let truth = json!(self.site.things[thing]
            .texts
            .iter()
            .map(|&tid| {
                let text = &self.site.world.texts[tid];
                let r = self.site.world.renderer(text.era);
                json!({
                    "era": text.era,
                    "kind": text.kind,
                    "text": self.site.world.surface(text),
                    "translation": scraped_lang::english::translate(&text.meaning, &|p| r.name(p)),
                })
            })
            .collect::<Vec<_>>());
        self.output(vec![frame, body, after], Some(truth))
    }

    fn more(&mut self) -> Output {
        self.page()
    }

    /// `define 3 as ka`: the player's own label for glyph 3 of the last
    /// writing read. Never checked against the truth.
    fn define(&mut self, words: &[String]) -> Output {
        let input = words.join(" ");
        let number = words
            .iter()
            .find_map(|w| w.parse::<usize>().ok())
            .unwrap_or(0);
        let label = words
            .iter()
            .position(|w| w == "as")
            .and_then(|i| words.get(i + 1))
            .or(words.last().filter(|w| w.parse::<usize>().is_err()))
            .map(|w| w.trim_matches(|c| c == '"' || c == '\'').to_string())
            .unwrap_or_default();
        let glyphs: Vec<(u32, usize)> = self
            .state
            .last_read
            .map(|t| {
                self.marks(t)
                    .into_iter()
                    .filter_map(|m| match m {
                        Mark::Glyph { era, index } => Some((era, index)),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        let c = ctx(&[
            ("number", Value::Number(number as i64)),
            ("label", Value::from(label.as_str())),
            ("count", Value::Number(glyphs.len() as i64)),
            ("input", Value::from(input)),
        ]);
        if number == 0 || number > glyphs.len() || label.is_empty() {
            let t = self.say("say.define_bad", c);
            return self.output(vec![t], None);
        }
        let (era, index) = glyphs[number - 1];
        self.state.labels.insert(format!("{era}:{index}"), label);
        let t = self.say("say.define", c);
        self.output(vec![t], None)
    }
}
