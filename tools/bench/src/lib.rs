//! WebAssembly entry point for the language bench page.
//!
//! One call generates every era of a language and returns JSON in linear
//! memory; the page reads it with `output_len`. No bindings crate needed.

use std::cell::RefCell;

use scraped_lang::corpus::Corpus;
use scraped_lang::difficulty::{Difficulty, NameMarking, Regularity, Separation};
use scraped_lang::script::ScriptKind;
use scraped_lang::sheet::GrammarSheet;
use scraped_lang::Language;
use serde_json::json;

thread_local! {
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Glyph line width used by the page.
const LINE: usize = 16;

/// Dial codes: `separation` 0 spaces / 1 dots / 2 none; `script` 0 by seed /
/// 1 alphabet / 2 abjad / 3 syllabary; `flags` bit 0 = mark names, bit 1 =
/// fused morphology.
#[no_mangle]
pub extern "C" fn generate(
    seed_lo: u32,
    seed_hi: u32,
    count: u32,
    eras: u32,
    separation: u32,
    script: u32,
    flags: u32,
) -> *const u8 {
    let seed = (u64::from(seed_hi) << 32) | u64::from(seed_lo);
    let difficulty = Difficulty {
        separation: match separation {
            1 => Separation::Dots,
            2 => Separation::None,
            _ => Separation::Spaces,
        },
        names: if flags & 1 != 0 {
            NameMarking::Determinative
        } else {
            NameMarking::None
        },
        script: match script {
            1 => Some(ScriptKind::Alphabet),
            2 => Some(ScriptKind::Abjad),
            3 => Some(ScriptKind::Syllabary),
            _ => None,
        },
        eras: eras.clamp(1, 5),
        regularity: if flags & 2 != 0 {
            Regularity::Fused
        } else {
            Regularity::Regular
        },
    };
    let out: Vec<_> = Language::generate_with(seed, difficulty)
        .eras()
        .iter()
        .map(|lang| {
            let corpus = Corpus::generate(lang, count);
            let r = corpus.renderer();
            json!({
                "era": lang.era,
                "corpus": corpus.to_json(true),
                "plain": corpus.inscriptions.iter().map(|i| corpus.text(i)).collect::<Vec<_>>(),
                "glyphs": corpus.inscriptions.iter().map(|i| r.glyph_lines(&i.rendered, LINE)).collect::<Vec<_>>(),
                "sheet": GrammarSheet::new(lang),
                "sheetText": GrammarSheet::new(lang).to_text(),
            })
        })
        .collect();
    OUT.with(|o| {
        let mut o = o.borrow_mut();
        *o = serde_json::to_vec(&json!({ "seed": seed.to_string(), "eras": out })).expect("json");
        o.as_ptr()
    })
}

#[no_mangle]
pub extern "C" fn output_len() -> u32 {
    OUT.with(|o| o.borrow().len() as u32)
}
