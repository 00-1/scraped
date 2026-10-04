//! Bakes the content pack into the player program (C01): the templates
//! only, with the authoring notes taken out.

use std::path::Path;

fn main() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    println!("cargo:rerun-if-changed={}", src.display());
    let out = Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("content");
    std::fs::create_dir_all(&out).expect("content dir");
    let mut names: Vec<String> = std::fs::read_dir(&src)
        .expect("content folder")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".toml"))
        .collect();
    names.sort();
    let mut list = String::from(
        "/// The content pack's files, as baked in.\npub const FILES: &[(&str, &str)] = &[\n",
    );
    for n in &names {
        println!("cargo:rerun-if-changed={}", src.join(n).display());
        let text = std::fs::read_to_string(src.join(n)).expect("content file");
        let kept: String = text
            .replace("\r\n", "\n")
            .lines()
            .filter(|l| !l.trim_start().starts_with("notes ="))
            .filter(|l| !l.trim_start().starts_with('#'))
            .map(|l| format!("{l}\n"))
            .collect();
        std::fs::write(out.join(n), kept).expect("write content");
        list.push_str(&format!(
            "    ({n:?}, include_str!(concat!(env!(\"OUT_DIR\"), \"/content/{n}\"))),\n"
        ));
    }
    list.push_str("];\n");
    std::fs::write(
        Path::new(&std::env::var("OUT_DIR").unwrap()).join("pack.rs"),
        list,
    )
    .expect("pack.rs");
}
