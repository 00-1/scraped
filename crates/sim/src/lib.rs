//! The physical world of *Scraped Again*: what the player can perceive,
//! the local properties of places and how they interact, mechanisms, items,
//! the body's needs, and creatures.
//!
//! Pure logic, no I/O, deterministic. Writing (M08) pushes the same
//! properties through the same rule table.

pub mod body;
pub mod creatures;
pub mod env;
pub mod fixtures;
pub mod items;
pub mod outdoors;
pub mod region;
pub mod rules;
pub mod writing;
