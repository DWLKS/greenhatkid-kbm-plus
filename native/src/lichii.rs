//! Writes the settings into Lichii: input profile, Player 1 controls, real mouse on, and the camera patch.
//! Everything else in Lichii's settings is kept as it is, byte for byte.

use crate::keys::{binding, binding_names, Values};
use crate::store::{qt_config, EMULATORS, TITLE_ID};
use std::path::{Path, PathBuf};

const BUILD_ID: &str = "E0F3CCFAF50065AE4EA01EB5AEB13B1A45DA3348";
const PROFILE: &str = "GreenHatKid-KBM-Plus";
const DEG_PER_UNIT: f64 = 0.012; // degrees per mouse count for speed 1.0 (speed 5 = 0.06, like the original mod)
const MARKER: &[u8] = b"GHKMCAM1";
/// Our camera patch (walking camera + aiming + riding), built from patch/ in the repository.
const PATCH: &[u8] = include_bytes!("../../patch/prebuilt/E0F3CCFAF50065AE4EA01EB5AEB13B1A45DA3348.ips");

pub fn is_data_folder(p: &str) -> bool {
    !p.is_empty() && qt_config(p).is_file()
}

/// qt-config.ini as Apply first found it (kept: Remove puts Player 1's entries back from it).
fn backup_path(root: &str) -> PathBuf {
    qt_config(root).with_file_name("qt-config.ini.pre-greenhatkid-kbm-plus")
}

/// Our controller profile.
fn profile_path(root: &str) -> PathBuf {
    Path::new(root).join("config").join("input").join(format!("{PROFILE}.ini"))
}

/// qt-config.ini as lines, and its line ending.
fn read_qt_lines(root: &str) -> Result<(Vec<String>, &'static str, usize), String> {
    let raw = String::from_utf8(std::fs::read(qt_config(root)).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let eol = if raw.contains("\r\n") { "\r\n" } else { "\n" };
    Ok((raw.split(eol).map(String::from).collect(), eol, raw.len()))
}

/// Writes qt-config.ini back (through a temporary file), refusing anything much shorter than before.
fn write_qt_lines(root: &str, lines: &[String], eol: &str, old_len: usize) -> Result<(), String> {
    let new = lines.join(eol);
    if (new.len() as f64) < old_len as f64 * 0.9 {
        return Err("internal check failed: qt-config.ini would shrink; nothing written".into());
    }
    let qt = qt_config(root);
    let tmp = qt.with_file_name("qt-config.ini.tmp");
    std::fs::write(&tmp, new).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &qt).map_err(|e| e.to_string())
}

/// The running emulator's exe name (Lichii rewrites its settings when it closes), if any.
pub fn emulator_running() -> Option<String> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
    };
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return None;
        }
        let mut e: PROCESSENTRY32W = std::mem::zeroed();
        e.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut found = None;
        let mut ok = Process32FirstW(snap, &mut e) != 0;
        while ok {
            let name = crate::store::from_wide(&e.szExeFile).to_lowercase();
            if EMULATORS.iter().any(|n| name.starts_with(n)) {
                found = Some(name);
                break;
            }
            ok = Process32NextW(snap, &mut e) != 0;
        }
        CloseHandle(snap);
        found
    }
}

// ---------------------------------------------------------------- qt-config.ini editing (keeps everything else)
fn section(lines: &[String], name: &str) -> Option<(usize, usize)> {
    let head = format!("[{name}]");
    let start = lines.iter().position(|l| *l == head)?;
    let end = (start + 1..lines.len()).find(|&i| lines[i].starts_with('[')).unwrap_or(lines.len());
    Some((start, end))
}

fn set(lines: &mut Vec<String>, sect: &str, key: &str, value: &str, is_default: bool) -> Result<(), String> {
    let (start, end) = section(lines, sect).ok_or(format!("section [{sect}] not found in qt-config.ini"))?;
    let (dkey, vkey) = (format!("{key}\\default="), format!("{key}="));
    let (mut d, mut v) = (None, None);
    for (i, line) in lines.iter().enumerate().take(end).skip(start + 1) {
        if line.starts_with(&dkey) {
            d = Some(i);
        } else if line.starts_with(&vkey) {
            v = Some(i);
        }
    }
    let dl = format!("{key}\\default={}", if is_default { "true" } else { "false" });
    let vl = format!("{key}={value}");
    match (d, v) {
        (Some(d), Some(v)) => {
            lines[d] = dl;
            lines[v] = vl;
        }
        (Some(d), None) => {
            lines[d] = dl;
            lines.insert(d + 1, vl);
        }
        (None, Some(v)) => {
            lines[v] = vl;
            lines.insert(v, dl);
        }
        (None, None) => {
            let mut at = end;
            while at > start + 1 && lines[at - 1].trim().is_empty() {
                at -= 1;
            }
            lines.insert(at, vl);
            lines.insert(at, dl);
        }
    }
    Ok(())
}

fn esc(s: &str) -> String {
    s.replace(':', "$0").replace(',', "$1")
}

/// Player 1's bindings, in the order Lichii lists them.
pub fn controls(v: &Values) -> Result<Vec<(&'static str, String)>, String> {
    let none = binding("none")?;
    let mut b = std::collections::HashMap::new();
    for n in binding_names() {
        b.insert(n, binding(v.key(n))?);
    }
    let lstick = format!(
        "engine:analog_from_button,right:{},modifier_scale:{:.6},up:{},down:{},left:{},modifier:{}",
        esc(&b["move_right"]), v.walk_speed, esc(&b["move_up"]), esc(&b["move_down"]), esc(&b["move_left"]),
        esc(&b["walk_key"])
    );
    // The patch reads the mouse itself: the right stick must not use it, or Lichii would turn "mouse moved while a
    // button is held" into stick input (attacking would also move the camera).
    let n = esc(&none);
    let rstick = format!("engine:analog_from_button,up:{n},down:{n},left:{n},right:{n},modifier:{n},modifier_scale:0.500000");
    Ok(vec![
        ("button_a", b["button_a"].clone()), ("button_b", b["button_b"].clone()), ("button_x", b["button_x"].clone()),
        ("button_y", b["button_y"].clone()), ("button_lstick", b["button_l3"].clone()),
        ("button_rstick", b["button_r3"].clone()), ("button_l", b["button_l"].clone()),
        ("button_r", b["button_r"].clone()), ("button_zl", b["button_zl"].clone()),
        ("button_zr", b["button_zr"].clone()), ("button_plus", b["button_plus"].clone()),
        ("button_minus", b["button_minus"].clone()), ("button_dleft", b["dpad_left"].clone()),
        ("button_dup", b["dpad_up"].clone()), ("button_dright", b["dpad_right"].clone()),
        ("button_ddown", b["dpad_down"].clone()), ("button_slleft", none.clone()), ("button_srleft", none.clone()),
        ("button_home", b["button_home"].clone()), ("button_screenshot", b["button_screenshot"].clone()),
        ("button_slright", none.clone()), ("button_srright", none.clone()), ("lstick", lstick), ("rstick", rstick),
        ("motionleft", none.clone()), ("motionright", none),
    ])
}

/// Degrees per mouse step: (yaw, pitch); a negative pitch inverts up/down.
pub fn speeds(v: &Values) -> (f32, f32) {
    let sx = if v.split_axes { v.mouse_sens_x } else { v.mouse_sens };
    let sy = if v.split_axes { v.mouse_sens_y } else { v.mouse_sens };
    let (yaw, pitch) = (sx * DEG_PER_UNIT, sy * DEG_PER_UNIT);
    (yaw as f32, if v.invert_y { -pitch as f32 } else { pitch as f32 })
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn patch_dir(root: &str) -> PathBuf {
    Path::new(root).join("load").join(TITLE_ID).join("MouseCamera-Native").join("exefs")
}

fn patch_file(root: &str) -> PathBuf {
    patch_dir(root).join(format!("{BUILD_ID}.ips"))
}

/// Our camera patch with the speed written after its GHKMCAM1 marker.
fn patch_bytes(v: &Values) -> Result<Vec<u8>, String> {
    let mut data = PATCH.to_vec();
    let at = find(&data, MARKER).ok_or("the camera patch has no settings block")? + MARKER.len();
    let (yaw, pitch) = speeds(v);
    data[at..at + 4].copy_from_slice(&yaw.to_le_bytes());
    data[at + 4..at + 8].copy_from_slice(&pitch.to_le_bytes());
    Ok(data)
}

/// Installs our camera patch for these speeds.
fn write_patch(root: &str, v: &Values) -> Result<(), String> {
    let data = patch_bytes(v)?;
    let path = patch_file(root);
    std::fs::create_dir_all(patch_dir(root)).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(path.with_extension("ips.off"));
    let tmp = path.with_extension("ips.tmp");
    std::fs::write(&tmp, &data).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

pub enum PatchState {
    Ours(f32),
    Other,
    Missing,
}

/// Which camera patch Lichii will load, and its speed when it is ours.
pub fn installed_patch_state(root: &str) -> PatchState {
    let Ok(data) = std::fs::read(patch_file(root)) else { return PatchState::Missing };
    let (Some(a), Some(o)) = (find(&data, MARKER), find(PATCH, MARKER)) else { return PatchState::Other };
    let same = data.len() == PATCH.len() && a == o && data[..a + 8] == PATCH[..o + 8] && data[a + 16..] == PATCH[o + 16..];
    if !same {
        return PatchState::Other;
    }
    let yaw = f32::from_le_bytes(data[a + 8..a + 12].try_into().unwrap());
    PatchState::Ours((yaw as f64 / DEG_PER_UNIT) as f32)
}

/// Whether Lichii already has what Apply would write for `v`: (Player 1's keys, profile and mouse; the camera
/// patch at these speeds and the hide-pointer setting).
pub fn applied(root: &str, v: &Values) -> (bool, bool) {
    let Ok(text) = std::fs::read_to_string(qt_config(root)) else {
        return (false, false);
    };
    let mut section = "";
    let mut values = std::collections::HashMap::new();
    for line in text.lines() {
        if line.starts_with('[') {
            section = line;
        } else if let Some((k, val)) = line.split_once('=') {
            if section == "[Controls]" || (section == "[UI]" && k == "hideInactiveMouse") {
                values.insert(k, val);
            }
        }
    }
    let controls = controls(v).is_ok_and(|pairs| {
        pairs.iter().all(|(k, val)| values.get(format!("player_0_{k}").as_str()) == Some(&format!("\"{val}\"").as_str()))
    }) && values.get("player_0_profile_name") == Some(&PROFILE)
        && values.get("mouse_enabled") == Some(&"true");
    let hide = values.get("hideInactiveMouse") == Some(&if v.hide_cursor { "true" } else { "false" });
    let patch = std::fs::read(patch_file(root)).ok();
    let camera = hide && patch.is_some() && patch_bytes(v).ok() == patch;
    (controls, camera)
}

/// The qt-config.ini entries Apply writes: (section, key).
fn written_entries() -> Vec<(&'static str, String)> {
    let mut v: Vec<(&'static str, String)> = controls(&Values::default())
        .unwrap_or_default()
        .into_iter()
        .map(|(k, _)| ("Controls", format!("player_0_{k}")))
        .collect();
    for k in ["player_0_type", "player_0_connected", "player_0_profile_name", "mouse_enabled"] {
        v.push(("Controls", k.to_string()));
    }
    v.push(("UI", "hideInactiveMouse".to_string()));
    v
}

/// The `key\default=` and `key=` lines of an entry in a section, if any.
fn entry_lines(lines: &[String], sect: &str, key: &str) -> (Option<String>, Option<String>) {
    let Some((start, end)) = section(lines, sect) else { return (None, None) };
    let (dkey, vkey) = (format!("{key}\\default="), format!("{key}="));
    let d = lines[start + 1..end].iter().find(|l| l.starts_with(&dkey)).cloned();
    let v = lines[start + 1..end].iter().find(|l| l.starts_with(&vkey)).cloned();
    (d, v)
}

/// What of the mod is in Lichii now.
#[derive(Clone, Copy, PartialEq, Default)]
pub struct Installed {
    /// Player 1 uses our controller profile
    pub settings: bool,
    pub profile: bool,
    pub patch: bool,
    pub icons: bool,
}

impl Installed {
    pub fn any(&self) -> bool {
        self.settings || self.profile || self.patch || self.icons
    }
}

pub fn installed(root: &str) -> Installed {
    let qt = std::fs::read_to_string(qt_config(root)).unwrap_or_default();
    let lines: Vec<String> = qt.lines().map(String::from).collect();
    Installed {
        settings: entry_lines(&lines, "Controls", "player_0_profile_name").1.as_deref()
            == Some(&format!("player_0_profile_name={PROFILE}")),
        profile: profile_path(root).is_file(),
        patch: matches!(installed_patch_state(root), PatchState::Ours(_)),
        icons: crate::icons::installed(root),
    }
}

/// Undoes Apply: Player 1's entries in qt-config.ini as the pre-mod backup has them (an entry the backup lacks is
/// removed: Apply added it), the controller profile, our camera patch and the in-game key icons. Everything else in
/// Lichii stays as it is, and so does the backup. `log` gets one line per step.
pub fn remove(root: &str, log: &mut dyn FnMut(String)) -> Result<(), String> {
    let backup = backup_path(root);
    let mut problems = Vec::new();
    if installed(root).settings || backup.is_file() {
        match std::fs::read_to_string(&backup) {
            Ok(old) => {
                let (mut lines, eol, old_len) = read_qt_lines(root)?;
                let old: Vec<String> = old.lines().map(String::from).collect();
                for (sect, key) in written_entries() {
                    let Some((start, end)) = section(&lines, sect) else { continue };
                    let (dkey, vkey) = (format!("{key}\\default="), format!("{key}="));
                    let (od, ov) = entry_lines(&old, sect, &key);
                    // drop what is there now, then put the backup's lines back where the entry was
                    let at = (start + 1..end).find(|&i| lines[i].starts_with(&dkey) || lines[i].starts_with(&vkey));
                    let Some(at) = at else { continue };
                    let mut i = start + 1;
                    let mut end = end;
                    while i < end {
                        if lines[i].starts_with(&dkey) || lines[i].starts_with(&vkey) {
                            lines.remove(i);
                            end -= 1;
                        } else {
                            i += 1;
                        }
                    }
                    for l in [ov, od].into_iter().flatten() {
                        lines.insert(at, l);
                    }
                }
                write_qt_lines(root, &lines, eol, old_len)?;
                log("Player 1's keys and mouse setting restored from before the first Apply".into());
            }
            Err(_) => problems.push(
                "Lichii's settings backup (config\\qt-config.ini.pre-greenhatkid-kbm-plus) is missing, so Player 1's \
                 keys were left as they are"
                    .to_string(),
            ),
        }
    }
    let profile = profile_path(root);
    if profile.is_file() {
        match std::fs::remove_file(&profile) {
            Ok(()) => log(format!("Controller profile {PROFILE} removed")),
            Err(e) => problems.push(format!("the controller profile could not be removed ({e})")),
        }
    }
    if matches!(installed_patch_state(root), PatchState::Ours(_)) {
        let dir = patch_dir(root);
        match std::fs::remove_file(patch_file(root)) {
            Ok(()) => {
                // the folders too, when nothing else is in them
                let _ = std::fs::remove_dir(&dir);
                if let Some(parent) = dir.parent() {
                    let _ = std::fs::remove_dir(parent);
                }
                log("Camera patch removed".into());
            }
            Err(e) => problems.push(format!("the camera patch could not be removed ({e})")),
        }
    }
    let icons = Path::new(root).join("load").join(TITLE_ID).join(crate::icons::MOD);
    if icons.is_dir() {
        match std::fs::remove_dir_all(&icons) {
            Ok(()) => log("In-game key icons removed".into()),
            Err(e) => problems.push(format!("the in-game key icons could not be removed ({e})")),
        }
    }
    if problems.is_empty() { Ok(()) } else { Err(problems.join("; ")) }
}

/// Writes everything; `log` gets one line per step.
pub fn apply(root: &str, v: &Values, log: &mut dyn FnMut(String)) -> Result<(), String> {
    let pairs = controls(v)?;
    let profile_file = profile_path(root);
    std::fs::create_dir_all(profile_file.parent().unwrap()).map_err(|e| e.to_string())?;
    let mut profile = String::from("[Controls]\n");
    for (k, val) in &pairs {
        profile += &format!("{k}\\default=false\n{k}=\"{val}\"\n");
    }
    std::fs::write(profile_file, profile).map_err(|e| e.to_string())?;
    log(format!("Input profile written: {PROFILE}"));

    let (mut lines, eol, old_len) = read_qt_lines(root)?;
    for (k, val) in &pairs {
        set(&mut lines, "Controls", &format!("player_0_{k}"), &format!("\"{val}\""), false)?;
    }
    set(&mut lines, "Controls", "player_0_type", "0", true)?;
    set(&mut lines, "Controls", "player_0_connected", "true", true)?;
    set(&mut lines, "Controls", "player_0_profile_name", PROFILE, false)?;
    set(&mut lines, "Controls", "mouse_enabled", "true", false)?;
    set(&mut lines, "UI", "hideInactiveMouse", if v.hide_cursor { "true" } else { "false" }, v.hide_cursor)?;
    let backup = backup_path(root);
    if !backup.exists() {
        std::fs::copy(qt_config(root), &backup).map_err(|e| e.to_string())?;
    }
    write_qt_lines(root, &lines, eol, old_len)?;
    log("Lichii controls updated (Player 1, real mouse on)".into());

    write_patch(root, v)?;
    let (yaw, pitch) = speeds(v);
    log(format!("Camera patch installed (aim + horse), {:.3} / {:.3} degrees per mouse step", yaw, pitch.abs()));
    Ok(())
}
