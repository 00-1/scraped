//! English helpers for templates: articles, plurals, lists, numbers,
//! bearings, distances and durations. Mechanical rules only; Jb's words go
//! around them.

/// "a hook", "an arc".
pub fn article(word: &str) -> String {
    let first = word
        .trim_start()
        .chars()
        .next()
        .map(|c| c.to_ascii_lowercase());
    let an = matches!(first, Some('a' | 'e' | 'i' | 'o' | 'u'));
    format!("{} {word}", if an { "an" } else { "a" })
}

/// Capital first letter.
pub fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[derive(serde::Deserialize)]
struct Plurals {
    unchanging: Vec<String>,
    irregular: std::collections::BTreeMap<String, String>,
}

fn plurals() -> &'static Plurals {
    static P: std::sync::OnceLock<Plurals> = std::sync::OnceLock::new();
    P.get_or_init(|| {
        toml::from_str(include_str!("../data/plurals.toml")).expect("data/plurals.toml is valid")
    })
}

/// Whether a name is the same in the plural ("deer", "provisions").
pub fn unchanging(word: &str) -> bool {
    let last = word.rsplit([' ', '-', '_']).next().unwrap_or(word);
    plurals().unchanging.iter().any(|u| u == last)
}

/// English plural, applied unless `n` is 1 (S03: irregular and unchanging
/// names listed in data; a name of several words takes it on its last).
pub fn plural_if(word: &str, n: i64) -> String {
    if n == 1 {
        return word.to_string();
    }
    let split = word.rfind([' ', '-', '_']).map_or(0, |i| i + 1);
    let (head, w) = word.split_at(split);
    let p = plurals();
    if p.unchanging.iter().any(|u| u == w) {
        return word.to_string();
    }
    if let Some(irr) = p.irregular.get(w) {
        return format!("{head}{irr}");
    }
    format!("{head}{}", regular_plural(w))
}

fn regular_plural(w: &str) -> String {
    if w.ends_with('s') || w.ends_with('x') || w.ends_with("ch") || w.ends_with("sh") {
        format!("{w}es")
    } else if w.ends_with('y') && !w.ends_with("ay") && !w.ends_with("ey") && !w.ends_with("oy") {
        format!("{}ies", &w[..w.len() - 1])
    } else {
        format!("{w}s")
    }
}

/// "a", "a and b", "a, b and c".
pub fn list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

const SMALL: [&str; 20] = [
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
];
const TENS: [&str; 10] = [
    "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
];

/// Numbers in words up to 999; digits beyond.
pub fn number(n: i64) -> String {
    if !(0..1000).contains(&n) {
        return n.to_string();
    }
    let n = n as usize;
    if n < 20 {
        return SMALL[n].to_string();
    }
    if n < 100 {
        return match n % 10 {
            0 => TENS[n / 10].to_string(),
            u => format!("{}-{}", TENS[n / 10], SMALL[u]),
        };
    }
    match n % 100 {
        0 => format!("{} hundred", SMALL[n / 100]),
        r => format!("{} hundred and {}", SMALL[n / 100], number(r as i64)),
    }
}

/// Eight-point compass direction for a bearing in degrees (0 = north).
pub fn bearing(degrees: i64) -> String {
    const POINTS: [&str; 8] = [
        "north",
        "north-east",
        "east",
        "south-east",
        "south",
        "south-west",
        "west",
        "north-west",
    ];
    let d = degrees.rem_euclid(360);
    POINTS[(((d * 2 + 45) / 90) % 8) as usize].to_string()
}

/// Rough distance from metres, the way a traveller would say it.
pub fn distance(metres: i64) -> String {
    match metres {
        m if m < 15 => "a few paces".to_string(),
        m if m < 100 => format!("about {} metres", round_to(m, 10)),
        m if m < 1000 => format!("about {} metres", round_to(m, 50)),
        m if m < 1500 => "about a kilometre".to_string(),
        m => format!("about {} kilometres", number(round_to(m, 1000) / 1000)),
    }
}

/// Rough duration from minutes.
pub fn duration(minutes: i64) -> String {
    match minutes {
        m if m < 3 => "a moment".to_string(),
        m if m < 45 => format!("about {} minutes", number(round_to(m, 5).max(5))),
        m if m < 90 => "about an hour".to_string(),
        m if m < 60 * 36 => format!("about {} hours", number((m + 30) / 60)),
        m => format!("about {} days", number((m + 720) / 1440)),
    }
}

fn round_to(n: i64, step: i64) -> i64 {
    ((n + step / 2) / step) * step
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers() {
        assert_eq!(article("arc"), "an arc");
        assert_eq!(article("hook"), "a hook");
        assert_eq!(plural_if("box", 2), "boxes");
        assert_eq!(plural_if("city", 3), "cities");
        assert_eq!(plural_if("jar", 1), "jar");
        assert_eq!(list(&["a".into(), "b".into(), "c".into()]), "a, b and c");
        assert_eq!(number(342), "three hundred and forty-two");
        assert_eq!(number(40), "forty");
        assert_eq!(bearing(44), "north-east");
        assert_eq!(bearing(350), "north");
        assert_eq!(bearing(-90), "west");
        assert_eq!(distance(5), "a few paces");
        assert_eq!(distance(330), "about 350 metres");
        assert_eq!(distance(4200), "about four kilometres");
        assert_eq!(duration(70), "about an hour");
        assert_eq!(duration(200), "about three hours");
    }
}
