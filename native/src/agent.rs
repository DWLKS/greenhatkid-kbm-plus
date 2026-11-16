//! The background agent: tray icon, mouse capture and auto mouse panning. Event-driven:
//!   - a WinEvent hook says when the foreground window changes; a 1 s timer notices the game starting or closing;
//!   - only while the game is in front, Raw Input delivers every mouse report (RIDEV_INPUTSINK) and a 20 ms timer
//!     covers the moments the mouse is still. Outside the game the mouse is not watched at all.
//!
//! The settings window (another process, --settings) talks to it with a registered window message.

use crate::capture::CaptureCore;
use crate::store::{self, wide, APP_NAME};
use std::cell::RefCell;
use std::time::Instant;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{
    CreateMutexW, GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_NORMAL, THREAD_PRIORITY_TIME_CRITICAL,
};
use windows_sys::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows_sys::Win32::UI::Input::{RegisterRawInputDevices, RAWINPUTDEVICE, RIDEV_INPUTSINK, RIDEV_REMOVE};
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const MUTEX: &str = "GreenHatKid-KBM-Plus-agent-mutex";
const WM_TRAY: u32 = WM_APP + 1;
const TIMER_SLOW: usize = 1;
const TIMER_FAST: usize = 2;
const TIMER_SELFTEST: usize = 99;
const MENU_OPEN: usize = 10;
const MENU_QUIT: usize = 11;

struct Agent {
    hwnd: HWND,
    icon: HICON,
    icon_shown: bool,
    core: CaptureCore,
    listening: bool,
    last_mouse: Option<Instant>,
    panning_text: (String, String),
    control: u32,
    taskbar_created: u32,
    hooks: Vec<HWINEVENTHOOK>,
}

thread_local! {
    static AGENT: RefCell<Option<Agent>> = const { RefCell::new(None) };
}

/// Runs `f` on the agent unless it is already in use (a modal loop such as the tray menu re-entering).
fn with_agent<R>(f: impl FnOnce(&mut Agent) -> R) -> Option<R> {
    AGENT.with(|a| a.try_borrow_mut().ok().and_then(|mut a| a.as_mut().map(f)))
}

fn copy_into(dst: &mut [u16], s: &str) {
    let w: Vec<u16> = s.encode_utf16().take(dst.len() - 1).collect();
    dst[..w.len()].copy_from_slice(&w);
    dst[w.len()] = 0;
}

impl Agent {
    fn nid(&self, flags: u32) -> NOTIFYICONDATAW {
        let mut n: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
        n.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        n.hWnd = self.hwnd;
        n.uID = 1;
        n.uFlags = flags;
        n.uCallbackMessage = WM_TRAY;
        n.hIcon = self.icon;
        copy_into(&mut n.szTip, &format!("{APP_NAME}\n{}", self.core.state.1));
        n
    }

    fn add_tray(&mut self) {
        let n = self.nid(NIF_MESSAGE | NIF_ICON | NIF_TIP);
        self.icon_shown = unsafe { Shell_NotifyIconW(NIM_ADD, &n) } != 0;
    }

    fn remove_tray(&mut self) {
        if self.icon_shown {
            let n = self.nid(0);
            unsafe { Shell_NotifyIconW(NIM_DELETE, &n) };
            self.icon_shown = false;
        }
    }

    fn tray_hint(&self) {
        let mut n = self.nid(NIF_INFO);
        copy_into(&mut n.szInfoTitle, APP_NAME);
        copy_into(&mut n.szInfo, "Still running in the tray: the mouse capture keeps working. Right-click the icon to quit.");
        unsafe { Shell_NotifyIconW(NIM_MODIFY, &n) };
    }

    /// Writes the status file and the tray tooltip when the capture's state or the panning message changed.
    fn publish(&mut self) {
        let log = self.core.panning.log.take();
        if let Some(l) = &log {
            self.panning_text = l.clone();
        }
        if self.core.status_changed || log.is_some() {
            self.core.status_changed = false;
            store::write_status(self.core.state.0, self.core.state.1, &self.panning_text.0, &self.panning_text.1);
            let n = self.nid(NIF_TIP);
            unsafe { Shell_NotifyIconW(NIM_MODIFY, &n) };
        }
    }

    /// Raw Input and the fast timer only while the game is in front.
    fn listen(&mut self, on: bool) {
        if on == self.listening {
            return;
        }
        let dev = RAWINPUTDEVICE {
            usUsagePage: 1,
            usUsage: 2,
            dwFlags: if on { RIDEV_INPUTSINK } else { RIDEV_REMOVE },
            hwndTarget: if on { self.hwnd } else { std::ptr::null_mut() },
        };
        unsafe {
            RegisterRawInputDevices(&dev, 1, std::mem::size_of::<RAWINPUTDEVICE>() as u32);
            if on {
                SetTimer(self.hwnd, TIMER_FAST, 20, None);
            } else {
                KillTimer(self.hwnd, TIMER_FAST);
            }
            // The emulator keeps every core busy: while the game is in front the agent must answer mouse reports at
            // once, or a fast move could carry the pointer out of the picture. It only waits for events.
            SetThreadPriority(
                GetCurrentThread(),
                if on { THREAD_PRIORITY_TIME_CRITICAL } else { THREAD_PRIORITY_NORMAL },
            );
        }
        self.listening = on;
    }

    fn focus_changed(&mut self) {
        let focused = self.core.focus_changed();
        self.listen(focused);
        if self.listening {
            self.core.mouse();
        }
        self.publish();
    }

    fn reload(&mut self) {
        let s = store::load_agent_settings();
        self.core.panning.data_folder = s.root;
        self.core.panning.enabled = s.hide_cursor;
        self.core.top = std::ptr::null_mut(); // read the aspect ratio again on the next search
        self.focus_changed();
    }

    fn cleanup(&mut self) {
        self.listen(false);
        for h in self.hooks.drain(..) {
            unsafe { UnhookWinEvent(h) };
        }
        self.core.release();
        self.remove_tray();
        store::write_status("stopped", "Mouse capture is not running", "", "");
    }
}

fn open_settings() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).arg("--settings").spawn();
    }
}

fn quit(hwnd: HWND) {
    with_agent(|a| a.remove_tray());
    unsafe { DestroyWindow(hwnd) };
}

fn show_menu(hwnd: HWND, status: &str) {
    unsafe {
        let m = CreatePopupMenu();
        let status = wide(status);
        let settings = wide("Settings");
        let quit_text = wide("Quit");
        AppendMenuW(m, MF_STRING | MF_GRAYED, 0, status.as_ptr());
        AppendMenuW(m, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(m, MF_STRING, MENU_OPEN, settings.as_ptr());
        AppendMenuW(m, MF_STRING, MENU_QUIT, quit_text.as_ptr());
        let mut pt = POINT { x: 0, y: 0 };
        GetCursorPos(&mut pt);
        SetForegroundWindow(hwnd); // so the menu closes when clicking elsewhere
        let cmd = TrackPopupMenu(m, TPM_RIGHTBUTTON | TPM_RETURNCMD | TPM_NONOTIFY, pt.x, pt.y, 0, hwnd, std::ptr::null());
        PostMessageW(hwnd, WM_NULL, 0, 0);
        DestroyMenu(m);
        match cmd as usize {
            MENU_OPEN => open_settings(),
            // with the settings window open, its own question decides (and closes it too)
            MENU_QUIT => match store::settings_window() {
                Some(w) => {
                    SetForegroundWindow(w);
                    PostMessageW(w, store::control_message(), store::CMD_ASK_QUIT, 0);
                }
                None => quit(hwnd),
            },
            _ => {}
        }
    }
}

unsafe extern "system" fn win_event(_h: HWINEVENTHOOK, _ev: u32, _hwnd: HWND, _o: i32, _c: i32, _t: u32, _ms: u32) {
    with_agent(|a| a.focus_changed());
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_INPUT => {
            with_agent(|a| {
                // fast mice report up to 8000 times a second: at most one capture pass every 0.4 ms
                if a.last_mouse.is_none_or(|t| t.elapsed().as_micros() >= 400) {
                    a.last_mouse = Some(Instant::now());
                    a.core.mouse();
                    a.publish();
                }
            });
        }
        WM_TIMER => {
            with_agent(|a| {
                if wparam == TIMER_FAST {
                    a.core.mouse();
                    a.publish();
                } else if wparam == TIMER_SLOW {
                    a.focus_changed();
                }
            });
        }
        WM_TRAY => {
            match lparam as u32 {
                WM_LBUTTONUP | WM_LBUTTONDBLCLK => open_settings(),
                WM_RBUTTONUP => {
                    // the menu runs a modal loop: read what it needs, then let go of the agent before showing it
                    let status = with_agent(|a| a.core.state.1.to_string()).unwrap_or_default();
                    show_menu(hwnd, &status);
                }
                _ => {}
            }
            return 0;
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            return 0;
        }
        _ => {
            let (control, taskbar) = with_agent(|a| (a.control, a.taskbar_created)).unwrap_or((0, 0));
            if msg == control && control != 0 {
                match wparam {
                    store::CMD_OPEN => open_settings(),
                    store::CMD_RELOAD => {
                        with_agent(|a| a.reload());
                    }
                    store::CMD_QUIT => quit(hwnd),
                    store::CMD_TRAY_HINT => {
                        with_agent(|a| a.tray_hint());
                    }
                    _ => {}
                }
                return 0;
            }
            if msg == taskbar && taskbar != 0 {
                with_agent(|a| a.add_tray()); // Explorer restarted: put the icon back
            }
        }
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

fn load_icon() -> HICON {
    unsafe {
        let hinst = GetModuleHandleW(std::ptr::null());
        let h = LoadImageW(hinst, 1 as _, IMAGE_ICON, GetSystemMetrics(SM_CXSMICON), GetSystemMetrics(SM_CYSMICON), 0);
        if h.is_null() {
            LoadIconW(std::ptr::null_mut(), IDI_APPLICATION)
        } else {
            h as HICON
        }
    }
}

/// Starts the agent (one per user). A second start only asks the running one to open the settings.
/// `seconds`: stop by itself after that long (self-test).
pub fn run(open_settings_now: bool, seconds: Option<u32>) -> i32 {
    unsafe {
        let mutex_name = wide(MUTEX);
        let mutex = CreateMutexW(std::ptr::null(), 0, mutex_name.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            if open_settings_now {
                store::tell_agent(store::CMD_OPEN);
            }
            CloseHandle(mutex);
            return 0;
        }
        let hinst = GetModuleHandleW(std::ptr::null());
        let class = wide(store::AGENT_CLASS);
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        RegisterClassExW(&wc);
        let title = wide("GreenHatKid-KBM-Plus agent");
        // a hidden top-level window (never shown): tray callbacks, Raw Input and "TaskbarCreated" arrive here
        let hwnd = CreateWindowExW(0, class.as_ptr(), title.as_ptr(), 0, 0, 0, 0, 0, std::ptr::null_mut(),
                                   std::ptr::null_mut(), hinst, std::ptr::null());
        let s = store::load_agent_settings();
        let mut core = CaptureCore::new(s.root);
        core.panning.enabled = s.hide_cursor;
        let taskbar = wide("TaskbarCreated");
        let mut agent = Agent {
            hwnd,
            icon: load_icon(),
            icon_shown: false,
            core,
            listening: false,
            last_mouse: None,
            panning_text: Default::default(),
            control: store::control_message(),
            taskbar_created: RegisterWindowMessageW(taskbar.as_ptr()),
            hooks: Vec::new(),
        };
        agent.add_tray();
        for ev in [EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_MINIMIZEEND] {
            agent.hooks.push(SetWinEventHook(ev, ev, std::ptr::null_mut(), Some(win_event), 0, 0, WINEVENT_OUTOFCONTEXT));
        }
        SetTimer(hwnd, TIMER_SLOW, 1000, None);
        if let Some(s) = seconds {
            SetTimer(hwnd, TIMER_SELFTEST, s * 1000, None);
        }
        AGENT.with(|a| *a.borrow_mut() = Some(agent));
        with_agent(|a| a.focus_changed());
        if open_settings_now {
            open_settings();
        }
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            if msg.message == WM_TIMER && msg.wParam == TIMER_SELFTEST {
                break;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        with_agent(|a| a.cleanup());
        AGENT.with(|a| *a.borrow_mut() = None);
        CloseHandle(mutex);
    }
    0
}
