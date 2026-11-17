//! The Basic Controls screen's labels, placed for the player's keys (guides::place) by editing the game's UI layout
//! archive (Common...blarc: a SARC of layout files, every one stored four times).
//!
//! What the game accepts, found by trying it:
//! - the archive keeps its exact size and layout (adding files or moving its data hangs the game at boot), so
//!   everything is changed in place;
//! - all four copies of the screen's layout (PaSystemController_00) are changed, or nothing shows;
//! - the game fills a label through the names of its text and icon panes, which belong to one side
//!   (T_GuideL_00 / T_IconL_00 for the left-hand buttons, ...R_00 for the right-hand ones), and some labels' text
//!   comes from the part file itself, linked by name hash. A label moved to the other side uses a part with its own
//!   side's file (names and hashes untouched) and the other side's look (icon and text positions, text alignment):
//!   the fourth copies of the two label parts are rewritten that way and renamed by pointing their names 12 bytes
//!   into the existing ones ("PaControllerKeyL_00.buil" -> "KeyL_00.buil");
//! - the groups that arranged their rows by themselves (alignment panes) become plain panes, so every row stays
//!   where it is put.
//!
//! Layout files ("BFFH" / "UIL"): at 0x110 the offsets of the string pool, the pane-name list and the pane-record
//! list; a pane record is 0x44 bytes (4-5 scale, 7-8 position relative to the parent, 10 parent, 11-12 size,
//! 14 type). A part pane's entry names its part layout (a string-pool offset) and holds the pane's number 0x40 on.

use crate::guides::{self, Placed};
use std::collections::BTreeMap;

const SCREEN: &str = "PaSystemController_00.buil";
const KEY_L: &str = "PaControllerKeyL_00";
const KEY_R: &str = "PaControllerKeyR_00";
/// The panes under N_Basic_00 sit 230 units below the screen's rows (guides' screen y).
const BASIC_Y: f32 = 230.0;
/// Row columns: the row's centre, and its label part inside it.
const ROW_X: f32 = 580.0;
const PART_X: f32 = 268.0;
/// The text of a second line ("Dash (Hold)", "Reset Camera (Press)") inside its row.
const SECOND_X: f32 = 28.0;
/// A text box's settings start with this (a font id, then its 0.8 scale); its two alignment values (0 left, 2 right)
/// are 16 and 24 bytes after the start, with a 1 between them. The screen has two such text boxes: "Dash (Hold)" then
/// "Reset Camera (Press)".
const TEXT_BOX: [u8; 8] = [0x26, 0x4a, 0x00, 0x00, 0xcd, 0xcc, 0x4c, 0x3f];
const SECOND_LINES: [&str; 2] = ["T_GuideBtnA_00", "T_GuideBtnRStick_00"];
/// Pane types: a plain pane, an alignment pane, and text anchored on the left / right.
const NULL_PANE: u32 = 0x0255_501b;
const ALIGN_PANE: u32 = 0x0255_5189;
const TEXT_LEFT: u32 = 0x0255_4919;
const TEXT_RIGHT: u32 = 0x0255_5919;

/// Each label: its row pane, its label part (None for a text-only line) or text pane, and whether its button is
/// one of the left-hand ones.
const ROWS: [(&str, &str, &str, bool); 18] = [
    ("button_zl", "N_ZL_00", "Pa_BtnZL_00", true),
    ("button_l", "N_L_00", "Pa_BtnL_00", true),
    ("button_minus", "N_Minus_00", "Pa_BtnMinus_00", true),
    ("move", "N_LStick_00", "Pa_BtnLStick_00", true),
    ("dpad_up", "N_CrossKeyUp_00", "Pa_BtnUp_00", true),
    ("dpad_left", "N_CrossKeyLeft_00", "Pa_BtnLeft_00", true),
    ("dpad_right", "N_CrossKeyRight_00", "Pa_BtnRight_00", true),
    ("dpad_down", "N_CrossKeyDown_00", "Pa_BtnDown_00", true),
    ("button_zr", "N_ZR_00", "Pa_BtnZR_00", false),
    ("button_r", "N_R_00", "Pa_BtnR_00", false),
    ("button_plus", "N_Plus_00", "Pa_BtnPlus_00", false),
    ("button_a", "N_BtnA_00", "Pa_BtnA_00", false),
    ("dash", "N_BtnA_01", "T_GuideBtnA_00", false),
    ("button_b", "N_BtnB_00", "Pa_BtnB_00", false),
    ("button_x", "N_BtnX_00", "Pa_BtnX_00", false),
    ("button_y", "N_BtnY_00", "Pa_BtnY_00", false),
    ("camera", "N_BtnRStick_00", "Pa_BtnRStick_00", false),
    ("button_r3", "N_BtnRStick_01", "T_GuideBtnRStick_00", false),
];

/// Group panes that held rows: they move to (0, 0) so every row's position counts from N_Basic_00.
const GROUPS: [&str; 7] = ["N_ABXY_00", "A_ABXY_00", "A_ABXY_01", "N_CrossKey_00", "A_CrossKey_00", "N_RStick_00", "A_RStick_00"];

/// The dark band panes behind the labels. Each label gets a band of its own: a pane covers a run of up to four
/// labels with a texture showing that many separate bands (`band_picture`), or Actions with its "Dash" line (one band
/// stretched over both); panes left over are hidden. A pane's material picks its texture by name.
const BANDS: [&str; 11] = ["P_LineZL_01", "P_LineRStick_01", "P_LineABXY_02", "P_LineABXY_01", "P_LineCrossKey_01", "P_LineLStick_01",
                           "P_LinePlus_01", "P_LineMinus_01", "P_LineR_01", "P_LineL_01", "P_LineZR_01"];
/// The band textures (only this screen uses them): redrawn with one, two, three and four bands.
pub const BAND_TEXTURES: [&str; 4] = ["SystemGuideBase_00", "SystemGuideBase_01", "SystemGuideBase_02", "SystemGuideBase_03"];
/// A band's x (its picture is mirrored on the right).
const BAND_X: f32 = 568.0;

/// The height of one label's band (its texture's full height, edges included): the game's 90, less when the rows
/// are closer, so neighbouring bands stay apart.
pub fn band_height(pitch: f64) -> f64 {
    90f64.min(pitch - 10.0)
}

/// A band pane: its side, the rows (screen y) of its first and last label, and its texture (BAND_TEXTURES).
pub struct Band {
    pub left: bool,
    pub top: f64,
    pub bottom: f64,
    pub texture: usize,
}

/// The band panes for the placed labels: each corner's labels in runs of up to four evenly spaced rows; Actions
/// alone, its band reaching down to its "Dash" line.
pub fn bands(placed: &[Placed]) -> Vec<Band> {
    let mut out = Vec::new();
    for (left, top) in [(true, true), (false, true), (true, false), (false, false)] {
        let mut rows: Vec<&Placed> = placed.iter().filter(|p| p.left == left && p.top == top && !p.second).collect();
        rows.sort_by(|a, b| b.y.total_cmp(&a.y));
        let mut run: Vec<f64> = Vec::new();
        let flush = |run: &mut Vec<f64>, out: &mut Vec<Band>| {
            if let (Some(&top), Some(&bottom)) = (run.first(), run.last()) {
                out.push(Band { left, top, bottom, texture: run.len() - 1 });
            }
            run.clear();
        };
        for p in rows {
            if p.what == "button_a" {
                flush(&mut run, &mut out);
                let dash = placed.iter().find(|q| q.what == "button_a" && q.second).map_or(p.y, |q| q.y);
                out.push(Band { left, top: p.y, bottom: dash, texture: 0 });
                continue;
            }
            if run.len() == BAND_TEXTURES.len() || run.last().is_some_and(|&last| (last - p.y - p.pitch).abs() > 1.0) {
                flush(&mut run, &mut out);
            }
            run.push(p.y);
        }
        flush(&mut run, &mut out);
    }
    out
}

/// A band texture redrawn with `count` separate bands, one per row `pitch` apart, each the game's one-band picture
/// (`one`, `one_w` x `one_h`) at a label's band height; the pane stretches the texture over its rows.
pub fn band_picture(one: &[u8], one_w: usize, one_h: usize, w: usize, h: usize, count: usize, pitch: f64) -> Vec<u8> {
    let band = band_height(pitch);
    let total = (count - 1) as f64 * pitch + band;
    let mut img = vec![0u8; w * h];
    for y in 0..h {
        // the place in the pane (screen units from its top), the nearest band, and the row of the one-band picture
        let u = (y as f64 + 0.5) / h as f64 * total;
        let j = ((u - band / 2.0) / pitch).round().clamp(0.0, (count - 1) as f64);
        let s = ((u - band / 2.0 - j * pitch) / band + 0.5) * one_h as f64 - 0.5;
        if s <= -1.0 || s >= one_h as f64 {
            continue;
        }
        let (s0, t) = (s.floor(), s - s.floor());
        let row = |r: f64| -> Option<usize> { (r >= 0.0 && r < one_h as f64).then_some(r as usize) };
        for x in 0..w {
            let sx = (x * one_w / w).min(one_w - 1);
            let at = |r: Option<usize>| r.map_or(0.0, |r| one[r * one_w + sx] as f64);
            img[y * w + x] = (at(row(s0)) * (1.0 - t) + at(row(s0 + 1.0)) * t).round() as u8;
        }
    }
    img
}

/// How the label parts show their text, for "Reset Camera" (a text of its own, made to look like a label): the
/// text's x in the left-hand and right-hand parts, its y, its colour (its material's) and its other colours (`LOOK`).
struct LabelText {
    x: [f32; 2],
    y: f32,
    colour: Colour,
    look: [Colour; 3],
}

type Colour = [f32; 4];

/// A text's other colours (its shadow's, found by comparing labels with the screen's second lines): colour numbers
/// this many bytes before its text box settings (TEXT_BOX), whose text record starts 0x40 before them. The font
/// size (x, y) is 0x58 before them.
const LOOK: [usize; 3] = [0x28, 0x0c, 0x08];
const TEXT_RECORD: [u8; 4] = [0x26, 0x20, 0x00, 0x00];
const FONT_SIZE: usize = 0x58;
/// The labels' font size: 44 in the file, which their part's own per-language table makes 49.92 for most languages
/// (measured on the game's screen); the screen's texts have no such table.
const LABEL_SIZE: f32 = 49.92;
const SECOND_SIZE: f32 = 44.0;

/// A layout's colours: every material and text names its colours by number in this table, which starts 0x28 bytes
/// after the "Locater" name (number 0 is transparent; the table ends where the entries stop being opaque colours).
fn colours(f: &[u8]) -> Result<Vec<Colour>, String> {
    let at = f.windows(8).position(|w| w == b"Locater ").ok_or("unexpected layout colours")? + 0x28;
    let mut out = Vec::new();
    for o in (at..f.len().saturating_sub(16)).step_by(16) {
        let c: Colour = std::array::from_fn(|i| f32_at(f, o + 4 * i));
        if !out.is_empty() && (c[3] != 1.0 || c.iter().any(|v| !(0.0..=1.0).contains(v))) {
            break;
        }
        out.push(c);
    }
    if out.len() < 2 || out[0] != [0.0; 4] {
        return Err("unexpected layout colours".into());
    }
    Ok(out)
}

/// The number of the colour in `table` nearest `c` (never the transparent one).
fn nearest(table: &[Colour], c: Colour) -> u32 {
    let d = |t: &Colour| t.iter().zip(c).map(|(a, b)| (a - b) * (a - b)).sum::<f32>();
    (1..table.len()).min_by(|&a, &b| d(&table[a]).total_cmp(&d(&table[b]))).unwrap_or(1) as u32
}

/// A layout's material record for the material number.
fn material(f: &[u8], m: usize) -> Result<usize, String> {
    let r = u64_at(f, u64_at(f, 0x140) + m * 8);
    if r + 0x2c > f.len() || u32_at(f, r + 0x14) != m as u32 {
        return Err("unexpected layout material".into());
    }
    Ok(r)
}
/// Where a material record names its colour.
const MATERIAL_COLOUR: usize = 0x28;

/// The one text box in a label part, or the k-th of the screen's.
fn text_boxes(f: &[u8]) -> Vec<usize> {
    f.windows(TEXT_BOX.len())
        .enumerate()
        .filter(|(o, w)| *w == TEXT_BOX && *o >= 0x40 && f[o - 0x40..o - 0x3c] == TEXT_RECORD)
        .map(|(o, _)| o)
        .collect()
}

fn label_text(left_part: &[u8], right_part: &[u8]) -> Result<LabelText, String> {
    let bad = || "unexpected label part layout".to_string();
    let pos = |f: &[u8], pane: &str| -> Result<(f32, f32), String> {
        let r = layout(f)?.rec(pane)?;
        Ok((f32_at(f, r + 4 * 7), f32_at(f, r + 4 * 8)))
    };
    let ((xl, yl), (xr, yr)) = (pos(left_part, "T_GuideL_00")?, pos(right_part, "T_GuideR_00")?);
    let [tb] = text_boxes(right_part)[..] else { return Err(bad()) };
    if yl != yr {
        return Err(bad());
    }
    let table = colours(right_part)?;
    let colour = |n: u32| table.get(n as usize).copied().ok_or_else(bad);
    let r = layout(right_part)?.rec("T_GuideR_00")?;
    let m = material(right_part, (u32_at(right_part, r + 4 * 15) & 0xFFFF) as usize)?;
    Ok(LabelText {
        x: [xl, xr],
        y: yr,
        colour: colour(u32_at(right_part, m + MATERIAL_COLOUR))?,
        look: [colour(u32_at(right_part, tb - LOOK[0]))?, colour(u32_at(right_part, tb - LOOK[1]))?, colour(u32_at(right_part, tb - LOOK[2]))?],
    })
}

fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn u64_at(b: &[u8], o: usize) -> usize {
    u64::from_le_bytes(b[o..o + 8].try_into().unwrap()) as usize
}
fn f32_at(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn put_u32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn put_f32(b: &mut [u8], o: usize, v: f32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn c_str(b: &[u8], o: usize) -> &[u8] {
    let end = b[o..].iter().position(|&c| c == 0).map_or(b.len(), |p| o + p);
    &b[o..end]
}

// ---------------------------------------------------------------- the archive
struct Node {
    hash: u32,
    attr: u32,
    start: u32,
    end: u32,
    name: String,
}

struct Sarc {
    sfat: usize,
    data: usize,
    names: usize,
    mult: u32,
    nodes: Vec<Node>,
}

fn sarc(b: &[u8]) -> Result<Sarc, String> {
    if b.get(..4) != Some(b"SARC") {
        return Err("unexpected layout archive".into());
    }
    let hdr = u16::from_le_bytes([b[4], b[5]]) as usize;
    let data = u32_at(b, 0xC) as usize;
    if b.get(hdr..hdr + 4) != Some(b"SFAT") {
        return Err("unexpected layout archive".into());
    }
    let count = u16::from_le_bytes([b[hdr + 6], b[hdr + 7]]) as usize;
    let mult = u32_at(b, hdr + 8);
    let names = hdr + 0xC + count * 16 + 8;
    let nodes = (0..count)
        .map(|i| {
            let o = hdr + 0xC + i * 16;
            let attr = u32_at(b, o + 4);
            let name = String::from_utf8_lossy(c_str(b, names + (attr & 0xFFFF) as usize * 4)).into_owned();
            Node { hash: u32_at(b, o), attr, start: u32_at(b, o + 8), end: u32_at(b, o + 12), name }
        })
        .collect();
    Ok(Sarc { sfat: hdr, data, names, mult, nodes })
}

fn name_hash(name: &str, mult: u32) -> u32 {
    name.bytes().fold(0u32, |h, c| h.wrapping_mul(mult).wrapping_add(c as u32))
}

impl Sarc {
    /// Byte ranges (in the archive) of every copy of a file.
    fn copies(&self, name: &str) -> Vec<(usize, usize)> {
        self.nodes
            .iter()
            .filter(|n| n.name == name)
            .map(|n| (self.data + n.start as usize, self.data + n.end as usize))
            .collect()
    }

    /// Writes the file table back (same number of entries, sorted by name hash as the game looks them up).
    fn write_table(&mut self, b: &mut [u8]) {
        self.nodes.sort_by_key(|n| n.hash);
        for (i, n) in self.nodes.iter().enumerate() {
            let o = self.sfat + 0xC + i * 16;
            put_u32(b, o, n.hash);
            put_u32(b, o + 4, n.attr);
            put_u32(b, o + 8, n.start);
            put_u32(b, o + 12, n.end);
        }
    }
}

// ---------------------------------------------------------------- a layout file
struct Layout {
    pool: usize,
    /// pane name -> record offset (in the file)
    panes: BTreeMap<String, usize>,
    /// pane name -> its number
    index: BTreeMap<String, u32>,
}

fn layout(f: &[u8]) -> Result<Layout, String> {
    if f.get(..4) != Some(b"BFFH") || f.get(0xF8..0xFB) != Some(b"UIL") {
        return Err("unexpected Basic Controls layout".into());
    }
    let (pool, names, records) = (u64_at(f, 0x110), u64_at(f, 0x118), u64_at(f, 0x120));
    if names <= records || (names - records) % 8 != 0 {
        return Err("unexpected Basic Controls layout".into());
    }
    let count = (names - records) / 8;
    let mut panes = BTreeMap::new();
    let mut index = BTreeMap::new();
    for i in 0..count {
        let name = String::from_utf8_lossy(c_str(f, u64_at(f, names + i * 8))).into_owned();
        panes.insert(name.clone(), u64_at(f, records + i * 8));
        index.insert(name, i as u32);
    }
    Ok(Layout { pool, panes, index })
}

impl Layout {
    fn rec(&self, name: &str) -> Result<usize, String> {
        self.panes.get(name).copied().ok_or(format!("the Basic Controls layout has no {name}"))
    }

    /// Offset of a string in the pool.
    fn string(&self, f: &[u8], s: &str) -> Result<usize, String> {
        let mut want = s.as_bytes().to_vec();
        want.push(0);
        f[self.pool..]
            .windows(want.len())
            .position(|w| w == want.as_slice())
            .map(|p| self.pool + p)
            .ok_or(format!("the Basic Controls layout has no {s}"))
    }

    /// The part entry's pointer to its part layout, for the part pane `name`.
    fn part_pointer(&self, f: &[u8], name: &str, layouts: [usize; 2]) -> Result<usize, String> {
        let pane = *self.index.get(name).ok_or(format!("the Basic Controls layout has no {name}"))?;
        (0..f.len().saturating_sub(0x48))
            .step_by(8)
            .find(|&o| layouts.contains(&u64_at(f, o)) && u32_at(f, o + 0x40) == pane)
            .ok_or(format!("the Basic Controls layout has no part entry for {name}"))
    }
}

/// The pane rows, sides and bands for the placed labels, written into one copy of the screen's layout.
fn place_screen(f: &mut [u8], placed: &[Placed], label: &LabelText) -> Result<(), String> {
    let l = layout(f)?;
    let (key_l, key_r) = (l.string(f, KEY_L)?, l.string(f, KEY_R)?);
    // the side-swap parts: "KeyL_00" / "KeyR_00", 12 bytes into the part names
    let (swap_l, swap_r) = (key_l + 12, key_r + 12);
    let field = |rec: usize, i: usize| rec + 4 * i;
    // the layout this was made for: Target's row at its original place
    let zl = l.rec("N_ZL_00")?;
    if f32_at(f, field(zl, 7)) != -ROW_X || f32_at(f, field(zl, 8)) != 210.0 {
        return Err("the Basic Controls layout is not the version this app knows".into());
    }
    for g in GROUPS {
        let r = l.rec(g)?;
        put_f32(f, field(r, 7), 0.0);
        put_f32(f, field(r, 8), 0.0);
        if u32_at(f, field(r, 14)) == ALIGN_PANE {
            put_u32(f, field(r, 14), NULL_PANE);
        }
    }
    for p in placed {
        let what = if p.what == "button_a" && p.second { "dash" } else { p.what };
        let Some(&(_, row, inner, left_button)) = ROWS.iter().find(|r| r.0 == what) else { continue };
        let side = if p.left { -1.0 } else { 1.0 };
        let r = l.rec(row)?;
        put_f32(f, field(r, 7), side * ROW_X);
        put_f32(f, field(r, 8), p.y as f32 - BASIC_Y);
        let i = l.rec(inner)?;
        if inner.starts_with("Pa_") {
            // a label: its part on the row's outer side, with the look of the side it is on
            put_f32(f, field(i, 7), side * PART_X);
            let ptr = l.part_pointer(f, inner, [key_l, key_r])?;
            let target = match (left_button, p.left) {
                (true, true) => key_l,
                (false, false) => key_r,
                (true, false) => swap_l, // left-hand names, right-hand look
                (false, true) => swap_r, // right-hand names, left-hand look
            };
            put_u32(f, ptr, target as u32);
        } else {
            // text anchored and aligned toward the picture: a second line close to its label's column, "Reset Camera"
            // where a label's text starts and with a label's look (its key icon is drawn on a line sheet, guides.rs)
            let reset = what == "button_r3";
            put_f32(f, field(i, 7), if reset { side * PART_X + label.x[usize::from(!p.left)] } else { side * SECOND_X });
            if reset {
                put_f32(f, field(i, 8), label.y);
            }
            put_u32(f, field(i, 14), if p.left { TEXT_RIGHT } else { TEXT_LEFT });
            let boxes = text_boxes(f);
            let k = SECOND_LINES.iter().position(|n| *n == inner).ok_or("unexpected Basic Controls text")?;
            let tb = *boxes.get(k).filter(|_| boxes.len() == SECOND_LINES.len()).ok_or("unexpected Basic Controls text boxes")?;
            if u32_at(f, tb + 20) != 1 || u32_at(f, tb + 16) > 2 || u32_at(f, tb + 24) > 2 {
                return Err("unexpected Basic Controls text box".into());
            }
            let align = if p.left { 2 } else { 0 };
            put_u32(f, tb + 16, align);
            put_u32(f, tb + 24, align);
            if reset {
                // the label's colours (the nearest the screen has) and size
                let table = colours(f)?;
                for (d, c) in LOOK.iter().zip(label.look) {
                    put_u32(f, tb - d, nearest(&table, c));
                }
                let m = material(f, (u32_at(f, field(i, 15)) & 0xFFFF) as usize)?;
                put_u32(f, m + MATERIAL_COLOUR, nearest(&table, label.colour));
                if f32_at(f, tb - FONT_SIZE) != SECOND_SIZE || f32_at(f, tb - FONT_SIZE + 4) != SECOND_SIZE {
                    return Err("unexpected Basic Controls text size".into());
                }
                put_f32(f, tb - FONT_SIZE, LABEL_SIZE);
                put_f32(f, tb - FONT_SIZE + 4, LABEL_SIZE);
            }
        }
    }
    // bands: a pane per run of labels (its material pointed at the texture with that many bands); the rest hidden
    let bands = bands(placed);
    if bands.len() > BANDS.len() {
        return Err("too many Basic Controls label bands".into());
    }
    let names = BAND_TEXTURES.map(|t| l.string(f, t));
    let names: Vec<usize> = names.into_iter().collect::<Result<_, _>>()?;
    let refs: Vec<usize> = (0..l.pool).step_by(8).filter(|&o| names.contains(&u64_at(f, o))).collect();
    let mats: Vec<usize> = BANDS.iter().map(|b| l.rec(b).map(|r| (u32_at(f, field(r, 15)) & 0xFFFF) as usize)).collect::<Result<_, _>>()?;
    let first = mats.iter().copied().min().unwrap_or(0);
    let mut sorted = mats.clone();
    sorted.sort();
    if refs.len() != BANDS.len() || sorted != (first..first + BANDS.len()).collect::<Vec<_>>() {
        return Err("unexpected Basic Controls band materials".into());
    }
    let height = band_height(placed.first().map_or(100.0, |p| p.pitch)) as f32;
    for (k, pane) in BANDS.iter().enumerate() {
        let r = l.rec(pane)?;
        let Some(b) = bands.get(k) else {
            put_f32(f, field(r, 4), 0.0);
            put_f32(f, field(r, 5), 0.0);
            continue;
        };
        let (top, bottom) = (b.top as f32, b.bottom as f32);
        put_f32(f, field(r, 4), if b.left { 1.0 } else { -1.0 });
        put_f32(f, field(r, 5), 1.0);
        put_f32(f, field(r, 7), if b.left { -BAND_X } else { BAND_X });
        put_f32(f, field(r, 8), (top + bottom) / 2.0);
        f[refs[mats[k] - first]..refs[mats[k] - first] + 8].copy_from_slice(&(names[b.texture] as u64).to_le_bytes());
        put_f32(f, field(r, 12), top - bottom + height);
    }
    Ok(())
}

/// `own` (a label part's file) with the look of `other` (the other side's part): every byte where they differ is taken
/// from `other`, except names (the string pool) and name hashes (the hash table and the links that use them).
fn other_look(own: &[u8], other: &[u8]) -> Result<Vec<u8>, String> {
    if own.len() != other.len() || own.get(..4) != Some(b"BFFH") {
        return Err("unexpected label part layout".into());
    }
    let hashes = |f: &[u8]| -> Vec<usize> {
        let (names, records, table) = (u64_at(f, 0x118), u64_at(f, 0x120), u64_at(f, 0x128));
        (0..names.saturating_sub(records) / 8).map(|i| u64_at(f, table + i * 8)).collect()
    };
    let (mut known, pool) = (hashes(own), u64_at(own, 0x110));
    known.extend(hashes(other));
    let mut out = own.to_vec();
    for i in 0..own.len() {
        let word = i & !7;
        if own[i] != other[i] && i < pool && word + 8 <= own.len() && !known.contains(&u64_at(own, word)) {
            out[i] = other[i];
        }
    }
    Ok(out)
}

/// The layout archive with the Basic Controls labels placed for `keys`: same size and layout as the game's own.
pub fn build(archive: &[u8], keys: &BTreeMap<String, String>) -> Result<Vec<u8>, String> {
    let mut b = archive.to_vec();
    let mut arc = sarc(&b)?;
    let placed = guides::place(keys);
    let screens = arc.copies(SCREEN);
    if screens.len() != 4 {
        return Err("unexpected layout archive (Basic Controls screen copies)".into());
    }
    let file_l = format!("{KEY_L}.buil");
    let file_r = format!("{KEY_R}.buil");
    let last = |arc: &Sarc, name: &str| arc.nodes.iter().rposition(|n| n.name == name);
    let (Some(il), Some(ir)) = (last(&arc, &file_l), last(&arc, &file_r)) else {
        return Err("unexpected layout archive (label parts)".into());
    };
    let range = |i: usize| (arc.data + arc.nodes[i].start as usize, arc.data + arc.nodes[i].end as usize);
    let ((ls, le), (rs, re)) = (range(il), range(ir));
    let (key_l, key_r) = (b[ls..le].to_vec(), b[rs..re].to_vec());
    let label = label_text(&key_l, &key_r)?;
    for (s, e) in screens {
        place_screen(&mut b[s..e], &placed, &label)?;
    }
    // the side-swap parts: the fourth copy of each label part holds the other side's file with this side's look
    // "KeyL_00": the left part's file with the right look, in the right part's slot; "KeyR_00" the other way
    let swap = [(ir, other_look(&key_l, &key_r)?, il), (il, other_look(&key_r, &key_l)?, ir)];
    let mut renamed = Vec::new();
    for (copy, contents, name_from) in swap {
        let (s, e) = range(copy);
        if e - s != contents.len() {
            return Err("unexpected label part layout".into());
        }
        b[s..e].copy_from_slice(&contents);
        let src = &arc.nodes[name_from];
        let new_name = src.name[12..].to_string();
        let attr = src.attr + 3; // the name 12 bytes (3 table units) further in
        let at = arc.names + (attr & 0xFFFF) as usize * 4;
        if c_str(&b, at) != new_name.as_bytes() {
            return Err("unexpected layout archive names".into());
        }
        renamed.push((copy, new_name, attr));
    }
    for (copy, name, attr) in renamed {
        let n = &mut arc.nodes[copy];
        n.hash = name_hash(&name, arc.mult);
        n.attr = attr;
        n.name = name;
    }
    arc.write_table(&mut b);
    debug_assert_eq!(b.len(), archive.len());
    Ok(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_matches_sarc() {
        // the archive's own hash of a known name (multiplier 0x65)
        assert_eq!(name_hash("a", 0x65), 0x61);
        assert_eq!(name_hash("ab", 0x65), 0x61 * 0x65 + 0x62);
    }
}
