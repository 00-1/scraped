//! How a sign looks at a glance (S01, S02): the impression a person would
//! see and remember (a tall, curved sign like a lamp, with a dot above),
//! never the stroke-by-stroke build that tracing gives.
//!
//! Impressions are worked out from the sign as drawn (the strokes laid on
//! a grid, as the debug drawing places them), so they are about the whole
//! shape: its proportions, whether it is curved or angular, spare or busy,
//! symmetric, how many spaces it encloses and pieces it falls into, which
//! way its weight leans, the parts of it that stand out and where, and
//! what everyday thing it resembles. They are stable: the same sign always
//! gives the same impression, so a player can match signs across texts.
//! Within one script each sign is told with just enough of these to tell
//! it from every other (on archaeologist, a few pairs are left alike on
//! purpose). A sign that is another sign plus a part or two (the script
//! logic of M02, a syllabary's vowel marks) is told as that sign with the
//! part added, so related signs read as related.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::script::{Direction, Glyph, Mark, Script, Spot, Stroke, Turn};

/// The parts a sign's shape can show, as the eye names them.
pub const PARTS: &[&str] = &[
    "line", "curl", "dot", "loop", "arch", "point", "sweep", "crossing", "saw edge",
];
/// Which way a part faces ("" when not told, or it looks the same any way
/// round): forward is the way the writing runs; lines stand or lie.
pub const FACINGS: &[&str] = &["", "forward", "back", "high", "low", "standing", "lying"];
/// Where a part sits, coarsely ("none": it is the body of the sign).
pub const PLACES: &[&str] = &["above", "below", "beside", "inside", "none"];
/// The sign's proportions.
pub const PROPORTIONS: &[&str] = &["tall", "wide", "squarish", "slight"];
/// Curved, angular or both.
pub const CURVES: &[&str] = &["curved", "angular", "mixed"];
/// How much is cut ("" when not told).
pub const BUSY: &[&str] = &["", "spare", "middling", "busy"];
/// Which way it is symmetric ("" when not told).
pub const SYMMETRY: &[&str] = &["", "both ways", "side to side", "high and low", "none"];
/// Which way its weight leans along the line of writing ("" when not
/// told): forward is the way the writing runs.
pub const LEANS: &[&str] = &["", "forward", "back", "evenly"];
/// Whether its weight sits high or low ("" when not told).
pub const WEIGHT: &[&str] = &["", "top-heavy", "bottom-heavy", "even"];
/// Where it sits on the line of writing ("" when not told).
pub const SITS: &[&str] = &["", "high", "low", "middle"];
/// Which way its lines mostly run ("" when not told).
pub const RUNS: &[&str] = &["", "across", "upright", "every way"];

/// Everyday things a sign can look like ("" if nothing fits).
pub const RESEMBLANCES: &[&str] = &[
    "",
    "eye",
    "wheel",
    "shield",
    "knot",
    "lamp",
    "egg",
    "key",
    "spindle",
    "seed",
    "bunch of grapes",
    "bird's foot",
    "arrow",
    "comb",
    "feather",
    "ladder",
    "fork",
    "star",
    "tree",
    "doorway",
    "bridge",
    "moon",
    "crook",
    "fish-hook",
    "horn",
    "flame",
    "house",
    "axe",
    "mountain",
    "wave",
    "snake",
    "hand",
];

/// A part of a sign, and where it sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Part {
    pub part: &'static str,
    pub place: &'static str,
    pub facing: &'static str,
}

impl Part {
    /// The part with its facing left untold.
    fn plain(self) -> Part {
        Part { facing: "", ..self }
    }
}

/// Everything the eye can take in about one sign's shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Shape {
    pub proportion: &'static str,
    pub curve: &'static str,
    pub busy: &'static str,
    pub symmetry: &'static str,
    /// Enclosed spaces.
    pub holes: usize,
    /// Separate pieces.
    pub pieces: usize,
    pub leans: &'static str,
    pub weight: &'static str,
    pub sits: &'static str,
    pub runs: &'static str,
    /// Its parts, the one that stands out most first.
    pub parts: Vec<Part>,
    pub resembles: &'static str,
}

/// What an impression tells: the features told, a related sign and what
/// is added to it. Features not told are "" or `None`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Told {
    pub proportion: &'static str,
    pub curve: &'static str,
    pub resembles: &'static str,
    /// Parts told, the one that stands out most first.
    pub parts: Vec<Part>,
    pub pieces: Option<usize>,
    pub holes: Option<usize>,
    pub symmetry: &'static str,
    pub busy: &'static str,
    pub leans: &'static str,
    pub weight: &'static str,
    pub sits: &'static str,
    pub runs: &'static str,
    /// A related sign it is told by (its index in the script's table),
    /// and the parts this one adds to it.
    pub like: Option<usize>,
    pub added: Vec<Part>,
}

/// The impression of one sign: what is told, and the whole shape (for a
/// sign examined closely).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Impression {
    pub told: Told,
    pub shape: Shape,
}

impl Impression {
    /// What it tells, for telling signs apart.
    pub fn told(&self) -> &Told {
        &self.told
    }
}

// ---------- geometry ----------

/// Grid cells per side of the sign's square.
const N: usize = 48;

type Point = (f64, f64);

/// A stroke as drawn: straight segments (curves flattened) and filled
/// discs, in the 100-unit square, with how much of its length is curved.
struct Drawn {
    segments: Vec<(Point, Point)>,
    discs: Vec<(Point, f64)>,
    half_width: f64,
    straight: f64,
    curved: f64,
    /// Length running across and upright.
    across: f64,
    upright: f64,
}

fn quad(a: Point, c: Point, b: Point) -> Vec<Point> {
    (0..=12)
        .map(|i| {
            let t = i as f64 / 12.0;
            let u = 1.0 - t;
            (
                u * u * a.0 + 2.0 * u * t * c.0 + t * t * b.0,
                u * u * a.1 + 2.0 * u * t * c.1 + t * t * b.1,
            )
        })
        .collect()
}

/// A circle as a polygon, without trigonometry: the rational
/// parametrisation of each half.
fn circle(c: Point, r: f64) -> Vec<Point> {
    let right: Vec<Point> = (0..=16)
        .map(|i| {
            let t = -1.0 + i as f64 / 8.0;
            let d = 1.0 + t * t;
            (c.0 + r * (1.0 - t * t) / d, c.1 + r * 2.0 * t / d)
        })
        .collect();
    let left: Vec<Point> = right
        .iter()
        .rev()
        .map(|&(x, y)| (2.0 * c.0 - x, y))
        .collect();
    right.into_iter().chain(left).collect()
}

fn length(p: &[Point]) -> f64 {
    p.windows(2)
        .map(|w| {
            let (dx, dy) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
            (dx * dx + dy * dy).sqrt()
        })
        .sum()
}

/// A mark as the debug drawing (`Glyph::svg`) places it.
fn drawn(m: &Mark) -> Drawn {
    let (x, y, w) = match m.spot {
        Spot::Centre => (30.0, 30.0, 40.0),
        Spot::Top => (35.0, 4.0, 30.0),
        Spot::Bottom => (35.0, 66.0, 30.0),
        Spot::Left => (4.0, 30.0, 30.0),
        Spot::Right => (66.0, 30.0, 30.0),
    };
    let s = w / 100.0;
    let place = |(px, py): Point| {
        let (dx, dy) = (px - 50.0, py - 50.0);
        let (rx, ry) = match m.turn {
            Turn::Up => (dx, dy),
            Turn::Right => (-dy, dx),
            Turn::Down => (-dx, -dy),
            Turn::Left => (dy, -dx),
        };
        (x + s * (50.0 + rx), y + s * (50.0 + ry))
    };
    let mut straight: Vec<Vec<Point>> = Vec::new();
    let mut curved: Vec<Vec<Point>> = Vec::new();
    let mut discs = Vec::new();
    match m.stroke {
        Stroke::Bar => straight.push(vec![(50.0, 5.0), (50.0, 95.0)]),
        Stroke::Hook => {
            straight.push(vec![(50.0, 95.0), (50.0, 30.0)]);
            curved.push(quad((50.0, 30.0), (50.0, 5.0), (80.0, 12.0)));
        }
        Stroke::Dot => discs.push((place((50.0, 50.0)), 13.0 * s)),
        Stroke::Ring => curved.push(circle((50.0, 50.0), 34.0)),
        Stroke::Arc => curved.push(quad((10.0, 80.0), (50.0, -10.0), (90.0, 80.0))),
        Stroke::Wedge => straight.push(vec![(15.0, 10.0), (85.0, 50.0), (15.0, 90.0)]),
        Stroke::Tail => {
            straight.push(vec![(50.0, 5.0), (50.0, 60.0)]);
            curved.push(quad((50.0, 60.0), (50.0, 95.0), (15.0, 88.0)));
        }
        Stroke::Cross => {
            straight.push(vec![(50.0, 8.0), (50.0, 92.0)]);
            straight.push(vec![(8.0, 50.0), (92.0, 50.0)]);
        }
        Stroke::Zigzag => {
            straight.push(vec![(15.0, 15.0), (85.0, 38.0), (15.0, 62.0), (85.0, 85.0)])
        }
    }
    let mut segments = Vec::new();
    let (mut across, mut upright) = (0.0, 0.0);
    for line in straight.iter().chain(curved.iter()) {
        let placed: Vec<Point> = line.iter().map(|&p| place(p)).collect();
        for w in placed.windows(2) {
            segments.push((w[0], w[1]));
            across += (w[1].0 - w[0].0).abs();
            upright += (w[1].1 - w[0].1).abs();
        }
    }
    Drawn {
        segments,
        discs,
        half_width: (4.5 * s).max(1.6),
        straight: straight.iter().map(|l| length(l) * s).sum(),
        curved: curved.iter().map(|l| length(l) * s).sum(),
        across,
        upright,
    }
}

fn distance_to_segment(p: Point, a: Point, b: Point) -> f64 {
    let (vx, vy) = (b.0 - a.0, b.1 - a.1);
    let (wx, wy) = (p.0 - a.0, p.1 - a.1);
    let len2 = vx * vx + vy * vy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        ((wx * vx + wy * vy) / len2).clamp(0.0, 1.0)
    };
    let (dx, dy) = (p.0 - (a.0 + t * vx), p.1 - (a.1 + t * vy));
    (dx * dx + dy * dy).sqrt()
}

/// The sign laid on an N×N grid: true where there is ink.
fn raster(marks: &[Drawn]) -> Vec<bool> {
    let cell = 100.0 / N as f64;
    let mut ink = vec![false; N * N];
    for j in 0..N {
        for i in 0..N {
            let p = ((i as f64 + 0.5) * cell, (j as f64 + 0.5) * cell);
            ink[j * N + i] = marks.iter().any(|d| {
                d.segments
                    .iter()
                    .any(|&(a, b)| distance_to_segment(p, a, b) <= d.half_width)
                    || d.discs.iter().any(|&(c, r)| {
                        let (dx, dy) = (p.0 - c.0, p.1 - c.1);
                        (dx * dx + dy * dy).sqrt() <= r + 0.5
                    })
            });
        }
    }
    ink
}

/// Connected regions of cells where `inside` holds (8-connected for ink,
/// 4-connected for the spaces between), each with whether it touches the
/// grid's edge and its size.
fn regions(inside: &dyn Fn(usize) -> bool, eight: bool) -> Vec<(bool, usize)> {
    const STEPS: [(i64, i64); 8] = [
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -1),
        (1, 1),
        (1, -1),
        (-1, 1),
        (-1, -1),
    ];
    let n = N as i64;
    let mut seen = vec![false; N * N];
    let mut out = Vec::new();
    for start in 0..N * N {
        if seen[start] || !inside(start) {
            continue;
        }
        seen[start] = true;
        let mut stack = vec![start];
        let (mut edge, mut size) = (false, 0);
        while let Some(c) = stack.pop() {
            size += 1;
            let (i, j) = ((c % N) as i64, (c / N) as i64);
            if i == 0 || j == 0 || i == n - 1 || j == n - 1 {
                edge = true;
            }
            for (di, dj) in STEPS.iter().take(if eight { 8 } else { 4 }) {
                let (ni, nj) = (i + di, j + dj);
                if ni < 0 || nj < 0 || ni >= n || nj >= n {
                    continue;
                }
                let next = nj as usize * N + ni as usize;
                if !seen[next] && inside(next) {
                    seen[next] = true;
                    stack.push(next);
                }
            }
        }
        out.push((edge, size));
    }
    out
}

/// The ink's bounds: first and last column, first and last row.
type Bounds = (usize, usize, usize, usize);

/// Whether the ink matches its mirror image (across the upright axis, or
/// across the level one when `level`), allowing a cell's slack.
fn mirrored(ink: &[bool], b: Bounds, level: bool) -> bool {
    let (x0, x1, y0, y1) = b;
    let n = N as i64;
    let at = |i: i64, j: i64| i >= 0 && j >= 0 && i < n && j < n && ink[(j * n + i) as usize];
    let (mut total, mut matched) = (0, 0);
    for j in y0..=y1 {
        for i in x0..=x1 {
            if !ink[j * N + i] {
                continue;
            }
            total += 1;
            let (mi, mj) = if level {
                (i as i64, (y0 + y1) as i64 - j as i64)
            } else {
                ((x0 + x1) as i64 - i as i64, j as i64)
            };
            let hit = (-1..=1).any(|d| {
                if level {
                    at(mi, mj + d)
                } else {
                    at(mi + d, mj)
                }
            });
            if hit {
                matched += 1;
            }
        }
    }
    matched * 100 >= total * 92
}

fn part_word(s: Stroke) -> &'static str {
    match s {
        Stroke::Bar => "line",
        Stroke::Hook => "curl",
        Stroke::Dot => "dot",
        Stroke::Ring => "loop",
        Stroke::Arc => "arch",
        Stroke::Wedge => "point",
        Stroke::Tail => "sweep",
        Stroke::Cross => "crossing",
        Stroke::Zigzag => "saw edge",
    }
}

/// Which way a mark faces, as a part of the shape.
fn facing(m: &Mark, forward_right: bool) -> &'static str {
    // Where its telling end points when it stands as drawn (0 high, 1 the
    // right, 2 low, 3 the left); lines and saw edges only stand or lie.
    let base = match m.stroke {
        Stroke::Dot | Stroke::Ring | Stroke::Cross => return "",
        Stroke::Bar => {
            return if m.turn == Turn::Up {
                "standing"
            } else {
                "lying"
            }
        }
        Stroke::Zigzag => {
            return if m.turn == Turn::Up {
                "lying"
            } else {
                "standing"
            }
        }
        Stroke::Hook | Stroke::Arc => 0,
        Stroke::Wedge => 1,
        Stroke::Tail => 2,
    };
    let turned = match m.turn {
        Turn::Up => 0,
        Turn::Right => 1,
        Turn::Down => 2,
        Turn::Left => 3,
    };
    match (base + turned) % 4 {
        0 => "high",
        2 => "low",
        1 if forward_right => "forward",
        3 if !forward_right => "forward",
        _ => "back",
    }
}

/// A mark as a part of the shape, with a coarse place.
fn part_of_shape(m: &Mark, g: &Glyph, forward_right: bool) -> Part {
    let place = match m.spot {
        _ if g.marks.len() == 1 => "none",
        Spot::Top => "above",
        Spot::Bottom => "below",
        Spot::Left | Spot::Right => "beside",
        Spot::Centre => {
            let in_loop = g
                .marks
                .iter()
                .any(|o| o != m && o.stroke == Stroke::Ring && o.spot == Spot::Centre);
            if in_loop {
                "inside"
            } else {
                "none"
            }
        }
    };
    Part {
        part: part_word(m.stroke),
        place,
        facing: facing(m, forward_right),
    }
}

/// The whole shape of a sign. `forward_right`: the writing runs to the
/// right (so weight to the right leans forward).
fn shape(g: &Glyph, forward_right: bool) -> Shape {
    let drawn: Vec<Drawn> = g.marks.iter().map(drawn).collect();
    let ink = raster(&drawn);
    let cells: Vec<(usize, usize)> = (0..N * N)
        .filter(|&c| ink[c])
        .map(|c| (c % N, c / N))
        .collect();
    let x0 = cells.iter().map(|c| c.0).min().unwrap_or(0);
    let x1 = cells.iter().map(|c| c.0).max().unwrap_or(0);
    let y0 = cells.iter().map(|c| c.1).min().unwrap_or(0);
    let y1 = cells.iter().map(|c| c.1).max().unwrap_or(0);
    let (w, h) = ((x1 - x0 + 1) as f64, (y1 - y0 + 1) as f64);
    let proportion = if w.max(h) <= N as f64 * 0.3 {
        "slight"
    } else if h >= w * 1.4 {
        "tall"
    } else if w >= h * 1.4 {
        "wide"
    } else {
        "squarish"
    };
    let straight: f64 = drawn.iter().map(|d| d.straight).sum();
    let curved: f64 = drawn.iter().map(|d| d.curved).sum();
    let line = straight + curved;
    let curve = if line == 0.0 || curved >= line * 0.6 {
        "curved"
    } else if curved <= line * 0.2 {
        "angular"
    } else {
        "mixed"
    };
    // DESIGN-Q: how much ink reads as spare or busy (stroke length drawn
    // in the 100-unit square, each piece counting extra: about the
    // lightest and heaviest third of signs).
    let ink_amount = line + 15.0 * g.marks.len() as f64;
    let busy = if ink_amount <= 90.0 {
        "spare"
    } else if ink_amount >= 160.0 {
        "busy"
    } else {
        "middling"
    };
    let b = (x0, x1, y0, y1);
    let (sides, ends) = (mirrored(&ink, b, false), mirrored(&ink, b, true));
    let symmetry = match (sides, ends) {
        (true, true) => "both ways",
        (true, false) => "side to side",
        (false, true) => "high and low",
        (false, false) => "none",
    };
    let holes = regions(&|c| !ink[c], false)
        .iter()
        .filter(|&&(edge, size)| !edge && size >= 3)
        .count();
    let pieces = regions(&|c| ink[c], true).len();
    // Where the weight lies, against the middle of the sign.
    let n = cells.len().max(1) as f64;
    let cx = cells.iter().map(|c| c.0 as f64).sum::<f64>() / n - (x0 + x1) as f64 / 2.0;
    let cy = cells.iter().map(|c| c.1 as f64).sum::<f64>() / n - (y0 + y1) as f64 / 2.0;
    // A shape even from side to side may still sit towards one side of
    // its space: that leans it.
    let half = N as f64 / 2.0;
    let mx = cells.iter().map(|c| c.0 as f64 + 0.5).sum::<f64>() / n - half;
    let my = cells.iter().map(|c| c.1 as f64 + 0.5).sum::<f64>() / n - half;
    let own = !sides && cx.abs() >= w * 0.02;
    let cx = if own { cx } else { mx };
    let leans = if !own && mx.abs() < N as f64 * 0.1 {
        "evenly"
    } else if (cx > 0.0) == forward_right {
        "forward"
    } else {
        "back"
    };
    let weight = if ends || cy.abs() < h * 0.02 {
        "even"
    } else if cy < 0.0 {
        "top-heavy"
    } else {
        "bottom-heavy"
    };
    let sits = if my < -(N as f64) * 0.1 {
        "high"
    } else if my > N as f64 * 0.1 {
        "low"
    } else {
        "middle"
    };
    let across: f64 = drawn.iter().map(|d| d.across).sum();
    let upright: f64 = drawn.iter().map(|d| d.upright).sum();
    let runs = if across > upright * 1.3 {
        "across"
    } else if upright > across * 1.3 {
        "upright"
    } else {
        "every way"
    };
    let parts: Vec<Part> = g
        .marks
        .iter()
        .map(|m| part_of_shape(m, g, forward_right))
        .collect();
    let mut s = Shape {
        proportion,
        curve,
        busy,
        symmetry,
        holes,
        pieces,
        leans,
        weight,
        sits,
        runs,
        parts,
        resembles: "",
    };
    s.resembles = resembles(&s);
    s
}

/// What a shape looks like, by rules on its features: the first that
/// fits.
// DESIGN-Q: the everyday things and the rules that pick them.
fn resembles(s: &Shape) -> &'static str {
    let has = |p: &str| s.parts.iter().any(|q| q.part == p);
    let has_at = |p: &str, place: &str| s.parts.iter().any(|q| q.part == p && q.place == place);
    let tall = s.proportion == "tall";
    let wide = s.proportion == "wide";
    let rules: [(&str, bool); 31] = [
        ("eye", s.holes >= 1 && has_at("dot", "inside")),
        ("wheel", s.holes >= 3 && s.pieces == 1),
        ("shield", s.holes == 2 && s.pieces == 1),
        ("knot", s.holes >= 2 && s.busy == "busy"),
        (
            "lamp",
            s.holes == 1 && tall && (has("point") || has("curl")),
        ),
        ("egg", s.holes == 1 && s.pieces == 1 && s.busy == "spare"),
        ("key", s.holes == 1 && s.pieces == 1 && s.curve != "curved"),
        ("spindle", s.holes == 1 && has("line")),
        ("seed", s.pieces >= 2 && has("dot") && s.curve == "curved"),
        ("bunch of grapes", s.pieces >= 3 && has("dot")),
        ("bird's foot", has("point") && has("line") && tall),
        ("arrow", has("point") && has("line")),
        ("comb", has("saw edge") && has("line")),
        ("feather", has("saw edge") && tall && s.curve == "mixed"),
        (
            "ladder",
            s.busy == "busy" && s.curve == "angular" && s.symmetry == "both ways",
        ),
        ("fork", has("saw edge") && tall),
        (
            "star",
            has("crossing") && s.symmetry == "both ways" && s.pieces == 1,
        ),
        ("tree", has("crossing") && tall),
        ("doorway", has("arch") && tall && s.holes == 0),
        ("bridge", has("arch") && wide && s.curve != "curved"),
        ("moon", has("arch") && s.busy == "spare"),
        ("crook", has("curl") && tall),
        ("fish-hook", has("sweep") && tall && s.holes == 0),
        ("horn", has("sweep") && s.curve == "curved"),
        ("flame", has("point") && s.curve != "angular" && tall),
        (
            "house",
            has("point") && s.proportion == "squarish" && s.symmetry == "side to side",
        ),
        ("axe", has("point") && wide && s.curve == "mixed"),
        ("mountain", has("point") && wide && s.curve == "angular"),
        (
            "wave",
            s.curve == "curved" && wide && s.holes == 0 && s.symmetry == "none",
        ),
        (
            "snake",
            s.curve == "curved" && tall && s.pieces == 1 && s.holes == 0,
        ),
        (
            "hand",
            s.busy == "busy" && s.pieces == 1 && s.symmetry == "none",
        ),
    ];
    rules
        .iter()
        .find(|(_, fits)| *fits)
        .map_or("", |(name, _)| name)
}

// ---------- telling signs apart ----------

/// Whether `q` is `g` less a mark or two (and not smaller than what is
/// added to it).
fn part_of(q: &Glyph, g: &Glyph) -> Option<Vec<Mark>> {
    if q.marks.len() >= g.marks.len() || !q.marks.iter().all(|m| g.marks.contains(m)) {
        return None;
    }
    let added: Vec<Mark> = g
        .marks
        .iter()
        .filter(|m| !q.marks.contains(m))
        .copied()
        .collect();
    (added.len() <= 2 && q.marks.len() >= added.len()).then_some(added)
}

/// Features told beyond proportions, curves and any resemblance, most
/// noticeable first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Feature {
    /// The next part that stands out.
    Part,
    Pieces,
    Holes,
    Symmetry,
    Busy,
    Leans,
    Weight,
    Sits,
    Runs,
    /// Which way each part told faces.
    Facing,
}

const FEATURES: [Feature; 10] = [
    Feature::Part,
    Feature::Pieces,
    Feature::Holes,
    Feature::Symmetry,
    Feature::Busy,
    Feature::Leans,
    Feature::Weight,
    Feature::Sits,
    Feature::Runs,
    Feature::Facing,
];

/// What is told of one sign: how many parts, and which other features.
#[derive(Debug, Clone, Default)]
struct Telling {
    parts: usize,
    features: Vec<Feature>,
}

impl Telling {
    fn has(&self, f: Feature) -> bool {
        self.features.contains(&f)
    }

    /// Whether `f` can still be added.
    fn open(&self, f: Feature, s: &Shape) -> bool {
        match f {
            Feature::Part => self.parts < s.parts.len(),
            // Facing only means something once a part is told.
            Feature::Facing => self.parts > 0 && !self.has(f),
            f => !self.has(f),
        }
    }

    /// Whether adding `f` would tell `a` from `b`.
    fn splits(&self, f: Feature, a: &Shape, b: &Shape) -> bool {
        let k = self.parts;
        match f {
            Feature::Part => a.parts.get(k).map(|p| p.plain()) != b.parts.get(k).map(|p| p.plain()),
            Feature::Pieces => a.pieces != b.pieces,
            Feature::Holes => a.holes != b.holes,
            Feature::Symmetry => a.symmetry != b.symmetry,
            Feature::Busy => a.busy != b.busy,
            Feature::Leans => a.leans != b.leans,
            Feature::Weight => a.weight != b.weight,
            Feature::Sits => a.sits != b.sits,
            Feature::Runs => a.runs != b.runs,
            Feature::Facing => a.parts.get(..k) != b.parts.get(..k),
        }
    }

    fn add(&mut self, f: Feature) {
        if f == Feature::Part {
            self.parts += 1;
        } else {
            self.features.push(f);
        }
    }

    /// The shape as told.
    fn tell(&self, s: &Shape) -> Told {
        let facing = self.has(Feature::Facing);
        let pick = |f: Feature, v: &'static str| if self.has(f) { v } else { "" };
        Told {
            proportion: s.proportion,
            curve: s.curve,
            resembles: s.resembles,
            parts: s.parts[..self.parts.min(s.parts.len())]
                .iter()
                .map(|p| if facing { *p } else { p.plain() })
                .collect(),
            pieces: self.has(Feature::Pieces).then_some(s.pieces),
            holes: self.has(Feature::Holes).then_some(s.holes),
            symmetry: pick(Feature::Symmetry, s.symmetry),
            busy: pick(Feature::Busy, s.busy),
            leans: pick(Feature::Leans, s.leans),
            weight: pick(Feature::Weight, s.weight),
            sits: pick(Feature::Sits, s.sits),
            runs: pick(Feature::Runs, s.runs),
            like: None,
            added: Vec::new(),
        }
    }
}

/// The impression of every sign of a script, in table order.
/// `confusable`: leave a few alike signs untold apart (archaeologist).
pub fn impressions(script: &Script, confusable: bool) -> Vec<Impression> {
    let glyphs: Vec<&Glyph> = script.glyphs.iter().map(|(_, g)| g).collect();
    let forward_right = script.direction != Direction::RightToLeft;
    let mut shapes: Vec<Shape> = glyphs.iter().map(|g| shape(g, forward_right)).collect();
    // The part that stands out is the one rarest in the script.
    let mut common: BTreeMap<Part, usize> = BTreeMap::new();
    for s in &shapes {
        for p in &s.parts {
            *common.entry(p.plain()).or_default() += 1;
        }
    }
    for s in &mut shapes {
        s.parts.sort_by_key(|p| (common[&p.plain()], *p));
    }
    // A sign with no sign inside it is told by its own look; the others
    // by the largest such sign inside them, with the parts added.
    let root: Vec<bool> = glyphs
        .iter()
        .enumerate()
        .map(|(i, g)| {
            !glyphs
                .iter()
                .enumerate()
                .any(|(j, q)| i != j && part_of(q, g).is_some())
        })
        .collect();
    let mut like: Vec<Option<(usize, Vec<Part>)>> = vec![None; glyphs.len()];
    for (i, g) in glyphs.iter().enumerate() {
        if root[i] {
            continue;
        }
        like[i] = glyphs
            .iter()
            .enumerate()
            .filter(|&(j, _)| root[j])
            .filter_map(|(j, q)| part_of(q, g).map(|a| (j, a)))
            .max_by_key(|(j, _)| (glyphs[*j].marks.len(), std::cmp::Reverse(*j)))
            .map(|(j, added)| {
                let mut parts: Vec<Part> = added
                    .iter()
                    .map(|m| part_of_shape(m, g, forward_right).plain())
                    .collect();
                parts.sort();
                (j, parts)
            });
    }
    // Every sign starts at its proportions, curves and resemblance. A sign
    // that reads like another gains the most noticeable feature that tells
    // them apart, and so on until none read alike.
    // DESIGN-Q: on archaeologist, a sign gains at most one feature, so a
    // few pairs stay alike.
    let cap = if confusable { 1 } else { usize::MAX };
    let mut telling: Vec<Telling> = vec![Telling::default(); glyphs.len()];
    loop {
        let told: Vec<Told> = (0..glyphs.len())
            .map(|i| match &like[i] {
                Some((j, added)) => Told {
                    like: Some(*j),
                    added: added.clone(),
                    ..telling[*j].tell(&shapes[*j])
                },
                None => telling[i].tell(&shapes[i]),
            })
            .collect();
        let mut changed = false;
        for i in 0..glyphs.len() {
            let clashes: Vec<usize> = (0..glyphs.len())
                .filter(|&j| j != i && told[j] == told[i])
                .collect();
            if clashes.is_empty() {
                continue;
            }
            if like[i].is_some() {
                // Told by its relation it reads like another sign: tell it
                // by its own look instead.
                like[i] = None;
                changed = true;
                continue;
            }
            let t = &telling[i];
            if t.parts + t.features.len() >= cap {
                continue;
            }
            let open: Vec<Feature> = FEATURES
                .into_iter()
                .filter(|&f| t.open(f, &shapes[i]))
                .collect();
            let next = open
                .iter()
                .find(|&&f| clashes.iter().any(|&j| t.splits(f, &shapes[i], &shapes[j])))
                .or(open.first());
            if let Some(&f) = next {
                telling[i].add(f);
                changed = true;
            }
        }
        if !changed {
            return told
                .into_iter()
                .zip(shapes)
                .map(|(told, shape)| Impression { told, shape })
                .collect();
        }
    }
}

/// Pairs of signs a script leaves alike (the same impression).
pub fn alike(imps: &[Impression]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for i in 0..imps.len() {
        for j in i + 1..imps.len() {
            if imps[i].told() == imps[j].told() {
                out.push((i, j));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::numerals::Numerals;
    use crate::phonology::Phonology;
    use crate::rng::{Rng, Stream};
    use crate::script::ScriptKind;

    fn script(seed: u64, kind: ScriptKind) -> Script {
        let p = Phonology::generate(seed);
        let n = Numerals::generate(seed);
        let mut rng = Rng::new(seed, Stream::Script);
        Script::generate(&mut rng, &p, &n, Some(kind))
    }

    #[test]
    fn every_sign_is_told_apart() {
        for seed in 0..40 {
            for kind in [
                ScriptKind::Alphabet,
                ScriptKind::Abjad,
                ScriptKind::Syllabary,
            ] {
                let s = script(seed, kind);
                let imps = impressions(&s, false);
                let same = alike(&imps);
                let shown: Vec<_> = same
                    .iter()
                    .map(|&(a, b)| (&s.glyphs[a].1, &s.glyphs[b].1))
                    .collect();
                assert!(same.is_empty(), "seed {seed} {kind:?}: {shown:?}");
            }
        }
    }

    #[test]
    fn related_signs_are_told_by_their_relation() {
        let mut related = 0;
        for seed in 0..20 {
            let s = script(seed, ScriptKind::Syllabary);
            for imp in impressions(&s, false) {
                if let Some(j) = imp.told.like {
                    related += 1;
                    assert!(!imp.told.added.is_empty());
                    assert!(j < s.glyphs.len());
                }
            }
        }
        assert!(related > 100, "{related}");
    }

    #[test]
    fn archaeologist_leaves_a_few_alike() {
        let mut alike_total = 0;
        for seed in 0..20 {
            let s = script(seed, ScriptKind::Alphabet);
            alike_total += alike(&impressions(&s, true)).len();
        }
        assert!(alike_total > 0 && alike_total <= 3 * 20, "{alike_total}");
    }

    #[test]
    fn impressions_are_stable() {
        let s = script(7, ScriptKind::Abjad);
        assert_eq!(impressions(&s, false), impressions(&s, false));
    }

    #[test]
    fn shapes_are_read_from_the_drawing() {
        let one = |marks: Vec<Mark>| shape(&Glyph { marks }, true);
        let ring = one(vec![Mark::new(Stroke::Ring, Turn::Up, Spot::Centre)]);
        assert_eq!(
            (ring.holes, ring.pieces, ring.symmetry, ring.curve),
            (1, 1, "both ways", "curved")
        );
        let eye = one(vec![
            Mark::new(Stroke::Dot, Turn::Up, Spot::Centre),
            Mark::new(Stroke::Ring, Turn::Up, Spot::Centre),
        ]);
        assert_eq!((eye.holes, eye.pieces, eye.resembles), (1, 2, "eye"));
        let bar = one(vec![Mark::new(Stroke::Bar, Turn::Up, Spot::Centre)]);
        assert_eq!(
            (bar.proportion, bar.curve, bar.holes),
            ("tall", "angular", 0)
        );
        let wheel = one(vec![
            Mark::new(Stroke::Cross, Turn::Up, Spot::Centre),
            Mark::new(Stroke::Ring, Turn::Up, Spot::Centre),
        ]);
        assert_eq!(wheel.holes, 4);
        // A sign turned about leans the other way.
        let hook = |t| one(vec![Mark::new(Stroke::Hook, t, Spot::Centre)]);
        assert_ne!(hook(Turn::Right).leans, hook(Turn::Left).leans);
    }
}
