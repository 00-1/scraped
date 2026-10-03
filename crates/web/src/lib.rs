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
                    "role": w.towns.get(s.id).map(|t| t.role.id()),
                })).collect::<Vec<_>>(),
                "features": w.features.iter().map(|f| json!({
                    "kind": f.kind, "x": f.cell.x, "y": f.cell.y,
                    "group": scraped_world::features::kind(f.kind).group,
                })).collect::<Vec<_>>(),
                "places": scraped_world::debug::places(w),
                "greats": w.greats.iter().map(|&(i, k)| json!({
                    "structure": i, "kind": k.id(), "spaces": w.structures[i].interior.rooms.len(),
                })).collect::<Vec<_>>(),
                "counts": { "structures": w.structures.len(), "texts": w.texts.len(), "events": w.history.events.len() },
                "timeline": scraped_world::debug::timeline(w),
            })
        })),
        // D04: floor plans of an interior, a level at a time (spoilers).
        "plan" => {
            let structure: usize = field(req, "structure")?;
            Ok(with_world(seed(req), |w| {
                let st = &w.structures[structure.min(w.structures.len() - 1)];
                let levels: std::collections::BTreeSet<i8> = st.interior.rooms.iter().map(|r| r.level).collect();
                json!({
                    "kind": st.kind,
                    "shape": scraped_world::interiors::shape(&st.interior),
                    "levels": levels.iter().rev().map(|&l| json!({
                        "level": l,
                        "svg": scraped_world::debug::floor_plan(&st.interior, l, &[], None),
                    })).collect::<Vec<_>>(),
                })
            }))
        }
        "play_new" => {
            let (pack, _) = pack(req)?;
            let legacy: Option<scraped_game::ending::Legacy> = opt(req, "legacy", None);
            let preset: String = opt(req, "difficulty", "standard".to_string());
            let mut game = scraped_game::Game::create(seed(req), pack, &preset, legacy);
            game.spoil = opt(req, "spoil", false);
            game.trace = opt(req, "trace", false);
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
            game.trace = opt(req, "trace", false);
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
        "registry" => {
            let (pack, _) = pack(req)?;
            Ok(registry_json(&scraped_game::slots::registry_for(&pack)))
        }
        "storylet_schema" => Ok(scraped_game::storylets::schema()),
        // The world to play for a requested seed: itself if fair, else a
        // fair one derived from it.
        "fair_seed" => {
            let preset: String = opt(req, "difficulty", "standard".to_string());
            Ok(json!({ "seed": scraped_game::fairness::fair_seed(seed(req), &preset).to_string() }))
        }
        // The browser player's interface labels, from Jb's ui.label slot.
        "ui_labels" => {
            let (pack, _) = pack(req)?;
            let registry = scraped_game::slots::registry_for(&pack);
            let lang = Language::generate(1);
            let hooks = LangHooks { lang: &lang };
            let mut r = Renderer::new(&registry, &pack, 1, &hooks);
            let labels: serde_json::Map<String, Value> = scraped_game::slots::UI_LABELS
                .iter()
                .map(|id| {
                    let c: scraped_content::Context = [("id".to_string(), scraped_content::Value::from(*id))].into_iter().collect();
                    (id.to_string(), json!(r.render("ui.label", &c)))
                })
                .collect();
            Ok(Value::Object(labels))
        }
        // The Android app's interface labels, from Jb's app.label slot.
        "app_labels" => {
            let (pack, _) = pack(req)?;
            let registry = scraped_game::slots::registry_for(&pack);
            let lang = Language::generate(1);
            let hooks = LangHooks { lang: &lang };
            let mut r = Renderer::new(&registry, &pack, 1, &hooks);
            let labels: serde_json::Map<String, Value> = scraped_game::slots::APP_LABELS
                .iter()
                .map(|(id, _)| {
                    let c: scraped_content::Context = [("id".to_string(), scraped_content::Value::from(*id))].into_iter().collect();
                    (id.to_string(), json!(r.render("app.label", &c)))
                })
                .collect();
            Ok(Value::Object(labels))
        }
        "seed_decode" => {
            let code: String = field(req, "code")?;
            match scraped_game::seedcode::decode(&code) {
                Some(c) => Ok(json!({ "seed": c.seed.to_string(), "difficulty": c.preset, "pack": c.pack })),
                None => Err("that code doesn't read".to_string()), // DEBUG-TEXT
            }
        }
        "play_code" => GAME.with(|g| match g.borrow_mut().as_mut() {
            Some(game) => {
                let code = game.seed_code();
                Ok(json!({ "code": code, "text": game.say_code(&code), "difficulty": game.preset, "pack": game.pack_version() }))
            }
            None => Err("no game started".to_string()),
        }),
        // Determinism across platforms: the same as the native test's hash.
        // Debug: the lines the hash is made of.
        "transcript" => {
            let (pack, _) = pack(req)?;
            let preset: String = opt(req, "difficulty", "standard".to_string());
            let steps: usize = opt(req, "steps", 40);
            Ok(json!({ "lines": scraped_game::coverage::transcript_lines(&pack, seed(req), &preset, steps) }))
        }
        "transcript_hash" => {
            let (pack, _) = pack(req)?;
            let preset: String = opt(req, "difficulty", "standard".to_string());
            let steps: usize = opt(req, "steps", 40);
            Ok(json!({ "hash": scraped_game::coverage::transcript_hash(&pack, seed(req), &preset, steps) }))
        }
        // Bots play the given seeds; which text players meet, and how often.
        "coverage" => {
            let (pack, _) = pack(req)?;
            let seeds: Vec<u64> = opt::<Vec<String>>(req, "seeds", vec!["1".into(), "42".into()])
                .iter()
                .filter_map(|s| s.parse().ok())
                .collect();
            let steps: usize = opt(req, "steps", 120);
            Ok(json!(scraped_game::coverage::run(
                &pack,
                &seeds,
                steps.min(1000)
            )))
        }
        // The D01 depth metrics, for the bench and the authoring tool.
        "depth" => {
            let (pack, _) = pack(req)?;
            let seeds: Vec<u64> = opt::<Vec<String>>(req, "seeds", vec!["1".into()])
                .iter()
                .filter_map(|s| s.parse().ok())
                .take(5)
                .collect();
            let hours: f64 = opt(req, "hours", 6.0);
            Ok(json!(scraped_game::depth::report(
                &pack,
                &seeds,
                hours.clamp(1.0, 72.0)
            )))
        }
        "voice" => {
            let (pack, _) = pack(req)?;
            Ok(json!({
                "glossary": scraped_content::voice::glossary(&pack, opt(req, "min", 3)),
                "echoes": scraped_content::voice::echoes(&pack),
                "stats": scraped_content::voice::stats(&pack),
            }))
        }
        // What changed since `old` (the committed pack the tool was built
        // with), and whether saves still replay the same.
        "diff" => {
            let new: Pack = field(req, "pack")?;
            let old: Pack = match req.get("old") {
                Some(o) if !o.is_null() => {
                    serde_json::from_value(o.clone()).map_err(|e| e.to_string())?
                }
                _ => {
                    let files: Vec<(String, String)> =
                        opt::<Vec<FileText>>(req, "old_files", Vec::new())
                            .into_iter()
                            .map(|f| (f.path, f.text))
                            .collect();
                    Pack::load(&files).0
                }
            };
            Ok(json!(scraped_content::voice::diff(&old, &new)))
        }
        // Hot reload: the running playtest takes the new text.
        "play_pack" => {
            let (pack, _) = pack(req)?;
            GAME.with(|g| match g.borrow_mut().as_mut() {
                Some(game) => {
                    game.set_pack(pack);
                    Ok(json!({ "ok": true }))
                }
                None => Err("no game started".to_string()),
            })
        }
        // The run inspector: what the player knows and has done.
        "play_inspect" => GAME.with(|g| match g.borrow().as_ref() {
            Some(game) => Ok(game.inspect()),
            None => Err("no game started".to_string()),
        }),
        "storylet_preview" => {
            let (pack, _) = pack(req)?;
            let id: String = field(req, "id")?;
            let mut g = scraped_game::Game::new(seed(req), pack);
            Ok(g.preview_storylet(&id))
        }
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
            let registry = scraped_game::slots::registry_for(&pack);
            let mut issues = lint(&registry, &pack, &errors, &hooks);
            issues.extend(scraped_game::storylets::lint(&pack));
            issues.extend(scraped_content::voice::echoes(&pack));
            let cov = coverage(&registry, &pack, &issues);
            Ok(json!({ "issues": issues, "coverage": cov, "version": pack.version() }))
        }
        "lint_variant" => {
            let v: Variant = field(req, "variant")?;
            let (pack, _) = pack(req)?;
            let registry = scraped_game::slots::registry_for(&pack);
            let slot = registry
                .get(&v.slot)
                .ok_or(format!("no slot '{}'", v.slot))?;
            Ok(json!({ "issues": scraped_content::lint::lint_variant(&registry, slot, &v) }))
        }
        // Responses as the attention model assembles them (D02), so a
        // writer sees a piece among its neighbours and writes for the budget.
        "in_context" => {
            let (pack, _) = pack(req)?;
            let slot: String = opt(req, "slot", String::new());
            let count: usize = opt(req, "count", 6);
            Ok(json!({ "rows": scraped_game::bots::in_context(&pack, seed(req), &slot, count) }))
        }
        "preview" => {
            let (pack, _) = pack(req)?;
            let slot: String = field(req, "slot")?;
            let count: usize = opt(req, "count", 8);
            let seed = seed(req);
            let lang = Language::generate(seed);
            let hooks = LangHooks { lang: &lang };
            let registry = scraped_game::slots::registry_for(&pack);
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
    json!({
        "slots": slots,
        "helpers": scraped_content::render::HELPERS.iter().map(|h| json!({"name": h.name, "usage": h.usage})).collect::<Vec<_>>(),
        "review": scraped_game::slots::REVIEW.iter().map(|(f, stage)| json!({"family": f, "stage": stage})).collect::<Vec<_>>(),
    })
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
    fn storylets_in_the_browser() {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../content/story.toml"
        ))
        .unwrap();
        let files = json!([{"path": "story.toml", "text": text}]);
        let schema = call(json!({"cmd": "storylet_schema"}));
        assert!(schema["hooks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|h| h == "opening"));
        let reg = call(json!({"cmd": "registry", "files": files}));
        assert!(reg["slots"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["id"] == "story.shrine"));
        let mut placed = 0;
        for seed in ["1", "42", "7"] {
            let p = call(
                json!({"cmd": "storylet_preview", "files": files, "seed": seed, "id": "shrine"}),
            );
            if p["placed"] == true {
                placed += 1;
                assert_eq!(p["building"]["kind"], "temple");
                assert!(p["inscription"]["romanised"].is_string());
            }
        }
        assert!(placed >= 2);
        let lint = call(json!({"cmd": "lint", "files": files}));
        assert!(lint["issues"]
            .as_array()
            .unwrap()
            .iter()
            .all(|i| i["severity"] != "error"
                || !i["kind"].as_str().unwrap().starts_with("storylet")));
    }

    #[test]
    fn authoring_v2_commands() {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../content/say.toml"
        ))
        .unwrap();
        let files = json!([{"path": "say.toml", "text": text}]);
        let cov = call(json!({"cmd": "coverage", "files": files, "seeds": ["1"], "steps": 20}));
        assert!(cov["slots"].as_array().unwrap().len() > 3, "{cov}");
        assert!(cov["never"].is_array());
        let voice = call(json!({"cmd": "voice", "files": files}));
        assert!(voice["stats"].is_array());
        let parsed = call(json!({"cmd": "parse", "files": files}));
        let mut changed = parsed["pack"].clone();
        changed["files"][0]["variants"][0]["text"] = json!("Something new.");
        let d = call(json!({"cmd": "diff", "pack": changed, "old_files": files}));
        assert_eq!(d["changed"].as_array().unwrap().len(), 1);
        assert_eq!(d["breaks_saves"], false);
        // A traced playtest, hot reloaded, inspected.
        let first = call(json!({"cmd": "play_new", "seed": "42", "files": files, "trace": true}));
        assert!(
            first["renders"].as_array().is_some_and(|r| !r.is_empty()),
            "{first}"
        );
        assert_eq!(
            call(json!({"cmd": "play_pack", "pack": changed}))["ok"],
            true
        );
        let r = call(json!({"cmd": "play", "line": "help"}));
        assert!(r["renders"][0]["slot"].is_string());
        let inspect = call(json!({"cmd": "play_inspect"}));
        assert!(inspect["record"]["regions"].is_array());
        let reg = call(json!({"cmd": "registry"}));
        assert_eq!(reg["review"][0]["family"], "story");
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
