//! Seed codes (M14): short, shareable names for a world, so players can
//! compare notebooks. A code carries the seed, the difficulty preset and a
//! fingerprint of the content pack, with a check character against typos:
//! `K5G0-9ZQ1`.

use serde::Serialize;

use scraped_lang::difficulty::PRESETS;

/// Crockford's base 32: no I, L, O or U, so codes read aloud cleanly.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// What a seed code holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SeedCode {
    pub seed: u64,
    pub preset: String,
    /// The first four hex digits of the pack version the code was made
    /// with.
    pub pack: String,
}

fn checksum(bytes: &[u8]) -> u8 {
    bytes
        .iter()
        .fold(0x5au8, |c, &b| c.rotate_left(3) ^ b.wrapping_mul(31))
}

/// The code for a world.
pub fn encode(seed: u64, preset: &str, pack_version: &str) -> String {
    let p = PRESETS.iter().position(|x| *x == preset).unwrap_or(1) as u8;
    let mut seed_bytes = seed.to_le_bytes().to_vec();
    while seed_bytes.len() > 1 && seed_bytes.last() == Some(&0) {
        seed_bytes.pop();
    }
    let pack = u16::from_str_radix(pack_version.get(..4).unwrap_or("0000"), 16).unwrap_or(0);
    let mut bytes = vec![p << 4 | seed_bytes.len() as u8];
    bytes.extend(&seed_bytes);
    bytes.extend(pack.to_le_bytes());
    bytes.push(checksum(&bytes));
    // Bytes to base 32, five bits at a time.
    let mut out = String::new();
    let (mut acc, mut bits) = (0u32, 0);
    for b in bytes {
        acc = acc << 8 | u32::from(b);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(ALPHABET[((acc >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(ALPHABET[((acc << (5 - bits)) & 31) as usize] as char);
    }
    out.as_bytes()
        .chunks(4)
        .map(|c| std::str::from_utf8(c).expect("ascii"))
        .collect::<Vec<_>>()
        .join("-")
}

/// Reads a code; `None` if it is mistyped. Case, dashes and spaces don't
/// matter, and O, I and L read as 0, 1 and 1.
pub fn decode(code: &str) -> Option<SeedCode> {
    let mut acc = 0u32;
    let mut bits = 0;
    let mut bytes = Vec::new();
    for c in code.chars().filter(|c| !matches!(c, '-' | ' ')) {
        let c = match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            c => c,
        };
        let v = ALPHABET.iter().position(|&a| a as char == c)? as u32;
        acc = (acc << 5 | v) & 0xffff;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            bytes.push((acc >> bits) as u8);
        }
    }
    let (&sum, body) = bytes.split_last()?;
    if body.is_empty() || checksum(body) != sum {
        return None;
    }
    let p = (body[0] >> 4) as usize;
    let n = (body[0] & 15) as usize;
    if n == 0 || n > 8 || body.len() != 1 + n + 2 || p >= PRESETS.len() {
        return None;
    }
    let mut seed = [0u8; 8];
    seed[..n].copy_from_slice(&body[1..1 + n]);
    let pack = u16::from_le_bytes([body[1 + n], body[2 + n]]);
    Some(SeedCode {
        seed: u64::from_le_bytes(seed),
        preset: PRESETS[p].to_string(),
        pack: format!("{pack:04x}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        for (seed, preset) in [
            (0u64, "gentle"),
            (42, "standard"),
            (987_654_321, "archaeologist"),
            (u64::MAX, "standard"),
        ] {
            let code = encode(seed, preset, "beef1234deadbeef");
            let back = decode(&code).expect(&code);
            assert_eq!(back.seed, seed);
            assert_eq!(back.preset, preset);
            assert_eq!(back.pack, "beef");
            assert_eq!(decode(&code.to_lowercase().replace('-', " ")), Some(back));
        }
        assert!(encode(42, "standard", "0000").len() <= 9);
    }

    #[test]
    fn typos_are_caught() {
        let code = encode(42, "standard", "beef");
        for i in 0..code.len() {
            let mut b: Vec<char> = code.chars().collect();
            if b[i] == '-' {
                continue;
            }
            b[i] = if b[i] == 'Z' { 'Y' } else { 'Z' };
            let typo: String = b.into_iter().collect();
            assert!(
                decode(&typo).is_none_or(|c| c.seed != 42 || c.pack != "beef"),
                "{typo}"
            );
        }
        assert!(decode("hello").is_none());
        assert!(decode("").is_none());
    }
}
