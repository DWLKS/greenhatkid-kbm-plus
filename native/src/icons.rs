//! The in-game key icons for the chosen keys (mod "ButtonPrompts-KeyboardMouse"): the button-icon font,
//! the 4 HUD letters (BC4), the 6 ocarina notes (ASTC) and the Basic Controls screen (embedded art, guide lines drawn
//! for the keys by guides.rs).
//! Drawings in font units, y up, centred on CY.

use crate::font::{self, Cff, Sfnt};
use crate::gamefiles;
use crate::outline::{bbox, cut, fill, poly, transform, Contour, Seg, P};
use crate::raster::ink;
use crate::textures::{self, Astc};
use crate::store::{qt_config, TITLE_ID};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

pub const MOD: &str = "ButtonPrompts-KeyboardMouse";
const FONT_REL: &str = "romfs/Font2/Scalable/External.Nin_Ounce_NVN2.bfotf.zs";
const BNTX_REL: &str = "romfs/UI/Layout2Archive/Common.Product.100.Nin_Ounce_NVN2.bntx.zs";
const MAX_OTF: usize = 14560; // the original font's size: the container must keep its size
const CY: f64 = 301.0;
/// glyphs the game never draws (no message uses them): emptied only if the font would not fit otherwise
const SPARE: [u32; 13] = [0xE048, 0xE04A, 0xE066, 0xE069, 0xE083, 0xE084, 0xE067, 0xE06B, 0xE06C, 0xE06D, 0xE06E, 0xE06F, 0xE070];

type Cs = Vec<Contour>;

// ------------------------------------------------------------------------------------------------ geometry
/// Rounded rectangle, counter-clockwise; r = (bottom-left, bottom-right, top-right, top-left).
fn rrect4(x0: f64, y0: f64, x1: f64, y1: f64, r: [f64; 4]) -> Contour {
    let [bl, br, tr, tl] = r;
    let k = 0.5523;
    let mut c = vec![Seg::Move(P(x0 + bl, y0)), Seg::Line(P(x1 - br, y0))];
    c.push(if br != 0.0 { Seg::Curve(P(x1 - br + k * br, y0), P(x1, y0 + br - k * br), P(x1, y0 + br)) } else { Seg::Line(P(x1, y0)) });
    c.push(Seg::Line(P(x1, y1 - tr)));
    c.push(if tr != 0.0 { Seg::Curve(P(x1, y1 - tr + k * tr), P(x1 - tr + k * tr, y1), P(x1 - tr, y1)) } else { Seg::Line(P(x1, y1)) });
    c.push(Seg::Line(P(x0 + tl, y1)));
    c.push(if tl != 0.0 { Seg::Curve(P(x0 + tl - k * tl, y1), P(x0, y1 - tl + k * tl), P(x0, y1 - tl)) } else { Seg::Line(P(x0, y1)) });
    c.push(Seg::Line(P(x0, y0 + bl)));
    c.push(if bl != 0.0 { Seg::Curve(P(x0, y0 + bl - k * bl), P(x0 + bl - k * bl, y0), P(x0 + bl, y0)) } else { Seg::Line(P(x0, y0)) });
    c.push(Seg::Close);
    c
}

fn rrect(x0: f64, y0: f64, x1: f64, y1: f64, r: f64) -> Contour {
    rrect4(x0, y0, x1, y1, [r; 4])
}

fn tri(cx: f64, cy: f64, size: f64, dir: &str) -> Contour {
    let h = size * 0.9;
    let pts = match dir {
        "up" => [P(cx - size / 2.0, cy - h / 2.0), P(cx + size / 2.0, cy - h / 2.0), P(cx, cy + h / 2.0)],
        "down" => [P(cx - size / 2.0, cy + h / 2.0), P(cx, cy - h / 2.0), P(cx + size / 2.0, cy + h / 2.0)],
        "left" => [P(cx + h / 2.0, cy - size / 2.0), P(cx + h / 2.0, cy + size / 2.0), P(cx - h / 2.0, cy)],
        _ => [P(cx - h / 2.0, cy - size / 2.0), P(cx + h / 2.0, cy), P(cx - h / 2.0, cy + size / 2.0)],
    };
    fill(poly(&pts, false))
}

/// scale about the origin, then move
fn place(cs: &[Contour], dx: f64, dy: f64, s: f64) -> Cs {
    transform(cs, &|p| P(dx + s * p.0, dy + s * p.1))
}

fn shifted(pts: &[(f64, f64)], cx: f64) -> Vec<P> {
    pts.iter().map(|&(x, y)| P(cx + x, CY + y)).collect()
}

fn space_symbol(cx: f64) -> Cs {
    let (s, x, y0, y1) = (40.0, 190.0, CY + 36.0, CY - 40.0);
    let pts = [P(cx - x - s / 2.0, y0), P(cx - x - s / 2.0, y1 - s / 2.0), P(cx + x + s / 2.0, y1 - s / 2.0), P(cx + x + s / 2.0, y0),
               P(cx + x - s / 2.0, y0), P(cx + x - s / 2.0, y1 + s / 2.0), P(cx - x + s / 2.0, y1 + s / 2.0), P(cx - x + s / 2.0, y0)];
    vec![cut(poly(&pts, false))]
}

fn shift_symbol(cx: f64) -> Cs {
    let p = [(0.0, 118.0), (-128.0, -8.0), (-52.0, -8.0), (-52.0, -112.0), (52.0, -112.0), (52.0, -8.0), (128.0, -8.0)];
    vec![cut(poly(&shifted(&p, cx), false))]
}

fn enter_symbol(cx: f64) -> Cs {
    let p = [(150.0, 78.0), (150.0, -40.0), (-34.0, -40.0), (-34.0, -86.0), (-146.0, -20.0), (-34.0, 46.0), (-34.0, 0.0),
             (104.0, 0.0), (104.0, 78.0)];
    vec![cut(poly(&shifted(&p, cx), false))]
}

/// Mouse outline (ring, split lines, wheel); press 'L'/'R'/'W' fills that button; arrows drawn beside it.
fn mouse(cx: f64, s: f64, press: Option<char>, arrows: &[&str]) -> Cs {
    let (w, h, r, ring, lw, gap) = (380.0, 570.0, 190.0, 48.0, 30.0, 22.0);
    let (iy0, iy1, ix0, ix1) = (-h / 2.0 + ring, h / 2.0 - ring, -w / 2.0 + ring, w / 2.0 - ring);
    let ys = 30.0;
    let mut cs = vec![
        rrect(-w / 2.0, -h / 2.0, w / 2.0, h / 2.0, r),
        cut(rrect(ix0, iy0, ix1, iy1, r - ring)),
        rrect(-lw / 2.0, ys, lw / 2.0, iy1 + 2.0, 0.0),
        rrect(ix0 - 2.0, ys - lw / 2.0, ix1 + 2.0, ys + lw / 2.0, 0.0),
    ];
    let ri = r - ring - gap;
    match press {
        Some('L') => cs.push(rrect4(ix0 + gap, ys + lw / 2.0 + gap, -lw / 2.0 - gap, iy1 - gap, [0.0, 0.0, 0.0, ri])),
        Some('R') => cs.push(rrect4(lw / 2.0 + gap, ys + lw / 2.0 + gap, ix1 - gap, iy1 - gap, [0.0, 0.0, ri, 0.0])),
        _ => {}
    }
    let wheel = if press == Some('W') { 84.0 } else { 44.0 };
    cs.push(rrect(-wheel / 2.0, ys + 60.0, wheel / 2.0, ys + 60.0 + if press == Some('W') { 190.0 } else { 120.0 }, wheel / 2.0));
    let cs: Cs = cs.into_iter().enumerate().map(|(i, c)| if i != 1 { fill(c) } else { c }).collect();
    let mut out = place(&cs, cx, CY, s);
    let off = 205.0 * s + 18.0;
    for d in arrows {
        match *d {
            "left" => out.push(tri(cx - off - 28.0, CY, 74.0, "left")),
            "right" => out.push(tri(cx + off + 28.0, CY, 74.0, "right")),
            "up" => out.push(tri(cx, CY + 285.0 * s + 46.0, 86.0, "up")),
            "down" => out.push(tri(cx, CY - 285.0 * s - 46.0, 86.0, "down")),
            _ => {}
        }
    }
    out
}

fn wide_key(adv: f64, content: &dyn Fn(f64) -> Cs, w: f64, h: f64) -> Cs {
    let cx = adv / 2.0;
    let mut out = vec![rrect(cx - w / 2.0, CY - h / 2.0, cx + w / 2.0, CY + h / 2.0, 70.0)];
    out.extend(content(cx));
    out
}

fn square_key(cx: f64) -> Contour {
    rrect(cx - 280.0, CY - 280.0, cx + 280.0, CY + 280.0, 92.0)
}

// ------------------------------------------------------------------------------------------------ letters
/// The game's letter font (cap height 640 units).
pub struct Letters {
    cff: Cff,
    cmap: HashMap<u32, u16>,
}

impl Letters {
    pub fn new(otf: &[u8]) -> Result<Letters, String> {
        let s = Sfnt::parse(otf)?;
        Ok(Letters { cmap: s.cmap()?, cff: Cff::parse(s.table(b"CFF ")?)? })
    }
    fn has(&self, c: char) -> bool {
        self.cmap.contains_key(&(c as u32))
    }
    fn glyph(&self, c: char, cap: f64) -> Cs {
        let Some(&g) = self.cmap.get(&(c as u32)) else { return Vec::new() };
        let cs: Cs = self.cff.glyph(g as usize).map(|r| r.0).unwrap_or_default().into_iter().map(cut).collect();
        transform(&cs, &|p| P(p.0 * cap / 640.0, p.1 * cap / 640.0))
    }
    /// a string as cut-out contours centred on (cx, cy), one shared baseline
    fn line(&self, s: &str, cap: f64, cx: f64, cy: f64, gap: f64) -> Cs {
        let chars: Vec<Cs> = s.chars().map(|c| self.glyph(c, cap)).filter(|g| bbox(g).is_some()).collect();
        let boxes: Vec<_> = chars.iter().map(|g| bbox(g).unwrap()).collect();
        let total: f64 = boxes.iter().map(|b| b.2 - b.0).sum::<f64>() + gap * (chars.len().max(1) - 1) as f64;
        let mut x = cx - total / 2.0;
        let mut out = Vec::new();
        for (g, b) in chars.iter().zip(&boxes) {
            let (dx, dy) = (x - b.0, cy - cap / 2.0);
            out.extend(transform(g, &|p| P(p.0 + dx, p.1 + dy)));
            x += (b.2 - b.0) + gap;
        }
        out
    }
}

// ------------------------------------------------------------------------------------------------ what a key looks like
enum Kind {
    None,
    Mouse(char),
    Space,
    Shift,
    Enter,
    Arrow(&'static str),
    Text(String),
}

fn kind_of(name: &str) -> Kind {
    let n = name.trim().to_lowercase();
    match n.as_str() {
        "" | "none" => return Kind::None,
        "mouse_left" => return Kind::Mouse('L'),
        "mouse_right" => return Kind::Mouse('R'),
        "mouse_middle" => return Kind::Mouse('W'),
        "space" => return Kind::Space,
        "shift" => return Kind::Shift,
        "enter" | "return" => return Kind::Enter,
        "up" => return Kind::Arrow("up"),
        "down" => return Kind::Arrow("down"),
        "left" => return Kind::Arrow("left"),
        "right" => return Kind::Arrow("right"),
        _ => {}
    }
    let mut it = n.chars();
    if let (Some(c), None) = (it.next(), it.next()) {
        if c.is_alphanumeric() {
            return Kind::Text(c.to_uppercase().collect());
        }
    }
    const TEXT: [(&str, &str); 16] = [("tab", "TAB"), ("esc", "ESC"), ("escape", "ESC"), ("backspace", "BKSP"), ("insert", "INS"),
        ("delete", "DEL"), ("home", "HOME"), ("end", "END"), ("pageup", "PGUP"), ("pagedown", "PGDN"), ("capslock", "CAPS"),
        ("ctrl", "CTRL"), ("control", "CTRL"), ("alt", "ALT"), ("mouse_back", "M4"), ("mouse_forward", "M5")];
    if let Some((_, t)) = TEXT.iter().find(|(k, _)| *k == n) {
        return Kind::Text(t.to_string());
    }
    if n.len() > 1 && n.starts_with('f') && n[1..].chars().all(|c| c.is_ascii_digit()) {
        return Kind::Text(n.to_uppercase());
    }
    const PUNCT: [(&str, &str); 11] = [("comma", ","), ("period", "."), ("minus", "-"), ("equals", "="), ("slash", "/"),
        ("backslash", "\\"), ("semicolon", ";"), ("apostrophe", "'"), ("bracketleft", "["), ("bracketright", "]"), ("grave", "`")];
    if let Some((_, t)) = PUNCT.iter().find(|(k, _)| *k == n) {
        return Kind::Text(t.to_string());
    }
    if n.starts_with("code:") {
        return Kind::Text("?".into());
    }
    Kind::Text(n.chars().take(4).collect::<String>().to_uppercase())
}

struct Drawer<'a> {
    z: &'a Letters,
}

impl Drawer<'_> {
    /// letters the letter font cannot draw get a word instead
    fn text(&self, s: &str) -> String {
        if s.chars().all(|c| self.z.has(c)) {
            return s.into();
        }
        const WORDS: [(&str, &str); 11] = [(",", "COMMA"), (".", "DOT"), ("-", "MINUS"), ("=", "EQ"), ("/", "SLASH"), ("\\", "BSLSH"),
            (";", "SEMI"), ("'", "QUOTE"), ("[", "["), ("]", "]"), ("`", "GRAVE")];
        if let Some((_, w)) = WORDS.iter().find(|(k, _)| *k == s) {
            if w.chars().all(|c| self.z.has(c)) {
                return w.to_string();
            }
        }
        let kept: String = s.chars().filter(|&c| self.z.has(c)).collect();
        if kept.is_empty() { "?".into() } else { kept }
    }

    fn letters(&self, s: &str, cap: f64, cx: f64, cy: f64, gap: f64) -> Cs {
        self.z.line(s, cap, cx, cy, gap)
    }

    fn letters_fit(&self, s: &str, cx: f64, cy: f64, bx: f64) -> Cs {
        let cap = match s.chars().count() {
            1 => 250.0,
            2 => 200.0,
            3 => 130.0,
            _ => 110.0,
        };
        let cs = self.letters(s, cap, cx, cy, 26.0);
        let (x0, _, x1, _) = bbox(&cs).unwrap_or((0.0, 0.0, 1.0, 1.0));
        if x1 - x0 > bx {
            let k = bx / (x1 - x0);
            return transform(&cs, &|p| P(cx + k * (p.0 - cx), cy + k * (p.1 - cy)));
        }
        cs
    }

    fn arrow_cut(&self, cx: f64, dir: &str, size: f64, cy: f64) -> Cs {
        vec![cut(tri(cx, cy, size, dir))]
    }

    /// one key, as a font glyph of width adv
    fn glyph(&self, adv: f64, name: &str) -> Cs {
        let cx = adv / 2.0;
        match kind_of(name) {
            Kind::Mouse(p) => mouse(cx, 1.0, Some(p), &[]),
            Kind::Space => wide_key(adv, &space_symbol, 600.0, 300.0),
            Kind::Shift => wide_key(adv, &shift_symbol, 640.0, 330.0),
            Kind::Enter => wide_key(adv, &enter_symbol, 560.0, 330.0),
            Kind::Arrow(d) => {
                let mut out = vec![square_key(cx)];
                out.extend(self.arrow_cut(cx, d, 230.0, CY));
                out
            }
            Kind::None => vec![square_key(cx)],
            Kind::Text(v) => {
                let s = self.text(&v);
                if s.chars().count() <= 2 {
                    let mut out = vec![square_key(cx)];
                    out.extend(self.letters_fit(&s, cx, CY, 420.0));
                    out
                } else {
                    wide_key(adv, &|c| self.letters_fit(&s, c, CY, 480.0), 560.0, 330.0)
                }
            }
        }
    }

    /// a small key inside a group (WASD, rows)
    fn small(&self, name: &str, x: f64, y: f64, s: f64, cap: f64, r: f64) -> Cs {
        let mut out = vec![rrect(x - s / 2.0, y - s / 2.0, x + s / 2.0, y + s / 2.0, r)];
        match kind_of(name) {
            Kind::Arrow(d) => out.extend(self.arrow_cut(x, d, s * 0.5, y)),
            Kind::Mouse(p) => {
                let t = match p {
                    'L' => "L",
                    'R' => "R",
                    _ => "M",
                };
                out.extend(self.letters(t, 100.0, x, y, 12.0).into_iter().map(cut));
            }
            Kind::Space => out.extend(self.letters_fit("SP", x, y, s * 0.8)),
            Kind::Shift => out.extend(self.letters_fit("SH", x, y, s * 0.8)),
            Kind::Enter => out.extend(self.letters_fit("EN", x, y, s * 0.8)),
            Kind::Text(v) => {
                let t = self.text(&v);
                if t.chars().count() == 1 {
                    out.extend(self.letters(&t, cap, x, y, 12.0));
                } else {
                    out.extend(self.letters_fit(&t, x, y, s * 0.78));
                }
            }
            Kind::None => {}
        }
        out
    }

    fn wasd(&self, adv: f64, up: &str, left: &str, down: &str, right: &str) -> Cs {
        let (cx, s, g) = (adv / 2.0, 190.0, 18.0);
        let pos = [(up, cx, CY + (s + g) / 2.0), (left, cx - (s + g), CY - (s + g) / 2.0), (down, cx, CY - (s + g) / 2.0),
                   (right, cx + (s + g), CY - (s + g) / 2.0)];
        pos.iter().flat_map(|&(n, x, y)| self.small(n, x, y, s, 112.0, 38.0)).collect()
    }

    fn row(&self, adv: f64, names: &[&str]) -> Cs {
        let n = names.len() as f64;
        let s = if names.len() == 2 { 230.0 } else { 190.0 };
        let g = 22.0;
        let x0 = adv / 2.0 - (n * s + (n - 1.0) * g) / 2.0;
        let cap = if names.len() == 2 { 150.0 } else { 112.0 };
        names.iter().enumerate().flat_map(|(i, name)| self.small(name, x0 + s / 2.0 + i as f64 * (s + g), CY, s, cap, 46.0)).collect()
    }

    fn dpad(&self, adv: f64, up: &str, left: &str, right: &str, down: &str) -> Cs {
        let digits: Vec<Option<u32>> = [up, left, right, down]
            .iter()
            .map(|n| match kind_of(n) {
                Kind::Text(v) if v.chars().count() == 1 => v.chars().next().and_then(|c| c.to_digit(10)),
                _ => None,
            })
            .collect();
        if digits.iter().all(|d| d.is_some()) {
            let mut ds: Vec<u32> = digits.into_iter().flatten().collect();
            ds.sort();
            if ds.windows(2).all(|w| w[1] == w[0] + 1) {
                // four digits in a row: "1-4"
                let label = format!("{}-{}", ds[0], ds[3]);
                return wide_key(adv, &|cx| self.letters(&label, 150.0, cx, CY, 26.0), 600.0, 330.0);
            }
        }
        self.row(adv, &[up, left, right, down])
    }

    /// code point -> outline for every glyph that shows a key
    fn glyph_map(&self, k: &BTreeMap<String, String>, adv: &dyn Fn(u32) -> f64) -> Vec<(u32, Cs)> {
        let key = |n: &str| k.get(n).map(|s| s.as_str()).unwrap_or("none");
        let g = |cp: u32, n: &str| (cp, self.glyph(adv(cp), key(n)));
        let m = |cp: u32, arrows: &[&str]| (cp, mouse(adv(cp) / 2.0, 0.8, None, arrows));
        vec![
            g(0xE040, "button_a"), g(0xE041, "button_b"), g(0xE042, "button_x"), g(0xE043, "button_y"),
            g(0xE044, "button_l"), g(0xE045, "button_r"), g(0xE046, "button_zl"), g(0xE047, "button_zr"),
            (0xE049, self.dpad(adv(0xE049), key("dpad_up"), key("dpad_left"), key("dpad_right"), key("dpad_down"))),
            g(0xE04B, "button_plus"), g(0xE04C, "button_minus"),
            g(0xE050, "move_down"), g(0xE052, "move_up"), g(0xE05A, "move_left"), g(0xE058, "move_right"),
            (0xE054, self.row(adv(0xE054), &[key("move_left"), key("move_right")])),
            (0xE05C, self.row(adv(0xE05C), &[key("move_up"), key("move_down")])),
            (0xE05E, self.wasd(adv(0xE05E), key("move_up"), key("move_left"), key("move_down"), key("move_right"))),
            (0xE05F, mouse(adv(0xE05F) / 2.0, 1.0, None, &[])), // the camera stays the mouse
            g(0xE057, "button_r3"),
            m(0xE051, &["down"]), m(0xE053, &["up"]), m(0xE05B, &["left"]), m(0xE059, &["right"]),
            m(0xE055, &["left", "right"]), m(0xE05D, &["up", "down"]), m(0xE068, &["left", "right"]),
            g(0xE060, "dpad_up"), g(0xE062, "dpad_left"), g(0xE063, "dpad_right"), g(0xE061, "dpad_down"),
            (0xE064, self.row(adv(0xE064), &[key("dpad_up"), key("dpad_down")])),
            (0xE065, self.row(adv(0xE065), &[key("dpad_left"), key("dpad_right")])),
            (0xE06A, self.row(adv(0xE06A), &[key("dpad_left"), key("dpad_down"), key("dpad_right")])),
        ]
    }

    /// HUD buttons and ocarina notes: (outline, sign, width share, height share)
    fn symbol(&self, name: &str) -> (Cs, f64, f64, f64) {
        match kind_of(name) {
            Kind::Mouse(p) => (mouse(0.0, 1.0, Some(p), &[]), 1.0, 0.50, 0.74),
            Kind::Space => (space_symbol(0.0), -1.0, 0.70, 0.42),
            Kind::Shift => (shift_symbol(0.0), -1.0, 0.50, 0.52),
            Kind::Enter => (enter_symbol(0.0), -1.0, 0.58, 0.46),
            Kind::Arrow(d) => (self.arrow_cut(0.0, d, 300.0, CY), -1.0, 0.46, 0.46),
            Kind::None => {
                let cs = if self.z.has('-') { self.letters("-", 250.0, 0.0, CY, 26.0) } else { self.arrow_cut(0.0, "right", 60.0, CY) };
                (cs, -1.0, 0.30, 0.10)
            }
            Kind::Text(v) => {
                let s = self.text(&v);
                let n = s.chars().count();
                let w = match n { 1 => 0.46, 2 => 0.62, 3 => 0.74, _ => 0.80 };
                let h = match n { 1 => 0.52, 2 => 0.46, 3 => 0.40, _ => 0.34 };
                (self.letters(&s, 250.0, 0.0, CY, 26.0), -1.0, w, h)
            }
        }
    }
}

// ------------------------------------------------------------------------------------------------ the font
/// The button-icon font with the key glyphs redrawn, cut down to the glyphs the game can show (those in the
/// cmap), without hints or subroutines; it must fit the original's size (spare glyphs are emptied if needed).
fn build_font(external: &[u8], z: &Letters, keys: &BTreeMap<String, String>) -> Result<Vec<u8>, String> {
    let base = Sfnt::parse(external)?;
    let cmap = base.cmap()?;
    let cff = Cff::parse(base.table(b"CFF ")?)?;
    let hmtx = base.hmtx(cff.charstrings.len())?;
    let adv = |cp: u32| cmap.get(&cp).map_or(0.0, |&g| hmtx[g as usize].0 as f64);
    let drawn = Drawer { z }.glyph_map(keys, &adv);
    // kept glyphs, in their original order: .notdef and every glyph a code point reaches
    let mut keep: Vec<usize> = cmap.values().map(|&g| g as usize).filter(|&g| g != 0).collect();
    keep.push(0);
    keep.sort();
    keep.dedup();
    let new_id: HashMap<usize, u16> = keep.iter().enumerate().map(|(i, &g)| (g, i as u16)).collect();
    let mut originals: Vec<Cs> = Vec::new();
    for &g in &keep {
        originals.push(cff.glyph(g)?.0);
    }
    let mut last = 0;
    for blanks in 0..=SPARE.len() {
        let mut outlines = originals.clone();
        for (cp, cs) in &drawn {
            if let Some(&g) = cmap.get(cp) {
                outlines[new_id[&(g as usize)] as usize] = cs.clone();
            }
        }
        for cp in &SPARE[..blanks] {
            if let Some(&g) = cmap.get(cp) {
                outlines[new_id[&(g as usize)] as usize] = Vec::new();
            }
        }
        let mut charstrings = Vec::new();
        let mut metrics: Vec<(u16, i16)> = Vec::new();
        let mut bounds: Vec<Option<[i32; 4]>> = Vec::new();
        for (i, &g) in keep.iter().enumerate() {
            let (bytes, b) = font::encode(&outlines[i], hmtx[g].0 as f64, cff.default_width, cff.nominal_width);
            charstrings.push(bytes);
            metrics.push((hmtx[g].0, b.map_or(0, |b| b[0] as i16)));
            bounds.push(b);
        }
        let all: Vec<[i32; 4]> = bounds.iter().flatten().copied().collect();
        let bbox = [all.iter().map(|b| b[0]).min().unwrap_or(0), all.iter().map(|b| b[1]).min().unwrap_or(0),
                    all.iter().map(|b| b[2]).max().unwrap_or(0), all.iter().map(|b| b[3]).max().unwrap_or(0)];
        let mut out = Sfnt { tables: base.tables.clone() };
        out.remove(b"DSIG"); // a signature of the original font would no longer match
        out.set(b"CFF ", cff.write(&keep, &charstrings, bbox));
        let pairs: Vec<(u32, u16)> = cmap.iter().map(|(&cp, &g)| (cp, new_id[&(g as usize)])).collect();
        out.set(b"cmap", font::cmap4(&pairs)?);
        if let Ok(gdef) = base.table(b"GDEF") {
            out.set(b"GDEF", font::gdef_subset(gdef, &keep)?);
        }
        let mut head = base.table(b"head")?.to_vec();
        for (i, v) in bbox.iter().enumerate() {
            head[36 + i * 2..38 + i * 2].copy_from_slice(&(*v as i16).to_be_bytes());
        }
        out.set(b"head", head);
        let mut maxp = base.table(b"maxp")?.to_vec();
        maxp[4..6].copy_from_slice(&(keep.len() as u16).to_be_bytes());
        out.set(b"maxp", maxp);
        // advances repeat at the end: only the first ones are stored in full
        let mut nh = metrics.len();
        while nh > 1 && metrics[nh - 1].0 == metrics[nh - 2].0 {
            nh -= 1;
        }
        let mut hm = Vec::new();
        for (g, (a, l)) in metrics.iter().enumerate() {
            if g < nh {
                hm.extend_from_slice(&a.to_be_bytes());
            }
            hm.extend_from_slice(&l.to_be_bytes());
        }
        out.set(b"hmtx", hm);
        let mut hhea = base.table(b"hhea")?.to_vec();
        let (mut min_l, mut min_r, mut max_x) = (i32::MAX, i32::MAX, i32::MIN);
        for (g, b) in bounds.iter().enumerate() {
            if let Some(b) = b {
                let (a, l) = (metrics[g].0 as i32, metrics[g].1 as i32);
                min_l = min_l.min(l);
                min_r = min_r.min(a - (l + b[2] - b[0]));
                max_x = max_x.max(l + b[2] - b[0]);
            }
        }
        if max_x != i32::MIN {
            hhea[12..14].copy_from_slice(&(min_l as i16).to_be_bytes());
            hhea[14..16].copy_from_slice(&(min_r as i16).to_be_bytes());
            hhea[16..18].copy_from_slice(&(max_x as i16).to_be_bytes());
        }
        hhea[34..36].copy_from_slice(&(nh as u16).to_be_bytes());
        out.set(b"hhea", hhea);
        let bytes = out.write();
        if bytes.len() <= MAX_OTF {
            return Ok(bytes);
        }
        last = bytes.len();
    }
    Err(format!("the key icons do not fit in the game's icon font ({last} bytes, limit {MAX_OTF}), even after emptying the spare glyphs"))
}

// ------------------------------------------------------------------------------------------------ the textures
/// HUD letters (BC4) and ocarina notes (ASTC) for the keys, and the Basic Controls screen, into the archive.
fn draw_textures(b: &mut [u8], z: &Letters, keys: &BTreeMap<String, String>) -> Result<(), String> {
    let tex = textures::all_textures(b);
    textures::put_basic_controls(b, &tex)?;
    // the Basic Controls guide lines, for these keys
    let mut guides = Vec::new();
    for n in 0..5 {
        guides.push(tex.get(&format!("ControllerGuide_Pro_0{n}")).ok_or("Basic Controls guide texture not found")?);
    }
    let sizes = [0, 1, 2, 3, 4].map(|n| (guides[n].w, guides[n].h));
    for (t, img) in guides.iter().zip(crate::guides::draw(keys, sizes)) {
        if t.fmt >> 8 != 0x1d {
            return Err("unexpected Basic Controls guide texture".into());
        }
        textures::write_bc4(b, t, &img);
    }
    let d = Drawer { z };
    let key = |n: &str| keys.get(n).map(|s| s.as_str()).unwrap_or("none").to_string();
    let syms: HashMap<char, String> = [('A', "button_a"), ('B', "button_b"), ('X', "button_x"), ('Y', "button_y"), ('L', "button_l"),
                                       ('R', "button_r")].iter().map(|&(c, n)| (c, key(n))).collect();
    for k in ['A', 'B', 'X', 'Y'] {
        let t = tex.get(&format!("KeyTexFont{k}_00")).ok_or("HUD button texture not found")?;
        let (cs, sign, fw, fh) = d.symbol(&syms[&k]);
        let m = ink(&cs, sign, 64, 32.0, 32.0, 64.0 * fw * 1.15, 64.0 * fh * 1.15);
        let img: Vec<u8> = m.iter().map(|v| (v * 255.0 + 0.5).clamp(0.0, 255.0) as u8).collect();
        if t.w != 64 || t.h != 64 || t.fmt >> 8 != 0x1d {
            return Err("unexpected HUD button texture".into());
        }
        textures::write_bc4(b, t, &img);
    }
    let mut astc = Astc::new()?;
    for k in ['A', 'B', 'X', 'Y', 'L', 'R'] {
        let t = tex.get(&format!("OcarinaButton{k}_00")).ok_or("ocarina note texture not found")?;
        let orig = astc.read(b, t)?; // the note keeps the game's silhouette (alpha)
        let (w, h) = (t.w, t.h);
        let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
        for y in 0..h {
            for x in 0..w {
                if orig[(y * w + x) * 4 + 3] >= 128 {
                    (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
                }
            }
        }
        if x0 == usize::MAX {
            return Err("ocarina note texture is empty".into());
        }
        let (cx, cy) = ((x0 + x1 + 1) as f64 / 2.0, (y0 + y1 + 1) as f64 / 2.0);
        let (bw, bh) = ((x1 + 1 - x0) as f64, (y1 + 1 - y0) as f64);
        let (cs, sign, fw, fh) = d.symbol(&syms[&k]);
        let m = ink(&cs, sign, w.max(h), cx, cy, bw * fw * 1.2, bh * fh * 1.2);
        let mut px = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let v = (255.0 * (1.0 - m[y * w.max(h) + x])).clamp(0.0, 255.0) as u8;
                let i = (y * w + x) * 4;
                px[i..i + 3].fill(v);
                px[i + 3] = orig[i + 3];
            }
        }
        astc.write(b, t, &px)?;
    }
    Ok(())
}

// ------------------------------------------------------------------------------------------------ the whole job
/// The game's .nup, from Lichii's game folders and recent files.
pub fn guess_nup(root: &str) -> Option<String> {
    let text = std::fs::read(qt_config(root)).ok()?;
    let text = String::from_utf8_lossy(&text);
    let title = TITLE_ID.to_lowercase();
    let (mut dirs, mut files) = (Vec::new(), Vec::new());
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else { continue };
        let v = v.trim().trim_matches('"');
        if k.ends_with("\\path") && Path::new(v).is_dir() {
            dirs.push(v.to_string());
        }
        for part in v.split(", ") {
            let part = part.trim().trim_matches('"');
            if part.to_lowercase().ends_with(".nup") {
                files.push(part.to_string());
            }
        }
    }
    let ours = |p: &Path| p.file_name().is_some_and(|n| n.to_string_lossy().to_lowercase().contains(&title));
    if let Some(f) = files.iter().find(|f| Path::new(f).is_file() && ours(Path::new(f))) {
        return Some(f.clone());
    }
    for d in dirs {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("nup")) && ours(&p) {
                return Some(p.to_string_lossy().into_owned());
            }
        }
    }
    None
}

/// Raised whenever the drawings change, so Apply draws them again after an update.
pub const DRAWING_VERSION: u64 = 4;

/// True when both files of the key icon mod are in Lichii's load folder.
pub fn installed(root: &str) -> bool {
    let m = Path::new(root).join("load").join(TITLE_ID).join(MOD);
    m.join(FONT_REL).is_file() && m.join(BNTX_REL).is_file()
}

/// Draws the in-game key icons for `keys` into Lichii's mod folder. The first time it needs the game's .nup.
pub fn build(root: &str, nup: &str, keys: &BTreeMap<String, String>, log: &mut dyn FnMut(String)) -> Result<(), String> {
    if !gamefiles::cache_ready() {
        if nup.is_empty() || !Path::new(nup).is_file() {
            return Err("the game file (.nup) is needed once to make the in-game key icons".into());
        }
        log("First time: reading the game files from the .nup...".into());
        gamefiles::extract(Path::new(nup), log)?;
    }
    let dicts = gamefiles::dictionaries()?;
    let dict = dicts.get("zs.zsdic").ok_or("the game's dictionary is missing")?;
    let container = gamefiles::zs_decompress(&gamefiles::cached("External.bfotf.zs")?, dict)?;
    let external = gamefiles::decrypt_bfotf(&container)?;
    let z = Letters::new(&gamefiles::decrypt_bfotf(&gamefiles::zs_decompress(&gamefiles::cached("Letters.bfotf.zs")?, dict)?)?)?;

    log("Drawing the icon font...".into());
    let otf = build_font(&external, &z, keys)?;
    let bf = gamefiles::encrypt_bfotf(&otf, container.len());
    if bf.len() != container.len() {
        return Err(format!("icon font size changed ({}, expected {})", bf.len(), container.len()));
    }
    let font_bytes = gamefiles::zs_compress(&bf, dict, 16)?;

    log("Drawing the HUD buttons and ocarina notes...".into());
    let mut b = gamefiles::zs_decompress_cached("Common.bntx.zs", dict)?;
    if b.get(..4) != Some(b"BNTX") {
        return Err("unexpected texture archive".into());
    }
    draw_textures(&mut b, &z, keys)?;

    let load = Path::new(root).join("load").join(TITLE_ID);
    std::fs::create_dir_all(&load).map_err(|e| e.to_string())?;
    // only one button-icon mod can be active
    if let Ok(rd) = std::fs::read_dir(&load) {
        let mut others: Vec<_> = rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|d| d.starts_with("ButtonPrompts-") && d != MOD && load.join(d).is_dir()).collect();
        others.sort();
        for d in others {
            let backups = Path::new(root).join("mod-backups");
            let mut dest = backups.join(&d);
            let mut n = 2;
            while dest.exists() {
                dest = backups.join(format!("{d}-{n}"));
                n += 1;
            }
            std::fs::create_dir_all(&backups).map_err(|e| e.to_string())?;
            std::fs::rename(load.join(&d), &dest).map_err(|e| e.to_string())?;
            log(format!("Moved {d} to mod-backups (only one button-icon mod can be active)."));
        }
    }
    let (font_path, bntx_path) = (load.join(MOD).join(FONT_REL), load.join(MOD).join(BNTX_REL));
    for p in [&font_path, &bntx_path] {
        std::fs::create_dir_all(p.parent().unwrap()).map_err(|e| e.to_string())?;
    }
    let (font_tmp, bntx_tmp) = (font_path.with_extension("zs.tmp"), bntx_path.with_extension("zs.tmp"));
    log("Packing the textures...".into());
    std::fs::write(&font_tmp, &font_bytes).map_err(|e| e.to_string())?;
    gamefiles::zs_compress_to_file(&b, dict, 9, &bntx_tmp)?;
    drop(b);
    std::fs::rename(&font_tmp, &font_path).map_err(|e| e.to_string())?;
    std::fs::rename(&bntx_tmp, &bntx_path).map_err(|e| e.to_string())?;
    log("In-game key icons installed.".into());
    Ok(())
}

