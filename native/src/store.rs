//! Paths, the settings and status files, and the messages between the agent and the settings window.

use std::path::{Path, PathBuf};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW, RegisterWindowMessageW};

pub const AGENT_CLASS: &str = "GreenHatKid-KBM-Plus-agent";
pub const CMD_OPEN: usize = 1;
pub const CMD_RELOAD: usize = 2;
pub const CMD_QUIT: usize = 3;
pub const CMD_TRAY_HINT: usize = 4;
/// to the settings window: the tray's Quit was chosen, ask the player there
pub const CMD_ASK_QUIT: usize = 5;
/// The app's name: the settings window's title (also how a second start finds the open window) and the tray's.
pub const APP_NAME: &str = "Green Hat Kid Keyboard & Mouse Plus";
/// The game's title ID (its folder names in Lichii).
pub const TITLE_ID: &str = "0400486024DEC000";
/// Exe names (start) of Lichii and the emulators it is built on.
pub const EMULATORS: [&str; 9] = ["lichii", "fritchii", "eden", "yuzu", "citron", "sudachi", "suyu", "torzu", "eglantine"];

/// A null-terminated UTF-16 string for Windows calls.
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn from_wide(buf: &[u16]) -> String {
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..n])
}

pub fn state_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("GreenHatKid-KBM-Plus")
}

pub fn settings_path() -> PathBuf {
    state_dir().join("settings.json")
}

pub fn status_path() -> PathBuf {
    state_dir().join("status.json")
}

/// Lichii's main settings file in a data folder.
pub fn qt_config(root: &str) -> PathBuf {
    Path::new(root).join("config").join("qt-config.ini")
}

/// The full exe path of a running process, if Windows tells.
pub fn process_path(pid: u32) -> Option<String> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    unsafe {
        let p = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if p.is_null() {
            return None;
        }
        let mut buf = [0u16; 1024];
        let mut n = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(p, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut n);
        CloseHandle(p);
        (ok != 0).then(|| String::from_utf16_lossy(&buf[..n as usize]))
    }
}

/// The parts of settings.json the agent needs.
pub struct AgentSettings {
    pub root: String,
    pub hide_cursor: bool,
}

pub fn load_agent_settings() -> AgentSettings {
    let mut s = AgentSettings { root: String::new(), hide_cursor: true };
    if let Ok(text) = std::fs::read_to_string(settings_path()) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(r) = v.get("root").and_then(|r| r.as_str()) {
                s.root = r.to_string();
            }
            if let Some(h) = v.pointer("/values/hide_cursor").and_then(|h| h.as_bool()) {
                s.hide_cursor = h;
            }
        }
    }
    if s.root.is_empty() {
        s.root = find_data_folder().unwrap_or_default();
    }
    s
}

/// Lichii's data folder (the one with config\qt-config.ini) when settings.json does not name one yet.
pub fn find_data_folder() -> Option<String> {
    let appdata = PathBuf::from(std::env::var_os("APPDATA")?);
    ["lichii", "fritchii", "eden"]
        .iter()
        .map(|n| appdata.join(n))
        .map(|p| p.to_string_lossy().into_owned())
        .find(|p| qt_config(p).is_file())
}

pub fn write_status(state: &str, text: &str, panning: &str, panning_help: &str) {
    let dir = state_dir();
    let _ = std::fs::create_dir_all(&dir);
    let body = serde_json::json!({"state": state, "text": text, "panning": panning, "panning_help": panning_help}).to_string();
    let tmp = dir.join("status.json.tmp");
    if std::fs::write(&tmp, body).is_ok() {
        let _ = std::fs::rename(&tmp, status_path());
    }
}

pub fn control_message() -> u32 {
    let name = wide("GreenHatKid-KBM-Plus.control");
    unsafe { RegisterWindowMessageW(name.as_ptr()) }
}

pub fn agent_window() -> HWND {
    let class = wide(AGENT_CLASS);
    unsafe { FindWindowW(class.as_ptr(), std::ptr::null()) }
}

/// The open settings window of this app (same title and same exe), if any.
pub fn settings_window() -> Option<HWND> {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
    let title = wide(APP_NAME);
    let h = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
    if h.is_null() {
        return None;
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(h, &mut pid) };
    let path = process_path(pid)?;
    let ours = std::env::current_exe().ok()?;
    path.eq_ignore_ascii_case(&ours.to_string_lossy()).then_some(h)
}

pub fn tell_agent(cmd: usize) -> bool {
    let h = agent_window();
    if h.is_null() {
        return false;
    }
    unsafe { PostMessageW(h, control_message(), cmd, 0) != 0 }
}
