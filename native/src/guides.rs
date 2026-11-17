//! The guide lines of the "Basic Controls" screen, drawn for the player's own keys.
//!
//! The game draws the keyboard-and-mouse picture (Controller_Pro / Controller_Pro1) and five line sheets
//! (ControllerGuide_Pro_00..04, one channel) on top of each other, with its text labels in two columns. Each label
//! shows the icon of the key it names (the key icon font), so each line must end on that same key.
//!
//! The labels are placed for the player's keys (`place`, written into the screen's layout by layout.rs): the four
//! corners of the screen take turns, each taking the label whose key is nearest its corner of the keyboard-and-mouse
//! picture, so most lines run straight from a label to a nearby key. The lines are orthogonal like the game's own:
//! the router tries many paths for every line and keeps the set with the fewest crossings, overlaps and lines
//! passing over other keys.

use std::collections::BTreeMap;

const LW: f64 = 6.0; // line width
const DOT: f64 = 9.0; // end dot radius
const TICK: f64 = 14.0; // half height of the tick on the text side
const LX: f64 = 144.0; // label columns (sheet pixels)
const RX: f64 = 1410.0;
/// "Reset Camera" has no label part of its own (only its text), so its key icon is drawn on a line sheet where a
/// label's icon sits: this far out from the label column (sheet pixels), this far above the row (screen units) and
/// this big (sheet pixels per font unit), all measured against the labels' own icons on the game's screen.
const ICON_DX: f64 = 72.0;
const ICON_DY: f64 = 0.0;
pub const ICON_SCALE: f64 = 0.12;

/// Where the "Reset Camera" key icon goes: the sheet it is drawn on and its centre (sheet pixels).
pub fn reset_icon(keys: &BTreeMap<String, String>) -> Option<(usize, f64, f64)> {
    let p = place(keys).into_iter().find(|p| p.what == "button_r3")?;
    let sheet = LABELS.iter().find(|l| l.0 == p.what).map_or(0, |l| l.1);
    Some((sheet, if p.left { LX - ICON_DX } else { RX + ICON_DX }, sheet_y(p.y + ICON_DY)))
}

/// The screen's labels: what each names (a button, the move keys, the camera, or pressing the camera stick: "Reset
/// Camera", a label of its own) and the line sheet its line is drawn on (the game shows all five sheets; the split
/// only keeps each texture's lines apart).
pub const LABELS: [(&str, usize); 17] = [
    ("button_zl", 0), ("button_l", 4), ("button_minus", 1), ("move", 0),
    ("dpad_up", 3), ("dpad_left", 3), ("dpad_right", 3), ("dpad_down", 3),
    ("button_zr", 2), ("button_r", 4), ("button_plus", 0), ("button_a", 0),
    ("button_b", 0), ("button_x", 0), ("button_y", 0), ("camera", 0), ("button_r3", 0),
];
const N: usize = LABELS.len();

/// Label rows in screen units (the layout's y, up is positive): the top row, the lowest row there is room for, the
/// usual spacing, and the gap between a column's top and bottom halves.
const TOP: f64 = 440.0;
const BOTTOM: f64 = -442.0;
const PITCH: f64 = 100.0;
const GAP: f64 = 40.0;
/// "Dash (Hold)" sits this part of a row under Actions: it describes Actions, so it stays close to it (it still
/// takes a full row's room, like on the game's own screen, so the rows around it keep their places).
const SECOND: f64 = 0.6;

/// Screen y -> line-sheet y (measured: rows 100 units apart are 113 sheet pixels apart).
pub fn sheet_y(y: f64) -> f64 {
    833.2 - 1.13 * y
}

/// A placed label: what it names, its side, its row (screen units), whether it is in a top corner, and whether it
/// is the second line under Actions ("Dash (Hold)", the same key held); `pitch` is the rows' spacing.
#[derive(Clone, Copy, Debug)]
pub struct Placed {
    pub what: &'static str,
    pub left: bool,
    pub y: f64,
    pub top: bool,
    pub second: bool,
    pub pitch: f64,
}

/// Where a label's line goes: its key (the W/A/S/D box for Move, the mouse body for the camera).
fn anchor(what: &str, key: &dyn Fn(&str) -> String) -> Option<Pt> {
    match what {
        "move" => {
            let names: Vec<String> = ["move_up", "move_down", "move_left", "move_right"].iter().map(|n| key(n)).collect();
            let refs: Vec<&str> = names.iter().map(String::as_str).collect();
            group_goal(&refs).map(|g| g.bounds.center())
        }
        // moving the whole mouse: it counts from the middle of the mouse (its line still ends on the body)
        "camera" => Some(ms(135.0, (81.0 + 390.0) / 2.0)),
        _ => goal_for(&key(what)).map(|g| g.bounds.center()),
    }
}

/// Places the labels for these keys. The corners take turns (top-left, top-right, bottom-left, bottom-right), each
/// taking the label whose key is nearest its corner of the picture. Within a corner the labels are ordered so their
/// lines cross least: every order is tried with simple lines (across from the label then up or down into the key, or
/// across, up or down beside the key, then into it from its side; each line takes the shape that crosses least) and
/// the order with the fewest crossings and lines on top of each other, then the shortest lines over the least of the
/// art, wins. Labels whose key is not on the picture fill the places left. Both columns use the same spacing.
pub fn place(keys: &BTreeMap<String, String>) -> Vec<Placed> {
    let key = |n: &str| keys.get(n).map(String::as_str).unwrap_or("none").to_string();
    let [kbd, mouse] = art();
    let (x0, y0, x1, y1) = (kbd.x0.min(mouse.x0), kbd.y0.min(mouse.y0), kbd.x1.max(mouse.x1), kbd.y1.max(mouse.y1));
    // where each label's line goes, in 0..1 across and down the picture
    let ends: Vec<Option<Pt>> = LABELS.iter().map(|(w, _)| anchor(w, &key)).collect();
    // the key's own box, for lines reaching it from the side (the camera: a small box on the body)
    let bounds: Vec<Option<Rect>> = LABELS
        .iter()
        .map(|(w, _)| match *w {
            "move" => {
                let names: Vec<String> = ["move_up", "move_down", "move_left", "move_right"].iter().map(|n| key(n)).collect();
                let refs: Vec<&str> = names.iter().map(String::as_str).collect();
                group_goal(&refs).map(|g| g.bounds)
            }
            "camera" => goal_for("camera").map(|g| g.bounds),
            w => goal_for(&key(w)).map(|g| g.bounds),
        })
        .collect();
    let at: Vec<Option<Pt>> = ends.iter().map(|e| e.map(|p| ((p.0 - x0) / (x1 - x0), (p.1 - y0) / (y1 - y0)))).collect();
    const CORNERS: [(f64, f64); 4] = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];
    let mut corner_of = [usize::MAX; N];
    for turn in 0..N {
        let c = turn % 4;
        let (cx, cy) = CORNERS[c];
        let pick = (0..N)
            .filter(|&i| corner_of[i] == usize::MAX)
            .min_by(|&a, &b| {
                let d = |i: usize| at[i].map_or(f64::MAX, |p| (p.0 - cx).powi(2) + (p.1 - cy).powi(2));
                d(a).total_cmp(&d(b)).then(a.cmp(&b))
            })
            .unwrap();
        corner_of[pick] = c;
    }
    // the rows of a column for given orders of its top and bottom corner (Actions takes a second row for "Dash"):
    // the top corner from the top down, the bottom corner ending at the bottom, any room left between them
    let rows = |top: &[usize], bottom: &[usize], pitch: f64| -> Vec<(usize, bool, f64)> {
        let expand = |list: &[usize]| -> Vec<(usize, bool)> {
            list.iter().flat_map(|&i| if LABELS[i].0 == "button_a" { vec![(i, false), (i, true)] } else { vec![(i, false)] }).collect()
        };
        // each row's distance from its corner's first row: a full row each, a second line drawn closer to its label
        let offsets = |list: &[(usize, bool)]| -> Vec<f64> {
            list.iter().enumerate().map(|(r, &(_, second))| r as f64 * pitch - if second { (1.0 - SECOND) * pitch } else { 0.0 }).collect()
        };
        let (t, b) = (expand(top), expand(bottom));
        let (to, bo) = (offsets(&t), offsets(&b));
        let mut out: Vec<(usize, bool, f64)> = t.iter().zip(&to).map(|(&(i, s), o)| (i, s, TOP - o)).collect();
        let height = b.len().saturating_sub(1) as f64 * pitch;
        out.extend(b.iter().zip(&bo).map(|(&(i, s), o)| (i, s, BOTTOM + height - o)));
        out
    };
    // the busier column decides the spacing: its rows must fit
    let rows_in = |c: usize| (0..N).filter(|&i| corner_of[i] == c).map(|i| if LABELS[i].0 == "button_a" { 2.0 } else { 1.0 }).sum::<f64>();
    let most = (rows_in(0) + rows_in(2)).max(rows_in(1) + rows_in(3)).max(2.0);
    let pitch = PITCH.min((TOP - BOTTOM - GAP) / (most - 1.0));
    let [kbd, mouse] = art();
    let mut out = Vec::new();
    for (left, top_c, bottom_c) in [(true, 0, 2), (false, 1, 3)] {
        let col_x = if left { LX } else { RX };
        // simple lines for label i at screen row y: (a) across to the key's x, then up/down into it; (b) across to
        // beside the key (on the label's side), up/down to its height, then into it from the side; (c) the same
        // beside the keyboard or mouse it is on
        let shapes = |i: usize, y: f64| -> Vec<Vec<Pt>> {
            let (Some(e), Some(b)) = (ends[i], bounds[i]) else { return Vec::new() };
            let ry = sheet_y(y);
            let pic = if e.0 > mouse.x0 - 20.0 { mouse } else { kbd };
            let (beside_key, beside_pic) = if left { (b.x0 - 24.0, pic.x0 - 26.0) } else { (b.x1 + 24.0, pic.x1 + 26.0) };
            let mut v = vec![vec![(col_x, ry), (e.0, ry), e]];
            for x in [beside_key, beside_pic] {
                v.push(vec![(col_x, ry), (x, ry), (x, e.1), e]);
            }
            v
        };
        let line_cost = |pts: &[Pt]| -> f64 {
            let mut c = 0.0;
            for (a, b) in segs(pts) {
                c += (b.0 - a.0).abs() + (b.1 - a.1).abs();
                for r in [&kbd, &mouse] {
                    c += 3.0 * inside_len(a, b, r);
                }
            }
            c
        };
        // crossings and lines running on top of each other
        let clash = |p: &[Pt], q: &[Pt]| -> f64 {
            let mut c = 0.0;
            for (a, b) in segs(p) {
                let ah = (a.1 - b.1).abs() < 0.5;
                for (d, e) in segs(q) {
                    let dh = (d.1 - e.1).abs() < 0.5;
                    if ah != dh {
                        let (h0, h1, v0, v1) = if ah { (a, b, d, e) } else { (d, e, a, b) };
                        let (x, y) = (v0.0, h0.1);
                        if x > h0.0.min(h1.0) + 1.0 && x < h0.0.max(h1.0) - 1.0 && y > v0.1.min(v1.1) + 1.0 && y < v0.1.max(v1.1) - 1.0 {
                            c += 10_000.0;
                        }
                    } else {
                        let (gap, lo, hi) = if ah {
                            ((a.1 - d.1).abs(), a.0.min(b.0).max(d.0.min(e.0)), a.0.max(b.0).min(d.0.max(e.0)))
                        } else {
                            ((a.0 - d.0).abs(), a.1.min(b.1).max(d.1.min(e.1)), a.1.max(b.1).min(d.1.max(e.1)))
                        };
                        if gap < 26.0 && hi - lo > 0.0 {
                            c += 10_000.0;
                        }
                    }
                }
            }
            c
        };
        let cost = |order: &[(usize, bool, f64)]| -> f64 {
            let opts: Vec<Vec<Vec<Pt>>> = order.iter().filter(|r| !r.1).map(|&(i, _, y)| shapes(i, y)).filter(|s| !s.is_empty()).collect();
            // each line takes its cheapest shape given the others' (a few rounds)
            let mut pick: Vec<usize> = vec![0; opts.len()];
            let total = |pick: &[usize]| -> f64 {
                let mut c = 0.0;
                for (i, o) in opts.iter().enumerate() {
                    c += line_cost(&o[pick[i]]);
                    for (j, q) in opts.iter().enumerate().skip(i + 1) {
                        c += clash(&o[pick[i]], &q[pick[j]]);
                    }
                }
                c
            };
            for _ in 0..3 {
                for i in 0..opts.len() {
                    let mut best = (f64::MAX, 0);
                    for s in 0..opts[i].len() {
                        pick[i] = s;
                        let t = total(&pick);
                        if t < best.0 {
                            best = (t, s);
                        }
                    }
                    pick[i] = best.1;
                }
            }
            total(&pick)
        };
        let mut top: Vec<usize> = (0..N).filter(|&i| corner_of[i] == top_c).collect();
        let mut bottom: Vec<usize> = (0..N).filter(|&i| corner_of[i] == bottom_c).collect();
        // labels without a key go last and are not reordered
        let keyed = |v: &mut Vec<usize>| -> Vec<usize> {
            let rest: Vec<usize> = v.iter().copied().filter(|&i| ends[i].is_none()).collect();
            v.retain(|&i| ends[i].is_some());
            rest
        };
        let (top_rest, bottom_rest) = (keyed(&mut top), keyed(&mut bottom));
        let mut best: Option<(f64, Vec<usize>, Vec<usize>)> = None;
        for t in permutations(&top) {
            for b in permutations(&bottom) {
                let (mut tt, mut bb) = (t.clone(), b.clone());
                tt.extend(&top_rest);
                bb.extend(&bottom_rest);
                let c = cost(&rows(&tt, &bb, pitch));
                if best.as_ref().is_none_or(|x| c < x.0) {
                    best = Some((c, tt, bb));
                }
            }
        }
        let (_, t, b) = best.unwrap_or_default();
        for (i, second, y) in rows(&t, &b, pitch) {
            out.push(Placed { what: LABELS[i].0, left, y, top: corner_of[i] < 2, second, pitch });
        }
    }
    out
}

/// Every order of `items` (a corner has at most five labels: 120 orders).
fn permutations(items: &[usize]) -> Vec<Vec<usize>> {
    if items.len() <= 1 {
        return vec![items.to_vec()];
    }
    let mut out = Vec::new();
    for k in 0..items.len() {
        let mut rest = items.to_vec();
        let first = rest.remove(k);
        for mut p in permutations(&rest) {
            p.insert(0, first);
            out.push(p);
        }
    }
    out
}

/// Key boxes of the keyboard art (its own pixels, 570x244), by key name.
const KEYS: [(&str, f64, f64, f64, f64); 77] = [
    ("esc", 17.0, 22.0, 49.0, 40.0), ("f1", 56.0, 22.0, 88.0, 40.0), ("f2", 95.0, 22.0, 127.0, 40.0),
    ("f3", 133.0, 22.0, 166.0, 40.0), ("f4", 172.0, 22.0, 204.0, 40.0), ("f5", 211.0, 22.0, 243.0, 40.0),
    ("f6", 250.0, 22.0, 282.0, 40.0), ("f7", 288.0, 22.0, 321.0, 40.0), ("f8", 327.0, 22.0, 359.0, 40.0),
    ("f9", 366.0, 22.0, 398.0, 40.0), ("f10", 405.0, 22.0, 437.0, 40.0), ("f11", 443.0, 22.0, 475.0, 40.0),
    ("f12", 482.0, 22.0, 514.0, 40.0),
    ("grave", 17.0, 46.0, 47.0, 76.0), ("1", 55.0, 46.0, 85.0, 76.0), ("2", 92.0, 46.0, 122.0, 76.0),
    ("3", 130.0, 46.0, 159.0, 76.0), ("4", 167.0, 46.0, 197.0, 76.0), ("5", 205.0, 46.0, 234.0, 76.0),
    ("6", 242.0, 46.0, 272.0, 76.0), ("7", 280.0, 46.0, 309.0, 76.0), ("8", 317.0, 46.0, 347.0, 76.0),
    ("9", 354.0, 46.0, 384.0, 76.0), ("0", 392.0, 46.0, 422.0, 76.0), ("minus", 429.0, 46.0, 459.0, 76.0),
    ("equals", 467.0, 46.0, 497.0, 76.0), ("backspace", 504.0, 46.0, 553.0, 76.0),
    ("tab", 17.0, 82.0, 66.0, 112.0), ("q", 74.0, 82.0, 103.0, 112.0), ("w", 111.0, 82.0, 141.0, 112.0),
    ("e", 148.0, 82.0, 178.0, 112.0), ("r", 186.0, 82.0, 216.0, 112.0), ("t", 223.0, 82.0, 253.0, 112.0),
    ("y", 261.0, 82.0, 291.0, 112.0), ("u", 298.0, 82.0, 328.0, 112.0), ("i", 336.0, 82.0, 365.0, 112.0),
    ("o", 373.0, 82.0, 403.0, 112.0), ("p", 411.0, 82.0, 440.0, 112.0), ("bracketleft", 448.0, 82.0, 478.0, 112.0),
    ("bracketright", 486.0, 82.0, 515.0, 112.0), ("backslash", 523.0, 82.0, 553.0, 112.0),
    ("capslock", 17.0, 118.0, 79.0, 148.0), ("a", 86.0, 118.0, 116.0, 148.0), ("s", 124.0, 118.0, 153.0, 148.0),
    ("d", 161.0, 118.0, 191.0, 148.0), ("f", 198.0, 118.0, 228.0, 148.0), ("g", 236.0, 118.0, 266.0, 148.0),
    ("h", 273.0, 118.0, 303.0, 148.0), ("j", 311.0, 118.0, 340.0, 148.0), ("k", 348.0, 118.0, 378.0, 148.0),
    ("l", 386.0, 118.0, 415.0, 148.0), ("semicolon", 423.0, 118.0, 453.0, 148.0),
    ("apostrophe", 461.0, 118.0, 490.0, 148.0), ("enter", 498.0, 118.0, 553.0, 148.0),
    ("shift", 17.0, 154.0, 97.0, 184.0), ("z", 104.0, 154.0, 134.0, 184.0), ("x", 142.0, 154.0, 171.0, 184.0),
    ("c", 179.0, 154.0, 209.0, 184.0), ("v", 217.0, 154.0, 246.0, 184.0), ("b", 254.0, 154.0, 284.0, 184.0),
    ("n", 292.0, 154.0, 321.0, 184.0), ("m", 329.0, 154.0, 359.0, 184.0), ("comma", 366.0, 154.0, 396.0, 184.0),
    ("period", 404.0, 154.0, 434.0, 184.0), ("slash", 441.0, 154.0, 471.0, 184.0),
    ("rshift", 479.0, 154.0, 553.0, 184.0),
    ("ctrl", 17.0, 190.0, 47.0, 226.0), ("win", 55.0, 190.0, 84.0, 226.0), ("fn", 92.0, 190.0, 122.0, 226.0),
    ("alt", 130.0, 190.0, 172.0, 226.0), ("space", 179.0, 190.0, 359.0, 226.0), ("ralt", 367.0, 190.0, 409.0, 226.0),
    ("rctrl", 416.0, 190.0, 446.0, 226.0),
    ("left", 452.0, 210.0, 482.0, 226.0), ("up", 488.0, 190.0, 518.0, 206.0), ("down", 488.0, 210.0, 518.0, 226.0),
    ("right", 523.0, 210.0, 553.0, 226.0),
];

/// Points of the mouse art (its own pixels, 271x419).
const MOUSE: [(&str, f64, f64); 6] = [
    ("mouse_left", 88.0, 150.0), ("mouse_right", 182.0, 150.0), ("mouse_middle", 135.0, 130.0),
    ("mouse_forward", 44.0, 205.0), ("mouse_back", 44.0, 250.0), ("camera", 135.0, 300.0),
];

type Pt = (f64, f64);

#[derive(Clone, Copy)]
struct Rect {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

impl Rect {
    fn center(&self) -> Pt {
        ((self.x0 + self.x1) / 2.0, (self.y0 + self.y1) / 2.0)
    }
}

/// Where the art sits in the 1160x680 picture: scale, and the picture position of the keyboard art's (17, 22) and of
/// the mouse body's top-left (36, 81). Must match tools/make_basic_controls.py, which draws the picture. Centred in
/// the picture, as big as leaves room for the lines beside it.
const KB_SC: f64 = 1.22;
const KB_ORG: Pt = (127.0, 216.0);
const MS_SC: f64 = 1.08;
const MS_ORG: Pt = (831.0, 131.0);

/// Picture pixel -> line-sheet pixel (the picture is shown 1.12x around the sheet point (777.5, 842)).
fn sheet(x: f64, y: f64) -> Pt {
    ((x - 580.0) * 1.12 + 777.5, (y - 340.0) * 1.12 + 842.0)
}

/// Keyboard art pixel -> line-sheet pixel.
fn kb(x: f64, y: f64) -> Pt {
    sheet(KB_ORG.0 + (x - 17.0) * KB_SC, KB_ORG.1 + (y - 22.0) * KB_SC)
}

/// Mouse art pixel -> line-sheet pixel.
fn ms(x: f64, y: f64) -> Pt {
    sheet(MS_ORG.0 + (x - 36.0) * MS_SC, MS_ORG.1 + (y - 81.0) * MS_SC)
}

fn key_rect(name: &str) -> Option<Rect> {
    let (_, x0, y0, x1, y1) = KEYS.iter().find(|k| k.0 == name)?;
    let (a, b) = (kb(*x0, *y0), kb(*x1, *y1));
    Some(Rect { x0: a.0, y0: a.1, x1: b.0, y1: b.1 })
}

/// Where a line ends: one key (a dot on it) or a group of keys (an outline around them: one box per key row,
/// joined, so the W/A/S/D outline has the keys' own shape).
#[derive(Clone, Copy)]
struct Goal {
    /// the whole goal's bounds
    bounds: Rect,
    group: bool,
    /// a group's rows (top to bottom)
    rows: [Option<Rect>; 4],
}

impl Goal {
    fn parts(&self) -> impl Iterator<Item = Rect> + '_ {
        self.rows.iter().flatten().copied()
    }

    /// Where a line arriving from above / below / the left / the right ends.
    fn top(&self) -> Pt {
        let r = self.parts().next().unwrap_or(self.bounds);
        if self.group { (r.center().0, r.y0) } else { self.bounds.center() }
    }
    fn bottom(&self) -> Pt {
        let r = self.parts().last().unwrap_or(self.bounds);
        if self.group { (r.center().0, r.y1) } else { self.bounds.center() }
    }
    fn left(&self) -> Pt {
        let r = self.parts().min_by(|a, b| a.x0.total_cmp(&b.x0)).unwrap_or(self.bounds);
        if self.group { (r.x0, r.center().1) } else { self.bounds.center() }
    }
    fn right(&self) -> Pt {
        let r = self.parts().max_by(|a, b| a.x1.total_cmp(&b.x1)).unwrap_or(self.bounds);
        if self.group { (r.x1, r.center().1) } else { self.bounds.center() }
    }
}

fn goal_for(key: &str) -> Option<Goal> {
    if let Some(r) = key_rect(key) {
        return Some(Goal { bounds: r, group: false, rows: [None; 4] });
    }
    let (_, x, y) = MOUSE.iter().find(|m| m.0 == key)?;
    let (cx, cy) = ms(*x, *y);
    Some(Goal { bounds: Rect { x0: cx - 12.0, y0: cy - 12.0, x1: cx + 12.0, y1: cy + 12.0 }, group: false, rows: [None; 4] })
}

fn group_goal(keys: &[&str]) -> Option<Goal> {
    let mut rects: Vec<Rect> = keys.iter().filter_map(|k| key_rect(k)).collect();
    if rects.is_empty() {
        return None;
    }
    let pad = 7.0;
    let join = |a: Rect, b: Rect| Rect { x0: a.x0.min(b.x0), y0: a.y0.min(b.y0), x1: a.x1.max(b.x1), y1: a.y1.max(b.y1) };
    rects.sort_by(|a, b| a.y0.total_cmp(&b.y0));
    // keys of one row of the keyboard share a box
    let mut rows: Vec<Rect> = Vec::new();
    for r in rects {
        match rows.last_mut() {
            Some(last) if (last.y0 - r.y0).abs() < 4.0 => *last = join(*last, r),
            _ => rows.push(r),
        }
    }
    let rows: Vec<Rect> = rows.into_iter().map(|r| Rect { x0: r.x0 - pad, y0: r.y0 - pad, x1: r.x1 + pad, y1: r.y1 + pad }).collect();
    // neighbouring rows touch: the gap between them belongs to the outline
    let mut out = [None; 4];
    for (i, r) in rows.iter().enumerate().take(4) {
        let mut r = *r;
        if i > 0 {
            let above = rows[i - 1];
            let mid = (above.y1 + r.y0) / 2.0;
            r.y0 = r.y0.min(mid);
        }
        if i + 1 < rows.len() {
            let below = rows[i + 1];
            r.y1 = r.y1.max((r.y1 + below.y0) / 2.0);
        }
        out[i] = Some(r);
    }
    let bounds = rows.iter().skip(1).fold(rows[0], |a, b| join(a, *b));
    Some(Goal { bounds, group: true, rows: out })
}

/// One line to draw: from the label's tick to its goal.
struct Line {
    start: Pt,
    dir: f64, // +1: label on the left (the line leaves to the right)
    sheet: usize,
    goal: Goal,
    /// key centres this line may cross without looking wrong (its own keys)
    own: Vec<Pt>,
}

// ---------------------------------------------------------------- routing
#[derive(Clone)]
struct Route {
    pts: Vec<Pt>,
    own_cost: f64,
}

fn segs(pts: &[Pt]) -> impl Iterator<Item = (Pt, Pt)> + '_ {
    pts.windows(2).map(|w| (w[0], w[1]))
}

fn dist_point_seg(p: Pt, a: Pt, b: Pt) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    let t = if l2 == 0.0 { 0.0 } else { (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0) };
    ((p.0 - a.0 - t * dx).powi(2) + (p.1 - a.1 - t * dy).powi(2)).sqrt()
}

/// Candidate paths for one line (orthogonal, at most four bends).
fn candidates(l: &Line) -> Vec<Vec<Pt>> {
    let (sx, sy) = l.start;
    let r = l.goal.bounds;
    let (cx, _) = r.center();
    let mut out = Vec::new();
    // lanes and channels keep clear of the art's edges: a line running along a border reads as part of the drawing
    let [kbd, mouse] = art();
    let near_edge = |v: f64, a: f64, b: f64| (v - a).abs() < 18.0 || (v - b).abs() < 18.0;
    let channels: Vec<f64> = (0..)
        .map(|i| 172.0 + 16.0 * i as f64)
        .take_while(|&x| x <= 1384.0)
        .filter(|&x| !near_edge(x, kbd.x0, kbd.x1) && !near_edge(x, mouse.x0, mouse.x1))
        .collect();
    let mut lanes: Vec<f64> = (0..)
        .map(|i| 300.0 + 14.0 * i as f64)
        .take_while(|&y| y <= 1390.0)
        .filter(|&y| !(y > kbd.y0 - 18.0 && y < kbd.y1 + 18.0) && !near_edge(y, mouse.y0, mouse.y1))
        .collect();
    // and the gaps between the keyboard's rows
    lanes.extend(row_gaps());
    let leaves = |x: f64| (x - sx) * l.dir >= 24.0;
    // straight across into the key when the label's row is within the key's height
    if !l.goal.group && sy > r.y0 + 5.0 && sy < r.y1 - 5.0 && leaves(cx) {
        out.push(vec![(sx, sy), (cx, sy)]);
    }
    // vertical arrival from above / below: the end point and the side it comes from
    let ends_v: [(Pt, f64); 2] = [(l.goal.top(), -1.0), (l.goal.bottom(), 1.0)];
    for (end, side) in ends_v {
        let edge = if side < 0.0 { r.y0 } else { r.y1 };
        // straight across, then down/up
        if leaves(end.0) && (sy - edge) * side > 12.0 {
            out.push(vec![(sx, sy), (end.0, sy), end]);
        }
        // across to a channel, to a lane, across to the goal, then down/up
        for &ly in &lanes {
            if (ly - edge) * side < 12.0 || (ly - sy).abs() < 14.0 {
                continue;
            }
            for &x in &channels {
                if leaves(x) && (x - end.0).abs() >= 14.0 {
                    out.push(vec![(sx, sy), (x, sy), (x, ly), (end.0, ly), end]);
                }
            }
        }
    }
    // horizontal arrival from the left / right
    let ends_h: [(Pt, f64); 2] = [(l.goal.left(), -1.0), (l.goal.right(), 1.0)];
    for (end, side) in ends_h {
        let edge = if side < 0.0 { r.x0 } else { r.x1 };
        for &x in &channels {
            if leaves(x) && (x - edge) * side > 14.0 && (end.1 - sy).abs() >= 14.0 {
                out.push(vec![(sx, sy), (x, sy), (x, end.1), end]);
            }
        }
        // around the picture: over or under it to a column beside it, then in from that side
        let pic = if cx > mouse.x0 - 20.0 { mouse } else { kbd };
        let beside = if side < 0.0 { pic.x0 } else { pic.x1 };
        for k in 1..=3 {
            let ex = beside + side * 26.0 * k as f64;
            if (ex - edge) * side <= 14.0 {
                continue;
            }
            for &ly in lanes.iter().filter(|&&y| y < kbd.y0.min(mouse.y0) - 12.0 || y > kbd.y1.max(mouse.y1) + 12.0) {
                if (ly - sy).abs() < 14.0 {
                    continue;
                }
                for &x in channels.iter().step_by(3) {
                    if leaves(x) && (x - ex).abs() >= 14.0 {
                        out.push(vec![(sx, sy), (x, sy), (x, ly), (ex, ly), (ex, end.1), end]);
                    }
                }
                if leaves(ex) {
                    out.push(vec![(sx, sy), (ex, sy), (ex, end.1), end]);
                }
            }
        }
    }
    out
}

/// Length of the stretch of segment a-b inside `r`.
fn inside_len(a: Pt, b: Pt, r: &Rect) -> f64 {
    if (a.1 - b.1).abs() < 0.5 {
        if a.1 <= r.y0 || a.1 >= r.y1 {
            return 0.0;
        }
        (a.0.max(b.0).min(r.x1) - a.0.min(b.0).max(r.x0)).max(0.0)
    } else {
        if a.0 <= r.x0 || a.0 >= r.x1 {
            return 0.0;
        }
        (a.1.max(b.1).min(r.y1) - a.1.min(b.1).max(r.y0)).max(0.0)
    }
}

/// The gaps between the keyboard's rows of keys (sheet y): a line along one covers no key.
fn row_gaps() -> [f64; 5] {
    [43.0, 79.0, 115.0, 151.0, 187.0].map(|y| kb(0.0, y).1)
}

/// The keyboard and the mouse body (sheet pixels): lines should cross as little of them as they can.
fn art() -> [Rect; 2] {
    let (k0, k1) = (kb(17.0, 22.0), kb(553.0, 226.0));
    let (m0, m1) = (ms(36.0, 81.0), ms(223.0, 390.0));
    [Rect { x0: k0.0, y0: k0.1, x1: k1.0, y1: k1.1 }, Rect { x0: m0.0, y0: m0.1, x1: m1.0, y1: m1.1 }]
}

/// Length, bends, short jogs, the stretch over the art and keys the path runs over (other than its own). The last
/// stretch, into its own keyboard or mouse, does not count as over the art: it has to get there.
fn own_cost(pts: &[Pt], own: &[Pt], all_keys: &[Pt]) -> f64 {
    let mut c = 0.0;
    let art = art();
    let n = pts.len();
    let goal = pts[n - 1];
    for (k, (a, b)) in segs(pts).enumerate() {
        let len = (b.0 - a.0).abs() + (b.1 - a.1).abs();
        c += len;
        let last = k + 2 == n;
        // along a gap between the keyboard's rows covers no key
        let in_gap = (a.1 - b.1).abs() < 0.5 && row_gaps().iter().any(|g| (g - a.1).abs() < 0.5);
        for r in &art {
            let home = goal.0 >= r.x0 && goal.0 <= r.x1 && goal.1 >= r.y0 && goal.1 <= r.y1;
            if !(last && home) {
                c += if in_gap { 2.0 } else { 7.0 } * inside_len(a, b, r);
            }
        }
        // a short step between two bends reads as a kink (except the step right after leaving the label)
        if !last && k > 1 && len < 35.0 {
            c += 1500.0;
        }
    }
    c += 60.0 * (n as f64 - 2.0);
    for k in all_keys {
        if own.iter().any(|o| (o.0 - k.0).abs() < 1.0 && (o.1 - k.1).abs() < 1.0) {
            continue;
        }
        if segs(pts).any(|(a, b)| dist_point_seg(*k, a, b) < 14.0) {
            c += 400.0; // a line over another key looks like it means that key
        }
    }
    c
}

/// How badly two paths get in each other's way.
fn pair_cost(a: &[Pt], b: &[Pt]) -> f64 {
    let mut c = 0.0;
    for (p, q) in segs(a) {
        let ph = (p.1 - q.1).abs() < 0.5;
        for (r, s) in segs(b) {
            let rh = (r.1 - s.1).abs() < 0.5;
            if ph == rh {
                // parallel: too close along a shared stretch reads as one line
                let (pa, pb, ra, rb, gap) = if ph {
                    (p.0.min(q.0), p.0.max(q.0), r.0.min(s.0), r.0.max(s.0), (p.1 - r.1).abs())
                } else {
                    (p.1.min(q.1), p.1.max(q.1), r.1.min(s.1), r.1.max(s.1), (p.0 - r.0).abs())
                };
                let shared = pb.min(rb) - pa.max(ra);
                if gap < 26.0 && shared > -6.0 {
                    c += 6000.0 + 10.0 * shared.max(0.0);
                }
            } else {
                // perpendicular: a crossing
                let (h0, h1, v0, v1) = if ph { (p, q, r, s) } else { (r, s, p, q) };
                let (x, y) = (v0.0, h0.1);
                let inside_h = x > h0.0.min(h1.0) + 1.0 && x < h0.0.max(h1.0) - 1.0;
                let inside_v = y > v0.1.min(v1.1) + 1.0 && y < v0.1.max(v1.1) - 1.0;
                if inside_h && inside_v {
                    c += 2000.0; // crossings make the screen hardest to read
                }
            }
        }
    }
    // a path running past another line's start or end looks connected to it
    for (end, other) in [(b[0], a), (*b.last().unwrap(), a), (a[0], b), (*a.last().unwrap(), b)] {
        if segs(other).any(|(p, q)| dist_point_seg(end, p, q) < 26.0) {
            c += 3000.0;
        }
    }
    c
}

fn route_all(lines: &[Line]) -> Vec<Vec<Pt>> {
    let all_keys: Vec<Pt> = KEYS
        .iter()
        .map(|k| Rect { x0: kb(k.1, k.2).0, y0: kb(k.1, k.2).1, x1: kb(k.3, k.4).0, y1: kb(k.3, k.4).1 }.center())
        .chain(MOUSE.iter().map(|m| ms(m.1, m.2)))
        .collect();
    // each line's paths, best on their own first; only the best few hundred can win once the others count
    let cands: Vec<Vec<Route>> = lines
        .iter()
        .map(|l| {
            let mut v: Vec<Route> = candidates(l)
                .into_iter()
                .map(|pts| Route { own_cost: own_cost(&pts, &l.own, &all_keys), pts })
                .collect();
            v.sort_by(|a, b| a.own_cost.total_cmp(&b.own_cost));
            v.truncate(300);
            v
        })
        .collect();
    let n = lines.len();
    let best_for = |i: usize, chosen: &[Option<usize>]| -> Option<usize> {
        let mut best: Option<(f64, usize)> = None;
        for (k, r) in cands[i].iter().enumerate() {
            let mut c = r.own_cost;
            if best.is_some_and(|b| c >= b.0) {
                break; // sorted by own cost: nothing later can win
            }
            for (j, ch) in chosen.iter().enumerate() {
                if j != i {
                    if let Some(o) = ch {
                        c += pair_cost(&r.pts, &cands[j][*o].pts);
                        if best.is_some_and(|b| c >= b.0) {
                            break;
                        }
                    }
                }
            }
            if best.is_none_or(|b| c < b.0) {
                best = Some((c, k));
            }
        }
        best.map(|b| b.1)
    };
    let total = |chosen: &[Option<usize>]| -> f64 {
        let mut c = 0.0;
        for i in 0..n {
            let Some(a) = chosen[i] else { continue };
            c += cands[i][a].own_cost;
            for j in i + 1..n {
                if let Some(b) = chosen[j] {
                    c += pair_cost(&cands[i][a].pts, &cands[j][b].pts);
                }
            }
        }
        c
    };
    // several starts (lines placed in different orders), each improved one line at a time; the best set wins
    let mut best: Option<(f64, Vec<Option<usize>>)> = None;
    let mut seed = 0x9e37_79b9_u32;
    for start in 0..16 {
        let mut order: Vec<usize> = (0..n).collect();
        if start == 1 {
            order.reverse();
        } else if start > 1 {
            for i in (1..n).rev() {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                order.swap(i, seed as usize % (i + 1));
            }
        }
        let mut chosen: Vec<Option<usize>> = vec![None; n];
        for &i in &order {
            chosen[i] = best_for(i, &chosen);
        }
        for _ in 0..12 {
            let mut changed = false;
            for &i in &order {
                let b = best_for(i, &chosen);
                if b != chosen[i] {
                    chosen[i] = b;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        let t = total(&chosen);
        if best.as_ref().is_none_or(|b| t < b.0) {
            best = Some((t, chosen));
        }
    }
    let chosen = best.map(|b| b.1).unwrap_or_default();
    chosen.iter().enumerate().map(|(i, c)| c.map(|k| cands[i][k].pts.clone()).unwrap_or_default()).collect()
}

// ---------------------------------------------------------------- drawing
struct Sheet {
    w: usize,
    h: usize,
    px: Vec<u8>,
}

impl Sheet {
    /// Coverage-based (anti-aliased) drawing of any shape given its distance function, within a box.
    fn shape(&mut self, b: Rect, dist: impl Fn(f64, f64) -> f64) {
        let x0 = (b.x0.floor().max(0.0)) as usize;
        let y0 = (b.y0.floor().max(0.0)) as usize;
        let x1 = (b.x1.ceil().min(self.w as f64 - 1.0)).max(0.0) as usize;
        let y1 = (b.y1.ceil().min(self.h as f64 - 1.0)).max(0.0) as usize;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let cov = (0.5 - dist(x as f64 + 0.5, y as f64 + 0.5)).clamp(0.0, 1.0);
                let v = (cov * 255.0 + 0.5) as u8;
                let p = &mut self.px[y * self.w + x];
                *p = (*p).max(v);
            }
        }
    }

    fn segment(&mut self, a: Pt, b: Pt, width: f64) {
        let r = width / 2.0;
        let bx = Rect { x0: a.0.min(b.0) - r - 1.0, y0: a.1.min(b.1) - r - 1.0, x1: a.0.max(b.0) + r + 1.0, y1: a.1.max(b.1) + r + 1.0 };
        self.shape(bx, |x, y| dist_point_seg((x, y), a, b) - r);
    }

    fn dot(&mut self, c: Pt, r: f64) {
        let bx = Rect { x0: c.0 - r - 1.0, y0: c.1 - r - 1.0, x1: c.0 + r + 1.0, y1: c.1 + r + 1.0 };
        self.shape(bx, |x, y| ((x - c.0).powi(2) + (y - c.1).powi(2)).sqrt() - r);
    }

    /// The outline of rounded boxes joined into one shape.
    fn outline(&mut self, parts: &[Rect], radius: f64, width: f64) {
        let Some(first) = parts.first() else { return };
        let m = width / 2.0 + 1.0;
        let b = parts.iter().fold(*first, |a, r| Rect { x0: a.x0.min(r.x0), y0: a.y0.min(r.y0), x1: a.x1.max(r.x1), y1: a.y1.max(r.y1) });
        let bx = Rect { x0: b.x0 - m, y0: b.y0 - m, x1: b.x1 + m, y1: b.y1 + m };
        // signed distance of each box (negative inside); the joined shape is the nearest of them
        let sd = |r: &Rect, x: f64, y: f64| {
            let (cx, cy) = r.center();
            let qx = (x - cx).abs() - ((r.x1 - r.x0) / 2.0 - radius);
            let qy = (y - cy).abs() - ((r.y1 - r.y0) / 2.0 - radius);
            (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - radius
        };
        self.shape(bx, |x, y| parts.iter().map(|r| sd(r, x, y)).fold(f64::MAX, f64::min).abs() - width / 2.0);
    }
}

/// The five guide sheets (`sizes`: each sheet's texture width and height) for the keys in `keys`.
pub fn draw(keys: &BTreeMap<String, String>, sizes: [(usize, usize); 5]) -> Vec<Vec<u8>> {
    let key = |n: &str| keys.get(n).map(String::as_str).unwrap_or("none").to_string();
    let mut lines = Vec::new();
    for p in place(keys) {
        if p.second {
            continue; // "Dash (Hold)": the same key as Actions, whose line is enough
        }
        let sheet = LABELS.iter().find(|l| l.0 == p.what).map_or(0, |l| l.1);
        let (left, y, what) = (p.left, sheet_y(p.y), p.what);
        let (goal, own) = match what {
            "move" => {
                let group = ["move_up", "move_down", "move_left", "move_right"];
                let names: Vec<String> = group.iter().map(|n| key(n)).collect();
                let refs: Vec<&str> = names.iter().map(String::as_str).collect();
                let Some(g) = group_goal(&refs) else { continue };
                // every key inside the outline counts as its own
                let own = KEYS
                    .iter()
                    .map(|k| Rect { x0: kb(k.1, k.2).0, y0: kb(k.1, k.2).1, x1: kb(k.3, k.4).0, y1: kb(k.3, k.4).1 }.center())
                    .filter(|c| g.parts().any(|r| c.0 > r.x0 && c.0 < r.x1 && c.1 > r.y0 && c.1 < r.y1))
                    .collect();
                (g, own)
            }
            _ => {
                let k = if what == "camera" { "camera".to_string() } else { key(what) };
                let Some(g) = goal_for(&k) else { continue };
                (g, vec![g.bounds.center()])
            }
        };
        lines.push(Line { start: (if left { LX } else { RX }, y), dir: if left { 1.0 } else { -1.0 }, sheet, goal, own });
    }
    let routes = route_all(&lines);
    if let Some(f) = std::env::var_os("GHK_GUIDES_DUMP") {
        // checks: every path as JSON lines (label row, points)
        let text: String = lines
            .iter()
            .zip(&routes)
            .map(|(l, r)| format!("{} {}\n", l.start.1, r.iter().map(|p| format!("{:.0},{:.0}", p.0, p.1)).collect::<Vec<_>>().join(" ")))
            .collect();
        let _ = std::fs::write(f, text);
    }
    let mut sheets: Vec<Sheet> = sizes.iter().map(|&(w, h)| Sheet { w, h, px: vec![0; w * h] }).collect();
    for (l, pts) in lines.iter().zip(&routes) {
        if pts.len() < 2 {
            continue;
        }
        let s = &mut sheets[l.sheet];
        s.segment((l.start.0, l.start.1 - TICK), (l.start.0, l.start.1 + TICK), LW);
        for (a, b) in segs(pts) {
            s.segment(a, b, LW);
        }
        if l.goal.group {
            let parts: Vec<Rect> = l.goal.parts().collect();
            s.outline(&parts, 12.0, LW - 1.0);
        } else {
            s.dot(*pts.last().unwrap(), DOT);
        }
    }
    sheets.into_iter().map(|s| s.px).collect()
}
