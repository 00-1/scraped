//! Builds Jb's content pack into the library, so the app needs no files of
//! its own to play.

use std::fmt::Write as _;
use std::path::Path;

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("content folder")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    files.sort();
    let mut out = String::from(
        "/// Jb's content, as (file name, text).\npub static CONTENT: &[(&str, &str)] = &[\n",
    );
    for f in files {
        println!("cargo:rerun-if-changed={}", f.display());
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        let path = f.canonicalize().unwrap();
        writeln!(
            out,
            "    ({name:?}, include_str!({:?})),",
            path.display().to_string()
        )
        .unwrap();
    }
    out.push_str("];\n");
    let dest = Path::new(&std::env::var("OUT_DIR").unwrap()).join("content.rs");
    std::fs::write(dest, out).unwrap();
}
