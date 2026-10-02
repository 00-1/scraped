//! Writes frames of the backdrop over each theme's background as raw RGB,
//! for looking at: `cargo run -p scraped-android --example backdrop OUT`.
fn main() {
    let out = std::env::args().nth(1).expect("output prefix");
    let (w, h) = (36usize, 78usize);
    for (dark, bg) in [(false, [0xf7u8, 0xf5, 0xef]), (true, [0x12, 0x11, 0x10])] {
        for t in [0.0f32, 30.0] {
            let mut px = vec![0u32; w * h];
            scraped_android::atmosphere::paint(&mut px, w, h, t, dark);
            let mut rgb = Vec::new();
            for p in px {
                let a = (p >> 24) as f32 / 255.0;
                for (i, sh) in [16, 8, 0].iter().enumerate() {
                    let c = ((p >> sh) & 255) as f32;
                    rgb.push((bg[i] as f32 * (1.0 - a) + c * a) as u8);
                }
            }
            std::fs::write(
                format!("{out}-{}-{t}.rgb", if dark { "dark" } else { "light" }),
                rgb,
            )
            .unwrap();
        }
    }
}
