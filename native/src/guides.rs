//! The guide lines of the "Basic Controls" screen, drawn for the player's own keys.
//!
//! The game draws the keyboard-and-mouse picture (Controller_Pro / Controller_Pro1) and five line sheets
//! (ControllerGuide_Pro_00..04, one channel) on top of each other, with its own text labels at fixed rows on the
//! left and right. Each label shows the icon of the key it names (the key icon font), so each line must end on
//! that same key. The lines are orthogonal like the game's own: the router tries many paths for every line and
//! keeps the set with the fewest crossings, overlaps and lines passing over other keys.
//! The label rows and columns were measured on the game's screen.

use std::collections::BTreeMap;

const LW: f64 = 6.0; // line width
const DOT: f64 = 9.0; // end dot radius
const TICK: f64 = 14.0; // half height of the tick on the text side
const LX: f64 = 144.0; // label columns (sheet pixels)
const RX: f64 = 1410.0;

/// (what the label names, label on the left, row y, sheet), rows measured on the in-game screen. The D-pad block
/// (three labels on neighbouring keys) gets one line to a box around its keys, from its middle row; Put Away / Jump /
/// Attack and Rotate / Reset Camera get a line per label, at the text's own row.
const ROWS: [(&str, bool, f64, usize); 14] = [
    ("button_zl", true, 336.0, 0),
    ("button_l", true, 449.0, 4),
    ("button_minus", true, 562.0, 1),
    ("move", true, 670.0, 0),
    ("dpad", true, 960.0, 3),
    ("button_zr", false, 335.0, 2),
    ("button_r", false, 449.0, 4),
    ("button_plus", false, 559.0, 0),
    ("button_a", false, 715.0, 0),
    ("button_b", false, 908.0, 0),
    ("button_x", false, 1005.0, 0),
    ("button_y", false, 1101.0, 0),
    ("camera", false, 1236.0, 0),
    ("button_r3", false, 1333.0, 0),
];

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
/// the mouse body's top-left (36, 81). Must match tools/make_basic_controls.py, which draws the picture. Small enough
/// that the lines have room beside the keyboard and the mouse.
const KB_SC: f64 = 1.10;
const KB_ORG: Pt = (157.0, 245.0);
const MS_SC: f64 = 0.95;
const MS_ORG: Pt = (824.0, 249.0);

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

/// Where a line ends: one key (a dot on it) or a group of keys (a box around them).
#[derive(Clone, Copy)]
struct Goal {
    rect: Rect,
    group: bool,
}

fn goal_for(key: &str) -> Option<Goal> {
    if let Some(r) = key_rect(key) {
        return Some(Goal { rect: r, group: false });
    }
    let (_, x, y) = MOUSE.iter().find(|m| m.0 == key)?;
    let (cx, cy) = ms(*x, *y);
    Some(Goal { rect: Rect { x0: cx - 12.0, y0: cy - 12.0, x1: cx + 12.0, y1: cy + 12.0 }, group: false })
}

fn group_goal(keys: &[&str]) -> Option<Goal> {
    let rects: Vec<Rect> = keys.iter().filter_map(|k| key_rect(k)).collect();
    if rects.is_empty() {
        return None;
    }
    let pad = 7.0;
    let r = rects.iter().fold(rects[0], |a, b| Rect { x0: a.x0.min(b.x0), y0: a.y0.min(b.y0), x1: a.x1.max(b.x1), y1: a.y1.max(b.y1) });
    Some(Goal { rect: Rect { x0: r.x0 - pad, y0: r.y0 - pad, x1: r.x1 + pad, y1: r.y1 + pad }, group: true })
}

/// One line to draw: from the label's tick to its goal.
struct Line {
    start: Pt,
    dir: f64, // +1: label on the left (the line leaves to the right)
    sheet: usize,
    goal: Goal,
    /// key centres this line may cross without looking wrong (its own keys)
    own: Vec<Pt>,
    /// a second dot the line passes through on its way in from below (the mouse body before the wheel)
    via: Option<Pt>,
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

/// Candidate paths for one line (orthogonal, at most four bends); a line with a via dot comes straight up through it.
fn candidates(l: &Line) -> Vec<Vec<Pt>> {
    let all = candidates_any(l);
    match l.via {
        None => all,
        Some(v) => all
            .into_iter()
            .filter(|pts| {
                let (a, b) = (pts[pts.len() - 2], pts[pts.len() - 1]);
                (a.0 - b.0).abs() < 0.5 && (a.0 - v.0).abs() < 0.5 && a.1 > v.1 + 20.0
            })
            .collect(),
    }
}

fn candidates_any(l: &Line) -> Vec<Vec<Pt>> {
    let (sx, sy) = l.start;
    let r = l.goal.rect;
    let (cx, cy) = r.center();
    let mut out = Vec::new();
    // lanes and channels keep clear of the art's edges: a line running along a border reads as part of the drawing
    let [kbd, mouse] = art();
    let near_edge = |v: f64, a: f64, b: f64| (v - a).abs() < 18.0 || (v - b).abs() < 18.0;
    let channels: Vec<f64> = (0..)
        .map(|i| 172.0 + 16.0 * i as f64)
        .take_while(|&x| x <= 1384.0)
        .filter(|&x| !near_edge(x, kbd.x0, kbd.x1) && !near_edge(x, mouse.x0, mouse.x1))
        .collect();
    let lanes: Vec<f64> = (0..)
        .map(|i| 300.0 + 14.0 * i as f64)
        .take_while(|&y| y <= 1390.0)
        .filter(|&y| !(y > kbd.y0 - 18.0 && y < kbd.y1 + 18.0) && !near_edge(y, mouse.y0, mouse.y1))
        .collect();
    let leaves = |x: f64| (x - sx) * l.dir >= 24.0;
    // vertical arrival from above / below: the end point and the side it comes from
    let ends_v: [(Pt, f64); 2] = if l.goal.group {
        [((cx, r.y0), -1.0), ((cx, r.y1), 1.0)]
    } else {
        [((cx, cy), -1.0), ((cx, cy), 1.0)]
    };
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
    let ends_h: [(Pt, f64); 2] = if l.goal.group {
        [((r.x0, cy), -1.0), ((r.x1, cy), 1.0)]
    } else {
        [((cx, cy), -1.0), ((cx, cy), 1.0)]
    };
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

/// The side of its picture a key is on: -1 left part, +1 right part, 0 near the middle.
fn half(c: Pt) -> f64 {
    let [kbd, mouse] = art();
    let pic = if c.0 > mouse.x0 - 20.0 { mouse } else { kbd };
    let off = c.0 - pic.center().0;
    if off.abs() < (pic.x1 - pic.x0) * 0.15 {
        0.0
    } else {
        off.signum()
    }
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

/// The keyboard and the mouse body (sheet pixels): lines should cross as little of them as they can.
fn art() -> [Rect; 2] {
    let (k0, k1) = (kb(17.0, 22.0), kb(553.0, 226.0));
    let (m0, m1) = (ms(36.0, 81.0), ms(223.0, 390.0));
    [Rect { x0: k0.0, y0: k0.1, x1: k1.0, y1: k1.1 }, Rect { x0: m0.0, y0: m0.1, x1: m1.0, y1: m1.1 }]
}

/// Length, bends, the stretch over the art and keys the path runs over (other than its own).
fn own_cost(pts: &[Pt], own: &[Pt], all_keys: &[Pt], single_key: bool) -> f64 {
    let mut c = 0.0;
    let art = art();
    for (a, b) in segs(pts) {
        c += (b.0 - a.0).abs() + (b.1 - a.1).abs();
        for r in &art {
            c += 7.0 * inside_len(a, b, r);
        }
    }
    c += 60.0 * (pts.len() as f64 - 2.0);
    // a key in the left half is best reached from the left, one in the right half from the right
    let n = pts.len();
    let (a, b) = (pts[n - 2], pts[n - 1]);
    if single_key && (a.1 - b.1).abs() < 0.5 && half(b) != 0.0 && (b.0 - a.0).signum() == -half(b) {
        c -= 250.0;
    }
    for k in all_keys {
        if own.iter().any(|o| (o.0 - k.0).abs() < 1.0 && (o.1 - k.1).abs() < 1.0) {
            continue;
        }
        if segs(pts).any(|(a, b)| dist_point_seg(*k, a, b) < 14.0) {
            c += 400.0;
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
                    c += 350.0;
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
                .map(|pts| Route { own_cost: own_cost(&pts, &l.own, &all_keys, !l.goal.group), pts })
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

    fn rounded_box(&mut self, b: Rect, radius: f64, width: f64) {
        let (cx, cy) = b.center();
        let (hw, hh) = ((b.x1 - b.x0) / 2.0, (b.y1 - b.y0) / 2.0);
        let m = width / 2.0 + 1.0;
        let bx = Rect { x0: b.x0 - m, y0: b.y0 - m, x1: b.x1 + m, y1: b.y1 + m };
        self.shape(bx, |x, y| {
            let qx = (x - cx).abs() - (hw - radius);
            let qy = (y - cy).abs() - (hh - radius);
            let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
            let sd = outside + qx.max(qy).min(0.0) - radius;
            sd.abs() - width / 2.0
        });
    }
}

/// The five guide sheets (`sizes`: each sheet's texture width and height) for the keys in `keys`.
pub fn draw(keys: &BTreeMap<String, String>, sizes: [(usize, usize); 5]) -> Vec<Vec<u8>> {
    let key = |n: &str| keys.get(n).map(String::as_str).unwrap_or("none").to_string();
    let merged_camera = key("button_r3") == "mouse_middle";
    let mut lines = Vec::new();
    for (what, left, y, sheet) in ROWS {
        let (goal, own) = match what {
            "move" | "dpad" => {
                let group = if what == "move" {
                    ["move_up", "move_down", "move_left", "move_right"]
                } else {
                    ["dpad_up", "dpad_down", "dpad_left", "dpad_right"]
                };
                let names: Vec<String> = group.iter().map(|n| key(n)).collect();
                let refs: Vec<&str> = names.iter().map(String::as_str).collect();
                let Some(g) = group_goal(&refs) else { continue };
                // every key inside the box counts as its own
                let own = KEYS
                    .iter()
                    .map(|k| Rect { x0: kb(k.1, k.2).0, y0: kb(k.1, k.2).1, x1: kb(k.3, k.4).0, y1: kb(k.3, k.4).1 }.center())
                    .filter(|c| c.0 > g.rect.x0 && c.0 < g.rect.x1 && c.1 > g.rect.y0 && c.1 < g.rect.y1)
                    .collect();
                (g, own)
            }
            // Rotate / Reset Camera is one stick, moved and pressed: with reset on the middle button it is one line
            // up through the mouse, a dot on the body and one on the wheel
            "button_r3" if merged_camera => continue,
            "camera" if merged_camera => {
                let Some(g) = goal_for("mouse_middle") else { continue };
                let body = ms(MOUSE[5].1, MOUSE[5].2);
                lines.push(Line {
                    start: (RX, y),
                    dir: -1.0,
                    sheet,
                    goal: g,
                    own: vec![g.rect.center(), body],
                    via: Some(body),
                });
                continue;
            }
            _ => {
                let k = if what == "camera" { "camera".to_string() } else { key(what) };
                let Some(g) = goal_for(&k) else { continue };
                (g, vec![g.rect.center()])
            }
        };
        lines.push(Line { start: (if left { LX } else { RX }, y), dir: if left { 1.0 } else { -1.0 }, sheet, goal, own, via: None });
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
            s.rounded_box(l.goal.rect, 12.0, LW - 1.0);
        } else {
            s.dot(*pts.last().unwrap(), DOT);
        }
        if let Some(v) = l.via {
            s.dot(v, DOT);
        }
    }
    sheets.into_iter().map(|s| s.px).collect()
}
