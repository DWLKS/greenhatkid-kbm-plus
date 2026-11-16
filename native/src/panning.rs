//! Turns on Lichii's mouse panning for the player (what Ctrl+F9 does): it hides the pointer over the game.
//!
//! Lichii cannot be asked whether panning is on and does not save it: every Lichii run starts with it off, and it
//! stays as it is between games of the same run. So:
//!   1. when a game starts (its window comes to the front) and panning is believed off, press Lichii's
//!      "Toggle Mouse Panning" shortcut (from its settings, default Ctrl+F9) at once, even on the shader screen;
//!   2. check during the game itself: with panning on, Lichii keeps the pointer hidden while the mouse moves; with it
//!      off, every move shows it again. A pointer that keeps showing while the mouse moves (VISIBLE_FOR) means
//!      panning is off: press once more. The check skips Lichii's shader screen, where the pointer always shows;
//!   3. never press while a modifier key is held (holding Shift to run would make it Ctrl+Shift+F9), at most twice
//!      per game, and not at all when Lichii does not hide the idle pointer (hideInactiveMouse).

use std::collections::HashMap;
use std::time::Instant;
use windows_sys::Win32::Graphics::Gdi::{DeleteObject, GetBitmapBits, GetObjectW, BITMAP};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetCursorInfo, GetIconInfo, CURSORINFO, CURSOR_SHOWING, ICONINFO};

const DEFAULT_SHORTCUT: &str = "Ctrl+F9";
const VISIBLE_FOR: f64 = 1.5; // s of pointer visible while moving in the game -> panning is off
const MOVING_WINDOW: f64 = 0.5; // s since the last movement that still counts as moving
const CONFIRM_WITHIN: f64 = 4.0; // s for the pointer to disappear after a press

thread_local! {
    static BLANK_CACHE: std::cell::RefCell<HashMap<isize, bool>> = std::cell::RefCell::new(HashMap::new());
}

/// True for a fully transparent cursor (Qt's BlankCursor: monochrome, AND mask all ones, XOR mask all zeros).
fn is_blank(hcursor: isize) -> bool {
    if let Some(b) = BLANK_CACHE.with(|c| c.borrow().get(&hcursor).copied()) {
        return b;
    }
    let mut blank = false;
    unsafe {
        let mut ii: ICONINFO = std::mem::zeroed();
        if GetIconInfo(hcursor as _, &mut ii) != 0 {
            if ii.hbmColor.is_null() && !ii.hbmMask.is_null() {
                let mut bm: BITMAP = std::mem::zeroed();
                if GetObjectW(ii.hbmMask as _, std::mem::size_of::<BITMAP>() as i32, &mut bm as *mut _ as _) != 0 {
                    let size = (bm.bmWidthBytes * bm.bmHeight) as usize;
                    let mut buf = vec![0u8; size];
                    if GetBitmapBits(ii.hbmMask, size as i32, buf.as_mut_ptr() as _) as usize == size {
                        let half = size / 2;
                        blank = buf[..half].iter().all(|&b| b == 0xFF) && buf[half..].iter().all(|&b| b == 0);
                    }
                }
            }
            if !ii.hbmMask.is_null() {
                DeleteObject(ii.hbmMask as _);
            }
            if !ii.hbmColor.is_null() {
                DeleteObject(ii.hbmColor as _);
            }
        }
    }
    BLANK_CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() > 64 {
            c.clear();
        }
        c.insert(hcursor, blank);
    });
    blank
}

/// Some(true) when the pointer shows, Some(false) when it is hidden or blank, None when Windows cannot tell.
fn pointer_visible() -> Option<bool> {
    unsafe {
        let mut ci: CURSORINFO = std::mem::zeroed();
        ci.cbSize = std::mem::size_of::<CURSORINFO>() as u32;
        if GetCursorInfo(&mut ci) == 0 {
            return None;
        }
        if ci.flags & CURSOR_SHOWING == 0 || ci.hCursor.is_null() {
            return Some(false);
        }
        Some(!is_blank(ci.hCursor as isize))
    }
}

fn qt_config(data_folder: &str) -> Option<String> {
    std::fs::read_to_string(crate::store::qt_config(data_folder)).ok()
}

/// Lichii's "Toggle Mouse Panning" key sequence, e.g. "Ctrl+F9" ("" when it was cleared).
fn read_shortcut(data_folder: &str) -> String {
    const KEY: &str = "Shortcuts\\Main%20Window\\Toggle%20Mouse%20Panning\\KeySeq=";
    qt_config(data_folder)
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix(KEY).map(|v| v.trim().replace("%20", " "))))
        .unwrap_or_else(|| DEFAULT_SHORTCUT.to_string())
}

pub fn hides_idle_pointer(data_folder: &str) -> bool {
    qt_config(data_folder)
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("hideInactiveMouse=").map(|v| v.trim() == "true")))
        .unwrap_or(true)
}

/// "Ctrl+F9" -> [VK_CONTROL, VK_F9]; None if it cannot be pressed.
fn parse_shortcut(text: &str) -> Option<Vec<u16>> {
    let first = text.split(',').next()?.trim();
    let parts: Vec<String> = first.split('+').map(|p| p.trim().to_uppercase()).filter(|p| !p.is_empty()).collect();
    let (key, mods) = parts.split_last()?;
    let mut vks = Vec::new();
    for m in mods {
        vks.push(match m.as_str() {
            "CTRL" => 0x11,
            "SHIFT" => 0x10,
            "ALT" => 0x12,
            "META" => 0x5B,
            _ => return None,
        });
    }
    let k = key.as_str();
    let vk = if k.len() == 1 && k.chars().all(|c| c.is_ascii_alphanumeric()) {
        k.as_bytes()[0] as u16
    } else if let Some(n) = k.strip_prefix('F').and_then(|n| n.parse::<u16>().ok()).filter(|n| (1..=24).contains(n)) {
        0x70 + n - 1
    } else {
        match k {
            "SPACE" => 0x20,
            "TAB" => 0x09,
            "ESC" | "ESCAPE" => 0x1B,
            "RETURN" | "ENTER" => 0x0D,
            "BACKSPACE" => 0x08,
            "INS" | "INSERT" => 0x2D,
            "DEL" | "DELETE" => 0x2E,
            "HOME" => 0x24,
            "END" => 0x23,
            "PGUP" => 0x21,
            "PGDOWN" => 0x22,
            "LEFT" => 0x25,
            "UP" => 0x26,
            "RIGHT" => 0x27,
            "DOWN" => 0x28,
            "PAUSE" => 0x13,
            "PRINT" => 0x2C,
            "SCROLLLOCK" => 0x91,
            _ => return None,
        }
    };
    vks.push(vk);
    Some(vks)
}

fn modifiers_down() -> bool {
    [0x10, 0x11, 0x12, 0x5B, 0x5C].iter().any(|&v| unsafe { GetAsyncKeyState(v) } as u16 & 0x8000 != 0)
}

fn key_event(vk: u16, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wVk: vk, wScan: 0, dwFlags: if up { KEYEVENTF_KEYUP } else { 0 }, time: 0, dwExtraInfo: 0 },
        },
    }
}

fn press(vks: &[u16]) -> bool {
    let mut events: Vec<INPUT> = vks.iter().map(|&v| key_event(v, false)).collect();
    events.extend(vks.iter().rev().map(|&v| key_event(v, true)));
    let n = unsafe { SendInput(events.len() as u32, events.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
    n as usize == events.len()
}

/// What the app believes about Lichii's mouse panning, per Lichii run.
pub struct AutoPanning {
    pub data_folder: String,
    pub enabled: bool,
    /// The latest thing it did, for the status file (taken by the agent): a short line and the full explanation.
    pub log: Option<(String, String)>,
    /// The game window of this game session (0: none).
    pub window: isize,
    /// The Lichii process the belief is about, and the belief: Lichii starts every run with panning off and keeps it
    /// between games of the same run.
    pid: u32,
    believed_on: bool,
    /// Presses made in this game session (one at the start, at most one more from the gameplay check).
    presses: u32,
    /// Leave panning alone for this session (Lichii does not hide the idle pointer, the shortcut cannot be pressed,
    /// or a press changed nothing).
    hands_off: bool,
    pressed_at: Option<Instant>,
    visible_since: Option<Instant>,
    last_move: Option<Instant>,
    last_check: Option<Instant>,
    last_pos: (i32, i32),
}

impl AutoPanning {
    pub fn new(data_folder: String) -> Self {
        AutoPanning {
            data_folder,
            enabled: true,
            log: None,
            window: 0,
            pid: 0,
            believed_on: false,
            presses: 0,
            hands_off: false,
            pressed_at: None,
            visible_since: None,
            last_move: None,
            last_check: None,
            last_pos: (i32::MIN, i32::MIN),
        }
    }

    /// A new game session (a game window the app has not seen yet).
    fn start_session(&mut self, window: isize, pid: u32) {
        if pid != self.pid {
            self.pid = pid;
            self.believed_on = false; // a new Lichii run starts with panning off
        }
        self.window = window;
        self.presses = 0;
        self.hands_off = false;
        self.pressed_at = None;
        self.visible_since = None;
        self.last_move = None;
        self.last_check = None;
        self.last_pos = (i32::MIN, i32::MIN);
        if !hides_idle_pointer(&self.data_folder) {
            self.hands_off = true;
            self.log = Some((
                "Mouse panning: left as it is".into(),
                "Lichii's \"Hide mouse on inactivity\" is off, so the app cannot see whether mouse panning is on and \
                 leaves it alone. Press Lichii's \"Toggle Mouse Panning\" shortcut (Ctrl+F9) yourself if the pointer \
                 shows while you play."
                    .into(),
            ));
        }
    }

    /// Presses Lichii's shortcut once (not while a modifier key is held); false when it could not.
    fn press_shortcut(&mut self, short: &str, help: String) -> bool {
        if modifiers_down() {
            return false; // holding Shift to run would make it Ctrl+Shift+F9: try again a moment later
        }
        let text = read_shortcut(&self.data_folder);
        let Some(vks) = parse_shortcut(&text) else {
            self.hands_off = true;
            self.log = Some((
                "Mouse panning: turn it on in Lichii".into(),
                format!(
                    "Lichii's \"Toggle Mouse Panning\" shortcut ({text}) cannot be pressed by the app. Turn mouse \
                     panning on yourself in Lichii, or give that shortcut a plain key combination such as Ctrl+F9."
                ),
            ));
            return false;
        };
        if !press(&vks) {
            return false;
        }
        self.presses += 1;
        self.believed_on = !self.believed_on;
        self.log = Some((short.to_string(), help.replace("{shortcut}", &text)));
        true
    }

    /// The game window is in front (called on every focus check, about once a second): at the start of a session
    /// with panning believed off, press the shortcut right away, even on the shader screen.
    pub fn game_in_front(&mut self, window: isize, pid: u32) {
        if !self.enabled || window == 0 {
            return;
        }
        if window != self.window {
            self.start_session(window, pid);
        }
        if self.hands_off || self.believed_on || self.presses > 0 {
            return;
        }
        self.press_shortcut(
            "Mouse panning: turned on",
            "Lichii starts with mouse panning off, so the app pressed its shortcut ({shortcut}) as soon as the game \
             came to the front. On the shader screen the pointer still shows; it hides once the game runs."
                .into(),
        );
    }

    /// Called on every capture pass while the capture is active, with the pointer position before any wrap.
    /// `in_game`: Lichii shows the game picture (not its shader screen, where the pointer always shows).
    /// The check: in the game, a pointer that keeps showing while the mouse moves means panning is off.
    pub fn tick(&mut self, window: isize, pid: u32, x: i32, y: i32, in_game: bool) {
        if !self.enabled {
            return;
        }
        if window != self.window {
            self.start_session(window, pid);
        }
        let now = Instant::now();
        if self.last_pos != (x, y) {
            self.last_pos = (x, y);
            self.last_move = Some(now);
        }
        if self.last_check.is_some_and(|t| now.duration_since(t).as_secs_f64() < 0.05) {
            return;
        }
        self.last_check = Some(now);
        let moving = self.last_move.is_some_and(|t| now.duration_since(t).as_secs_f64() < MOVING_WINDOW);
        if !in_game || !moving || self.hands_off {
            self.visible_since = None;
            return;
        }
        let visible = pointer_visible();
        if visible == Some(false) {
            // hidden while moving in the game: panning is on
            self.believed_on = true;
            self.visible_since = None;
            if self.pressed_at.take().is_some() {
                self.log = Some(("Mouse panning: on".into(), "Mouse panning is on: Lichii hides the pointer over the game.".into()));
            }
            return;
        }
        if visible != Some(true) {
            return;
        }
        let since = *self.visible_since.get_or_insert(now);
        if let Some(pressed) = self.pressed_at {
            if now.duration_since(pressed).as_secs_f64() > CONFIRM_WITHIN && now.duration_since(since).as_secs_f64() >= VISIBLE_FOR {
                self.pressed_at = None;
                self.hands_off = true;
                let text = read_shortcut(&self.data_folder);
                self.log = Some((
                    format!("Mouse panning: press {text}"),
                    format!(
                        "The app pressed Lichii's \"Toggle Mouse Panning\" shortcut ({text}) but the pointer still \
                         shows in the game. Press {text} yourself in the game window; the app does not try again \
                         until the next game start."
                    ),
                ));
            }
            return;
        }
        if now.duration_since(since).as_secs_f64() < VISIBLE_FOR || self.presses >= 2 {
            return;
        }
        // seen in the game: panning is off, whatever the app believed (it never presses more than twice per session)
        self.believed_on = false;
        if self.press_shortcut(
            "Mouse panning: turned on",
            "The pointer kept showing while you moved the mouse in the game, so mouse panning was off: the app \
             pressed Lichii's shortcut for it ({shortcut})."
                .into(),
        ) {
            self.pressed_at = Some(now);
            self.visible_since = None;
        }
    }
}
