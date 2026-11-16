//! The mouse capture for the native camera (the same method as the original mod's helper program).
//!
//! The game patch reads the real mouse through Lichii, which only reports where the pointer is on the game picture.
//! While the game is in front and the pointer is over the picture, this keeps the pointer within half a period
//! (512 x 384 game pixels) of the middle by moving it back by whole periods; the patch undoes jumps of exactly that
//! size. It also switches off Windows pointer acceleration and sets the pointer speed so one mouse count is one
//! pixel of the 1280x720 game picture (1:1), restored as soon as the game is not in front (never saved; a restore
//! file brings the settings back after a crash).

use crate::panning::AutoPanning;
use crate::store::{process_path, qt_config, state_dir, EMULATORS, TITLE_ID};
use std::time::Instant;
use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const GAME_TITLES: [&str; 2] = ["Ocarina", "\u{30aa}\u{30ab}\u{30ea}\u{30ca}"];
const WRAP_X: f64 = 512.0; // game pixels; must match WRAP_X / WRAP_Y in the camera patch (cave.c)
const WRAP_Y: f64 = 384.0;
/// Windows pointer speed 1..20 -> pixels moved per mouse count (acceleration off).
const SPEED_MULT: [f64; 21] = [
    0.0, 1.0 / 32.0, 1.0 / 16.0, 1.0 / 8.0, 2.0 / 8.0, 3.0 / 8.0, 4.0 / 8.0, 5.0 / 8.0, 6.0 / 8.0, 7.0 / 8.0, 1.0,
    1.25, 1.5, 1.75, 2.0, 2.25, 2.5, 2.75, 3.0, 3.25, 3.5,
];
/// Lichii aspect_ratio 0..4 -> picture height / width (0 = Stretch: the whole render area).
const ASPECTS: [f64; 5] = [9.0 / 16.0, 3.0 / 4.0, 9.0 / 21.0, 10.0 / 16.0, 0.0];

type Rect = (i32, i32, i32, i32);

fn window_text(h: HWND) -> String {
    let mut buf = [0u16; 512];
    let n = unsafe { GetWindowTextW(h, buf.as_mut_ptr(), buf.len() as i32) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

fn class_name(h: HWND) -> String {
    let mut buf = [0u16; 64];
    let n = unsafe { GetClassNameW(h, buf.as_mut_ptr(), buf.len() as i32) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

fn pid_of(h: HWND) -> u32 {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(h, &mut pid) };
    pid
}

fn exe_name(pid: u32) -> String {
    process_path(pid).map(|p| p.rsplit('\\').next().unwrap_or("").to_lowercase()).unwrap_or_default()
}

fn is_game_title(h: HWND) -> bool {
    let t = window_text(h);
    GAME_TITLES.iter().any(|s| t.contains(s))
}

unsafe extern "system" fn find_cb(h: HWND, lp: LPARAM) -> BOOL {
    let out = &mut *(lp as *mut HWND);
    if IsWindowVisible(h) != 0 && is_game_title(h) {
        let pid = pid_of(h);
        if pid != GetCurrentProcessId()
            && (class_name(h).starts_with("Qt") || EMULATORS.iter().any(|e| exe_name(pid).starts_with(e)))
        {
            *out = h;
            return 0;
        }
    }
    1
}

/// The emulator window whose title names the game (never this app, never a browser tab).
fn find_game_window() -> HWND {
    let mut found: HWND = std::ptr::null_mut();
    unsafe { EnumWindows(Some(find_cb), &mut found as *mut HWND as LPARAM) };
    found
}

/// Lichii's aspect ratio for the game (its per-game settings first), as picture height / width.
fn read_aspect(data_folder: &str) -> f64 {
    fn read(path: std::path::PathBuf) -> Option<i64> {
        let text = std::fs::read_to_string(path).ok()?;
        text.lines()
            .filter_map(|l| l.strip_prefix("aspect_ratio="))
            .next_back()
            .and_then(|v| v.trim().parse().ok())
    }
    let mut v = None;
    if !data_folder.is_empty() {
        let custom = std::path::Path::new(data_folder).join("config").join("custom").join(format!("{TITLE_ID}.ini"));
        v = read(custom).or_else(|| read(qt_config(data_folder)));
    }
    match v {
        Some(i) if (0..=4).contains(&i) => ASPECTS[i as usize],
        _ => 9.0 / 16.0,
    }
}

fn fit(r: Rect, aspect: f64) -> Rect {
    let (w, h) = (r.2 - r.0, r.3 - r.1);
    let (mut pw, mut ph) = (w, h);
    if aspect > 0.0 {
        if h as f64 / w as f64 > aspect {
            ph = (w as f64 * aspect + 0.5) as i32;
        } else {
            pw = (h as f64 / aspect + 0.5) as i32;
        }
    }
    let (x0, y0) = (r.0 + (w - pw) / 2, r.1 + (h - ph) / 2);
    (x0, y0, x0 + pw, y0 + ph)
}

unsafe extern "system" fn largest_child_cb(h: HWND, lp: LPARAM) -> BOOL {
    let best = &mut *(lp as *mut (Option<Rect>, i64));
    let mut r: RECT = std::mem::zeroed();
    if IsWindowVisible(h) != 0 && GetWindowRect(h, &mut r) != 0 {
        let a = (r.right - r.left) as i64 * (r.bottom - r.top) as i64;
        if a > best.1 {
            *best = (Some((r.left, r.top, r.right, r.bottom)), a);
        }
    }
    1
}

/// Screen rectangle of the game picture, and whether the emulator shows its game picture: it draws into its largest
/// child window, which it hides while its shader screen shows.
fn picture_rect(top: HWND, aspect: f64) -> (Option<Rect>, bool) {
    let mut best: (Option<Rect>, i64) = (None, 0);
    unsafe { EnumChildWindows(top, Some(largest_child_cb), &mut best as *mut _ as LPARAM) };
    let shown = best.0.is_some_and(|r| r.2 - r.0 > 64 && r.3 - r.1 > 36);
    // no usable child window: the whole client area
    let area = match best.0 {
        Some(r) if r.2 - r.0 > 64 => Some(r),
        _ => client_rect(top),
    };
    (area.filter(|r| r.2 - r.0 >= 64 && r.3 - r.1 >= 36).map(|r| fit(r, aspect)), shown)
}

/// A window's client area in screen coordinates.
fn client_rect(h: HWND) -> Option<Rect> {
    unsafe {
        let mut cr: RECT = std::mem::zeroed();
        if GetClientRect(h, &mut cr) == 0 {
            return None;
        }
        let mut a = POINT { x: cr.left, y: cr.top };
        let mut b = POINT { x: cr.right, y: cr.bottom };
        ClientToScreen(h, &mut a);
        ClientToScreen(h, &mut b);
        Some((a.x, a.y, b.x, b.y))
    }
}

/// Clock hand: move the pointer back by whole periods so it ends within half a period of the middle.
fn wrap(r: Rect, x: i32, y: i32) -> Option<(i32, i32)> {
    let (cx, cy) = ((r.0 + r.2) / 2, (r.1 + r.3) / 2);
    let wx = ((r.2 - r.0) as f64 * WRAP_X / 1280.0 + 0.5) as i32;
    let wy = ((r.3 - r.1) as f64 * WRAP_Y / 720.0 + 0.5) as i32;
    if wx < 2 || wy < 2 {
        return None;
    }
    let (mut dx, mut dy) = (x - cx, y - cy);
    while dx > wx / 2 {
        dx -= wx;
    }
    while dx < -(wx / 2) {
        dx += wx;
    }
    while dy > wy / 2 {
        dy -= wy;
    }
    while dy < -(wy / 2) {
        dy += wy;
    }
    if cx + dx == x && cy + dy == y {
        None
    } else {
        Some((cx + dx, cy + dy))
    }
}

fn inside(r: Rect, x: i32, y: i32) -> bool {
    r.0 <= x && x < r.2 && r.1 <= y && y < r.3
}

/// Windows pointer speed and acceleration while the game is in front (never saved to the user's settings).
struct Pointer {
    saved: Option<(i32, [i32; 3])>,
}

impl Pointer {
    fn restore_file() -> std::path::PathBuf {
        state_dir().join("pointer-restore.txt")
    }

    fn set(speed: i32, mouse: [i32; 3]) {
        let mut m = mouse;
        unsafe {
            SystemParametersInfoW(SPI_SETMOUSE, 0, m.as_mut_ptr() as *mut _, 0); // last argument 0: not saved
            SystemParametersInfoW(SPI_SETMOUSESPEED, 0, speed as usize as *mut _, 0);
        }
    }

    fn forget() {
        let _ = std::fs::remove_file(Self::restore_file());
    }

    /// A previous run that ended without restoring (crash) left its saved values: put them back.
    fn recover(&self) {
        if let Ok(text) = std::fs::read_to_string(Self::restore_file()) {
            let v: Vec<i32> = text.split_whitespace().filter_map(|s| s.parse().ok()).collect();
            if v.len() == 4 && (1..=20).contains(&v[0]) {
                Self::set(v[0], [v[1], v[2], v[3]]);
            }
        }
        Self::forget();
    }

    fn apply(&mut self, picture_width: i32) {
        if self.saved.is_none() {
            let mut speed = 0i32;
            let mut mouse = [0i32; 3];
            let ok = unsafe {
                SystemParametersInfoW(SPI_GETMOUSESPEED, 0, &mut speed as *mut i32 as *mut _, 0) != 0
                    && SystemParametersInfoW(SPI_GETMOUSE, 0, mouse.as_mut_ptr() as *mut _, 0) != 0
            };
            if !ok {
                return;
            }
            let _ = std::fs::create_dir_all(state_dir());
            let _ = std::fs::write(
                Self::restore_file(),
                format!("{} {} {} {}\n", speed, mouse[0], mouse[1], mouse[2]),
            );
            self.saved = Some((speed, mouse));
        }
        let want = picture_width as f64 / 1280.0;
        let mut best = 10usize;
        for (s, mult) in SPEED_MULT.iter().enumerate().skip(1) {
            if (mult - want).abs() < (SPEED_MULT[best] - want).abs() {
                best = s;
            }
        }
        let (_, m) = self.saved.unwrap();
        Self::set(best as i32, [m[0], m[1], 0]);
    }

    fn restore(&mut self) {
        if let Some((speed, mouse)) = self.saved.take() {
            Self::set(speed, mouse);
        }
        Self::forget();
    }
}

/// The capture, driven by the agent's events: `focus_changed` when the foreground window changes or the slow
/// timer runs (returns true while the game is in front), `mouse` on every mouse report or the fast timer.
pub struct CaptureCore {
    /// also holds Lichii's data folder (`panning.data_folder`)
    pub panning: AutoPanning,
    pointer: Pointer,
    pub state: (&'static str, &'static str),
    pub status_changed: bool,
    pub top: HWND,
    root: HWND,
    aspect: f64,
    rect: Option<Rect>,
    /// the emulator shows the game picture (not its shader screen)
    in_game: bool,
    rect_time: Option<Instant>,
    active: bool,
    active_width: i32,
}

impl CaptureCore {
    pub fn new(data_folder: String) -> Self {
        let pointer = Pointer { saved: None };
        pointer.recover();
        let aspect = read_aspect(&data_folder);
        CaptureCore {
            panning: AutoPanning::new(data_folder),
            pointer,
            state: ("waiting", "Waiting for the game"),
            status_changed: true,
            top: std::ptr::null_mut(),
            root: std::ptr::null_mut(),
            aspect,
            rect: None,
            in_game: false,
            rect_time: None,
            active: false,
            active_width: 0,
        }
    }

    fn say(&mut self, state: &'static str, text: &'static str) {
        if self.state.0 != state || self.state.1 != text {
            self.state = (state, text);
            self.status_changed = true;
        }
    }

    /// Gives the normal mouse back (the Windows pointer settings are restored).
    pub fn release(&mut self) {
        self.pointer.restore();
        self.active = false;
        self.active_width = 0;
    }

    pub fn focus_changed(&mut self) -> bool {
        unsafe {
            // the emulator keeps its window when the game stops (F5): its title then no longer names the game
            if !self.top.is_null() && (IsWindow(self.top) == 0 || !is_game_title(self.top)) {
                self.top = std::ptr::null_mut();
                self.panning.window = 0;
            }
            if self.top.is_null() {
                self.top = find_game_window();
                if !self.top.is_null() {
                    self.aspect = read_aspect(&self.panning.data_folder);
                }
            }
            let fg = GetForegroundWindow();
            let root = if fg.is_null() { fg } else { GetAncestor(fg, GA_ROOT) };
            let focused = !self.top.is_null()
                && !root.is_null()
                && pid_of(root) == pid_of(self.top)
                && IsIconic(root) == 0
                && (root == self.top || is_game_title(root));
            if focused {
                if root != self.root {
                    self.root = root;
                    self.rect_time = None;
                }
                if !self.active {
                    self.say("paused", "Game in front: point at the picture");
                }
                self.panning.game_in_front(self.top as isize, pid_of(self.top));
                return true;
            }
            self.root = std::ptr::null_mut();
            if self.active {
                self.release();
            }
            if self.top.is_null() {
                self.say("waiting", "Waiting for the game");
            } else {
                self.say("paused", "Game in the background: normal mouse");
            }
            false
        }
    }

    pub fn mouse(&mut self) {
        if self.root.is_null() {
            return;
        }
        if self.rect_time.is_none_or(|t| t.elapsed().as_millis() > 200) {
            (self.rect, self.in_game) = picture_rect(self.root, self.aspect); // the window may move or change size
            self.rect_time = Some(Instant::now());
        }
        let Some(rect) = self.rect else { return };
        let mut pt = POINT { x: 0, y: 0 };
        if unsafe { GetCursorPos(&mut pt) } == 0 {
            return;
        }
        if !self.active {
            if !inside(rect, pt.x, pt.y) {
                return; // starts once the pointer is over the picture
            }
            self.active = true;
            self.say("active", "Active: the mouse turns the camera");
        }
        self.panning.tick(self.top as isize, pid_of(self.top), pt.x, pt.y, self.in_game);
        let width = rect.2 - rect.0;
        if width != self.active_width {
            self.pointer.apply(width);
            self.active_width = width;
        }
        if let Some((x, y)) = wrap(rect, pt.x, pt.y) {
            unsafe { SetCursorPos(x, y) };
        }
    }
}
