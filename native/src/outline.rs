//! Glyph outlines. Font units, y up. Fills run counter-clockwise
//! (area > 0), cut-outs (letters, symbols) clockwise.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct P(pub f64, pub f64);

#[derive(Clone, Debug)]
pub enum Seg {
    Move(P),
    Line(P),
    Curve(P, P, P),
    Close,
}

pub type Contour = Vec<Seg>;

impl Seg {
    fn end(&self) -> Option<P> {
        match *self {
            Seg::Move(p) | Seg::Line(p) | Seg::Curve(_, _, p) => Some(p),
            Seg::Close => None,
        }
    }
    fn points(&self) -> Vec<P> {
        match *self {
            Seg::Move(p) | Seg::Line(p) => vec![p],
            Seg::Curve(a, b, c) => vec![a, b, c],
            Seg::Close => vec![],
        }
    }
    fn map(&self, f: &dyn Fn(P) -> P) -> Seg {
        match *self {
            Seg::Move(p) => Seg::Move(f(p)),
            Seg::Line(p) => Seg::Line(f(p)),
            Seg::Curve(a, b, c) => Seg::Curve(f(a), f(b), f(c)),
            Seg::Close => Seg::Close,
        }
    }
}

/// Signed area of the polygon through the on-curve points.
pub fn area(c: &Contour) -> f64 {
    let p: Vec<P> = c.iter().filter_map(|s| s.end()).collect();
    let n = p.len();
    (0..n).map(|i| p[i].0 * p[(i + 1) % n].1 - p[(i + 1) % n].0 * p[i].1).sum::<f64>() / 2.0
}

/// Bounds of all points, control points included: (x0, y0, x1, y1).
pub fn bbox(cs: &[Contour]) -> Option<(f64, f64, f64, f64)> {
    let mut b: Option<(f64, f64, f64, f64)> = None;
    for p in cs.iter().flatten().flat_map(|s| s.points()) {
        b = Some(match b {
            None => (p.0, p.1, p.0, p.1),
            Some((x0, y0, x1, y1)) => (x0.min(p.0), y0.min(p.1), x1.max(p.0), y1.max(p.1)),
        });
    }
    b
}

/// The same contour drawn the other way round (same start point).
pub fn reverse(c: &Contour) -> Contour {
    let Some(Seg::Move(start)) = c.first() else { return c.clone() };
    // segments as (from, controls, to)
    type Piece = (P, Option<(P, P)>, P);
    let mut segs: Vec<Piece> = Vec::new();
    let mut cur = *start;
    for s in &c[1..] {
        match *s {
            Seg::Line(p) => {
                segs.push((cur, None, p));
                cur = p;
            }
            Seg::Curve(a, b, p) => {
                segs.push((cur, Some((a, b)), p));
                cur = p;
            }
            _ => {}
        }
    }
    if cur != *start {
        segs.push((cur, None, *start)); // the implied closing line
    }
    let mut out = vec![Seg::Move(*start)];
    for (from, ctrl, _) in segs.iter().rev() {
        out.push(match ctrl {
            None => Seg::Line(*from),
            Some((a, b)) => Seg::Curve(*b, *a, *from),
        });
    }
    if let Some(Seg::Line(p)) = out.last() {
        if *p == *start {
            out.pop(); // closing back to the start is implied
        }
    }
    out.push(Seg::Close);
    out
}

pub fn transform(cs: &[Contour], f: &dyn Fn(P) -> P) -> Vec<Contour> {
    cs.iter().map(|c| c.iter().map(|s| s.map(f)).collect()).collect()
}

/// Closed polygon; points counter-clockwise; cw reverses it (cut-out).
pub fn poly(points: &[P], cw: bool) -> Contour {
    let pts: Vec<P> = if cw { points.iter().rev().copied().collect() } else { points.to_vec() };
    let mut c = vec![Seg::Move(pts[0])];
    c.extend(pts[1..].iter().map(|&p| Seg::Line(p)));
    c.push(Seg::Close);
    c
}

pub fn fill(c: Contour) -> Contour {
    if area(&c) > 0.0 { c } else { reverse(&c) }
}

pub fn cut(c: Contour) -> Contour {
    if area(&c) > 0.0 { reverse(&c) } else { c }
}

/// Polygon through the outline, curves split in 12 (what the texture drawing fills).
pub fn flatten(c: &Contour) -> Vec<P> {
    let mut pts = Vec::new();
    let mut cur = P(0.0, 0.0);
    for s in c {
        match *s {
            Seg::Move(p) | Seg::Line(p) => {
                cur = p;
                pts.push(p);
            }
            Seg::Curve(p1, p2, p3) => {
                let p0 = cur;
                for i in 1..=12 {
                    let t = i as f64 / 12.0;
                    let u = 1.0 - t;
                    let f = |a: f64, b: f64, c: f64, d: f64| u * u * u * a + 3.0 * u * u * t * b + 3.0 * u * t * t * c + t * t * t * d;
                    pts.push(P(f(p0.0, p1.0, p2.0, p3.0), f(p0.1, p1.1, p2.1, p3.1)));
                }
                cur = p3;
            }
            Seg::Close => {}
        }
    }
    pts
}
