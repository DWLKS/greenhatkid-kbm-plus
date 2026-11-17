//! Textures inside the game's UI texture archive (BNTX): finding them, and writing BC4 (one channel, 8-byte
//! blocks) and ASTC 4x4 (16-byte blocks) pictures in the console's block-linear layout. The ASTC encoder is ARM's
//! astcenc (medium preset).

use std::collections::HashMap;

pub struct Tex {
    pub fmt: u32,
    pub w: usize,
    pub h: usize,
    pub bh_log2: u32,
    pub mips: u16,
    pub off: usize,
    pub size: usize,
}

fn rd<const N: usize>(b: &[u8], o: usize) -> Option<[u8; N]> {
    b.get(o..o + N)?.try_into().ok()
}

/// Every texture header ("BRTI") in the archive, by name.
pub fn all_textures(b: &[u8]) -> HashMap<String, Tex> {
    let mut out = HashMap::new();
    let mut i = 0;
    while i + 4 <= b.len() {
        let Some(p) = b[i..].windows(4).position(|w| w == b"BRTI") else { break };
        let o = i + p;
        i = o + 4;
        let parse = || -> Option<(String, Tex)> {
            let mips = u16::from_le_bytes(rd(b, o + 0x16)?);
            let fmt = u32::from_le_bytes(rd(b, o + 0x1C)?);
            let w = i32::from_le_bytes(rd(b, o + 0x24)?);
            let h = i32::from_le_bytes(rd(b, o + 0x28)?);
            let layout = u32::from_le_bytes(rd(b, o + 0x34)?);
            let data_size = u32::from_le_bytes(rd(b, o + 0x50)?) as usize;
            let ptrs = u64::from_le_bytes(rd(b, o + 0x70)?) as usize;
            let name_off = u64::from_le_bytes(rd(b, o + 0x60)?) as usize;
            if mips == 0 || w <= 0 || h <= 0 {
                return None;
            }
            let off = u64::from_le_bytes(rd(b, ptrs)?) as usize;
            let n = u16::from_le_bytes(rd(b, name_off)?) as usize;
            let name = std::str::from_utf8(b.get(name_off + 2..name_off + 2 + n)?).ok()?.to_string();
            if off + data_size > b.len() {
                return None;
            }
            Some((name, Tex { fmt, w: w as usize, h: h as usize, bh_log2: layout & 7, mips, off, size: data_size }))
        };
        if let Some((name, t)) = parse() {
            out.insert(name, t);
        }
    }
    out
}

/// Block height (in GOBs) the Switch uses for a level `hb` blocks tall.
fn mip_block_height(hb: usize, mut bh: usize) -> usize {
    while bh > 1 && hb <= (bh / 2) * 8 {
        bh /= 2;
    }
    bh
}

/// Byte offset of block (x, y) in the block-linear layout, for 8-byte (BC4) blocks.
fn offset_bc4(x: usize, y: usize, wb: usize, bh: usize) -> usize {
    let gw = (wb * 8).div_ceil(64);
    let gob = (y / (8 * bh)) * 512 * bh * gw + ((x * 8) / 64) * 512 * bh + ((y % (8 * bh)) / 8) * 512;
    gob + ((x & 1) << 3) + ((y & 1) << 4) + (((y >> 1) & 1) << 5) + (((x >> 1) & 1) << 6) + (((y >> 2) & 1) << 7)
        + (((x >> 2) & 1) << 8)
}

/// The same for 16-byte (ASTC) blocks; this game's GOB order is y0, y1, x0, y2, x1.
fn offset_astc(x: usize, y: usize, wb: usize, bh: usize) -> usize {
    let gw = (wb * 16).div_ceil(64);
    let xb = x * 16;
    let gob = (y / (8 * bh)) * 512 * bh * gw + (xb / 64) * 512 * bh + ((y % (8 * bh)) / 8) * 512;
    gob + (y & 1) * 16 + ((y >> 1) & 1) * 32 + (x & 1) * 64 + ((y >> 2) & 1) * 128 + ((x >> 1) & 1) * 256
}

/// One BC4 block (16 values, row by row) with the 8-level palette: exact for two-level content.
fn bc4_block(v: &[u8; 16]) -> [u8; 8] {
    let r0 = *v.iter().max().unwrap() as i32;
    let r1 = *v.iter().min().unwrap() as i32;
    let mut pal = [0i32; 8];
    pal[0] = r0;
    pal[1] = r1;
    for i in 1..7 {
        pal[1 + i] = ((7 - i as i32) * r0 + i as i32 * r1) / 7;
    }
    let mut bits = 0u64;
    if r0 != r1 {
        for (i, &p) in v.iter().enumerate() {
            let mut best = 0;
            for k in 1..8 {
                if (p as i32 - pal[k]).abs() < (p as i32 - pal[best]).abs() {
                    best = k;
                }
            }
            bits |= (best as u64) << (3 * i);
        }
    }
    let mut out = [0u8; 8];
    out[0] = r0 as u8;
    out[1] = r1 as u8;
    out[2..].copy_from_slice(&bits.to_le_bytes()[..6]);
    out
}

/// Writes a one-channel picture (texture size, row by row) into a BC4 texture.
pub fn write_bc4(b: &mut [u8], t: &Tex, img: &[u8]) {
    let (wb, hb) = (t.w.div_ceil(4), t.h.div_ceil(4));
    let bh = mip_block_height(hb, 1 << t.bh_log2);
    for by in 0..hb {
        for bx in 0..wb {
            let mut v = [0u8; 16];
            for r in 0..4 {
                for c in 0..4 {
                    let (x, y) = (bx * 4 + c, by * 4 + r);
                    if x < t.w && y < t.h {
                        v[r * 4 + c] = img[y * t.w + x];
                    }
                }
            }
            let o = t.off + offset_bc4(bx, by, wb, bh);
            b[o..o + 8].copy_from_slice(&bc4_block(&v));
        }
    }
}

/// Reads a BC4 texture's first level as a one-channel picture (texture size, row by row).
pub fn read_bc4(b: &[u8], t: &Tex) -> Vec<u8> {
    let (wb, hb) = (t.w.div_ceil(4), t.h.div_ceil(4));
    let bh = mip_block_height(hb, 1 << t.bh_log2);
    let mut img = vec![0u8; t.w * t.h];
    for by in 0..hb {
        for bx in 0..wb {
            let o = t.off + offset_bc4(bx, by, wb, bh);
            let Some(blk) = b.get(o..o + 8) else { continue };
            let (r0, r1) = (blk[0] as i32, blk[1] as i32);
            let mut pal = [r0, r1, 0, 0, 0, 0, 0, 255];
            if r0 > r1 {
                for i in 1..7 {
                    pal[1 + i] = ((7 - i as i32) * r0 + i as i32 * r1) / 7;
                }
            } else {
                for i in 1..5 {
                    pal[1 + i] = ((5 - i as i32) * r0 + i as i32 * r1) / 5;
                }
            }
            let mut bits = [0u8; 8];
            bits[..6].copy_from_slice(&blk[2..8]);
            let bits = u64::from_le_bytes(bits);
            for i in 0..16 {
                let (x, y) = (bx * 4 + i % 4, by * 4 + i / 4);
                if x < t.w && y < t.h {
                    img[y * t.w + x] = pal[((bits >> (3 * i)) & 7) as usize] as u8;
                }
            }
        }
    }
    img
}

pub struct Astc {
    ctx: ctt_astcenc::Context,
}

impl Astc {
    /// 4x4 blocks, sRGB (every ASTC texture we write is format 0x2d06).
    pub fn new() -> Result<Astc, String> {
        let cfg = ctt_astcenc::config_init(ctt_astcenc::Profile::LdrSrgb, 4, 4, 1, ctt_astcenc::Preset::Medium,
                                           ctt_astcenc::Flags::empty())
            .map_err(|e| e.to_string())?;
        Ok(Astc { ctx: ctt_astcenc::Context::new(&cfg).map_err(|e| e.to_string())? })
    }

    fn check(t: &Tex) -> Result<(), String> {
        if t.fmt != 0x2d06 || t.mips != 1 {
            return Err(format!("unexpected texture format {:#x}", t.fmt));
        }
        Ok(())
    }

    /// RGBA pixels (texture size) of an ASTC texture.
    pub fn read(&mut self, b: &[u8], t: &Tex) -> Result<Vec<u8>, String> {
        Self::check(t)?;
        let (wb, hb) = (t.w.div_ceil(4), t.h.div_ceil(4));
        let bh = mip_block_height(hb, 1 << t.bh_log2);
        let mut lin = vec![0u8; wb * hb * 16];
        for y in 0..hb {
            for x in 0..wb {
                let o = t.off + offset_astc(x, y, wb, bh);
                lin[(y * wb + x) * 16..][..16].copy_from_slice(&b[o..o + 16]);
            }
        }
        let mut px = vec![0u8; t.w * t.h * 4];
        let mut planes = [px.as_mut_ptr() as *mut std::ffi::c_void];
        let mut image = ctt_astcenc::bindings::astcenc_image {
            dim_x: t.w as u32,
            dim_y: t.h as u32,
            dim_z: 1,
            data_type: ctt_astcenc::bindings::astcenc_type_ASTCENC_TYPE_U8,
            data: planes.as_mut_ptr(),
        };
        self.ctx.decompress(&lin, &mut image, ctt_astcenc::Swizzle::IDENTITY).map_err(|e| e.to_string())?;
        self.ctx.decompress_reset().map_err(|e| e.to_string())?;
        Ok(px)
    }

    /// Encodes RGBA pixels (texture size) into the texture; the picture is edge-extended to whole blocks.
    pub fn write(&mut self, b: &mut [u8], t: &Tex, rgba: &[u8]) -> Result<(), String> {
        Self::check(t)?;
        let (wb, hb) = (t.w.div_ceil(4), t.h.div_ceil(4));
        let (pw, ph) = (wb * 4, hb * 4);
        let mut padded = vec![0u8; pw * ph * 4];
        for y in 0..ph {
            for x in 0..pw {
                let s = (y.min(t.h - 1) * t.w + x.min(t.w - 1)) * 4;
                padded[(y * pw + x) * 4..][..4].copy_from_slice(&rgba[s..s + 4]);
            }
        }
        let mut planes = [padded.as_mut_ptr() as *mut std::ffi::c_void];
        let mut image = ctt_astcenc::bindings::astcenc_image {
            dim_x: pw as u32,
            dim_y: ph as u32,
            dim_z: 1,
            data_type: ctt_astcenc::bindings::astcenc_type_ASTCENC_TYPE_U8,
            data: planes.as_mut_ptr(),
        };
        let mut lin = vec![0u8; wb * hb * 16];
        self.ctx.compress(&mut image, ctt_astcenc::Swizzle::IDENTITY, &mut lin).map_err(|e| e.to_string())?;
        self.ctx.compress_reset().map_err(|e| e.to_string())?;
        let bh = mip_block_height(hb, 1 << t.bh_log2);
        for y in 0..hb {
            for x in 0..wb {
                let o = t.off + offset_astc(x, y, wb, bh);
                if o + 16 > t.off + t.size {
                    return Err("texture layout does not fit".into());
                }
                b[o..o + 16].copy_from_slice(&lin[(y * wb + x) * 16..][..16]);
            }
        }
        Ok(())
    }
}

/// The "Basic Controls" keyboard and mouse pictures (drawn by tools/make_basic_controls.py; the guide lines are drawn for the
/// player's keys by guides.rs), stored as the archive holds them: name -> swizzled texture bytes.
const BASIC_CONTROLS: &[u8] = include_bytes!("../res/basic-controls.bin.zst");

pub fn put_basic_controls(b: &mut [u8], tex: &HashMap<String, Tex>) -> Result<(), String> {
    let raw = zstd::stream::decode_all(BASIC_CONTROLS).map_err(|e| e.to_string())?;
    let mut p = 0;
    while p < raw.len() {
        let n = u16::from_le_bytes([raw[p], raw[p + 1]]) as usize;
        let name = std::str::from_utf8(&raw[p + 2..p + 2 + n]).map_err(|e| e.to_string())?;
        let size = u32::from_le_bytes(raw[p + 2 + n..p + 6 + n].try_into().unwrap()) as usize;
        let data = &raw[p + 6 + n..p + 6 + n + size];
        let t = tex.get(name).ok_or(format!("texture {name} not found in the game's archive"))?;
        if t.size != size {
            return Err(format!("texture {name} has an unexpected size"));
        }
        b[t.off..t.off + size].copy_from_slice(data);
        p += 6 + n + size;
    }
    Ok(())
}
