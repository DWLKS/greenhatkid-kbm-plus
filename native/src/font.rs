//! Just enough OpenType/CFF to redraw some glyphs of the game's button-icon font: read the table directory,
//! cmap and hmtx, read the CFF (charstrings, subroutines), turn Type 2 charstrings into outlines, encode new
//! outlines, and write the font back. Everything the game does not see changed is copied byte for byte.

use crate::outline::{Contour, Seg, P};
use std::collections::HashMap;

fn be16(b: &[u8], o: usize) -> Result<u16, String> {
    b.get(o..o + 2).map(|s| u16::from_be_bytes([s[0], s[1]])).ok_or("font data ends early".into())
}
fn be32(b: &[u8], o: usize) -> Result<u32, String> {
    b.get(o..o + 4).map(|s| u32::from_be_bytes(s.try_into().unwrap())).ok_or("font data ends early".into())
}

// ------------------------------------------------------------------------------------------------ sfnt
pub struct Sfnt {
    /// tag, data; in the order the data sits in the file
    pub tables: Vec<([u8; 4], Vec<u8>)>,
}

impl Sfnt {
    pub fn parse(d: &[u8]) -> Result<Sfnt, String> {
        if d.get(..4) != Some(b"OTTO") {
            return Err("not a CFF OpenType font".into());
        }
        let n = be16(d, 4)? as usize;
        let mut recs = Vec::new();
        for i in 0..n {
            let r = 12 + i * 16;
            let tag: [u8; 4] = d[r..r + 4].try_into().unwrap();
            let (off, len) = (be32(d, r + 8)? as usize, be32(d, r + 12)? as usize);
            let data = d.get(off..off + len).ok_or("font table outside the file")?.to_vec();
            recs.push((off, tag, data));
        }
        recs.sort_by_key(|r| r.0);
        Ok(Sfnt { tables: recs.into_iter().map(|(_, t, d)| (t, d)).collect() })
    }

    pub fn table(&self, tag: &[u8; 4]) -> Result<&[u8], String> {
        self.tables
            .iter()
            .find(|(t, _)| t == tag)
            .map(|(_, d)| d.as_slice())
            .ok_or(format!("font table {} missing", String::from_utf8_lossy(tag)))
    }

    pub fn set(&mut self, tag: &[u8; 4], data: Vec<u8>) {
        if let Some(t) = self.tables.iter_mut().find(|(t, _)| t == tag) {
            t.1 = data;
        }
    }

    pub fn remove(&mut self, tag: &[u8; 4]) {
        self.tables.retain(|(t, _)| t != tag);
    }

    /// code point -> glyph id, from the Windows Unicode subtable (format 12 or 4)
    pub fn cmap(&self) -> Result<HashMap<u32, u16>, String> {
        let c = self.table(b"cmap")?;
        let n = be16(c, 2)? as usize;
        let mut best: Option<(u32, usize)> = None;
        for i in 0..n {
            let (pid, eid, off) = (be16(c, 4 + i * 8)?, be16(c, 6 + i * 8)?, be32(c, 8 + i * 8)? as usize);
            let rank = match (pid, eid) {
                (3, 10) => 4,
                (0, 4) | (0, 6) => 3,
                (3, 1) => 2,
                (0, _) => 1,
                _ => 0,
            };
            if rank > 0 && best.is_none_or(|b| rank > b.0) {
                best = Some((rank, off));
            }
        }
        let off = best.ok_or("no Unicode cmap")?.1;
        let mut out = HashMap::new();
        match be16(c, off)? {
            4 => {
                let segs = be16(c, off + 6)? as usize / 2;
                let (ends, starts, deltas, ranges) = (off + 14, off + 16 + segs * 2, off + 16 + segs * 4, off + 16 + segs * 6);
                for s in 0..segs {
                    let (end, start) = (be16(c, ends + s * 2)? as u32, be16(c, starts + s * 2)? as u32);
                    let delta = be16(c, deltas + s * 2)?;
                    let ro = be16(c, ranges + s * 2)? as usize;
                    for cp in start..=end.min(0xFFFE) {
                        let g = if ro == 0 {
                            (cp as u16).wrapping_add(delta)
                        } else {
                            let a = ranges + s * 2 + ro + (cp - start) as usize * 2;
                            let g = be16(c, a)?;
                            if g == 0 { 0 } else { g.wrapping_add(delta) }
                        };
                        if g != 0 {
                            out.insert(cp, g);
                        }
                    }
                }
            }
            12 => {
                let groups = be32(c, off + 12)? as usize;
                for i in 0..groups {
                    let o = off + 16 + i * 12;
                    let (s, e, g) = (be32(c, o)?, be32(c, o + 4)?, be32(c, o + 8)?);
                    for cp in s..=e {
                        out.insert(cp, (g + cp - s) as u16);
                    }
                }
            }
            f => return Err(format!("cmap format {f} not supported")),
        }
        Ok(out)
    }

    /// (advance, left side bearing) per glyph
    pub fn hmtx(&self, glyphs: usize) -> Result<Vec<(u16, i16)>, String> {
        let n = be16(self.table(b"hhea")?, 34)? as usize;
        let h = self.table(b"hmtx")?;
        let mut out = Vec::with_capacity(glyphs);
        for g in 0..glyphs {
            if g < n {
                out.push((be16(h, g * 4)?, be16(h, g * 4 + 2)? as i16));
            } else {
                out.push((out.last().map_or(0, |m: &(u16, i16)| m.0), be16(h, n * 4 + (g - n) * 2)? as i16));
            }
        }
        Ok(out)
    }

    /// The file: directory sorted by tag, tables 4-byte aligned, head.checkSumAdjustment set.
    pub fn write(&self) -> Vec<u8> {
        let n = self.tables.len();
        let mut es = 0;
        while (2usize << es) <= n {
            es += 1;
        }
        let sr = (1usize << es) * 16;
        let mut out = Vec::new();
        out.extend_from_slice(b"OTTO");
        for v in [n, sr, es, n * 16 - sr] {
            out.extend_from_slice(&(v as u16).to_be_bytes());
        }
        let mut sorted: Vec<usize> = (0..n).collect();
        sorted.sort_by_key(|&i| self.tables[i].0);
        let dir = out.len();
        out.resize(dir + n * 16, 0);
        let mut head_at = None;
        let mut offs = vec![0usize; n];
        for (i, (tag, data)) in self.tables.iter().enumerate() {
            offs[i] = out.len();
            let mut data = data.clone();
            if tag == b"head" {
                data[8..12].fill(0);
                head_at = Some(out.len());
            }
            out.extend_from_slice(&data);
            while out.len() % 4 != 0 {
                out.push(0);
            }
        }
        for (k, &i) in sorted.iter().enumerate() {
            let (tag, data) = &self.tables[i];
            let r = dir + k * 16;
            let sum = checksum(&out[offs[i]..offs[i] + data.len().div_ceil(4) * 4]);
            out[r..r + 4].copy_from_slice(tag);
            out[r + 4..r + 8].copy_from_slice(&sum.to_be_bytes());
            out[r + 8..r + 12].copy_from_slice(&(offs[i] as u32).to_be_bytes());
            out[r + 12..r + 16].copy_from_slice(&(data.len() as u32).to_be_bytes());
        }
        if let Some(h) = head_at {
            let adj = 0xB1B0_AFBAu32.wrapping_sub(checksum(&out));
            out[h + 8..h + 12].copy_from_slice(&adj.to_be_bytes());
        }
        out
    }
}

fn checksum(d: &[u8]) -> u32 {
    let mut s = 0u32;
    for w in d.chunks(4) {
        let mut b = [0u8; 4];
        b[..w.len()].copy_from_slice(w);
        s = s.wrapping_add(u32::from_be_bytes(b));
    }
    s
}

// ------------------------------------------------------------------------------------------------ CFF
/// INDEX at `o`: (items, end offset)
fn read_index(d: &[u8], o: usize) -> Result<(Vec<Vec<u8>>, usize), String> {
    let count = be16(d, o)? as usize;
    if count == 0 {
        return Ok((Vec::new(), o + 2));
    }
    let os = *d.get(o + 2).ok_or("bad CFF INDEX")? as usize;
    let off = |i: usize| -> Result<usize, String> {
        let p = o + 3 + i * os;
        let b = d.get(p..p + os).ok_or("bad CFF INDEX")?;
        Ok(b.iter().fold(0usize, |a, &x| a << 8 | x as usize))
    };
    let base = o + 3 + (count + 1) * os - 1;
    let mut items = Vec::with_capacity(count);
    for i in 0..count {
        let (s, e) = (base + off(i)?, base + off(i + 1)?);
        items.push(d.get(s..e).ok_or("bad CFF INDEX")?.to_vec());
    }
    Ok((items, base + off(count)?))
}

fn write_index(items: &[Vec<u8>]) -> Vec<u8> {
    let mut out = (items.len() as u16).to_be_bytes().to_vec();
    if items.is_empty() {
        return out;
    }
    let total: usize = items.iter().map(|i| i.len()).sum::<usize>() + 1;
    let os = if total < 1 << 8 { 1 } else if total < 1 << 16 { 2 } else if total < 1 << 24 { 3 } else { 4 };
    out.push(os as u8);
    let mut off = 1usize;
    for i in 0..=items.len() {
        out.extend_from_slice(&(off as u32).to_be_bytes()[4 - os..]);
        if i < items.len() {
            off += items[i].len();
        }
    }
    for i in items {
        out.extend_from_slice(i);
    }
    out
}

/// One DICT entry: operator (12 x as 1200 + x), its operand bytes as stored, and their values.
#[derive(Clone)]
struct DictEntry {
    op: u16,
    raw: Vec<u8>,
    vals: Vec<f64>,
}

fn read_dict(d: &[u8]) -> Result<Vec<DictEntry>, String> {
    let mut out = Vec::new();
    let (mut i, mut start, mut vals) = (0, 0, Vec::new());
    while i < d.len() {
        let b0 = d[i];
        match b0 {
            0..=21 => {
                let op = if b0 == 12 {
                    i += 1;
                    1200 + *d.get(i).ok_or("bad CFF DICT")? as u16
                } else {
                    b0 as u16
                };
                out.push(DictEntry { op, raw: d[start..if b0 == 12 { i - 1 } else { i }].to_vec(), vals: std::mem::take(&mut vals) });
                i += 1;
                start = i;
            }
            28 => {
                vals.push(i16::from_be_bytes([d[i + 1], d[i + 2]]) as f64);
                i += 3;
            }
            29 => {
                vals.push(i32::from_be_bytes(d[i + 1..i + 5].try_into().unwrap()) as f64);
                i += 5;
            }
            30 => {
                // real number: nibbles until 0xf
                let mut s = String::new();
                i += 1;
                'n: loop {
                    let b = *d.get(i).ok_or("bad CFF DICT")?;
                    i += 1;
                    for nib in [b >> 4, b & 15] {
                        match nib {
                            0..=9 => s.push((b'0' + nib) as char),
                            0xa => s.push('.'),
                            0xb => s.push('E'),
                            0xc => s.push_str("E-"),
                            0xe => s.push('-'),
                            0xf => break 'n,
                            _ => {}
                        }
                    }
                }
                vals.push(s.parse().unwrap_or(0.0));
            }
            32..=246 => {
                vals.push(b0 as f64 - 139.0);
                i += 1;
            }
            247..=250 => {
                vals.push((b0 as f64 - 247.0) * 256.0 + d[i + 1] as f64 + 108.0);
                i += 2;
            }
            251..=254 => {
                vals.push(-(b0 as f64 - 251.0) * 256.0 - d[i + 1] as f64 - 108.0);
                i += 2;
            }
            _ => return Err("bad CFF DICT".into()),
        }
    }
    Ok(out)
}

fn dict_int(v: i32) -> Vec<u8> {
    match v {
        -107..=107 => vec![(v + 139) as u8],
        108..=1131 => vec![((v - 108) / 256 + 247) as u8, ((v - 108) % 256) as u8],
        -1131..=-108 => vec![((-v - 108) / 256 + 251) as u8, ((-v - 108) % 256) as u8],
        -32768..=32767 => {
            let b = (v as i16).to_be_bytes();
            vec![28, b[0], b[1]]
        }
        _ => {
            let b = v.to_be_bytes();
            vec![29, b[0], b[1], b[2], b[3]]
        }
    }
}

fn dict_int5(v: i32) -> Vec<u8> {
    let b = v.to_be_bytes();
    vec![29, b[0], b[1], b[2], b[3]]
}

fn write_dict(entries: &[DictEntry]) -> Vec<u8> {
    let mut out = Vec::new();
    for e in entries {
        out.extend_from_slice(&e.raw);
        if e.op >= 1200 {
            out.extend_from_slice(&[12, (e.op - 1200) as u8]);
        } else {
            out.push(e.op as u8);
        }
    }
    out
}

pub struct Cff {
    header: Vec<u8>,
    names: Vec<u8>,
    top: Vec<DictEntry>,
    strings: Vec<Vec<u8>>,
    gsubrs: Vec<Vec<u8>>,
    /// string id of each glyph's name (.notdef: 0)
    glyph_sids: Vec<u16>,
    pub charstrings: Vec<Vec<u8>>,
    private: Vec<DictEntry>,
    pub subrs: Vec<Vec<u8>>,
    pub default_width: f64,
    pub nominal_width: f64,
}

fn entry(d: &[DictEntry], op: u16) -> Option<&DictEntry> {
    d.iter().find(|e| e.op == op)
}

/// Top DICT operators whose operand is a string id
const SID_OPS: [u16; 8] = [0, 1, 2, 3, 4, 1200, 1221, 1222];
/// Private DICT operators about hints (the new charstrings have none)
const HINT_OPS: [u16; 14] = [6, 7, 8, 9, 10, 11, 1209, 1210, 1211, 1212, 1213, 1214, 1217, 1218];
const STD_STRINGS: u16 = 391;

impl Cff {
    pub fn parse(d: &[u8]) -> Result<Cff, String> {
        let hdr = *d.get(2).ok_or("bad CFF")? as usize;
        let (_, e_names) = read_index(d, hdr)?;
        let (tops, e_top) = read_index(d, e_names)?;
        let (strings, e_strings) = read_index(d, e_top)?;
        let (gsubrs, _) = read_index(d, e_strings)?;
        let top = read_dict(tops.first().ok_or("CFF without fonts")?)?;
        if entry(&top, 1230).is_some() {
            return Err("CID-keyed CFF fonts are not supported".into());
        }
        if entry(&top, 16).and_then(|e| e.vals.first()).is_some_and(|&v| v > 1.0) {
            return Err("CFF fonts with a custom encoding are not supported".into());
        }
        let cs_off = entry(&top, 17).and_then(|e| e.vals.first()).ok_or("CFF without CharStrings")?;
        let (charstrings, _) = read_index(d, *cs_off as usize)?;
        let n = charstrings.len();
        let mut glyph_sids = vec![0u16];
        match entry(&top, 15).and_then(|e| e.vals.first()).map(|v| *v as usize) {
            None | Some(0) => glyph_sids.extend(1..n as u16), // ISOAdobe: string id = glyph id
            Some(1..=2) => return Err("CFF with a predefined expert charset".into()),
            Some(o) => match d[o] {
                0 => {
                    for g in 1..n {
                        glyph_sids.push(be16(d, o + 1 + (g - 1) * 2)?);
                    }
                }
                f @ (1 | 2) => {
                    let mut p = o + 1;
                    while glyph_sids.len() < n {
                        let first = be16(d, p)?;
                        let left = if f == 1 { *d.get(p + 2).ok_or("bad CFF charset")? as u16 } else { be16(d, p + 2)? };
                        for k in 0..=left {
                            glyph_sids.push(first + k);
                        }
                        p += if f == 1 { 3 } else { 4 };
                    }
                    glyph_sids.truncate(n);
                }
                _ => return Err("bad CFF charset".into()),
            },
        }
        let pe = entry(&top, 18).ok_or("CFF without Private DICT")?;
        let (psize, poff) = (pe.vals[0] as usize, pe.vals[1] as usize);
        let private = read_dict(d.get(poff..poff + psize).ok_or("bad Private DICT")?)?;
        let subrs = match entry(&private, 19).and_then(|e| e.vals.first()) {
            Some(&o) => read_index(d, poff + o as usize)?.0,
            None => Vec::new(),
        };
        let num = |op, def| entry(&private, op).and_then(|e| e.vals.first().copied()).unwrap_or(def);
        Ok(Cff {
            header: d[..hdr].to_vec(),
            names: d[hdr..e_names].to_vec(),
            top,
            strings,
            gsubrs,
            glyph_sids,
            default_width: num(20, 0.0),
            nominal_width: num(21, 0.0),
            charstrings,
            private,
            subrs,
        })
    }

    /// The outline of a glyph and its width.
    pub fn glyph(&self, gid: usize) -> Result<(Vec<Contour>, f64), String> {
        let mut it = Interp { cff: self, stack: Vec::new(), x: 0.0, y: 0.0, stems: 0, width: None, seen_width: false,
                              contours: Vec::new(), cur: Vec::new(), depth: 0 };
        let cs = self.charstrings.get(gid).ok_or("glyph out of range")?;
        it.run(cs)?;
        it.close();
        let w = it.width.map_or(self.default_width, |w| w + self.nominal_width);
        Ok((it.contours, w))
    }

    /// A CFF table with only the glyphs `keep` (old glyph ids, .notdef first) and their new charstrings: no
    /// subroutines and no hints; the strings, charset and FontBBox follow.
    pub fn write(&self, keep: &[usize], charstrings: &[Vec<u8>], bbox: [i32; 4]) -> Vec<u8> {
        // strings still used get new ids, in order of use
        let mut strings: Vec<Vec<u8>> = Vec::new();
        let mut remap: HashMap<u16, u16> = HashMap::new();
        let mut sid = |old: u16| -> u16 {
            if old < STD_STRINGS {
                return old;
            }
            *remap.entry(old).or_insert_with(|| {
                strings.push(self.strings.get((old - STD_STRINGS) as usize).cloned().unwrap_or_default());
                STD_STRINGS + strings.len() as u16 - 1
            })
        };
        let mut top: Vec<DictEntry> = Vec::new();
        for e in &self.top {
            if [15, 17, 18, 5].contains(&e.op) {
                continue;
            }
            let mut e = e.clone();
            if SID_OPS.contains(&e.op) && e.vals.len() == 1 {
                e.raw = dict_int(sid(e.vals[0] as u16) as i32);
            }
            top.push(e);
        }
        top.push(DictEntry { op: 5, raw: bbox.iter().flat_map(|&v| dict_int(v)).collect(), vals: vec![] });
        let sids: Vec<u16> = keep[1..].iter().map(|&g| sid(self.glyph_sids[g])).collect();
        // charset: ranges (format 1) when shorter than the plain list (format 0)
        let mut ranges: Vec<(u16, u16)> = Vec::new();
        for &s in &sids {
            match ranges.last_mut() {
                Some((first, left)) if *first + *left + 1 == s && *left < 255 => *left += 1,
                _ => ranges.push((s, 0)),
            }
        }
        let charset: Vec<u8> = if ranges.len() * 3 < sids.len() * 2 {
            let mut c = vec![1u8];
            for (f, l) in &ranges {
                c.extend_from_slice(&f.to_be_bytes());
                c.push(*l as u8);
            }
            c
        } else {
            let mut c = vec![0u8];
            for s in &sids {
                c.extend_from_slice(&s.to_be_bytes());
            }
            c
        };
        let private: Vec<DictEntry> =
            self.private.iter().filter(|e| e.op != 19 && !HINT_OPS.contains(&e.op)).cloned().collect();
        let private_bytes = write_dict(&private);
        let cs_index = write_index(charstrings);
        let strings_index = write_index(&strings);
        let gsubrs_index = write_index(&[]);
        // offsets are written as 5-byte numbers, so the Top DICT's size does not depend on them
        let with = |cso: usize, cs: usize, po: usize| {
            let mut t = top.clone();
            t.push(DictEntry { op: 15, raw: dict_int5(cso as i32), vals: vec![] });
            t.push(DictEntry { op: 17, raw: dict_int5(cs as i32), vals: vec![] });
            let mut raw = dict_int5(private_bytes.len() as i32);
            raw.extend(dict_int5(po as i32));
            t.push(DictEntry { op: 18, raw, vals: vec![] });
            write_index(&[write_dict(&t)])
        };
        let top_len = with(0, 0, 0).len();
        let charset_at = self.header.len() + self.names.len() + top_len + strings_index.len() + gsubrs_index.len();
        let cs_at = charset_at + charset.len();
        let priv_at = cs_at + cs_index.len();
        let mut out = self.header.clone();
        out.extend_from_slice(&self.names);
        out.extend(with(charset_at, cs_at, priv_at));
        out.extend(strings_index);
        out.extend(gsubrs_index);
        out.extend(charset);
        out.extend(cs_index);
        out.extend(private_bytes);
        out
    }
}

fn bias(n: usize) -> i64 {
    if n < 1240 { 107 } else if n < 33900 { 1131 } else { 32768 }
}

/// Type 2 charstring interpreter (outlines only; hints are counted and skipped).
struct Interp<'a> {
    cff: &'a Cff,
    stack: Vec<f64>,
    x: f64,
    y: f64,
    stems: usize,
    width: Option<f64>,
    seen_width: bool,
    contours: Vec<Contour>,
    cur: Contour,
    depth: usize,
}

impl Interp<'_> {
    fn close(&mut self) {
        if self.cur.len() > 1 {
            let mut c = std::mem::take(&mut self.cur);
            c.push(Seg::Close);
            self.contours.push(c);
        }
        self.cur.clear();
    }
    /// The first stack-clearing operator may carry the advance width as an extra first operand.
    fn take_width(&mut self, odd_means_width: bool) {
        if !self.seen_width {
            self.seen_width = true;
            if odd_means_width && !self.stack.is_empty() {
                self.width = Some(self.stack.remove(0));
            }
        }
    }
    fn move_by(&mut self, dx: f64, dy: f64) {
        self.close();
        self.x += dx;
        self.y += dy;
        self.cur.push(Seg::Move(P(self.x, self.y)));
    }
    fn line_by(&mut self, dx: f64, dy: f64) {
        self.x += dx;
        self.y += dy;
        self.cur.push(Seg::Line(P(self.x, self.y)));
    }
    fn curve_by(&mut self, a: (f64, f64), b: (f64, f64), c: (f64, f64)) {
        let p1 = P(self.x + a.0, self.y + a.1);
        let p2 = P(p1.0 + b.0, p1.1 + b.1);
        let p3 = P(p2.0 + c.0, p2.1 + c.1);
        (self.x, self.y) = (p3.0, p3.1);
        self.cur.push(Seg::Curve(p1, p2, p3));
    }

    /// Runs a charstring; true when it ended the glyph.
    fn run(&mut self, code: &[u8]) -> Result<bool, String> {
        self.depth += 1;
        if self.depth > 12 {
            return Err("charstring subroutines nest too deep".into());
        }
        let mut i = 0;
        while i < code.len() {
            let b0 = code[i];
            i += 1;
            let s = &mut self.stack;
            match b0 {
                28 => {
                    s.push(i16::from_be_bytes([code[i], code[i + 1]]) as f64);
                    i += 2;
                    continue;
                }
                32..=246 => {
                    s.push(b0 as f64 - 139.0);
                    continue;
                }
                247..=250 => {
                    s.push((b0 as f64 - 247.0) * 256.0 + code[i] as f64 + 108.0);
                    i += 1;
                    continue;
                }
                251..=254 => {
                    s.push(-(b0 as f64 - 251.0) * 256.0 - code[i] as f64 - 108.0);
                    i += 1;
                    continue;
                }
                255 => {
                    s.push(i32::from_be_bytes(code[i..i + 4].try_into().unwrap()) as f64 / 65536.0);
                    i += 4;
                    continue;
                }
                _ => {}
            }
            match b0 {
                1 | 3 | 18 | 23 => {
                    self.take_width(self.stack.len() % 2 == 1);
                    self.stems += self.stack.len() / 2;
                }
                19 | 20 => {
                    self.take_width(self.stack.len() % 2 == 1);
                    self.stems += self.stack.len() / 2;
                    i += self.stems.div_ceil(8);
                }
                21 => {
                    self.take_width(self.stack.len() > 2);
                    let (dx, dy) = (self.stack[0], self.stack[1]);
                    self.move_by(dx, dy);
                }
                22 => {
                    self.take_width(self.stack.len() > 1);
                    let dx = self.stack[0];
                    self.move_by(dx, 0.0);
                }
                4 => {
                    self.take_width(self.stack.len() > 1);
                    let dy = self.stack[0];
                    self.move_by(0.0, dy);
                }
                5 => {
                    let a = std::mem::take(&mut self.stack);
                    for p in a.as_chunks::<2>().0 {
                        self.line_by(p[0], p[1]);
                    }
                }
                6 | 7 => {
                    let a = std::mem::take(&mut self.stack);
                    for (k, v) in a.iter().enumerate() {
                        if (k % 2 == 0) == (b0 == 6) { self.line_by(*v, 0.0) } else { self.line_by(0.0, *v) }
                    }
                }
                8 => {
                    let a = std::mem::take(&mut self.stack);
                    for c in a.as_chunks::<6>().0 {
                        self.curve_by((c[0], c[1]), (c[2], c[3]), (c[4], c[5]));
                    }
                }
                24 => {
                    let a = std::mem::take(&mut self.stack);
                    let n = a.len().saturating_sub(2);
                    for c in a[..n].as_chunks::<6>().0 {
                        self.curve_by((c[0], c[1]), (c[2], c[3]), (c[4], c[5]));
                    }
                    if a.len() >= 2 {
                        self.line_by(a[n], a[n + 1]);
                    }
                }
                25 => {
                    let a = std::mem::take(&mut self.stack);
                    let n = a.len().saturating_sub(6);
                    for p in a[..n].as_chunks::<2>().0 {
                        self.line_by(p[0], p[1]);
                    }
                    if a.len() >= 6 {
                        let c = &a[n..];
                        self.curve_by((c[0], c[1]), (c[2], c[3]), (c[4], c[5]));
                    }
                }
                26 | 27 => {
                    // vvcurveto / hhcurveto: optional first offset across, then groups of 4
                    let mut a = std::mem::take(&mut self.stack);
                    let mut first = 0.0;
                    if a.len() % 4 == 1 {
                        first = a.remove(0);
                    }
                    for c in a.as_chunks::<4>().0 {
                        if b0 == 26 {
                            self.curve_by((first, c[0]), (c[1], c[2]), (0.0, c[3]));
                        } else {
                            self.curve_by((c[0], first), (c[1], c[2]), (c[3], 0.0));
                        }
                        first = 0.0;
                    }
                }
                30 | 31 => {
                    // vhcurveto / hvcurveto: alternating start tangents; a last odd operand ends off-axis
                    let a = std::mem::take(&mut self.stack);
                    let mut horizontal = b0 == 31;
                    let mut k = 0;
                    while k + 4 <= a.len() {
                        let last = if a.len() - (k + 4) == 1 { a[k + 4] } else { 0.0 };
                        if horizontal {
                            self.curve_by((a[k], 0.0), (a[k + 1], a[k + 2]), (last, a[k + 3]));
                        } else {
                            self.curve_by((0.0, a[k]), (a[k + 1], a[k + 2]), (a[k + 3], last));
                        }
                        horizontal = !horizontal;
                        k += 4;
                    }
                }
                10 | 29 => {
                    let n = self.stack.pop().ok_or("empty stack at a subroutine call")? as i64;
                    let list = if b0 == 10 { &self.cff.subrs } else { &self.cff.gsubrs };
                    let sub = list.get((n + bias(list.len())) as usize).ok_or("subroutine out of range")?;
                    if self.run(sub)? {
                        self.depth -= 1;
                        return Ok(true);
                    }
                    continue;
                }
                11 => {
                    self.depth -= 1;
                    return Ok(false);
                }
                14 => {
                    self.take_width(self.stack.len() == 1 || self.stack.len() == 5);
                    if self.stack.len() >= 4 {
                        return Err("accented (seac) glyphs are not supported".into());
                    }
                    self.close();
                    self.stack.clear();
                    self.depth -= 1;
                    return Ok(true);
                }
                12 => {
                    let b1 = *code.get(i).ok_or("charstring ends early")?;
                    i += 1;
                    let a = std::mem::take(&mut self.stack);
                    match b1 {
                        35 if a.len() >= 12 => {
                            self.curve_by((a[0], a[1]), (a[2], a[3]), (a[4], a[5]));
                            self.curve_by((a[6], a[7]), (a[8], a[9]), (a[10], a[11]));
                        }
                        34 if a.len() >= 7 => {
                            self.curve_by((a[0], 0.0), (a[1], a[2]), (a[3], 0.0));
                            self.curve_by((a[4], 0.0), (a[5], -a[2]), (a[6], 0.0));
                        }
                        36 if a.len() >= 9 => {
                            self.curve_by((a[0], a[1]), (a[2], a[3]), (a[4], 0.0));
                            self.curve_by((a[5], 0.0), (a[6], a[7]), (a[8], -(a[1] + a[3] + a[7])));
                        }
                        37 if a.len() >= 11 => {
                            let dx: f64 = (0..5).map(|k| a[2 * k]).sum();
                            let dy: f64 = (0..5).map(|k| a[2 * k + 1]).sum();
                            let last = if dx.abs() > dy.abs() { (a[10], -dy) } else { (-dx, a[10]) };
                            self.curve_by((a[0], a[1]), (a[2], a[3]), (a[4], a[5]));
                            self.curve_by((a[6], a[7]), (a[8], a[9]), last);
                        }
                        _ => return Err(format!("charstring operator 12 {b1} not supported")),
                    }
                }
                _ => return Err(format!("charstring operator {b0} not supported")),
            }
            self.stack.clear();
        }
        self.depth -= 1;
        Ok(false)
    }
}

// ------------------------------------------------------------------------------------------------ new glyphs
fn cs_int(out: &mut Vec<u8>, v: i32) {
    match v {
        -107..=107 => out.push((v + 139) as u8),
        108..=1131 => out.extend_from_slice(&[((v - 108) / 256 + 247) as u8, ((v - 108) % 256) as u8]),
        -1131..=-108 => out.extend_from_slice(&[((-v - 108) / 256 + 251) as u8, ((-v - 108) % 256) as u8]),
        _ => {
            let b = (v.clamp(-32768, 32767) as i16).to_be_bytes();
            out.extend_from_slice(&[28, b[0], b[1]]);
        }
    }
}

fn round(v: f64) -> i32 {
    (v + 0.5).floor() as i32
}

/// One drawing step between whole-unit points, relative to the previous point.
#[derive(Clone, Copy)]
enum Step {
    Line(i32, i32),
    Curve([i32; 6]),
}

/// A charstring for an outline (points rounded to whole units, no hints), and its bounds. Uses the short
/// forms (hlineto, hvcurveto, ...) wherever a step runs along an axis, like fontTools' specializer.
pub fn encode(cs: &[Contour], width: f64, default_width: f64, nominal_width: f64) -> (Vec<u8>, Option<[i32; 4]>) {
    let mut out = Vec::new();
    let w = round(width);
    let mut pending_width = (w != round(default_width)).then(|| w - round(nominal_width));
    let (mut x, mut y) = (0i32, 0i32);
    for c in cs {
        let Some(Seg::Move(start)) = c.first() else { continue };
        let start = (round(start.0), round(start.1));
        let mut segs: Vec<Seg> = c[1..].iter().filter(|s| !matches!(s, Seg::Close | Seg::Move(_))).cloned().collect();
        if let Some(Seg::Line(p)) = segs.last() {
            if (round(p.0), round(p.1)) == start {
                segs.pop(); // the closing line is implied
            }
        }
        if segs.is_empty() {
            continue;
        }
        let mut args: Vec<i32> = pending_width.take().into_iter().collect();
        let (dx, dy) = (start.0 - x, start.1 - y);
        let op = if dx == 0 && dy != 0 {
            args.push(dy);
            4
        } else if dy == 0 {
            args.push(dx);
            22
        } else {
            args.extend([dx, dy]);
            21
        };
        emit(&mut out, &args, op);
        (x, y) = start;
        let mut steps = Vec::new();
        for s in &segs {
            match *s {
                Seg::Line(p) => {
                    let q = (round(p.0), round(p.1));
                    steps.push(Step::Line(q.0 - x, q.1 - y));
                    (x, y) = q;
                }
                Seg::Curve(a, b, p) => {
                    let (a, b, q) = ((round(a.0), round(a.1)), (round(b.0), round(b.1)), (round(p.0), round(p.1)));
                    steps.push(Step::Curve([a.0 - x, a.1 - y, b.0 - a.0, b.1 - a.1, q.0 - b.0, q.1 - b.1]));
                    (x, y) = q;
                }
                _ => {}
            }
        }
        emit_steps(&mut out, &steps);
    }
    if let Some(wv) = pending_width {
        cs_int(&mut out, wv);
    }
    out.push(14);
    (out, bounds(cs))
}

const MAX_ARGS: usize = 46;

fn emit(out: &mut Vec<u8>, args: &[i32], op: u8) {
    for &a in args {
        cs_int(out, a);
    }
    out.push(op);
}

fn emit_steps(out: &mut Vec<u8>, steps: &[Step]) {
    let mut i = 0;
    while i < steps.len() {
        match steps[i] {
            Step::Line(dx, dy) if dx == 0 || dy == 0 => {
                // hlineto / vlineto: alternating horizontal and vertical lines
                let horizontal = dy == 0;
                let (mut args, mut h) = (Vec::new(), horizontal);
                while i < steps.len() && args.len() < MAX_ARGS {
                    match steps[i] {
                        Step::Line(dx, 0) if h => args.push(dx),
                        Step::Line(0, dy) if !h => args.push(dy),
                        _ => break,
                    }
                    h = !h;
                    i += 1;
                }
                emit(out, &args, if horizontal { 6 } else { 7 });
            }
            Step::Line(..) => {
                let mut args = Vec::new();
                while i < steps.len() && args.len() + 2 <= MAX_ARGS {
                    match steps[i] {
                        Step::Line(dx, dy) if dx != 0 && dy != 0 => args.extend([dx, dy]),
                        _ => break,
                    }
                    i += 1;
                }
                // lines then a curve: rlinecurve
                if let Some(Step::Curve(c)) = steps.get(i) {
                    if args.len() + 6 <= MAX_ARGS && !axis_curve(c) {
                        args.extend(c);
                        i += 1;
                        emit(out, &args, 25);
                        continue;
                    }
                }
                emit(out, &args, 5);
            }
            Step::Curve(_) => {
                let (n, args, op) = best_curves(&steps[i..]);
                emit(out, &args, op);
                i += n;
            }
        }
    }
}

/// a curve one of the short forms can take
fn axis_curve(c: &[i32; 6]) -> bool {
    c[1] == 0 || c[0] == 0
}

/// The curve operator that covers the most of the curves starting at steps[0]: (curves, args, operator).
fn best_curves(steps: &[Step]) -> (usize, Vec<i32>, u8) {
    let curves: Vec<[i32; 6]> = steps.iter().map_while(|s| if let Step::Curve(c) = s { Some(*c) } else { None }).collect();
    let mut options: Vec<(usize, Vec<i32>, u8)> = Vec::new();
    // hvcurveto / vhcurveto: start tangents alternate; the last curve may end off the axis
    for start_h in [true, false] {
        let (mut args, mut h, mut n) = (Vec::new(), start_h, 0);
        for c in &curves {
            if args.len() + 5 > MAX_ARGS {
                break;
            }
            let starts = if h { c[1] == 0 } else { c[0] == 0 };
            if !starts {
                break;
            }
            let ends_on_axis = if h { c[4] == 0 } else { c[5] == 0 };
            if h {
                args.extend([c[0], c[2], c[3], c[5]]);
            } else {
                args.extend([c[1], c[2], c[3], c[4]]);
            }
            n += 1;
            if !ends_on_axis {
                args.push(if h { c[4] } else { c[5] });
                break;
            }
            h = !h;
        }
        if n > 0 {
            options.push((n, args, if start_h { 31 } else { 30 }));
        }
    }
    // hhcurveto / vvcurveto: all along one axis, the first may start off it
    for horizontal in [true, false] {
        let (mut args, mut n) = (Vec::new(), 0);
        for (k, c) in curves.iter().enumerate() {
            if args.len() + 5 > MAX_ARGS {
                break;
            }
            let (across_start, across_end) = if horizontal { (c[1], c[5]) } else { (c[0], c[4]) };
            if across_end != 0 || (k > 0 && across_start != 0) {
                break;
            }
            if k == 0 && across_start != 0 {
                args.push(across_start);
            }
            if horizontal {
                args.extend([c[0], c[2], c[3], c[4]]);
            } else {
                args.extend([c[1], c[2], c[3], c[5]]);
            }
            n += 1;
        }
        if n > 0 {
            options.push((n, args, if horizontal { 27 } else { 26 }));
        }
    }
    // rrcurveto: the curves none of the short forms takes
    let (mut args, mut n) = (Vec::new(), 0);
    for (k, c) in curves.iter().enumerate() {
        if args.len() + 6 > MAX_ARGS || (k > 0 && axis_curve(c)) {
            break;
        }
        args.extend(c);
        n += 1;
    }
    options.push((n, args, 8));
    // most curves per argument wins
    options.into_iter().min_by(|a, b| (a.1.len() * 1000 / a.0).cmp(&(b.1.len() * 1000 / b.0))).unwrap()
}

/// Bounds of an outline after rounding (as `encode` stores it).
pub fn bounds(cs: &[Contour]) -> Option<[i32; 4]> {
    let mut b: Option<[i32; 4]> = None;
    for c in cs {
        if c.len() < 2 {
            continue;
        }
        for s in c {
            let pts: Vec<P> = match *s {
                Seg::Move(p) | Seg::Line(p) => vec![p],
                Seg::Curve(a, bb, c) => vec![a, bb, c],
                Seg::Close => vec![],
            };
            for p in pts {
                let (px, py) = (round(p.0), round(p.1));
                b = Some(match b {
                    None => [px, py, px, py],
                    Some([a, bb, c, d]) => [a.min(px), bb.min(py), c.max(px), d.max(py)],
                });
            }
        }
    }
    b
}

// ------------------------------------------------------------------------------------------------ other tables
/// A cmap with one format 4 subtable for (code point, glyph id) pairs, under Unicode BMP and Windows Unicode.
pub fn cmap4(pairs: &[(u32, u16)]) -> Result<Vec<u8>, String> {
    let mut pairs = pairs.to_vec();
    pairs.sort();
    if pairs.iter().any(|p| p.0 > 0xFFFE) {
        return Err("code points beyond the BMP are not supported".into());
    }
    // segments of consecutive code points with consecutive glyph ids
    let mut segs: Vec<(u16, u16, u16)> = Vec::new(); // start, end, first glyph
    for &(cp, g) in &pairs {
        match segs.last_mut() {
            Some((s, e, g0)) if *e as u32 + 1 == cp && *g0 as u32 + (cp - *s as u32) == g as u32 => *e = cp as u16,
            _ => segs.push((cp as u16, cp as u16, g)),
        }
    }
    segs.push((0xFFFF, 0xFFFF, 0)); // the closing segment: 0xFFFF to glyph 0
    let n = segs.len();
    let mut es = 0;
    while (2usize << es) <= n {
        es += 1;
    }
    let sr = 2 * (1usize << es);
    let mut sub = Vec::new();
    for v in [4usize, 16 + n * 8, 0, n * 2, sr, es, n * 2 - sr] {
        sub.extend_from_slice(&(v as u16).to_be_bytes());
    }
    for s in &segs {
        sub.extend_from_slice(&s.1.to_be_bytes());
    }
    sub.extend_from_slice(&[0, 0]);
    for s in &segs {
        sub.extend_from_slice(&s.0.to_be_bytes());
    }
    for s in &segs {
        sub.extend_from_slice(&s.2.wrapping_sub(s.0).to_be_bytes());
    }
    for _ in &segs {
        sub.extend_from_slice(&[0, 0]);
    }
    let mut out = vec![0, 0, 0, 2];
    for (pid, eid) in [(0u16, 3u16), (3, 1)] {
        out.extend_from_slice(&pid.to_be_bytes());
        out.extend_from_slice(&eid.to_be_bytes());
        out.extend_from_slice(&20u32.to_be_bytes());
    }
    out.extend(sub);
    Ok(out)
}

/// GDEF with only a glyph class table, for the kept glyphs (new id = position in `keep`).
pub fn gdef_subset(gdef: &[u8], keep: &[usize]) -> Result<Vec<u8>, String> {
    let version = be32(gdef, 0)?;
    let rest = if version >= 0x0001_0002 { 5 } else { 4 };
    for k in 1..rest {
        if be16(gdef, 4 + k * 2)? != 0 {
            return Err("unexpected GDEF contents".into());
        }
    }
    let cd = be16(gdef, 4)? as usize;
    let mut class: HashMap<usize, u16> = HashMap::new();
    if cd != 0 {
        match be16(gdef, cd)? {
            1 => {
                let (start, count) = (be16(gdef, cd + 2)? as usize, be16(gdef, cd + 4)? as usize);
                for i in 0..count {
                    class.insert(start + i, be16(gdef, cd + 6 + i * 2)?);
                }
            }
            2 => {
                for r in 0..be16(gdef, cd + 2)? as usize {
                    let o = cd + 4 + r * 6;
                    let (s, e, c) = (be16(gdef, o)? as usize, be16(gdef, o + 2)? as usize, be16(gdef, o + 4)?);
                    for g in s..=e {
                        class.insert(g, c);
                    }
                }
            }
            _ => return Err("unexpected GDEF class table".into()),
        }
    }
    let mut ranges: Vec<(u16, u16, u16)> = Vec::new();
    for (new, old) in keep.iter().enumerate() {
        let c = class.get(old).copied().unwrap_or(0);
        if c == 0 {
            continue;
        }
        match ranges.last_mut() {
            Some((_, e, rc)) if *e as usize + 1 == new && *rc == c => *e = new as u16,
            _ => ranges.push((new as u16, new as u16, c)),
        }
    }
    let mut out = vec![0, 1, 0, 0, 0, 12, 0, 0, 0, 0, 0, 0];
    out.extend_from_slice(&2u16.to_be_bytes());
    out.extend_from_slice(&(ranges.len() as u16).to_be_bytes());
    for (s, e, c) in ranges {
        for v in [s, e, c] {
            out.extend_from_slice(&v.to_be_bytes());
        }
    }
    Ok(out)
}
