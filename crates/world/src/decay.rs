//! The present: what time, weather and abandonment have done.

use serde::Serialize;

use scraped_lang::rng::{Rng, Stream};

use crate::history::{Effect, EventKind, History};
use crate::structures::{Condition, Material, PassageState, Structure, StructureKind};
use crate::terrain::Terrain;

/// Physical evidence of an event that is not writing: a ruin, a wall.
#[derive(Debug, Clone, Serialize)]
pub struct Trace {
    pub event: usize,
    pub structure: usize,
}

/// Ages every structure to the present and returns the traces left by
/// events, plus the effects old writing still has on the land.
pub fn apply(
    seed: u64,
    t: &Terrain,
    h: &History,
    structures: &mut [Structure],
) -> (Vec<Trace>, Vec<Effect>) {
    let mut rng = Rng::new(seed, Stream::World(5));
    let mut traces = Vec::new();
    for s in structures.iter_mut() {
        let age = h.present - s.built;
        let wet = *t.moisture.get(s.cell.ux(), s.cell.uy());
        let sturdy = match s
            .interior
            .rooms
            .first()
            .and_then(|r| r.features.first())
            .map(|f| f.material)
        {
            Some(Material::Stone) | None => 1.0,
            Some(Material::Clay) => 0.8,
            _ => 0.6,
        };
        let sturdy = if matches!(
            s.kind,
            StructureKind::Tomb | StructureKind::Temple | StructureKind::Bridge
        ) {
            sturdy * 1.4
        } else {
            sturdy
        };
        let abandoned = s
            .settlement
            .and_then(|i| h.settlements[i].abandoned)
            .map(|a| h.present - a);
        // Wear grows with age and damp; abandonment speeds it up.
        let wear = (f64::from(age) / 650.0) * (0.5 + wet) / sturdy
            + abandoned.map_or(0.0, |a| f64::from(a) / 300.0);
        let roll = f64::from(rng.below(100)) / 100.0;
        s.condition = match wear + roll * 0.6 {
            w if w < 0.7 => Condition::Intact,
            w if w < 1.3 => Condition::Worn,
            w if w < 2.0 => Condition::Damaged,
            w if w < 2.8 => Condition::Ruined,
            _ => Condition::Buried,
        };
        // Damaged buildings lose rooms; the entrance always survives so a
        // ruin can still be entered.
        let collapse = match s.condition {
            Condition::Intact => 0,
            Condition::Worn => 5,
            Condition::Damaged => 20,
            Condition::Ruined => 40,
            Condition::Buried => 55,
        };
        for r in s.interior.rooms.iter_mut().skip(1) {
            if rng.chance(collapse) {
                r.collapsed = true;
            }
        }
        for l in &mut s.interior.links {
            if rng.chance(collapse / 2) {
                l.state = PassageState::Blocked;
            } else if rng.chance(25) {
                l.state = PassageState::Closed;
            }
        }
    }
    for e in &h.events {
        match &e.kind {
            EventKind::Abandonment { settlement, .. } => {
                for s in structures
                    .iter_mut()
                    .filter(|s| s.settlement == Some(*settlement))
                {
                    if s.condition < Condition::Ruined {
                        s.condition = Condition::Ruined;
                    }
                    traces.push(Trace {
                        event: e.id,
                        structure: s.id,
                    });
                }
            }
            EventKind::War { .. } => {
                for s in structures.iter().filter(|s| s.event == Some(e.id)) {
                    traces.push(Trace {
                        event: e.id,
                        structure: s.id,
                    });
                }
            }
            _ => {}
        }
    }
    let effects = h
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Writing { effect, .. } => Some(effect.clone()),
            _ => None,
        })
        .collect();
    (traces, effects)
}
