//! Things the player can carry and use, by kind.

/// One kind of item and what it is good for. Numbers are coarse on purpose.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemKind {
    pub id: &'static str,
    /// In rough kilograms.
    pub weight: u32,
    /// Degrees of warmth when worn.
    pub warmth: i32,
    /// Gives light when lit.
    pub light: bool,
    /// Minutes it burns when lit (torch), or holds (lamp), or feeds a fire
    /// or lamp (wood, oil).
    pub burns: u32,
    /// Hours of hunger a meal of it takes away.
    pub meal: u32,
    /// Drinks of water it can hold.
    pub holds: u32,
    /// What it is for, beyond the above.
    pub tool: Option<&'static str>,
}

const BASE: ItemKind = ItemKind {
    id: "",
    weight: 1,
    warmth: 0,
    light: false,
    burns: 0,
    meal: 0,
    holds: 0,
    tool: None,
};

/// Every item kind. Ids double as content variable values.
// DESIGN-Q: the item list and its numbers (a torch burns an hour, a bundle
// of wood two hours of fire, a meal of provisions lasts twelve hours).
pub const ITEMS: &[ItemKind] = &[
    ItemKind {
        id: "torch",
        light: true,
        burns: 60,
        ..BASE
    },
    ItemKind {
        id: "lamp",
        weight: 2,
        light: true,
        burns: 240,
        ..BASE
    },
    ItemKind {
        id: "oil",
        burns: 360,
        tool: Some("lamp_fuel"),
        ..BASE
    },
    ItemKind {
        id: "firesteel",
        tool: Some("fire"),
        ..BASE
    },
    ItemKind {
        id: "wood",
        weight: 3,
        burns: 120,
        tool: Some("fuel"),
        ..BASE
    },
    ItemKind {
        id: "waterskin",
        holds: 4,
        ..BASE
    },
    ItemKind {
        id: "cloak",
        weight: 2,
        warmth: 8,
        ..BASE
    },
    ItemKind {
        id: "provisions",
        meal: 12,
        ..BASE
    },
    ItemKind {
        id: "berries",
        meal: 4,
        ..BASE
    },
    ItemKind {
        id: "pry_bar",
        weight: 3,
        tool: Some("pry"),
        ..BASE
    },
    ItemKind {
        id: "scraper",
        tool: Some("writing"),
        ..BASE
    },
    ItemKind {
        id: "stylus",
        tool: Some("writing"),
        ..BASE
    },
    ItemKind {
        id: "lens",
        tool: Some("writing"),
        ..BASE
    },
    // Portable things from the world itself.
    ItemKind {
        id: "jar",
        weight: 2,
        holds: 3,
        ..BASE
    },
    ItemKind {
        id: "tablet",
        weight: 2,
        ..BASE
    },
    ItemKind {
        id: "scroll",
        ..BASE
    },
];

/// Item ids, for content enums.
pub fn ids() -> Vec<&'static str> {
    ITEMS.iter().map(|i| i.id).collect()
}

pub fn kind(id: &str) -> Option<&'static ItemKind> {
    ITEMS.iter().find(|i| i.id == id)
}

/// How much the player can carry, in the same units as `weight`.
// DESIGN-Q: a carrying limit of 15.
pub const CARRY: u32 = 15;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_are_unique_and_sensible() {
        let mut ids = ids();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), ITEMS.len());
        for i in ITEMS {
            assert!(i.weight >= 1 && i.weight <= CARRY);
            assert!(
                !i.light || i.burns > 0,
                "{} gives light, so it must burn",
                i.id
            );
        }
    }
}
