//! Creatures: a few archetypes with simple behaviour. They wander, notice
//! the player, approach, and flee. There is no combat: fire, light and noise
//! keep them off, and the player avoids them.
//!
//! The movement rule holds here too: a creature can only strike unseen (an
//! ambush) from where the player could not see it a moment before.

use serde::{Deserialize, Serialize};

use crate::fixtures::{Spawn, Spot};
use crate::outdoors::{hash, Pos};

/// How a kind of creature behaves. Ids for content.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Archetype {
    pub id: &'static str,
    /// How close the player must be for it to notice, in metres.
    pub notice: f64,
    /// Metres it moves in an hour of pursuit.
    pub speed: f64,
    /// Out by day, by night, or both.
    pub by_day: bool,
    pub by_night: bool,
    pub fears_fire: bool,
    pub fears_noise: bool,
    /// Strikes only from hiding.
    pub ambush: bool,
    /// Levels of injury when it strikes.
    pub harm: u32,
    /// Takes food rather than doing harm.
    pub thief: bool,
}

// DESIGN-Q: four archetypes and their numbers. Scavengers steal food and
// fear fire and noise; grazers charge if you come close; predators strike
// only from hiding and fear fire; deep things live in dark underground
// rooms and fear light.
pub const ARCHETYPES: &[Archetype] = &[
    Archetype {
        id: "scavenger",
        notice: 400.0,
        speed: 3000.0,
        by_day: true,
        by_night: true,
        fears_fire: true,
        fears_noise: true,
        ambush: false,
        harm: 0,
        thief: true,
    },
    Archetype {
        id: "grazer",
        notice: 150.0,
        speed: 2000.0,
        by_day: true,
        by_night: false,
        fears_fire: true,
        fears_noise: false,
        ambush: false,
        harm: 1,
        thief: false,
    },
    Archetype {
        id: "predator",
        notice: 250.0,
        speed: 4000.0,
        by_day: false,
        by_night: true,
        fears_fire: true,
        fears_noise: false,
        ambush: true,
        harm: 2,
        thief: false,
    },
    Archetype {
        id: "deep",
        notice: 0.0,
        speed: 0.0,
        by_day: true,
        by_night: true,
        fears_fire: true,
        fears_noise: false,
        ambush: true,
        harm: 2,
        thief: false,
    },
];

pub fn archetype(id: &str) -> &'static Archetype {
    ARCHETYPES
        .iter()
        .find(|a| a.id == id)
        .expect("known archetype")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Wander,
    Approach,
    Flee,
}

/// A creature now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Creature {
    pub pos: Pos,
    pub mode: Mode,
    /// Whether the player could see it at the last check.
    pub seen: bool,
    /// It keeps away until this minute after striking or being driven off.
    pub calm_until: u32,
}

impl Creature {
    pub fn new(s: &Spawn) -> Self {
        Creature {
            pos: s.pos,
            mode: Mode::Wander,
            seen: false,
            calm_until: 0,
        }
    }
}

/// What happened between a creature and the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// It came into view.
    Sighted,
    /// It struck; `ambush` if from hiding.
    Struck { harm: u32, ambush: bool },
    /// It took food.
    Stole,
    /// Fire, light or noise drove it off.
    Fled,
}

/// What a creature can sense of the player.
pub struct Senses<'a> {
    pub seed: u64,
    pub id: usize,
    pub minutes: u32,
    /// Minutes since the last update.
    pub dt: u32,
    pub player: Pos,
    pub player_spot: Spot,
    pub night: bool,
    /// A lit torch, lamp or fire with the player.
    pub light: bool,
    /// 0 quiet to 3 very loud.
    pub noise: u8,
    pub carrying_food: bool,
    /// Whether the player could see a point (for creatures outdoors).
    pub visible: &'a dyn Fn(Pos) -> bool,
}

/// Moves a creature on and reports any meeting with the player.
pub fn update(c: &mut Creature, home: &Spawn, s: &Senses) -> Option<Event> {
    let a = archetype(home.archetype);
    if let Spot::Room { .. } = home.home {
        return update_deep(c, a, home, s);
    }
    let far = c.pos.dist(s.player) > 3000.0;
    let awake = if s.night { a.by_night } else { a.by_day };
    let calm = s.minutes < c.calm_until;
    let scared = (a.fears_fire && s.light) || (a.fears_noise && s.noise >= 2);
    let d = c.pos.dist(s.player);
    // Seen where it stood a moment ago (or at the last check).
    let was_seen = c.seen || (!far && (s.visible)(c.pos));
    let before = c.mode;
    c.mode = if far || calm || !awake || s.player_spot != Spot::out(s.player) {
        Mode::Wander
    } else if scared && d <= a.notice * 2.0 {
        Mode::Flee
    } else if d <= a.notice {
        Mode::Approach
    } else {
        Mode::Wander
    };
    let step = a.speed * f64::from(s.dt.max(1)) / 60.0;
    match c.mode {
        Mode::Wander => {
            // Drifts about its home, a new spot every hour.
            let h = hash(&[s.seed, 0xc7ea, s.id as u64, u64::from(s.minutes / 60)]);
            let off = |v: u64| (v % 801) as i32 - 400;
            c.pos = Pos::new(home.pos.x + off(h), home.pos.y + off(h >> 24));
        }
        Mode::Approach => {
            let k = (step / d.max(1.0)).min(1.0);
            c.pos = c.pos.moved(
                f64::from(s.player.x - c.pos.x) * k,
                f64::from(s.player.y - c.pos.y) * k,
            );
        }
        Mode::Flee => {
            let k = step / d.max(1.0);
            c.pos = c.pos.moved(
                f64::from(c.pos.x - s.player.x) * k,
                f64::from(c.pos.y - s.player.y) * k,
            );
        }
    }
    let now_seen = !far && (s.visible)(c.pos);
    c.seen = now_seen;
    if c.mode == Mode::Flee && before == Mode::Approach {
        c.calm_until = s.minutes + 120;
        return Some(Event::Fled);
    }
    if c.mode == Mode::Approach && c.pos.dist(s.player) <= 40.0 {
        c.calm_until = s.minutes + 6 * 60;
        c.mode = Mode::Flee;
        if a.thief {
            return s.carrying_food.then_some(Event::Stole);
        }
        if a.ambush {
            // Only from hiding: never from where the player could see it.
            return (!was_seen).then_some(Event::Struck {
                harm: a.harm,
                ambush: true,
            });
        }
        return Some(Event::Struck {
            harm: a.harm,
            ambush: false,
        });
    }
    if now_seen && !was_seen && d <= 1500.0 {
        return Some(Event::Sighted);
    }
    None
}

/// Something in a dark underground room: light keeps it back; in the dark,
/// it strikes.
fn update_deep(c: &mut Creature, a: &Archetype, home: &Spawn, s: &Senses) -> Option<Event> {
    if s.player_spot != home.home || s.minutes < c.calm_until {
        c.mode = Mode::Wander;
        c.seen = false;
        return None;
    }
    if s.light {
        c.calm_until = s.minutes + 60;
        let fled = c.mode != Mode::Flee;
        c.mode = Mode::Flee;
        return fled.then_some(Event::Fled);
    }
    // First it stirs, giving the player a moment to make light or leave;
    // then it strikes.
    if c.mode != Mode::Approach {
        c.mode = Mode::Approach;
        return Some(Event::Sighted);
    }
    c.mode = Mode::Flee;
    c.calm_until = s.minutes + 12 * 60;
    Some(Event::Struck {
        harm: a.harm,
        ambush: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn(arch: &'static str) -> Spawn {
        Spawn {
            species: None,
            archetype: arch,
            home: Spot::out(Pos::new(10_000, 10_000)),
            pos: Pos::new(10_000, 10_000),
        }
    }

    fn senses<'a>(
        player: Pos,
        minutes: u32,
        light: bool,
        visible: &'a dyn Fn(Pos) -> bool,
    ) -> Senses<'a> {
        Senses {
            seed: 1,
            id: 0,
            minutes,
            dt: 10,
            player,
            player_spot: Spot::out(player),
            night: true,
            light,
            noise: 0,
            carrying_food: true,
            visible,
        }
    }

    #[test]
    fn predators_strike_only_from_hiding() {
        let sp = spawn("predator");
        let player = Pos::new(10_100, 10_000);
        // In plain sight all along: it closes in but never strikes.
        let always = |_: Pos| true;
        let mut c = Creature::new(&sp);
        for m in 0..30 {
            let e = update(&mut c, &sp, &senses(player, m * 10, false, &always));
            assert!(!matches!(e, Some(Event::Struck { .. })), "struck from view");
        }
        // Hidden until it is upon the player: it strikes.
        let never = |_: Pos| false;
        let mut c = Creature::new(&sp);
        let mut struck = false;
        for m in 0..30 {
            if let Some(Event::Struck { ambush, .. }) =
                update(&mut c, &sp, &senses(player, m * 10, false, &never))
            {
                assert!(ambush);
                struck = true;
            }
        }
        assert!(struck);
        // A torch keeps it off.
        let mut c = Creature::new(&sp);
        for m in 0..30 {
            let e = update(&mut c, &sp, &senses(player, m * 10, true, &never));
            assert!(!matches!(e, Some(Event::Struck { .. })));
        }
    }

    #[test]
    fn scavengers_steal_and_deep_things_fear_light() {
        let sp = Spawn {
            species: None,
            archetype: "scavenger",
            ..spawn("scavenger")
        };
        let mut c = Creature::new(&sp);
        let player = Pos::new(10_200, 10_000);
        let always = |_: Pos| true;
        let events: Vec<Event> = (0..30)
            .filter_map(|m| update(&mut c, &sp, &senses(player, m * 10, false, &always)))
            .collect();
        assert!(events.contains(&Event::Stole), "{events:?}");

        let room = Spot::Room {
            structure: 3,
            room: 2,
        };
        let deep = Spawn {
            species: None,
            archetype: "deep",
            home: room,
            pos: Pos::new(0, 0),
        };
        let none = |_: Pos| false;
        let mut s = senses(Pos::new(0, 0), 100, true, &none);
        s.player_spot = room;
        let mut c = Creature::new(&deep);
        assert_eq!(update(&mut c, &deep, &s), Some(Event::Fled));
        let mut c = Creature::new(&deep);
        s.light = false;
        assert_eq!(update(&mut c, &deep, &s), Some(Event::Sighted));
        assert!(matches!(
            update(&mut c, &deep, &s),
            Some(Event::Struck { ambush: true, .. })
        ));
    }
}
