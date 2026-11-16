//! The game files the in-game key icons need, read from the player's own .nup (nothing from the game ships with the
//! app): the zstd dictionaries, the icon font, the letter font and the UI texture archive. Kept, still
//! compressed, in a small cache so the .nup is needed once (about 65 MB, mostly the texture archive).

use crate::store::state_dir;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// (name in the cache, folder in the game, end of the file name): exactly one file name may match. The letter
/// font is found by the end of its name only (no other font name ends that way).
const WANT: [(&str, &str, &str); 4] = [
    ("ZsDic.pack.zs", "romfs/Pack", "/ZsDic.pack.zs"),
    ("External.bfotf.zs", "romfs/Font2/Scalable", "/External.Nin_Ounce_NVN2.bfotf.zs"),
    ("Letters.bfotf.zs", "romfs/Font2/Scalable", "Glyphs-v3-Plain.Nin_Ounce_NVN2.bfotf.zs"),
    ("Common.bntx.zs", "romfs/UI/Layout2Archive", "/Common.Product.100.Nin_Ounce_NVN2.bntx.zs"),
];

pub fn cache_dir() -> PathBuf {
    state_dir().join("game-cache")
}

pub fn cache_ready() -> bool {
    WANT.iter().all(|(n, _, _)| cache_dir().join(n).is_file())
}

fn u16le(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}
fn u32le(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn u64le(b: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(b[o..o + 8].try_into().unwrap())
}

struct NupFile {
    size: u64,
    offset: u64,
    stored: u64,
    codec: u8,
}

/// Lichii's NUP container: packages (base game, update) with exefs/romfs trees.
fn nup_members(f: &mut File) -> Result<Vec<(String, u32, NupFile)>, String> {
    let mut sb = [0u8; 0x80];
    f.read_exact(&mut sb).map_err(|e| e.to_string())?;
    if &sb[..4] != b"NUP\x01" {
        return Err("this is not a .nup game file".into());
    }
    let (toc_off, toc_size) = (u64le(&sb, 8), u64le(&sb, 16));
    let mut toc = vec![0u8; toc_size as usize];
    f.seek(SeekFrom::Start(toc_off)).map_err(|e| e.to_string())?;
    f.read_exact(&mut toc).map_err(|e| e.to_string())?;
    let pkg_count = u32le(&toc, 4) as usize;
    let dir_count = u64le(&toc, 8) as usize;
    let file_count = u64le(&toc, 16) as usize;
    let pool_off = u64le(&toc, 24) as usize;
    let name = |o: u32, l: u32| String::from_utf8_lossy(&toc[pool_off + o as usize..pool_off + (o + l) as usize]).into_owned();
    let mut pos = 0x30;
    let mut pkgs = Vec::new();
    for _ in 0..pkg_count {
        let (tid, exefs, romfs, ver, kind) = (u64le(&toc, pos), u32le(&toc, pos + 12), u32le(&toc, pos + 16), u32le(&toc, pos + 20), toc[pos + 24]);
        pkgs.push((tid, exefs, romfs, ver, kind));
        pos += 0x40;
    }
    let mut dirs = Vec::new();
    for _ in 0..dir_count {
        dirs.push((name(u32le(&toc, pos), u32le(&toc, pos + 4)), u32le(&toc, pos + 8) as usize, u32le(&toc, pos + 12) as usize));
        pos += 0x18;
    }
    let mut files = Vec::new();
    for _ in 0..file_count {
        let n = name(u32le(&toc, pos), u32le(&toc, pos + 4));
        let rec = NupFile { size: u64le(&toc, pos + 8), offset: u64le(&toc, pos + 16), stored: u64le(&toc, pos + 24), codec: toc[pos + 0x40] };
        files.push((n, rec));
        pos += 0x50;
    }
    let nchild = (pool_off - pos) / 4;
    let children: Vec<usize> = (0..nchild).map(|i| u32le(&toc, pos + i * 4) as usize).collect();
    let mut out = Vec::new();
    fn walk(d: usize, prefix: String, dirs: &[(String, usize, usize)], children: &[usize], dc: usize, out: &mut Vec<(String, usize)>,
            names: &[(String, NupFile)]) {
        let (_, fc, cc) = &dirs[d];
        for &c in &children[*fc..*fc + *cc] {
            if c < dc {
                walk(c, format!("{prefix}{}/", dirs[c].0), dirs, children, dc, out, names);
            } else {
                out.push((format!("{prefix}{}", names[c - dc].0), c - dc));
            }
        }
    }
    let mut paths = Vec::new();
    for (tid, exefs, romfs, ver, kind) in &pkgs {
        for (k, root) in [("exefs", *exefs), ("romfs", *romfs)] {
            if root != 0xFFFF_FFFF {
                walk(root as usize, format!("pkg{tid:x}_v{ver}_k{kind}/{k}/"), &dirs, &children, dir_count, &mut paths, &files);
            }
        }
    }
    for (path, i) in paths {
        let ver: u32 = path.split("_v").nth(1).and_then(|s| s.split('_').next()).and_then(|v| v.parse().ok()).unwrap_or(0);
        let r = &files[i].1;
        out.push((path, ver, NupFile { size: r.size, offset: r.offset, stored: r.stored, codec: r.codec }));
    }
    Ok(out)
}

/// Reads the four game files from the .nup (the newest package wins) into the cache.
pub fn extract(nup: &Path, log: &mut dyn FnMut(String)) -> Result<(), String> {
    let mut f = File::open(nup).map_err(|e| format!("cannot open the game file: {e}"))?;
    let members = nup_members(&mut f)?;
    std::fs::create_dir_all(cache_dir()).map_err(|e| e.to_string())?;
    for (name, dir, end) in WANT {
        let hits: Vec<&(String, u32, NupFile)> = members
            .iter()
            .filter(|(p, _, _)| {
                let (d, f) = p.rsplit_once('/').unwrap_or(("", p));
                d.ends_with(&format!("/{dir}")) && format!("/{f}").ends_with(end)
            })
            .collect();
        let mut names: Vec<&str> = hits.iter().map(|(p, _, _)| p.rsplit('/').next().unwrap_or("")).collect();
        names.sort();
        names.dedup();
        if names.len() > 1 {
            return Err(format!("This .nup has several files that could be the one needed ({}).", names.join(", ")));
        }
        // a file in both the base game and the update: the newest wins
        let best = hits
            .into_iter()
            .max_by_key(|(_, v, _)| *v)
            .ok_or(format!("This .nup does not look like the game ({dir}: {} is missing).", end.trim_start_matches('/')))?;
        let rec = &best.2;
        let mut data = vec![0u8; rec.stored as usize];
        f.seek(SeekFrom::Start(rec.offset)).map_err(|e| e.to_string())?;
        f.read_exact(&mut data).map_err(|e| e.to_string())?;
        if rec.codec == 1 {
            data = zstd::bulk::decompress(&data, rec.size as usize).map_err(|e| e.to_string())?;
        }
        let tmp = cache_dir().join(format!("{name}.tmp"));
        std::fs::write(&tmp, &data).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, cache_dir().join(name)).map_err(|e| e.to_string())?;
        log(format!("Read {name} ({} KB)", data.len() / 1024));
    }
    // files an older version kept in the cache
    for e in std::fs::read_dir(cache_dir()).into_iter().flatten().flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        if !WANT.iter().any(|(w, _, _)| *w == n) {
            let _ = std::fs::remove_file(e.path());
        }
    }
    Ok(())
}

pub fn cached(name: &str) -> Result<Vec<u8>, String> {
    std::fs::read(cache_dir().join(name)).map_err(|e| format!("missing game file {name}: {e}"))
}

/// The game's zstd dictionaries (a SARC inside ZsDic.pack.zs): name -> dictionary.
pub fn dictionaries() -> Result<HashMap<String, Vec<u8>>, String> {
    let raw = zstd::stream::decode_all(&cached("ZsDic.pack.zs")?[..]).map_err(|e| e.to_string())?;
    sarc_files(&raw)
}

fn sarc_files(d: &[u8]) -> Result<HashMap<String, Vec<u8>>, String> {
    if &d[..4] != b"SARC" {
        return Err("unexpected dictionary pack".into());
    }
    let hdr_len = u16le(d, 4) as usize;
    let data_off = u32le(d, 0xC) as usize;
    let sfat = hdr_len;
    let count = u16le(d, sfat + 6) as usize;
    let names_base = sfat + 0xC + count * 16 + 8;
    let mut out = HashMap::new();
    for i in 0..count {
        let o = sfat + 0xC + i * 16;
        let (attr, s, e) = (u32le(d, o + 4), u32le(d, o + 8) as usize, u32le(d, o + 12) as usize);
        let no = names_base + (attr & 0xFFFF) as usize * 4;
        let end = d[no..].iter().position(|&b| b == 0).map(|p| no + p).unwrap_or(no);
        out.insert(String::from_utf8_lossy(&d[no..end]).into_owned(), d[data_off + s..data_off + e].to_vec());
    }
    Ok(out)
}

pub fn zs_decompress(blob: &[u8], dict: &[u8]) -> Result<Vec<u8>, String> {
    let size = zstd::zstd_safe::get_frame_content_size(blob)
        .ok()
        .flatten()
        .ok_or("compressed game file without a size")?;
    let mut d = zstd::bulk::Decompressor::with_dictionary(dict).map_err(|e| e.to_string())?;
    d.decompress(blob, size as usize).map_err(|e| e.to_string())
}

/// A cached .zs file, decompressed while reading (the compressed copy is never all in memory).
pub fn zs_decompress_cached(name: &str, dict: &[u8]) -> Result<Vec<u8>, String> {
    let mut f = File::open(cache_dir().join(name)).map_err(|e| format!("missing game file {name}: {e}"))?;
    let mut head = [0u8; 18];
    let n = f.read(&mut head).map_err(|e| e.to_string())?;
    f.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let size = zstd::zstd_safe::get_frame_content_size(&head[..n]).ok().flatten().ok_or("compressed game file without a size")?;
    let mut out = Vec::with_capacity(size as usize);
    let mut d = zstd::stream::read::Decoder::with_dictionary(std::io::BufReader::new(f), dict).map_err(|e| e.to_string())?;
    d.read_to_end(&mut out).map_err(|e| e.to_string())?;
    Ok(out)
}

/// Compresses straight into a file (no second copy of a large file in memory); the frame records its size.
pub fn zs_compress_to_file(raw: &[u8], dict: &[u8], level: i32, path: &Path) -> Result<(), String> {
    let f = File::create(path).map_err(|e| e.to_string())?;
    let mut c = zstd::stream::write::Encoder::with_dictionary(std::io::BufWriter::new(f), level, dict).map_err(|e| e.to_string())?;
    c.include_contentsize(true).map_err(|e| e.to_string())?;
    c.set_pledged_src_size(Some(raw.len() as u64)).map_err(|e| e.to_string())?;
    c.write_all(raw).map_err(|e| e.to_string())?;
    c.finish().and_then(|mut w| w.flush()).map_err(|e| e.to_string())
}

pub fn zs_compress(raw: &[u8], dict: &[u8], level: i32) -> Result<Vec<u8>, String> {
    let mut c = zstd::bulk::Compressor::with_dictionary(level, dict).map_err(|e| e.to_string())?;
    c.compress(raw).map_err(|e| e.to_string())
}

/// The game's .bfotf font container: XOR of 4-byte big-endian words, then (magic, size) and the OTF.
const BFOTF_KEY: u32 = 0xA601_8502;
const BFOTF_MAGIC: u32 = 0x7F9A_0218;

fn xor_words(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len());
    for w in raw.as_chunks::<4>().0 {
        out.extend_from_slice(&(u32::from_be_bytes(*w) ^ BFOTF_KEY).to_be_bytes());
    }
    out
}

pub fn decrypt_bfotf(container: &[u8]) -> Result<Vec<u8>, String> {
    let dec = xor_words(container);
    let magic = u32::from_be_bytes(dec[..4].try_into().unwrap());
    let size = u32::from_be_bytes(dec[4..8].try_into().unwrap()) as usize;
    if magic != BFOTF_MAGIC {
        return Err(format!("unexpected font container (magic {magic:#x})"));
    }
    Ok(dec[8..8 + size].to_vec())
}

/// OTF -> container of exactly `total` bytes (the game's resource table stores file sizes).
pub fn encrypt_bfotf(otf: &[u8], total: usize) -> Vec<u8> {
    let mut raw = Vec::with_capacity(total);
    raw.extend_from_slice(&BFOTF_MAGIC.to_be_bytes());
    raw.extend_from_slice(&(otf.len() as u32).to_be_bytes());
    raw.extend_from_slice(otf);
    while raw.len() % 4 != 0 {
        raw.push(0);
    }
    while raw.len() < total {
        raw.push(0);
    }
    xor_words(&raw)
}
