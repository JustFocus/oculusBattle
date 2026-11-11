//! Procedural parchment map.
//!
//! Everything is derived from one seed: a noise height field (radial continent plus islands) is
//! contoured with marching squares into smooth coastlines, then mountains, forests, rivers, lakes,
//! roads, labels and decorations are scattered on top and emitted as one SVG document.

use std::collections::{BTreeMap, HashSet};
use std::fmt::Write;

/// Output size; 2x the 800x500 world so it stays sharp on high-DPI screens.
pub const WIDTH: u32 = 1600;
pub const HEIGHT: u32 = 1000;
// The game draws the map 1:1 onto its 800x500 world without cropping.
const _: () = assert!(
    WIDTH * 500 == HEIGHT * 800,
    "map must match the 16:10 world aspect"
);
/// Seed for the committed map. Change it to get a different continent.
pub const SEED: u64 = 3;

const W: f64 = WIDTH as f64;
const H: f64 = HEIGHT as f64;
const INK: &str = "#4a3320";
const LAND_FILL: &str = "#ecdcb2";
/// Height-field level that separates sea from land.
const SEA: f64 = 0.2;
/// Gameplay band (bases and units): no labels or dense glyphs here.
const BAND_Y: f64 = 500.0;
const BAND_HALF: f64 = 125.0;

type P = (f64, f64);

// ---------------------------------------------------------------------------------------------
// Random numbers and noise
// ---------------------------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix(self.0)
    }
    /// Uniform in [0, 1).
    fn f(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.f()
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    fn chance(&mut self, p: f64) -> bool {
        self.f() < p
    }
}

fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn lattice(seed: u64, x: i64, y: i64) -> f64 {
    let h = mix(seed
        ^ (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F));
    (h >> 11) as f64 / (1u64 << 53) as f64
}

fn value_noise(seed: u64, x: f64, y: f64) -> f64 {
    let (xi, yi) = (x.floor(), y.floor());
    let (fx, fy) = (x - xi, y - yi);
    let s = |t: f64| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
    let (u, v) = (s(fx), s(fy));
    let (xi, yi) = (xi as i64, yi as i64);
    let a = lattice(seed, xi, yi);
    let b = lattice(seed, xi + 1, yi);
    let c = lattice(seed, xi, yi + 1);
    let d = lattice(seed, xi + 1, yi + 1);
    (a + (b - a) * u) + ((c + (d - c) * u) - (a + (b - a) * u)) * v
}

/// Fractal noise, roughly in [0, 1] and centered on 0.5.
fn fbm(seed: u64, x: f64, y: f64, octaves: u32) -> f64 {
    let (mut sum, mut amp, mut total, mut freq) = (0.0, 1.0, 0.0, 1.0);
    for o in 0..octaves {
        sum += amp * value_noise(seed.wrapping_add(o as u64 * 7919), x * freq, y * freq);
        total += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / total
}

fn smoothstep(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// ---------------------------------------------------------------------------------------------
// Geometry helpers
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct R {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

impl R {
    fn new(x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        R { x0, y0, x1, y1 }
    }
    fn hit(&self, o: &R) -> bool {
        self.x0 < o.x1 && o.x0 < self.x1 && self.y0 < o.y1 && o.y0 < self.y1
    }
    fn pad(&self, p: f64) -> R {
        R::new(self.x0 - p, self.y0 - p, self.x1 + p, self.y1 + p)
    }
    fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x0 && x <= self.x1 && y >= self.y0 && y <= self.y1
    }
}

fn num(v: f64) -> String {
    let s = format!("{v:.1}");
    s.strip_suffix(".0").map(str::to_string).unwrap_or(s)
}

fn chaikin(pts: &[P], iters: u32, closed: bool) -> Vec<P> {
    let mut cur = pts.to_vec();
    for _ in 0..iters {
        let n = cur.len();
        if n < 3 {
            break;
        }
        let mut next = Vec::with_capacity(n * 2);
        let segs = if closed { n } else { n - 1 };
        if !closed {
            next.push(cur[0]);
        }
        for i in 0..segs {
            let a = cur[i];
            let b = cur[(i + 1) % n];
            next.push((0.75 * a.0 + 0.25 * b.0, 0.75 * a.1 + 0.25 * b.1));
            next.push((0.25 * a.0 + 0.75 * b.0, 0.25 * a.1 + 0.75 * b.1));
        }
        if !closed {
            next.push(cur[n - 1]);
        }
        cur = next;
    }
    cur
}

fn path_d(pts: &[P], closed: bool) -> String {
    let mut s = String::with_capacity(pts.len() * 12);
    for (i, p) in pts.iter().enumerate() {
        let _ = write!(
            s,
            "{}{},{}",
            if i == 0 { "M" } else { "L" },
            num(p.0),
            num(p.1)
        );
    }
    if closed {
        s.push('Z');
    }
    s
}

fn perimeter(pts: &[P]) -> f64 {
    (0..pts.len())
        .map(|i| {
            let a = pts[i];
            let b = pts[(i + 1) % pts.len()];
            ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
        })
        .sum()
}

// ---------------------------------------------------------------------------------------------
// Height field and contouring
// ---------------------------------------------------------------------------------------------

const CELL: f64 = 5.0;
const GW: usize = 321;
const GH: usize = 201;

/// Anisotropic gaussian bump: centre, radii along its own axes, rotation, amplitude.
struct Bump {
    x: f64,
    y: f64,
    rx: f64,
    ry: f64,
    rot: f64,
    amp: f64,
}

struct World {
    s_shape: u64,
    s_warp: u64,
    s_ridge: u64,
    s_wood: u64,
    s_hill: u64,
    s_coast: u64,
    /// Blobs that make up the continent, then island bumps.
    bumps: Vec<Bump>,
}

impl World {
    fn new(rng: &mut Rng, keep_out: &[R]) -> Self {
        let mut bumps: Vec<Bump> = Vec::new();
        // Continent: a chain of overlapping lobes along a wandering spine, so the outline gets
        // peninsulas and bays instead of an oval.
        let lobes = 9;
        let mut tries = 0;
        let mut k = 0;
        while k < lobes && tries < 2000 {
            tries += 1;
            let t = k as f64 / (lobes - 1) as f64;
            let x = 300.0 + t * 1000.0 + rng.range(-50.0, 50.0);
            let y = 500.0 + rng.range(-1.0, 1.0) * if k % 2 == 0 { 190.0 } else { 130.0 };
            let rx = rng.range(110.0, 190.0);
            let ry = rng.range(100.0, 180.0);
            let rot = rng.range(-0.6, 0.6);
            let ext = rx.max(ry) * 1.15;
            let probe = R::new(x - ext, y - ext, x + ext, y + ext);
            if x - ext < 95.0
                || x + ext > W - 95.0
                || y - ext < 105.0
                || y + ext > H - 105.0
                || keep_out.iter().any(|q| q.hit(&probe))
            {
                continue;
            }
            bumps.push(Bump {
                x,
                y,
                rx,
                ry,
                rot,
                amp: rng.range(0.7, 0.9),
            });
            k += 1;
        }
        // Two end lobes so both home bases sit well inside the land.
        for bx in [290.0, 1310.0] {
            bumps.push(Bump {
                x: bx,
                y: 500.0 + rng.range(-40.0, 40.0),
                rx: 175.0,
                ry: 150.0,
                rot: rng.range(-0.4, 0.4),
                amp: 0.8,
            });
        }
        // Islands: small clusters of irregular bumps in open sea.
        let mut centres: Vec<P> = Vec::new();
        let mut tries = 0;
        while centres.len() < 6 && tries < 800 {
            tries += 1;
            let x = rng.range(110.0, 1490.0);
            let y = rng.range(100.0, 900.0);
            let probe = R::new(x - 120.0, y - 120.0, x + 120.0, y + 120.0);
            let near_land = bumps.iter().any(|b| {
                ((b.x - x).powi(2) + (b.y - y).powi(2)).sqrt() < b.rx.max(b.ry) * 1.35 + 60.0
            });
            let spaced = centres
                .iter()
                .all(|c| ((c.0 - x).powi(2) + (c.1 - y).powi(2)).sqrt() > 240.0);
            if !near_land && spaced && keep_out.iter().all(|q| !q.hit(&probe)) {
                centres.push((x, y));
            }
        }
        for (x, y) in centres {
            let parts = 1 + rng.below(3);
            for _ in 0..parts {
                bumps.push(Bump {
                    x: x + rng.range(-30.0, 30.0),
                    y: y + rng.range(-24.0, 24.0),
                    rx: rng.range(22.0, 60.0),
                    ry: rng.range(14.0, 34.0),
                    rot: rng.range(0.0, std::f64::consts::PI),
                    amp: rng.range(0.6, 0.85),
                });
            }
        }
        World {
            s_shape: rng.next_u64(),
            s_warp: rng.next_u64(),
            s_ridge: rng.next_u64(),
            s_wood: rng.next_u64(),
            s_hill: rng.next_u64(),
            s_coast: rng.next_u64(),
            bumps,
        }
    }

    fn height(&self, x: f64, y: f64) -> f64 {
        let wx = x + 110.0 * (fbm(self.s_warp, x / 300.0, y / 300.0, 3) - 0.5);
        let wy = y + 110.0 * (fbm(self.s_warp ^ 0xABCD, x / 300.0, y / 300.0, 3) - 0.5);
        let mut h = 0.0;
        for b in &self.bumps {
            let (dx, dy) = (wx - b.x, wy - b.y);
            let (c, s) = (b.rot.cos(), b.rot.sin());
            let (u, v) = ((dx * c + dy * s) / b.rx, (-dx * s + dy * c) / b.ry);
            h += b.amp * (-(u * u + v * v)).exp();
        }
        // Broad bays plus fine coastline detail.
        let big = fbm(self.s_shape, wx / 320.0, wy / 320.0, 3);
        let fine = fbm(self.s_coast, wx / 70.0, wy / 70.0, 4);
        h += 0.9 * (big - 0.5) + 0.32 * (fine - 0.5);
        // Safety fade at the frame; the shape itself stays well clear of it.
        let m = x.min(W - x).min(y).min(H - y);
        h - 0.75 * smoothstep(0.0, 1.0, (165.0 - m) / 125.0)
            - 1.6 * smoothstep(0.0, 1.0, (70.0 - m) / 30.0)
    }
}

struct Field {
    h: Vec<f64>,
}

impl Field {
    fn build(w: &World) -> Self {
        let mut h = Vec::with_capacity(GW * GH);
        for j in 0..GH {
            for i in 0..GW {
                h.push(w.height(i as f64 * CELL, j as f64 * CELL));
            }
        }
        Field { h }
    }

    fn at(&self, x: f64, y: f64) -> f64 {
        let gx = (x / CELL).clamp(0.0, (GW - 1) as f64 - 0.001);
        let gy = (y / CELL).clamp(0.0, (GH - 1) as f64 - 0.001);
        let (i, j) = (gx as usize, gy as usize);
        let (fx, fy) = (gx - i as f64, gy - j as f64);
        let v = |i: usize, j: usize| self.h[j * GW + i];
        let top = v(i, j) * (1.0 - fx) + v(i + 1, j) * fx;
        let bot = v(i, j + 1) * (1.0 - fx) + v(i + 1, j + 1) * fx;
        top * (1.0 - fy) + bot * fy
    }

    /// Height above sea level; positive on land.
    fn e(&self, x: f64, y: f64) -> f64 {
        self.at(x, y) - SEA
    }

    /// Closed iso-lines of the field at `level` (marching squares, unsmoothed).
    fn contour(&self, level: f64) -> Vec<Vec<P>> {
        type Id = (u8, i32, i32);
        let v = |i: usize, j: usize| self.h[j * GW + i];
        let mut adj: BTreeMap<Id, Vec<Id>> = BTreeMap::new();
        let mut link = |a: Id, b: Id| {
            adj.entry(a).or_default().push(b);
            adj.entry(b).or_default().push(a);
        };
        for j in 0..GH - 1 {
            for i in 0..GW - 1 {
                let (a, b, c, d) = (v(i, j), v(i + 1, j), v(i + 1, j + 1), v(i, j + 1));
                let case = (a >= level) as u8
                    | ((b >= level) as u8) << 1
                    | ((c >= level) as u8) << 2
                    | ((d >= level) as u8) << 3;
                let (ii, jj) = (i as i32, j as i32);
                let t: Id = (0, ii, jj);
                let bt: Id = (0, ii, jj + 1);
                let l: Id = (1, ii, jj);
                let r: Id = (1, ii + 1, jj);
                let center_high = (a + b + c + d) / 4.0 >= level;
                match case {
                    1 | 14 => link(l, t),
                    2 | 13 => link(t, r),
                    3 | 12 => link(l, r),
                    4 | 11 => link(r, bt),
                    6 | 9 => link(t, bt),
                    7 | 8 => link(l, bt),
                    5 => {
                        if center_high {
                            link(t, r);
                            link(l, bt);
                        } else {
                            link(l, t);
                            link(r, bt);
                        }
                    }
                    10 => {
                        if center_high {
                            link(l, t);
                            link(r, bt);
                        } else {
                            link(t, r);
                            link(l, bt);
                        }
                    }
                    _ => {}
                }
            }
        }
        let pos = |id: Id| -> P {
            let (i, j) = (id.1 as usize, id.2 as usize);
            let (v0, v1, p0) = if id.0 == 0 {
                (v(i, j), v(i + 1, j), (i as f64, j as f64))
            } else {
                (v(i, j), v(i, j + 1), (i as f64, j as f64))
            };
            let t = (level - v0) / (v1 - v0);
            if id.0 == 0 {
                ((p0.0 + t) * CELL, p0.1 * CELL)
            } else {
                (p0.0 * CELL, (p0.1 + t) * CELL)
            }
        };
        let mut seen: HashSet<Id> = HashSet::new();
        let mut loops = Vec::new();
        let keys: Vec<Id> = adj.keys().copied().collect();
        for start in keys {
            if seen.contains(&start) {
                continue;
            }
            let mut pts = Vec::new();
            let (mut prev, mut cur) = (None, start);
            loop {
                seen.insert(cur);
                pts.push(pos(cur));
                let next = adj[&cur].iter().copied().find(|n| Some(*n) != prev);
                match next {
                    Some(n) if !seen.contains(&n) => {
                        prev = Some(cur);
                        cur = n;
                    }
                    _ => break,
                }
            }
            loops.push(pts);
        }
        loops
    }
}

// ---------------------------------------------------------------------------------------------
// Names
// ---------------------------------------------------------------------------------------------

struct Namer {
    used: Vec<String>,
}

impl Namer {
    fn name(&mut self, rng: &mut Rng) -> String {
        const ONSET: [&str; 22] = [
            "Ves", "Kor", "Ilm", "Pas", "Ner", "Tov", "Bry", "Hosk", "Lum", "Wen", "Yar", "Osk",
            "Sev", "Dov", "Mir", "Ael", "Zan", "Kal", "Orv", "Fiz", "Quen", "Rask",
        ];
        const MID: [&str; 6] = ["a", "e", "i", "o", "al", "en"];
        const END: [&str; 12] = [
            "wick", "mere", "vance", "holt", "ora", "isk", "eth", "dren", "sa", "mund", "ria",
            "thorn",
        ];
        loop {
            let mut s = String::from(ONSET[rng.below(ONSET.len())]);
            if rng.chance(0.3) {
                s.push_str(MID[rng.below(MID.len())]);
            }
            s.push_str(END[rng.below(END.len())]);
            let key = &s[..3];
            if !self.used.iter().any(|u| u.starts_with(key)) {
                self.used.push(s.clone());
                return s;
            }
        }
    }
}

fn text_w(s: &str, size: f64, spacing: f64) -> f64 {
    s.chars().count() as f64 * (size * 0.74 + spacing)
}

// ---------------------------------------------------------------------------------------------
// Glyphs
// ---------------------------------------------------------------------------------------------

fn mountain(out: &mut String, x: f64, y: f64, s: f64, rng: &mut Rng) {
    let w = s * 0.95;
    let h = s * 1.15;
    let mut peak = |out: &mut String, px: f64, py: f64, w: f64, h: f64, hatch: bool| {
        let top = (px + rng.range(-0.15, 0.15) * w, py - h);
        let _ = write!(
            out,
            r#"<path d="M{},{}L{},{}L{},{}" fill="{LAND_FILL}" stroke="{INK}" stroke-width="1.3" stroke-linejoin="round"/>"#,
            num(px - w),
            num(py),
            num(top.0),
            num(top.1),
            num(px + w),
            num(py)
        );
        if hatch {
            let mut d = String::new();
            for k in 1..=3 {
                let t = k as f64 * 0.24;
                let sx = top.0 + (px + w - top.0) * t;
                let sy = top.1 + (py - top.1) * t;
                let ex = sx - w * 0.28 * (1.0 - t * 0.5);
                let ey = sy + (py - sy) * 0.78;
                let _ = write!(d, "M{},{}L{},{}", num(sx), num(sy), num(ex), num(ey));
            }
            let _ = write!(
                out,
                r#"<path d="{d}" stroke="{INK}" stroke-width="0.9" stroke-linecap="round" fill="none"/>"#
            );
        }
    };
    if s > 11.0 {
        peak(out, x - w * 0.95, y + 1.0, w * 0.62, h * 0.62, true);
    }
    peak(out, x, y, w, h, true);
}

fn tree(out: &mut String, x: f64, y: f64, rng: &mut Rng) {
    if rng.chance(0.45) {
        let h = rng.range(7.0, 9.5);
        let _ = write!(
            out,
            r##"<path d="M{},{}L{},{}L{},{}ZM{},{}V{}" fill="#cdbb8a" stroke="{INK}" stroke-width="0.9" stroke-linejoin="round"/>"##,
            num(x),
            num(y - h),
            num(x + 3.4),
            num(y - 1.5),
            num(x - 3.4),
            num(y - 1.5),
            num(x),
            num(y - 1.5),
            num(y + 1.0)
        );
    } else {
        let r = rng.range(3.0, 4.0);
        let _ = write!(
            out,
            r##"<path d="M{},{}V{}" stroke="{INK}" stroke-width="1" fill="none"/><ellipse cx="{}" cy="{}" rx="{}" ry="{}" fill="#cdbb8a" stroke="{INK}" stroke-width="0.9"/>"##,
            num(x),
            num(y + 1.0),
            num(y - 2.0),
            num(x),
            num(y - 5.0),
            num(r),
            num(r * 1.15)
        );
    }
}

fn hill(out: &mut String, x: f64, y: f64, s: f64) {
    let _ = write!(
        out,
        r#"<path d="M{},{}Q{},{} {},{}" fill="none" stroke="{INK}" stroke-width="1" stroke-linecap="round"/>"#,
        num(x - s),
        num(y),
        num(x),
        num(y - s * 1.5),
        num(x + s),
        num(y)
    );
}

fn tuft(out: &mut String, x: f64, y: f64) {
    let _ = write!(
        out,
        r#"<path d="M{},{}h8M{},{}v-4M{},{}v-5M{},{}v-4" stroke="{INK}" stroke-width="0.9" stroke-linecap="round" fill="none"/>"#,
        num(x - 4.0),
        num(y),
        num(x - 2.0),
        num(y),
        num(x),
        num(y),
        num(x + 2.0),
        num(y)
    );
}

fn compass(out: &mut String, cx: f64, cy: f64, r: f64) {
    let _ = write!(out, r#"<g transform="translate({},{})">"#, num(cx), num(cy));
    let _ = write!(
        out,
        r##"<circle r="{}" fill="#efe1bb" fill-opacity="0.55" stroke="{INK}" stroke-width="1.4"/><circle r="{}" fill="none" stroke="{INK}" stroke-width="0.8"/>"##,
        num(r),
        num(r * 0.8)
    );
    let mut ticks = String::new();
    for k in 0..48 {
        let a = k as f64 * std::f64::consts::TAU / 48.0;
        let r0 = if k % 4 == 0 { r * 0.8 } else { r * 0.9 };
        let _ = write!(
            ticks,
            "M{},{}L{},{}",
            num(a.sin() * r0),
            num(-a.cos() * r0),
            num(a.sin() * r),
            num(-a.cos() * r)
        );
    }
    let _ = write!(
        out,
        r#"<path d="{ticks}" stroke="{INK}" stroke-width="0.8" fill="none"/>"#
    );
    // Thin 16-wind lines.
    let mut thin = String::new();
    for k in 0..16 {
        if k % 2 == 1 {
            let a = k as f64 * std::f64::consts::TAU / 16.0;
            let _ = write!(
                thin,
                "M0,0L{},{}",
                num(a.sin() * r * 0.62),
                num(-a.cos() * r * 0.62)
            );
        }
    }
    let _ = write!(
        out,
        r#"<path d="{thin}" stroke="{INK}" stroke-width="0.7" fill="none"/>"#
    );
    // Star points: (rotation, tip length, shoulder radius). Each point is split light/dark.
    for (rot, len, sh) in [(45.0_f64, 0.58, 0.13), (0.0, 0.8, 0.17)] {
        for k in 0..4 {
            let a0 = (rot + k as f64 * 90.0).to_radians();
            let tip = (a0.sin() * r * len, -a0.cos() * r * len);
            let side = |off: f64| {
                let a = a0 + off.to_radians();
                (a.sin() * r * sh * 1.9, -a.cos() * r * sh * 1.9)
            };
            let (l, rr) = (side(-45.0), side(45.0));
            let _ = write!(
                out,
                r##"<path d="M0,0L{},{}L{},{}Z" fill="{INK}" stroke="{INK}" stroke-width="0.8" stroke-linejoin="round"/><path d="M0,0L{},{}L{},{}Z" fill="#f2e6c4" stroke="{INK}" stroke-width="0.8" stroke-linejoin="round"/>"##,
                num(l.0),
                num(l.1),
                num(tip.0),
                num(tip.1),
                num(rr.0),
                num(rr.1),
                num(tip.0),
                num(tip.1)
            );
        }
    }
    let _ = write!(
        out,
        r##"<circle r="{}" fill="#f2e6c4" stroke="{INK}" stroke-width="1"/><circle r="{}" fill="{INK}"/>"##,
        num(r * 0.06),
        num(r * 0.025)
    );
    // North mark: small fleur-like diamond with the letter above.
    let _ = write!(
        out,
        r#"<path d="M0,{}l5,-8l-5,-8l-5,8z" fill="{INK}"/><text y="{}" font-family="Cinzel" font-size="17" font-weight="700" text-anchor="middle" fill="{INK}">N</text>"#,
        num(-r - 4.0),
        num(-r - 24.0)
    );
    out.push_str("</g>");
}

fn cartouche(out: &mut String, r: R, kicker: &str, title: &str) {
    let (w, h) = (r.x1 - r.x0, r.y1 - r.y0);
    let _ = write!(
        out,
        r##"<g transform="translate({},{})"><rect width="{}" height="{}" rx="8" fill="#efe1bb" fill-opacity="0.92" stroke="{INK}" stroke-width="2.2"/><rect x="6" y="6" width="{}" height="{}" rx="5" fill="none" stroke="{INK}" stroke-width="0.9"/>"##,
        num(r.x0),
        num(r.y0),
        num(w),
        num(h),
        num(w - 12.0),
        num(h - 12.0)
    );
    let _ = write!(
        out,
        r#"<path d="M26,{y}h{l}M{x},{y}h{l}" stroke="{INK}" stroke-width="0.8"/><path d="M14,{y}l6,-5l6,5l-6,5zM{x2},{y}l6,-5l6,5l-6,5z" fill="{INK}"/>"#,
        y = num(h * 0.34),
        l = num(w * 0.17),
        x = num(w - 26.0 - w * 0.17),
        x2 = num(w - 26.0),
    );
    let size = (h * 0.4).min(300.0 / (title.chars().count() as f64 * 0.95));
    let _ = write!(
        out,
        r#"<text x="{cx}" y="{ky}" font-family="Cinzel" font-size="12" letter-spacing="5" text-anchor="middle" fill="{INK}">{kicker}</text><text x="{cx}" y="{ty}" font-family="Cinzel" font-size="{sz}" font-weight="700" letter-spacing="6" text-anchor="middle" fill="{INK}">{title}</text></g>"#,
        cx = num(w / 2.0),
        ky = num(h * 0.4),
        ty = num(h * 0.8),
        sz = num(size),
    );
}

// ---------------------------------------------------------------------------------------------
// The map
// ---------------------------------------------------------------------------------------------

struct Label {
    text: String,
    x: f64,
    y: f64,
    size: f64,
    spacing: f64,
    weight: u32,
    opacity: f64,
    anchor: &'static str,
}

fn in_band(y: f64, pad: f64) -> bool {
    (y - BAND_Y).abs() < BAND_HALF + pad
}

/// True when every sample of `r` is comfortably on land.
fn rect_on_land(f: &Field, r: &R, min_e: f64) -> bool {
    let mut y = r.y0;
    while y <= r.y1 + 0.1 {
        let mut x = r.x0;
        while x <= r.x1 + 0.1 {
            if f.e(x, y) < min_e {
                return false;
            }
            x += 10.0;
        }
        y += 10.0;
    }
    true
}

fn rect_on_sea(f: &Field, r: &R, max_e: f64) -> bool {
    let mut y = r.y0;
    while y <= r.y1 + 0.1 {
        let mut x = r.x0;
        while x <= r.x1 + 0.1 {
            if f.e(x, y) > max_e {
                return false;
            }
            x += 10.0;
        }
        y += 10.0;
    }
    true
}

/// Returns a complete SVG document (viewBox 0 0 1600 1000) for the map.
pub fn generate(seed: u64) -> String {
    let mut rng = Rng::new(seed);
    let mut namer = Namer { used: Vec::new() };

    // Fixed decoration slots; islands stay out of them.
    let frame_in = R::new(34.0, 34.0, W - 34.0, H - 34.0);
    let cart_rect = R::new(58.0, 56.0, 458.0, 150.0);
    let compass_c = (1455.0, 850.0);
    let compass_r = 82.0;
    let compass_rect = R::new(
        compass_c.0 - compass_r - 30.0,
        compass_c.1 - compass_r - 36.0,
        compass_c.0 + compass_r + 30.0,
        compass_c.1 + compass_r + 30.0,
    );
    let keep_out = [cart_rect.pad(30.0), compass_rect.pad(40.0)];

    let world = World::new(&mut rng, &keep_out);
    let field = Field::build(&world);

    // ---- Coastlines ----
    let mut coast: Vec<Vec<P>> = field
        .contour(SEA)
        .into_iter()
        .filter(|l| perimeter(l) > 45.0)
        .map(|l| chaikin(&l, 3, true))
        .collect();
    coast.sort_by_key(|a| std::cmp::Reverse(a.len()));
    let land_d: String = coast.iter().map(|l| path_d(l, true)).collect();

    // ---- Labels (placed before glyphs so glyphs can avoid them) ----
    let mut reserved: Vec<R> = vec![cart_rect.pad(8.0), compass_rect];
    let mut labels: Vec<Label> = Vec::new();
    let mut sea_label_rects: Vec<R> = Vec::new();
    let realm = namer.name(&mut rng);

    // Region names: big spaced capitals on land, away from the band.
    for (idx, size) in [34.0_f64, 30.0, 28.0].into_iter().enumerate() {
        let text = namer.name(&mut rng).to_uppercase();
        let spacing = size * 0.35;
        let w = text_w(&text, size, spacing);
        for _ in 0..600 {
            let x = rng.range(140.0, W - 140.0);
            let y = rng.range(120.0, H - 120.0);
            let rect = R::new(x - w / 2.0, y - size, x + w / 2.0, y + size * 0.4);
            if in_band(y - size * 0.3, size)
                && rect.y1 > BAND_Y - BAND_HALF - 10.0
                && rect.y0 < BAND_Y + BAND_HALF + 10.0
            {
                continue;
            }
            if reserved.iter().any(|r| r.hit(&rect.pad(10.0)))
                || !rect_on_land(&field, &rect.pad(4.0), 0.06)
            {
                continue;
            }
            // Spread regions apart a little.
            if idx > 0
                && labels
                    .iter()
                    .take(idx)
                    .any(|l| (l.x - x).abs() < 380.0 && (l.y - y).abs() < 130.0)
            {
                continue;
            }
            reserved.push(rect.pad(12.0));
            labels.push(Label {
                text,
                x,
                y,
                size,
                spacing,
                weight: 600,
                opacity: 0.5,
                anchor: "middle",
            });
            break;
        }
    }
    let region_count = labels.len();

    // Cities and towns: (font size, marker kind).
    let mut markers: Vec<(P, u8)> = Vec::new();
    for (k, size) in [18.0_f64, 16.0, 16.0, 15.0, 14.0, 14.0, 13.0, 13.0]
        .into_iter()
        .enumerate()
    {
        let text = namer.name(&mut rng);
        let w = text_w(&text, size * 0.86, 1.0);
        for _ in 0..800 {
            let x = rng.range(110.0, W - 110.0);
            let y = rng.range(100.0, H - 100.0);
            if in_band(y, 22.0) || field.e(x, y) < 0.1 {
                continue;
            }
            if markers
                .iter()
                .any(|m| ((m.0 .0 - x).powi(2) + (m.0 .1 - y).powi(2)).sqrt() < 150.0)
            {
                continue;
            }
            let right = x < W / 2.0 || rng.chance(0.5);
            let (x0, x1) = if right {
                (x - 8.0, x + 12.0 + w)
            } else {
                (x - 12.0 - w, x + 8.0)
            };
            let rect = R::new(x0, y - size * 0.95, x1, y + size * 0.5);
            if !frame_in.pad(-50.0).contains(rect.x0, rect.y0)
                || !frame_in.pad(-50.0).contains(rect.x1, rect.y1)
            {
                continue;
            }
            if reserved.iter().any(|r| r.hit(&rect.pad(6.0)))
                || !rect_on_land(&field, &rect.pad(2.0), 0.03)
            {
                continue;
            }
            reserved.push(rect.pad(6.0));
            markers.push((
                (x, y),
                if k == 0 {
                    2
                } else if k < 4 {
                    1
                } else {
                    0
                },
            ));
            let (tx, anchor) = if right {
                (x + 11.0, "start")
            } else {
                (x - 11.0, "end")
            };
            labels.push(Label {
                text,
                x: tx,
                y: y + size * 0.32,
                size,
                spacing: 1.0,
                weight: if k < 4 { 600 } else { 500 },
                opacity: 0.9,
                anchor,
            });
            break;
        }
    }

    // Sea name: italic-ish spaced capitals in open water near the top or bottom edge.
    let sea_text = format!("{} SEA", namer.name(&mut rng).to_uppercase());
    let sea_size = 21.0;
    let sea_sp = 9.0;
    let sea_w = text_w(&sea_text, sea_size, sea_sp);
    let mut sea_label: Option<(f64, f64)> = None;
    for _ in 0..400 {
        let x = rng.range(300.0, W - 300.0);
        let y = if rng.chance(0.5) {
            rng.range(84.0, 125.0)
        } else {
            rng.range(880.0, 925.0)
        };
        let rect = R::new(x - sea_w / 2.0, y - sea_size, x + sea_w / 2.0, y + 8.0);
        if reserved.iter().any(|r| r.hit(&rect.pad(10.0)))
            || !rect_on_sea(&field, &rect.pad(14.0), -0.04)
        {
            continue;
        }
        reserved.push(rect.pad(8.0));
        sea_label_rects.push(rect.pad(6.0));
        sea_label = Some((x, y));
        break;
    }

    // ---- Mountains (candidates first: rivers start from them) ----
    let mut mountains: Vec<(f64, f64, f64)> = Vec::new();
    {
        let mut r = Rng::new(seed ^ 0x4d4f);
        let step = 12.0;
        let mut y = 60.0;
        while y < H - 60.0 {
            let mut x = 60.0;
            while x < W - 60.0 {
                let (px, py) = (x + r.range(-5.0, 5.0), y + r.range(-4.0, 4.0));
                x += step;
                if in_band(py, 10.0) || field.e(px, py) < 0.12 {
                    continue;
                }
                let ridge = 1.0 - (2.0 * fbm(world.s_ridge, px / 170.0, py / 170.0, 4) - 1.0).abs();
                let reach = smoothstep(0.12, 0.3, field.e(px, py));
                let score = ridge * reach;
                if score > 0.88 && r.chance(0.6) {
                    mountains.push((
                        px,
                        py,
                        8.0 + 9.0 * smoothstep(0.88, 0.97, score) + r.range(0.0, 2.0),
                    ));
                }
            }
            y += step;
        }
    }
    mountains.retain(|m| !reserved.iter().any(|r| r.pad(18.0).contains(m.0, m.1)));

    // ---- Rivers ----
    let mut rivers: Vec<Vec<P>> = Vec::new();
    {
        let mut r = Rng::new(seed ^ 0x7269);
        // Sources: mountains first, then random inland points (so some rivers cross the centre).
        let mut cands: Vec<P> = mountains.iter().map(|m| (m.0, m.1)).collect();
        for i in (1..cands.len()).rev() {
            cands.swap(i, r.below(i + 1));
        }
        for _ in 0..300 {
            let (x, y) = (r.range(120.0, W - 120.0), r.range(100.0, H - 100.0));
            if field.e(x, y) > 0.3 {
                cands.push((x, y));
            }
        }
        let order: Vec<usize> = (0..cands.len()).collect();
        let mut sources: Vec<P> = Vec::new();
        for idx in order {
            if rivers.len() >= 5 {
                break;
            }
            let (sx, sy) = cands[idx];
            if sources
                .iter()
                .any(|s| ((s.0 - sx).powi(2) + (s.1 - sy).powi(2)).sqrt() < 260.0)
            {
                continue;
            }
            let (mut gi, mut gj) = ((sx / CELL) as i32, (sy / CELL) as i32);
            let mut seen: HashSet<(i32, i32)> = HashSet::new();
            let mut path = vec![(gi, gj)];
            seen.insert((gi, gj));
            let mut ok = false;
            for _ in 0..500 {
                if field.h[gj as usize * GW + gi as usize] < SEA {
                    ok = true;
                    break;
                }
                let mut best: Option<(f64, i32, i32)> = None;
                for (di, dj) in [
                    (-1, -1),
                    (0, -1),
                    (1, -1),
                    (-1, 0),
                    (1, 0),
                    (-1, 1),
                    (0, 1),
                    (1, 1),
                ] {
                    let (ni, nj) = (gi + di, gj + dj);
                    if ni < 1
                        || nj < 1
                        || ni >= GW as i32 - 1
                        || nj >= GH as i32 - 1
                        || seen.contains(&(ni, nj))
                    {
                        continue;
                    }
                    let hv = field.h[nj as usize * GW + ni as usize] + r.range(0.0, 0.004);
                    if best.is_none_or(|b| hv < b.0) {
                        best = Some((hv, ni, nj));
                    }
                }
                let Some((_, ni, nj)) = best else { break };
                gi = ni;
                gj = nj;
                seen.insert((gi, gj));
                path.push((gi, gj));
            }
            let hits_label = path.iter().any(|&(i, j)| {
                reserved
                    .iter()
                    .any(|q| q.contains(i as f64 * CELL, j as f64 * CELL))
            });
            if !ok || hits_label || path.len() < 36 {
                continue;
            }
            let pts: Vec<P> = path
                .iter()
                .step_by(3)
                .map(|&(i, j)| (i as f64 * CELL, j as f64 * CELL))
                .collect();
            // Meander: sideways wobble that grows toward the mouth.
            let n = pts.len();
            let ph = r.range(0.0, 50.0);
            let pts: Vec<P> = (0..n)
                .map(|k| {
                    let (a, b) = (pts[k.saturating_sub(1)], pts[(k + 1).min(n - 1)]);
                    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                    let len = (dx * dx + dy * dy).sqrt().max(1.0);
                    let w =
                        (value_noise(world.s_hill, ph + k as f64 * 1.3, 3.3) - 0.5) * 2.0 * 26.0;
                    let t = (k as f64 / n as f64).min(1.0);
                    (
                        pts[k].0 - dy / len * w * (0.3 + t),
                        pts[k].1 + dx / len * w * (0.3 + t),
                    )
                })
                .collect();
            let smooth = chaikin(&pts, 3, false);
            sources.push((sx, sy));
            rivers.push(smooth);
        }
    }
    let river_cells: HashSet<(i32, i32)> = rivers
        .iter()
        .flatten()
        .map(|p| ((p.0 / 9.0) as i32, (p.1 / 9.0) as i32))
        .collect();
    let near_river = |x: f64, y: f64| {
        let (ci, cj) = ((x / 9.0) as i32, (y / 9.0) as i32);
        (-1..=1).any(|a| (-1..=1).any(|b| river_cells.contains(&(ci + a, cj + b))))
    };
    mountains.retain(|m| !near_river(m.0, m.1));

    // ---- Lakes ----
    let mut lakes: Vec<(P, f64)> = Vec::new();
    {
        let mut r = Rng::new(seed ^ 0x6c61);
        for _ in 0..400 {
            if lakes.len() >= 2 {
                break;
            }
            let (x, y) = (r.range(150.0, W - 150.0), r.range(120.0, H - 120.0));
            let rad = r.range(11.0, 19.0);
            let probe = R::new(x - rad * 1.6, y - rad * 1.6, x + rad * 1.6, y + rad * 1.6);
            if in_band(y, rad)
                || field.e(x, y) < 0.22
                || !rect_on_land(&field, &probe, 0.12)
                || reserved.iter().any(|q| q.hit(&probe))
                || mountains
                    .iter()
                    .any(|m| (m.0 - x).abs() < rad * 2.0 && (m.1 - y).abs() < rad * 2.0)
                || near_river(x, y)
                || lakes
                    .iter()
                    .any(|l| ((l.0 .0 - x).powi(2) + (l.0 .1 - y).powi(2)).sqrt() < 350.0)
            {
                continue;
            }
            reserved.push(probe);
            lakes.push(((x, y), rad));
        }
    }

    // ---- Roads between nearby cities ----
    let mut roads: Vec<Vec<P>> = Vec::new();
    {
        let mut r = Rng::new(seed ^ 0x726f);
        let mut pairs: Vec<(f64, usize, usize)> = Vec::new();
        for a in 0..markers.len() {
            for b in a + 1..markers.len() {
                let (p, q) = (markers[a].0, markers[b].0);
                pairs.push((((p.0 - q.0).powi(2) + (p.1 - q.1).powi(2)).sqrt(), a, b));
            }
        }
        let crosses =
            |a: usize, b: usize| (markers[a].0 .1 - BAND_Y) * (markers[b].0 .1 - BAND_Y) < 0.0;
        pairs.sort_by(|x, y| {
            (!crosses(x.1, x.2))
                .cmp(&!crosses(y.1, y.2))
                .then(x.0.total_cmp(&y.0))
        });
        for (dist, a, b) in pairs {
            if roads.len() >= 3 {
                break;
            }
            if !(150.0..820.0).contains(&dist) {
                continue;
            }
            let (p, q) = (markers[a].0, markers[b].0);
            let (dx, dy) = ((q.0 - p.0) / dist, (q.1 - p.1) / dist);
            for attempt in 0..14 {
                let spread = 0.1 + 0.025 * attempt as f64;
                let bend = r.range(-spread, spread) * dist;
                let pts: Vec<P> = (0..=12)
                    .map(|k| {
                        let t = k as f64 / 12.0;
                        let wob = if k == 0 || k == 12 {
                            0.0
                        } else {
                            r.range(-3.0, 3.0)
                        };
                        let off = bend * (t * std::f64::consts::PI).sin() + wob;
                        (
                            p.0 + (q.0 - p.0) * t - dy * off,
                            p.1 + (q.1 - p.1) * t + dx * off,
                        )
                    })
                    .collect();
                let smooth = chaikin(&pts, 3, false);
                let cut = smooth.len() / 8;
                let clean = smooth.iter().enumerate().all(|(i, c)| {
                    (i < cut
                        || i + cut > smooth.len()
                        || !reserved.iter().any(|q| q.contains(c.0, c.1)))
                        && field.e(c.0, c.1) > 0.04
                        && !lakes.iter().any(|l| {
                            ((l.0 .0 - c.0).powi(2) + (l.0 .1 - c.1).powi(2)).sqrt() < l.1 * 1.6
                        })
                        && !mountains
                            .iter()
                            .any(|m| (m.0 - c.0).abs() < 9.0 && (m.1 - c.1).abs() < 9.0)
                });
                if clean {
                    roads.push(smooth);
                    break;
                }
            }
        }
    }

    // ---- Assemble SVG ----
    let mut s = String::with_capacity(400_000);
    let pseed = (seed % 9973) as u32;
    let _ = write!(
        s,
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {WIDTH} {HEIGHT}" width="{WIDTH}" height="{HEIGHT}">"##
    );
    let _ = write!(
        s,
        r##"<defs>
<radialGradient id="pg" cx="0.5" cy="0.5" r="0.75"><stop offset="0" stop-color="#eddcb0"/><stop offset="0.6" stop-color="#e3cd9d"/><stop offset="1" stop-color="#d2b57e"/></radialGradient>
<radialGradient id="vg" cx="0.5" cy="0.5" r="0.72"><stop offset="0.55" stop-color="#4a2a0c" stop-opacity="0"/><stop offset="1" stop-color="#4a2a0c" stop-opacity="0.42"/></radialGradient>
<radialGradient id="st" cx="0.5" cy="0.5" r="0.5"><stop offset="0" stop-color="#8a5a24" stop-opacity="0.16"/><stop offset="0.6" stop-color="#8a5a24" stop-opacity="0.07"/><stop offset="1" stop-color="#8a5a24" stop-opacity="0"/></radialGradient>
<filter id="blot" x="0" y="0" width="100%" height="100%" color-interpolation-filters="sRGB"><feTurbulence type="fractalNoise" baseFrequency="0.0035 0.0055" numOctaves="4" seed="{pseed}"/><feColorMatrix type="matrix" values="0 0 0 0 0.55  0 0 0 0 0.36  0 0 0 0 0.14  0 0 0 2.4 -1.05"/></filter>
<filter id="fiber" x="0" y="0" width="100%" height="100%" color-interpolation-filters="sRGB"><feTurbulence type="fractalNoise" baseFrequency="0.55 0.9" numOctaves="2" seed="{pseed}"/><feColorMatrix type="matrix" values="0 0 0 0 0.4  0 0 0 0 0.26  0 0 0 0 0.1  0 0 0 1.6 -0.7"/></filter>
<filter id="soft" x="-20%" y="-20%" width="140%" height="140%"><feGaussianBlur stdDeviation="30"/></filter>
<filter id="soft2" x="-20%" y="-20%" width="140%" height="140%"><feGaussianBlur stdDeviation="3"/></filter>
<clipPath id="inner"><rect x="36" y="36" width="{iw}" height="{ih}"/></clipPath>
<mask id="rm" maskUnits="userSpaceOnUse" x="0" y="0" width="{WIDTH}" height="{HEIGHT}"><rect width="{WIDTH}" height="{HEIGHT}" fill="#fff"/>"##,
        iw = W - 72.0,
        ih = H - 72.0
    );
    for r in &sea_label_rects {
        let _ = write!(
            s,
            r##"<rect x="{}" y="{}" width="{}" height="{}" rx="8" fill="#000"/>"##,
            num(r.x0),
            num(r.y0),
            num(r.x1 - r.x0),
            num(r.y1 - r.y0)
        );
    }
    let _ = write!(
        s,
        r##"<circle cx="{}" cy="{}" r="{}" fill="#000"/></mask></defs>"##,
        num(compass_c.0),
        num(compass_c.1),
        num(compass_r + 6.0)
    );

    // Parchment.
    let _ = write!(
        s,
        r##"<rect width="{WIDTH}" height="{HEIGHT}" fill="url(#pg)"/><rect width="{WIDTH}" height="{HEIGHT}" filter="url(#blot)" opacity="0.5"/><rect width="{WIDTH}" height="{HEIGHT}" filter="url(#fiber)" opacity="0.07"/>"##
    );
    let mut sr = Rng::new(seed ^ 0x7374);
    for _ in 0..5 {
        let (x, y) = (sr.range(150.0, W - 150.0), sr.range(120.0, H - 120.0));
        let (rx, ry) = (sr.range(110.0, 240.0), sr.range(80.0, 170.0));
        let _ = write!(
            s,
            r##"<ellipse cx="{}" cy="{}" rx="{}" ry="{}" fill="url(#st)"/>"##,
            num(x),
            num(y),
            num(rx),
            num(ry)
        );
    }
    let (rx, ry) = (sr.range(300.0, 1300.0), sr.range(200.0, 800.0));
    let _ = write!(
        s,
        r##"<circle cx="{}" cy="{}" r="{}" fill="none" stroke="#8a5a24" stroke-opacity="0.07" stroke-width="7" filter="url(#soft2)"/>"##,
        num(rx),
        num(ry),
        num(sr.range(60.0, 110.0))
    );

    // Sea wash (everything except land) and the lighter land fill.
    let _ = write!(
        s,
        r##"<g clip-path="url(#inner)"><path d="M0,0H{WIDTH}V{HEIGHT}H0Z{land_d}" fill="#8a6a3a" fill-opacity="0.3" fill-rule="evenodd"/><path d="{land_d}" fill="#f3e6c1" fill-opacity="0.5" fill-rule="evenodd"/>"##
    );

    // Ripple lines in the sea, following the coast.
    let _ = write!(
        s,
        r#"<g mask="url(#rm)" fill="none" stroke="{INK}" stroke-linecap="round" stroke-linejoin="round">"#
    );
    for (lvl, op, w, dash) in [
        (SEA - 0.028, 0.7, 1.2, "26 5 11 6 3 7"),
        (SEA - 0.065, 0.5, 1.1, "18 8 30 7 6 9"),
        (SEA - 0.11, 0.32, 1.0, "10 9 22 12"),
    ] {
        for l in field.contour(lvl) {
            if perimeter(&l) < 70.0 {
                continue;
            }
            let sm = chaikin(&l, 3, true);
            let _ = write!(
                s,
                r#"<path d="{}" stroke-opacity="{op}" stroke-width="{w}" stroke-dasharray="{dash}"/>"#,
                path_d(&sm, true)
            );
        }
    }
    s.push_str("</g>");

    // Rivers (tapering: later segments are wider).
    for rv in &rivers {
        let n = rv.len();
        let chunk = 6;
        let mut i = 0;
        while i + 1 < n {
            let end = (i + chunk).min(n - 1);
            let t = i as f64 / n as f64;
            let _ = write!(
                s,
                r##"<path d="{}" fill="none" stroke="#5c4229" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round" stroke-opacity="0.9"/>"##,
                path_d(&rv[i..=end], false),
                num(0.5 + 1.7 * t)
            );
            i = end;
        }
    }

    // Roads.
    for rd in &roads {
        let _ = write!(
            s,
            r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="1.6" stroke-linecap="round" stroke-dasharray="0.1 5" stroke-opacity="0.8"/>"#,
            path_d(rd, false)
        );
    }

    // Lakes.
    let mut lr = Rng::new(seed ^ 0x6c6b);
    for &((cx, cy), rad) in &lakes {
        let n = 14;
        let phase = lr.range(0.0, 10.0);
        let pts: Vec<P> = (0..n)
            .map(|k| {
                let a = k as f64 * std::f64::consts::TAU / n as f64;
                let rr = rad
                    * (0.75
                        + 0.5
                            * value_noise(
                                world.s_hill,
                                phase + a.cos() * 1.4,
                                phase + a.sin() * 1.4,
                            ));
                (cx + a.cos() * rr * 1.3, cy + a.sin() * rr * 0.85)
            })
            .collect();
        let sm = chaikin(&pts, 3, true);
        let ring: Vec<P> = sm
            .iter()
            .map(|p| (cx + (p.0 - cx) * 1.25, cy + (p.1 - cy) * 1.3))
            .collect();
        let _ = write!(
            s,
            r##"<path d="{}" fill="none" stroke="{INK}" stroke-width="0.8" stroke-opacity="0.45" stroke-dasharray="14 6"/><path d="{}" fill="#d9c89c" fill-opacity="0.9" stroke="{INK}" stroke-width="1.5"/>"##,
            path_d(&ring, true),
            path_d(&sm, true)
        );
    }

    // Coastline ink.
    let _ = write!(
        s,
        r#"<path d="{land_d}" fill="none" stroke="{INK}" stroke-width="2.2" stroke-linejoin="round" stroke-opacity="0.92"/>"#
    );

    // Glyphs: forests, hills, marsh, mountains. Painted back to front.
    let mut gr = Rng::new(seed ^ 0x676c);
    let blocked = |x: f64, y: f64| reserved.iter().any(|r| r.pad(7.0).contains(x, y));
    let mut trees: Vec<(f64, f64, f64)> = Vec::new();
    let mut hills: Vec<(f64, f64, f64)> = Vec::new();
    let mut tufts: Vec<(f64, f64)> = Vec::new();
    let mut fields: Vec<(f64, f64, f64)> = Vec::new();
    {
        let mut y = 60.0;
        while y < H - 60.0 {
            let mut x = 60.0;
            while x < W - 60.0 {
                let (px, py) = (x + gr.range(-3.5, 3.5), y + gr.range(-3.5, 3.5));
                x += 10.0;
                let e = field.e(px, py);
                if e < 0.05 || blocked(px, py) || near_river(px, py) || in_band(py, 0.0) {
                    continue;
                }
                let wood = fbm(world.s_wood, px / 75.0, py / 75.0, 3);
                let core = smoothstep(0.56, 0.74, wood);
                let p = if wood > 0.56 {
                    0.25 + 0.7 * core
                } else if wood > 0.5 {
                    0.07
                } else {
                    0.0
                };
                if e < 0.7 && gr.chance(p) {
                    trees.push((px, py, 0.75 + 0.45 * core + gr.range(-0.1, 0.15)));
                }
            }
            y += 10.0;
        }
        let mut y = 70.0;
        while y < H - 70.0 {
            let mut x = 70.0;
            while x < W - 70.0 {
                let (px, py) = (x + gr.range(-8.0, 8.0), y + gr.range(-8.0, 8.0));
                x += 26.0;
                let e = field.e(px, py);
                if e < 0.07 || blocked(px, py) || near_river(px, py) {
                    continue;
                }
                let band = in_band(py, 0.0);
                let hv = fbm(world.s_hill, px / 120.0, py / 120.0, 3);
                if band {
                    // Calm centre: sparse low hills and grass tufts, nothing dense.
                    if hv > 0.55 && gr.chance(0.16) {
                        hills.push((px, py, gr.range(4.0, 6.0)));
                    } else if gr.chance(0.2) {
                        tufts.push((px, py));
                    } else if gr.chance(0.05) {
                        fields.push((px, py, gr.range(-0.3, 0.3)));
                    }
                } else if hv > 0.58
                    && gr.chance(0.6)
                    && !mountains
                        .iter()
                        .any(|m| (m.0 - px).abs() < 16.0 && (m.1 - py).abs() < 16.0)
                {
                    hills.push((px, py, gr.range(5.0, 8.0)));
                } else if hv < 0.34 && e < 0.25 && gr.chance(0.35) {
                    tufts.push((px, py));
                }
            }
            y += 26.0;
        }
    }
    trees.retain(|t| {
        !mountains
            .iter()
            .any(|m| (m.0 - t.0).abs() < 14.0 && (m.1 - t.1).abs() < 14.0)
    });
    trees.sort_by(|a, b| a.1.total_cmp(&b.1));
    for f in fields {
        // Faint field hatching: a small patch of parallel strokes.
        let mut d = String::new();
        for k in 0..6 {
            let oy = f.1 + k as f64 * 4.5;
            let _ = write!(d, "M{},{}l26,{}", num(f.0), num(oy), num(f.2 * 26.0));
        }
        let _ = write!(
            s,
            r#"<path d="{d}" stroke="{INK}" stroke-width="0.7" stroke-opacity="0.35" stroke-linecap="round" fill="none"/>"#
        );
    }
    for t in tufts {
        tuft(&mut s, t.0, t.1);
    }
    for h in hills {
        hill(&mut s, h.0, h.1, h.2);
    }
    for t in &trees {
        let _ = write!(
            s,
            r#"<g transform="translate({} {}) scale({}) translate({} {})">"#,
            num(t.0),
            num(t.1),
            num(t.2),
            num(-t.0),
            num(-t.1)
        );
        tree(&mut s, t.0, t.1, &mut gr);
        s.push_str("</g>");
    }
    mountains.sort_by(|a, b| a.1.total_cmp(&b.1));
    for m in &mountains {
        mountain(&mut s, m.0, m.1, m.2, &mut gr);
    }

    // City markers.
    for &((x, y), kind) in &markers {
        match kind {
            2 => {
                let _ = write!(
                    s,
                    r##"<circle cx="{}" cy="{}" r="6.5" fill="#f3e6c1" stroke="{INK}" stroke-width="1.6"/><circle cx="{}" cy="{}" r="2.6" fill="{INK}"/>"##,
                    num(x),
                    num(y),
                    num(x),
                    num(y)
                );
            }
            1 => {
                let _ = write!(
                    s,
                    r##"<circle cx="{}" cy="{}" r="4.6" fill="#f3e6c1" stroke="{INK}" stroke-width="1.5"/><circle cx="{}" cy="{}" r="1.7" fill="{INK}"/>"##,
                    num(x),
                    num(y),
                    num(x),
                    num(y)
                );
            }
            _ => {
                let _ = write!(
                    s,
                    r#"<rect x="{}" y="{}" width="6" height="6" fill="{INK}" transform="rotate(45 {} {})"/>"#,
                    num(x - 3.0),
                    num(y - 3.0),
                    num(x),
                    num(y)
                );
            }
        }
    }

    // Text labels.
    for (i, l) in labels.iter().enumerate() {
        let halo = if i < region_count {
            ""
        } else {
            r##" stroke="#ecdcb2" stroke-opacity="0.75" stroke-width="3.5" paint-order="stroke" stroke-linejoin="round""##
        };
        let _ = write!(
            s,
            r#"<text x="{}" y="{}" font-family="Cinzel" font-size="{}" font-weight="{}" letter-spacing="{}" text-anchor="{}" fill="{INK}" fill-opacity="{}"{halo}>{}</text>"#,
            num(l.x),
            num(l.y),
            num(l.size),
            l.weight,
            num(l.spacing),
            l.anchor,
            l.opacity,
            l.text
        );
    }
    if let Some((x, y)) = sea_label {
        let _ = write!(
            s,
            r#"<g transform="translate({} {}) skewX(-14)"><text font-family="Cinzel" font-size="{}" letter-spacing="{}" text-anchor="middle" fill="{INK}" fill-opacity="0.75">{}</text></g>"#,
            num(x),
            num(y),
            num(sea_size),
            num(sea_sp),
            sea_text
        );
    }
    s.push_str("</g>");

    // Decorations.
    compass(&mut s, compass_c.0, compass_c.1, compass_r);
    cartouche(&mut s, cart_rect, "THE REALM OF", &realm.to_uppercase());

    // Border frame.
    let _ = write!(
        s,
        r#"<rect x="20" y="20" width="{}" height="{}" fill="none" stroke="{INK}" stroke-width="3.6" stroke-opacity="0.9"/><rect x="31" y="31" width="{}" height="{}" fill="none" stroke="{INK}" stroke-width="1.2" stroke-opacity="0.9"/>"#,
        W - 40.0,
        H - 40.0,
        W - 62.0,
        H - 62.0
    );
    for (cx, cy) in [
        (20.0, 20.0),
        (W - 20.0, 20.0),
        (20.0, H - 20.0),
        (W - 20.0, H - 20.0),
    ] {
        let _ = write!(
            s,
            r#"<rect x="{}" y="{}" width="12" height="12" fill="{INK}" transform="rotate(45 {} {})"/>"#,
            num(cx - 6.0),
            num(cy - 6.0),
            num(cx),
            num(cy)
        );
    }

    // Aging on top: scorched edges and vignette.
    let _ = write!(
        s,
        r##"<rect x="-30" y="-30" width="{}" height="{}" fill="none" stroke="#4a2a0c" stroke-width="90" stroke-opacity="0.3" filter="url(#soft)"/><rect width="{WIDTH}" height="{HEIGHT}" fill="url(#vg)"/></svg>"##,
        W + 60.0,
        H + 60.0
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        assert_eq!(generate(7), generate(7));
    }

    #[test]
    fn seeds_differ() {
        assert_ne!(generate(1), generate(2));
    }

    #[test]
    fn parses_as_svg() {
        let svg = generate(SEED);
        let mut opt = resvg::usvg::Options::default();
        opt.fontdb_mut()
            .load_font_data(include_bytes!("../../../assets/fonts/Cinzel.ttf").to_vec());
        let tree = resvg::usvg::Tree::from_str(&svg, &opt).expect("valid svg");
        assert_eq!(tree.size().width() as u32, WIDTH);
        assert!(!svg.contains('\u{2014}'));
    }
}
