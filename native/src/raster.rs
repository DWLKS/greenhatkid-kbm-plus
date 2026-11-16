//! Filling outlines into a small anti-aliased mask (the HUD letters and ocarina notes). Each
//! contour is filled at 8x the size, largest first, solid where it is a fill and clear where it is a cut-out,
//! then every 8x8 square is averaged.

use crate::outline::{area, bbox, flatten, Contour, P};

const SS: usize = 8;

/// Mask (0..1, size x size, row by row) of `cs` scaled to fit fit_w x fit_h, centred on (cx, cy) in pixels.
pub fn ink(cs: &[Contour], sign: f64, size: usize, cx: f64, cy: f64, fit_w: f64, fit_h: f64) -> Vec<f32> {
    let n = size * SS;
    let mut big = vec![0u8; n * n];
    let Some((x0, y0, x1, y1)) = bbox(cs) else { return vec![0.0; size * size] };
    let s = (fit_w / (x1 - x0).max(1e-6)).min(fit_h / (y1 - y0).max(1e-6));
    let (mx, my) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let ss = SS as f64;
    let tf = |p: P| P((cx + (p.0 - mx) * s) * ss, (cy - (p.1 - my) * s) * ss);
    let mut order: Vec<&Contour> = cs.iter().collect();
    order.sort_by(|a, b| area(b).abs().partial_cmp(&area(a).abs()).unwrap_or(std::cmp::Ordering::Equal));
    for c in order {
        let value = if area(c) * sign > 0.0 { 255 } else { 0 };
        let pts: Vec<P> = flatten(c).into_iter().map(tf).collect();
        fill_polygon(&mut big, n, &pts, value);
    }
    let mut out = vec![0.0f32; size * size];
    for y in 0..size {
        for x in 0..size {
            let mut sum = 0u32;
            for yy in 0..SS {
                let row = (y * SS + yy) * n + x * SS;
                sum += big[row..row + SS].iter().map(|&v| v as u32).sum::<u32>();
            }
            out[y * size + x] = sum as f32 / (255.0 * (SS * SS) as f32);
        }
    }
    out
}

/// Scanline fill (even-odd), sampling pixel centres.
fn fill_polygon(img: &mut [u8], n: usize, pts: &[P], value: u8) {
    if pts.len() < 3 {
        return;
    }
    let ymin = pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min).floor().max(0.0) as usize;
    let ymax = (pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max).ceil() as isize).clamp(0, n as isize) as usize;
    let mut xs = Vec::new();
    for y in ymin..ymax {
        let sy = y as f64 + 0.5;
        xs.clear();
        for i in 0..pts.len() {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            if (a.1 <= sy) != (b.1 <= sy) {
                xs.push(a.0 + (sy - a.1) / (b.1 - a.1) * (b.0 - a.0));
            }
        }
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        for pair in xs.as_chunks::<2>().0 {
            // pixels whose centre lies inside [x0, x1)
            let from = (pair[0] - 0.5).ceil().max(0.0) as usize;
            let to = ((pair[1] - 0.5).ceil().max(0.0) as usize).min(n);
            if from < to {
                img[y * n + from..y * n + to].fill(value);
            }
        }
    }
}
