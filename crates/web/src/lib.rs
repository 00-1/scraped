//! The engine for browsers: one WebAssembly module behind a tiny JSON
//! interface, shared by the language bench and the authoring tool.
//!
//! JavaScript writes a JSON request into memory from `alloc`, calls `call`,
//! and reads the JSON reply from the returned pointer (`output_len` bytes).
//! No bindings generator: the interface is three functions.

use std::cell::RefCell;

use scraped_content::{coverage, lint, Pack, Registry, Renderer, Variant, SAMPLE_SEEDS};
use scraped_lang::corpus::Corpus;
use scraped_lang::difficulty::{Difficulty, NameMarking, Regularity, Separation};
use scraped_lang::script::ScriptKind;
use scraped_lang::sheet::GrammarSheet;
use scraped_lang::slots::{self, LangHooks};
use scraped_lang::Language;
use serde::Deserialize;
use serde_json::{json, Value};

thread_local! {
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    /// The game being played in the browser.
    static GAME: RefCell<Option<scraped_game::Game>> = const { RefCell::new(None) };
    /// The last world generated, so site views don't regenerate it.
    static WORLD: RefCell<Option<scraped_world::World>> = const { RefCell::new(None) };
}

/// Runs `f` on the world for `seed`, generating it only when the seed changes.
fn with_world<T>(seed: u64, f: impl FnOnce(&scraped_world::World) -> T) -> T {
    WORLD.with(|w| {
        let mut w = w.borrow_mut();
        if w.as_ref().is_none_or(|w| w.seed != seed) {
            *w = Some(scraped_world::World::generate(seed));
        }
        f(w.as_ref().expect("just generated"))
    })
}

/// Reserves `len` bytes for a request and returns where to write it.
#[no_mangle]
pub extern "C" fn alloc(len: u32) -> *mut u8 {
    let mut buf = vec![0u8; len as usize];
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// Handles the JSON request at `ptr` (`len` bytes, from `alloc`) and returns
/// a pointer to the JSON reply.
///
/// # Safety
/// `ptr` must come from `alloc(len)` and hold `len` bytes of UTF-8.
#[no_mangle]
pub unsafe extern "C" fn call(ptr: *mut u8, len: u32) -> *const u8 {
    let request = Vec::from_raw_parts(ptr, len as usize, len as usize);
    let reply = handle(&String::from_utf8_lossy(&request));
    OUT.with(|o| {
        let mut o = o.borrow_mut();
        *o = reply.into_bytes();
        o.as_ptr()
    })
}

/// Length of the last reply.
#[no_mangle]
pub extern "C" fn output_len() -> u32 {
    OUT.with(|o| o.borrow().len() as u32)
}

/// Handles one request. Public so it can be tested natively.
pub fn handle(request: &str) -> String {
    let reply = match serde_json::from_str::<Value>(request) {
        Ok(req) => dispatch(&req).unwrap_or_else(|e| json!({ "error": e })),
        Err(e) => json!({ "error": format!("bad request: {e}") }),
    };
    reply.to_string()
}

fn field<'a, T: Deserialize<'a>>(req: &'a Value, name: &str) -> Result<T, String> {
    T::deserialize(&req[name]).map_err(|e| format!("'{name}': {e}"))
}

fn opt<'a, T: Deserialize<'a>>(req: &'a Value, name: &str, default: T) -> T {
    if req[name].is_null() {
        default
    } else {
        T::deserialize(&req[name]).unwrap_or(default)
    }
}

fn seed(req: &Value) -> u64 {
    match &req["seed"] {
        Value::String(s) => s.trim().parse().unwrap_or(42),
        Value::Number(n) => n.as_u64().unwrap_or(42),
        _ => 42,
    }
}

/// The pack, from `pack` (JSON, as the tool holds it) or `files` (TOML texts).
fn pack(req: &Value) -> Result<(Pack, Vec<scraped_content::PackError>), String> {
    if !req["pack"].is_null() {
        return Ok((field(req, "pack")?, Vec::new()));
    }
    let files: Vec<(String, String)> = opt::<Vec<FileText>>(req, "files", Vec::new())
        .into_iter()
        .map(|f| (f.path, f.text))
        .collect();
    Ok(Pack::load(&files))
}

#[derive(Deserialize, serde::Serialize)]
struct FileText {
    path: String,
    text: String,
}

fn dispatch(req: &Value) -> Result<Value, String> {
    let cmd: String = field(req, "cmd")?;
    let registry = scraped_game::slots::registry();
    match cmd.as_str() {
        "bench" => Ok(bench(req)),
        "world" => Ok(with_world(seed(req), |w| {
            let (side, px) = scraped_world::debug::pixels(w, 1);
            let lang = |e: u32| &w.languages[e as usize];
            json!({
                "seed": w.seed.to_string(),
                "side": side,
                "pixels": base64(&px),
                "shape": w.terrain.shape,
                "trajectory": w.history.trajectory,
                "settlements": w.history.settlements.iter().map(|s| json!({
                    "id": s.id,
                    "name": scraped_lang::render::capitalise(&lang(s.era).romanise(&s.name)),
                    "x": s.cell.x, "y": s.cell.y,
                    "era": s.era, "abandoned": s.abandoned.is_some(), "capital": s.capital,
                })).collect::<Vec<_>>(),
                "counts": { "structures": w.structures.len(), "texts": w.texts.len(), "events": w.history.events.len() },
                "timeline": scraped_world::debug::timeline(w),
            })
        })),
        "play_new" => {
            let (pack, _) = pack(req)?;
            let legacy: Option<scraped_game::ending::Legacy> = opt(req, "legacy", None);
            let mut game = scraped_game::Game::with_legacy(seed(req), pack, legacy);
            game.spoil = opt(req, "spoil", false);
            let first = game.start();
            GAME.with(|g| *g.borrow_mut() = Some(game));
            Ok(json!(first))
        }
        "play" => {
            let line: String = field(req, "line")?;
            GAME.with(|g| match g.borrow_mut().as_mut() {
                Some(game) => Ok(json!(game.step(&line))),
                None => Err("no game started".to_string()),
            })
        }
        // After the end of a run: its legacy (if any) and the notebook.
        "play_end" => GAME.with(|g| match g.borrow_mut().as_mut() {
            Some(game) => {
                let (transcript, places) = game.notebook_files();
                Ok(json!({
                    "ended": game.ending(),
                    "legacy": game.legacy(),
                    "transcript": transcript,
                    "places": places,
                    "record": game.record(),
                }))
            }
            None => Err("no game started".to_string()),
        }),
        "play_save" => GAME.with(|g| match g.borrow().as_ref() {
            Some(game) => Ok(json!(game.save())),
            None => Err("no game started".to_string()),
        }),
        "play_load" => {
            let (pack, _) = pack(req)?;
            let save: scraped_game::Save = field(req, "save")?;
            let (mut game, changed) = scraped_game::Game::load(&save, pack);
            game.spoil = opt(req, "spoil", false);
            let mut text = game.message("say.loaded");
            if changed {
                text.push_str("\n\n");
                text.push_str(&game.message("say.pack_changed"));
            }
            let look = game.step("look");
            GAME.with(|g| *g.borrow_mut() = Some(game));
            Ok(
                json!({ "text": format!("{text}\n\n{}", look.text), "state": look.state, "truth": look.truth }),
            )
        }
        "regions" => {
            let g = scraped_game::Game::new(seed(req), scraped_content::Pack::default());
            Ok(json!({ "text": g.regions_debug(opt(req, "days", 180)) }))
        }
        "writing" => {
            let site = scraped_game::site::Site::new(seed(req));
            Ok(
                json!({ "text": scraped_sim::writing::debug(&site.world, &site.land, &site.writing, &site.writing.scraped) }),
            )
        }
        "site" => {
            let id: usize = field(req, "settlement")?;
            Ok(with_world(
                seed(req),
                |w| json!({ "text": scraped_world::debug::site(w, id) }),
            ))
        }
        "registry" => Ok(registry_json(&registry)),
        "parse" => {
            let (pack, errors) = pack(req)?;
            Ok(json!({ "pack": pack, "errors": errors, "version": pack.version() }))
        }
        "write" => {
            let pack: Pack = field(req, "pack")?;
            let files: Vec<FileText> = pack
                .files
                .iter()
                .map(|f| FileText {
                    path: f.path.clone(),
                    text: f.to_toml(),
                })
                .collect();
            Ok(json!({ "files": files, "version": pack.version() }))
        }
        "lint" => {
            let (pack, errors) = pack(req)?;
            let lang = Language::generate(42);
            let hooks = LangHooks { lang: &lang };
            let issues = lint(&registry, &pack, &errors, &hooks);
            let cov = coverage(&registry, &pack, &issues);
            Ok(json!({ "issues": issues, "coverage": cov, "version": pack.version() }))
        }
        "lint_variant" => {
            let v: Variant = field(req, "variant")?;
            let slot = registry
                .get(&v.slot)
                .ok_or(format!("no slot '{}'", v.slot))?;
            Ok(json!({ "issues": scraped_content::lint::lint_variant(&registry, slot, &v) }))
        }
        "preview" => {
            let (pack, _) = pack(req)?;
            let slot: String = field(req, "slot")?;
            let count: usize = opt(req, "count", 8);
            let seed = seed(req);
            let lang = Language::generate(seed);
            let hooks = LangHooks { lang: &lang };
            let mut r = Renderer::new(&registry, &pack, seed, &hooks);
            let rows = slots::preview(&mut r, &slot, seed, count);
            Ok(json!({ "seed": seed.to_string(), "rows": rows }))
        }
        other => Err(format!("unknown command '{other}'")),
    }
}

/// Standard base64, for shipping pixels to JavaScript.
fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (u32::from(c[0]) << 16)
            | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
            | u32::from(*c.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= c.len() {
                out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The registry plus a few sample variable sets per slot, for the tool.
fn registry_json(registry: &Registry) -> Value {
    let slots: Vec<Value> = registry
        .slots
        .iter()
        .map(|s| {
            let mut v = serde_json::to_value(s).expect("slot serialises");
            let samples: Vec<_> = s
                .samples(&SAMPLE_SEEDS[..2])
                .into_iter()
                .take(4)
                .map(|(_, c)| c)
                .collect();
            v["family"] = json!(s.family());
            v["examples"] = json!(samples);
            v
        })
        .collect();
    json!({ "slots": slots, "helpers": scraped_content::render::HELPERS.iter().map(|h| json!({"name": h.name, "usage": h.usage})).collect::<Vec<_>>() })
}

/// Every era of a language, for the bench page.
fn bench(req: &Value) -> Value {
    let flags: u32 = opt(req, "flags", 0);
    let difficulty = Difficulty {
        separation: match opt::<u32>(req, "separation", 0) {
            1 => Separation::Dots,
            2 => Separation::None,
            _ => Separation::Spaces,
        },
        names: if flags & 1 != 0 {
            NameMarking::Determinative
        } else {
            NameMarking::None
        },
        script: match opt::<u32>(req, "script", 0) {
            1 => Some(ScriptKind::Alphabet),
            2 => Some(ScriptKind::Abjad),
            3 => Some(ScriptKind::Syllabary),
            _ => None,
        },
        eras: opt::<u32>(req, "eras", 3).clamp(1, 5),
        regularity: if flags & 2 != 0 {
            Regularity::Fused
        } else {
            Regularity::Regular
        },
    };
    let seed = seed(req);
    let count: u32 = opt(req, "count", 30);
    let eras: Vec<Value> = Language::generate_with(seed, difficulty)
        .eras()
        .iter()
        .map(|lang| {
            let corpus = Corpus::generate(lang, count);
            let r = corpus.renderer();
            json!({
                "era": lang.era,
                "corpus": corpus.to_json(true),
                "plain": corpus.inscriptions.iter().map(|i| corpus.text(i)).collect::<Vec<_>>(),
                "glyphs": corpus.inscriptions.iter().map(|i| r.glyph_lines(&i.rendered, Corpus::GLYPH_LINE)).collect::<Vec<_>>(),
                "sheet": GrammarSheet::new(lang),
                "sheetText": GrammarSheet::new(lang).to_text(),
            })
        })
        .collect();
    json!({ "seed": seed.to_string(), "eras": eras })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(req: Value) -> Value {
        serde_json::from_str(&handle(&req.to_string())).unwrap()
    }

    #[test]
    fn registry_lists_glyph_slots() {
        let r = call(json!({"cmd": "registry"}));
        let ids: Vec<&str> = r["slots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["id"].as_str().unwrap())
            .collect();
        assert!(ids.contains(&"glyph.stroke"));
        assert!(!r["slots"][0]["examples"].as_array().unwrap().is_empty());
    }

    #[test]
    fn parse_write_round_trip_and_lint() {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../content/glyphs.toml"
        ))
        .unwrap();
        let parsed =
            call(json!({"cmd": "parse", "files": [{"path": "glyphs.toml", "text": text}]}));
        assert!(parsed["errors"].as_array().unwrap().is_empty());
        let written = call(json!({"cmd": "write", "pack": parsed["pack"]}));
        let again = call(json!({"cmd": "parse", "files": written["files"]}));
        assert_eq!(again["pack"], parsed["pack"]);
        let lint = call(json!({"cmd": "lint", "pack": parsed["pack"]}));
        assert!(lint["coverage"].as_array().unwrap().len() >= 2);
    }

    #[test]
    fn preview_renders_real_glyphs() {
        let pack = json!({"files": [{"path": "glyphs.toml", "variants": [
            {"slot": "glyph.stroke", "text": "{a stroke}"},
            {"slot": "glyph.describe", "text": "{cap list strokes}."}
        ]}]});
        let r = call(
            json!({"cmd": "preview", "pack": pack, "slot": "glyph.describe", "seed": "7", "count": 3}),
        );
        let rows = r["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 3);
        assert!(rows[0]["text"].as_str().unwrap().starts_with('A'));
    }

    #[test]
    fn lint_variant_reports_unknown_variables() {
        let r = call(
            json!({"cmd": "lint_variant", "variant": {"slot": "glyph.stroke", "text": "{colour}"}}),
        );
        assert_eq!(r["issues"][0]["kind"], "unknown-variable");
    }

    #[test]
    fn bench_matches_native_corpus() {
        let r = call(json!({"cmd": "bench", "seed": "42", "count": 40}));
        let lang = Language::generate(42);
        let corpus = Corpus::generate(&lang, 40);
        assert_eq!(
            r["eras"][0]["plain"][0],
            corpus.text(&corpus.inscriptions[0])
        );
    }

    #[test]
    fn world_and_site_views() {
        let r = call(json!({"cmd": "world", "seed": "3"}));
        assert_eq!(r["side"], 160);
        assert!(!r["settlements"].as_array().unwrap().is_empty());
        let site = call(json!({"cmd": "site", "seed": "3", "settlement": 0}));
        assert!(site["text"].as_str().unwrap().starts_with("SITE 0"));
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
    }

    #[test]
    fn play_in_the_browser() {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../content/say.toml"
        ))
        .unwrap();
        let first = call(
            json!({"cmd": "play_new", "seed": "42", "files": [{"path": "say.toml", "text": text}]}),
        );
        assert!(!first["text"].as_str().unwrap().is_empty());
        let r = call(json!({"cmd": "play", "line": "i"}));
        assert!(r["text"].as_str().unwrap().contains("nothing"));
        let end = call(json!({"cmd": "play_end"}));
        assert!(end["ended"].is_null(), "{end}");
        assert!(end["record"]["regions"].is_array());
        let save = call(json!({"cmd": "play_save"}));
        assert_eq!(save["commands"][0], "i");
    }

    #[test]
    fn bad_requests_get_errors_not_panics() {
        assert!(call(json!({"cmd": "nope"}))["error"].is_string());
        assert!(handle("not json").contains("error"));
    }
}
