//! Content for *Scraped Again*: slots, the template language, and the pack.
//!
//! The engine never contains prose. It declares **slots** (places where it
//! needs text, with variables); Jb fills them with **templates** in a
//! **content pack** of TOML files under `content/`. This crate parses,
//! renders, lints and measures coverage of that pack. Pure logic, no I/O, so
//! the authoring tool runs it in the browser.

pub mod english;
pub mod lint;
pub mod pack;
pub mod render;
pub mod slot;
pub mod template;

pub use lint::{coverage, lint, release_check, Coverage, Issue, Severity, Status};
pub use pack::{Pack, PackError, PackFile, Variant};
pub use render::{Hooks, NoHooks, Renderer};
pub use slot::{Context, Registry, SlotDef, Value, VarType, SAMPLE_SEEDS};
