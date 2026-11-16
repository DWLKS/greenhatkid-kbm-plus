//! The settings window (egui), only running while it is open.
//! Drawn on the CPU (no graphics driver: a few MB instead of the hundreds an OpenGL context takes) into the
//! window with softbuffer; winit gives the window and input, egui-winit translates the input.

use crate::icons;
use crate::keys::{self, Values};
use crate::lichii::{self, PatchState};
use crate::panning::hides_idle_pointer;
use crate::settings::{self, Settings};
use crate::store;
use egui::{
    Align, Color32, FontData, FontDefinitions, FontFamily, FontId, Frame, Id, Key, Layout, Margin,
    Modal, PointerButton, RichText, Sense, Stroke, Ui, Vec2,
};
use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

const APP: &str = store::APP_NAME;
const VERSION: &str = env!("CARGO_PKG_VERSION");
const APPLIED: &str = "All changes applied \u{2014} ready to play";

const BG: Color32 = Color32::from_rgb(0x17, 0x18, 0x1b);
const SIDE: Color32 = Color32::from_rgb(0x11, 0x12, 0x14);
const CARD: Color32 = Color32::from_rgb(0x1f, 0x21, 0x25);
const CARD_LINE: Color32 = Color32::from_rgb(0x2a, 0x2d, 0x33);
const TEXT: Color32 = Color32::from_rgb(0xe6, 0xe6, 0xe6);
const SUBTLE: Color32 = Color32::from_rgb(0x8d, 0x91, 0x99);
const NAV_TEXT: Color32 = Color32::from_rgb(0xb9, 0xbc, 0xc2);
const NAV_SEL: Color32 = Color32::from_rgb(0x2a, 0x3b, 0x2d);
const NAV_HOVER: Color32 = Color32::from_rgb(0x1d, 0x1f, 0x23);
const KEY_BG: Color32 = Color32::from_rgb(0x26, 0x29, 0x2e);
const BUTTON_BG: Color32 = Color32::from_rgb(0x2a, 0x2d, 0x33);
const BUTTON_LINE: Color32 = Color32::from_rgb(0x3a, 0x3e, 0x46);
const GREEN: Color32 = Color32::from_rgb(0x3f, 0x8f, 0x4f);
const GREEN_HI: Color32 = Color32::from_rgb(0x4f, 0xa8, 0x60);
const OK_TEXT: Color32 = Color32::from_rgb(0x7c, 0xc4, 0x8a);
const WARN_BG: Color32 = Color32::from_rgb(0x3a, 0x2f, 0x12);
const WARN_LINE: Color32 = Color32::from_rgb(0x6b, 0x55, 0x20);
const WARN_TEXT: Color32 = Color32::from_rgb(0xff, 0xd9, 0x8a);
const ERROR: Color32 = Color32::from_rgb(0xe0, 0x7a, 0x5f);
const DUPLICATE: Color32 = Color32::from_rgb(0xc0, 0x50, 0x4d);
const QUIET: Color32 = Color32::from_rgb(0x7d, 0x81, 0x89);

const PAGES: [&str; 4] = ["Controls", "Camera", "In-game key icons", "App"];
/// The first pages are steps: Next on each, Apply on the last.
const STEPS: usize = 3;
const PAGE_CONTROLS: usize = 0;
const PAGE_CAMERA: usize = 1;
const PAGE_ICONS: usize = 2;
const PAGE_APP: usize = 3;

#[derive(Clone, Copy, PartialEq)]
enum Tone {
    Info,
    Pending,
    Ok,
    Error,
}

fn tone_color(t: Tone) -> Color32 {
    match t {
        Tone::Info => SUBTLE,
        Tone::Pending => WARN_TEXT,
        Tone::Ok => GREEN_HI,
        Tone::Error => ERROR,
    }
}

fn bold_family() -> FontFamily {
    FontFamily::Name("bold".into())
}

fn bold(text: impl Into<String>) -> RichText {
    RichText::new(text).family(bold_family())
}

/// A round "?" that shows `help` while the mouse is over it (drawn in `rect`).
fn help_mark(ui: &Ui, rect: egui::Rect, resp: egui::Response, radius: f32, fill: Color32, text: Color32, help: impl FnOnce(&mut Ui)) {
    ui.painter().circle_filled(rect.center(), radius, fill);
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "?", FontId::new(radius + 3.0, bold_family()), text);
    resp.on_hover_cursor(egui::CursorIcon::Help).on_hover_ui(help);
}

/// A path with a Change button on the right (or `missing` in red when there is none); true when Change was clicked.
fn path_row(ui: &mut Ui, path: &str, missing: &str) -> bool {
    let mut pick = false;
    ui.horizontal(|ui| {
        if path.is_empty() {
            ui.label(RichText::new(missing).color(ERROR));
        } else {
            ui.add(egui::Label::new(path).truncate());
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            pick = ui.add(egui::Button::new("Change").min_size(Vec2::new(76.0, 28.0))).clicked();
        });
    });
    pick
}

/// The frame of the window's dialogs.
fn dialog_frame(margin: Margin) -> Frame {
    Frame::new().fill(CARD).stroke(Stroke::new(1.0, BUTTON_LINE)).corner_radius(10).inner_margin(margin)
}

// ---------------------------------------------------------------- fonts: Windows' own, nothing bundled
fn load_fonts(ctx: &egui::Context) {
    let dir = std::path::PathBuf::from(std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into())).join("Fonts");
    let read = |names: &[&str]| names.iter().find_map(|n| std::fs::read(dir.join(n)).ok());
    let mut fonts = FontDefinitions::empty();
    let mut add = |key: &str, data: Option<Vec<u8>>| -> bool {
        match data {
            Some(d) => {
                fonts.font_data.insert(key.to_string(), Arc::new(FontData::from_owned(d)));
                true
            }
            None => false,
        }
    };
    let regular = add("regular", read(&["segoeui.ttf", "arial.ttf"]));
    let bold_ok = add("bold", read(&["seguisb.ttf", "segoeuib.ttf", "arialbd.ttf"]));
    let symbols = add("symbols", read(&["seguisym.ttf"]));
    let mono = add("mono", read(&["consola.ttf", "cour.ttf"]));
    let chain = |first: &str, ok: bool| {
        let mut v = Vec::new();
        if ok {
            v.push(first.to_string());
        }
        if regular && first != "regular" {
            v.push("regular".to_string());
        }
        if symbols {
            v.push("symbols".to_string());
        }
        v
    };
    fonts.families.insert(FontFamily::Proportional, chain("regular", regular));
    fonts.families.insert(FontFamily::Name("bold".into()), chain("bold", bold_ok));
    fonts.families.insert(FontFamily::Monospace, chain("mono", mono));
    ctx.set_fonts(fonts);
}

fn style(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    ctx.all_styles_mut(|s| {
        s.text_styles.insert(egui::TextStyle::Body, FontId::proportional(14.0));
        s.text_styles.insert(egui::TextStyle::Button, FontId::proportional(14.0));
        s.text_styles.insert(egui::TextStyle::Small, FontId::proportional(12.0));
        s.text_styles.insert(egui::TextStyle::Monospace, FontId::monospace(12.5));
        s.visuals.override_text_color = Some(TEXT);
        s.visuals.panel_fill = BG;
        s.visuals.window_fill = CARD;
        s.visuals.extreme_bg_color = KEY_BG;
        s.visuals.selection.bg_fill = GREEN;
        s.visuals.widgets.inactive.bg_fill = BUTTON_BG;
        s.visuals.widgets.inactive.weak_bg_fill = BUTTON_BG;
        s.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BUTTON_LINE);
        s.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x33, 0x37, 0x3f);
        s.visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x33, 0x37, 0x3f);
        s.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, GREEN_HI);
        s.visuals.widgets.active.weak_bg_fill = Color32::from_rgb(0x23, 0x26, 0x2b);
        s.visuals.slider_trailing_fill = true;
        s.spacing.item_spacing = Vec2::new(10.0, 8.0);
        s.spacing.slider_width = 420.0;
        s.interaction.tooltip_delay = 0.0;
    });
}

// ---------------------------------------------------------------- small widgets
fn card(ui: &mut Ui, title: &str, sub: Option<&str>, add: impl FnOnce(&mut Ui)) {
    Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, CARD_LINE))
        .corner_radius(10)
        .inner_margin(Margin::symmetric(18, 14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if !title.is_empty() {
                // the section's header: larger, with a line between it and its content
                ui.label(bold(title).size(16.5).color(Color32::WHITE));
                ui.add_space(2.0);
                let y = ui.cursor().top();
                let x = ui.max_rect().x_range();
                ui.painter().hline(x, y, Stroke::new(1.0, BUTTON_LINE));
                ui.add_space(8.0);
            }
            if let Some(s) = sub {
                ui.label(RichText::new(s).color(SUBTLE));
            }
            add(ui);
        });
    ui.add_space(6.0);
}

fn toggle(ui: &mut Ui, on: &mut bool, label: &str) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(36.0, 20.0), Sense::click());
        if resp.clicked() {
            *on = !*on;
            changed = true;
        }
        let t = ui.ctx().animate_bool(resp.id, *on);
        let off_bg = if resp.hovered() { Color32::from_rgb(0x45, 0x4a, 0x53) } else { BUTTON_LINE };
        let on_bg = if resp.hovered() { Color32::from_rgb(0x47, 0xa0, 0x58) } else { GREEN };
        let bg = lerp_color(off_bg, on_bg, t);
        let knob = lerp_color(Color32::from_rgb(0xc9, 0xcc, 0xd1), Color32::WHITE, t);
        ui.painter().rect_filled(rect, 10.0, bg);
        let x = egui::lerp((rect.left() + 10.0)..=(rect.right() - 10.0), t);
        ui.painter().circle_filled(egui::pos2(x, rect.center().y), 7.0, knob);
        // only the switch itself turns it on or off, not its text
        ui.add(egui::Label::new(label).selectable(false));
    });
    changed
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

/// The colours of a coloured button: normal, under the mouse, and pressed.
#[derive(Clone, Copy)]
struct Look {
    fill: Color32,
    line: Color32,
    hover_fill: Color32,
    hover_line: Color32,
    press_fill: Color32,
}

const GREEN_LOOK: Look = Look {
    fill: GREEN,
    line: GREEN_HI,
    hover_fill: GREEN_HI,
    hover_line: Color32::from_rgb(0x7c, 0xc9, 0x8a),
    press_fill: Color32::from_rgb(0x35, 0x7a, 0x43),
};

const RED_LOOK: Look = Look {
    fill: Color32::from_rgb(0x8f, 0x3a, 0x30),
    line: ERROR,
    hover_fill: Color32::from_rgb(0xa8, 0x46, 0x3a),
    hover_line: Color32::from_rgb(0xf0, 0x95, 0x7c),
    press_fill: Color32::from_rgb(0x7a, 0x31, 0x28),
};

/// A key button; one used twice keeps its red edge under the mouse so the warning stays visible.
fn key_look(dup: bool) -> Look {
    Look {
        fill: KEY_BG,
        line: if dup { DUPLICATE } else { BUTTON_LINE },
        hover_fill: Color32::from_rgb(0x33, 0x37, 0x3f),
        hover_line: if dup { DUPLICATE } else { GREEN_HI },
        press_fill: Color32::from_rgb(0x23, 0x26, 0x2b),
    }
}

/// Adds a button in these colours (lighter under the mouse, darker while pressed); a disabled one is grey.
fn add_look(ui: &mut Ui, enabled: bool, button: egui::Button, look: Look) -> egui::Response {
    ui.scope(|ui| {
        if !enabled {
            return ui.add_enabled(false, button.fill(BUTTON_BG).stroke(Stroke::new(1.0, BUTTON_LINE)));
        }
        let w = &mut ui.visuals_mut().widgets;
        w.inactive.weak_bg_fill = look.fill;
        w.inactive.bg_stroke = Stroke::new(1.0, look.line);
        w.hovered.weak_bg_fill = look.hover_fill;
        w.hovered.bg_stroke = Stroke::new(1.0, look.hover_line);
        w.active.weak_bg_fill = look.press_fill;
        w.active.bg_stroke = Stroke::new(1.0, look.hover_line);
        ui.add(button)
    })
    .inner
}

/// A slider with its value in a box on the right (centred, bold); returns true when the value changed.
fn slider_box(ui: &mut Ui, id: &str, value: &mut f64, lo: f64, hi: f64, step: f64, decimals: usize) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let box_w = 76.0;
        ui.spacing_mut().slider_width = (ui.available_width() - box_w - 16.0).max(120.0);
        let r = ui.add(egui::Slider::new(value, lo..=hi).step_by(step).show_value(false).trailing_fill(true));
        changed |= r.changed();
        let buf_id = Id::unique(("valuebox", id));
        let mut text = ui
            .data(|d| d.get_temp::<String>(buf_id))
            .unwrap_or_else(|| format!("{:.*}", decimals, value).replace('.', ","));
        let edit = egui::TextEdit::singleline(&mut text)
            .align(egui::Align2::CENTER_CENTER)
            .font(FontId::new(15.5, FontFamily::Name("bold".into())))
            .desired_width(box_w)
            .background_color(KEY_BG);
        let resp = ui.add(edit);
        if resp.has_focus() {
            ui.data_mut(|d| d.insert_temp(buf_id, text.clone()));
        } else {
            if resp.lost_focus() {
                if let Ok(v) = text.replace(',', ".").trim().parse::<f64>() {
                    let v = ((v.clamp(lo, hi) / step).round() * step * 1e6).round() / 1e6;
                    if (v - *value).abs() > 1e-9 {
                        *value = v;
                        changed = true;
                    }
                }
            }
            ui.data_mut(|d| d.remove::<String>(buf_id));
        }
    });
    changed
}

/// Puts the settings of one page (Controls or Camera) back to the defaults (your layout).
fn page_defaults(v: &mut Values, page: usize) {
    let d = Values::default();
    if page == PAGE_CONTROLS {
        for n in keys::binding_names() {
            v.keys.insert(n.to_string(), d.key(n).to_string());
        }
        v.walk_speed = d.walk_speed;
    } else {
        v.mouse_sens = d.mouse_sens;
        v.split_axes = d.split_axes;
        v.mouse_sens_x = d.mouse_sens_x;
        v.mouse_sens_y = d.mouse_sens_y;
        v.invert_y = d.invert_y;
        v.hide_cursor = d.hide_cursor;
    }
}

fn key_name_of(key: Key) -> Option<&'static str> {
    use Key::*;
    Some(match key {
        A => "a", B => "b", C => "c", D => "d", E => "e", F => "f", G => "g", H => "h", I => "i", J => "j",
        K => "k", L => "l", M => "m", N => "n", O => "o", P => "p", Q => "q", R => "r", S => "s", T => "t",
        U => "u", V => "v", W => "w", X => "x", Y => "y", Z => "z",
        Num0 => "0", Num1 => "1", Num2 => "2", Num3 => "3", Num4 => "4", Num5 => "5", Num6 => "6", Num7 => "7",
        Num8 => "8", Num9 => "9",
        F1 => "f1", F2 => "f2", F3 => "f3", F4 => "f4", F5 => "f5", F6 => "f6", F7 => "f7", F8 => "f8",
        F9 => "f9", F10 => "f10", F11 => "f11", F12 => "f12",
        Space => "space", Enter => "enter", Tab => "tab", Insert => "insert", Home => "home", End => "end",
        PageUp => "pageup", PageDown => "pagedown", ArrowLeft => "left", ArrowRight => "right", ArrowUp => "up",
        ArrowDown => "down", Comma => "comma", Minus => "minus", Period => "period", Slash => "slash",
        Semicolon => "semicolon", Equals => "equals", OpenBracket => "bracketleft", Backslash => "backslash",
        CloseBracket => "bracketright", Backtick => "grave", Quote => "apostrophe",
        _ => return None,
    })
}

/// What a dialog's Yes does.
#[derive(Clone, Copy, PartialEq)]
enum Confirm {
    /// close the window although the in-game key icons are still being made
    CloseDuringJob,
    /// put one page's settings back to the defaults
    Defaults(usize),
    /// remove the mod from Lichii
    Remove,
    /// quit the whole app (no tray)
    Quit,
}

/// A message in the window's own style (Windows' message boxes ignore the dark theme).
struct Dialog {
    text: String,
    confirm: Option<Confirm>,
}

struct KeyDialog {
    name: &'static str,
    label: &'static str,
    opened: Instant,
    mods: egui::Modifiers,
}

// ---------------------------------------------------------------- the window
pub struct App {
    s: Settings,
    /// the values in settings.json (what Apply last wrote) and the values on screen
    saved: Values,
    vals: Values,
    page: usize,
    message: Vec<(String, Tone)>,
    log: Vec<String>,
    key_dialog: Option<KeyDialog>,
    last_refresh: Option<Instant>,
    /// the agent's state, its text, the short mouse panning line and its explanation
    agent: (String, String, String, String),
    /// the camera patch line on the App page: (ours is installed, text)
    patch_text: Option<(bool, String)>,
    autostart: bool,
    quit_confirmed: bool,
    /// the steps left with Next since their last change
    next_done: [bool; STEPS],
    /// what Lichii already has for the current values: (controls, camera, in-game key icons)
    applied: (bool, bool, bool),
    /// what of the mod is in Lichii now, and the result of the last removal (succeeded, text)
    installed: lichii::Installed,
    remove_result: Option<(bool, String)>,
    dialog: Option<Dialog>,
    /// the user agreed to close during a drawing: ask the window to close again
    job_close_ok: bool,
    close_again: bool,
    exit_now: bool,
    /// making the in-game key icons (background thread): its messages and the keys it draws
    job: Option<(mpsc::Receiver<JobMsg>, BTreeMap<String, String>)>,
}

enum JobMsg {
    Log(String),
    Done(Result<(), String>),
}

impl App {
    fn new(ctx: &egui::Context) -> App {
        load_fonts(ctx);
        style(ctx);
        let mut s = Settings::load();
        if !lichii::is_data_folder(&s.str("root")) {
            let found = store::find_data_folder().unwrap_or_default();
            s.set_str("root", &found);
        }
        if !std::path::Path::new(&s.str("nup")).is_file() && !s.str("root").is_empty() {
            let guess = icons::guess_nup(&s.str("root")).unwrap_or_default();
            s.set_str("nup", &guess);
        }
        let vals = s.values();
        // the mouse capture and the tray icon live in the background agent: make sure it runs
        if store::agent_window().is_null() {
            if let Ok(exe) = std::env::current_exe() {
                let _ = std::process::Command::new(exe).arg("--tray").spawn();
            }
        }
        let mut app = App {
            s,
            saved: vals.clone(),
            vals,
            page: 0,
            message: Vec::new(),
            log: Vec::new(),
            key_dialog: None,
            last_refresh: None,
            agent: Default::default(),
            patch_text: None,
            autostart: settings::autostart_enabled(),
            quit_confirmed: false,
            next_done: [false; STEPS],
            applied: (false, false, false),
            installed: Default::default(),
            remove_result: None,
            dialog: None,
            job_close_ok: false,
            close_again: false,
            exit_now: false,
            job: None,
        };
        app.refresh();
        app.update_message(None);
        app
    }

    fn unsaved(&self) -> bool {
        self.vals != self.saved
    }

    /// Stores these values in settings.json and tells the agent.
    fn save_values(&mut self, v: Values) {
        self.s.set_values(&v);
        self.s.save();
        self.saved = v;
        store::tell_agent(store::CMD_RELOAD);
    }

    /// False (after saying so) when Lichii is running: it rewrites its settings when it closes.
    fn lichii_closed(&mut self, undo: &str) -> bool {
        match lichii::emulator_running() {
            Some(running) => {
                self.show(&format!(
                    "Close Lichii first ({running} is running): it rewrites its settings when it closes, which would undo {undo}."
                ));
                false
            }
            None => true,
        }
    }

    fn say(&mut self, text: &str, tone: Tone) {
        self.message = if text.is_empty() { Vec::new() } else { vec![(text.to_string(), tone)] };
    }

    /// Notes about the keys (duplicates, keys Lichii uses) and whether changes are waiting for Apply.
    fn update_message(&mut self, prefix: Option<&str>) {
        let mut notes: Vec<String> = prefix.map(|p| vec![p.to_string()]).unwrap_or_default();
        let dups = self.duplicate_keys();
        if !dups.is_empty() {
            notes.push(format!(
                "Same key on two controls: {}.",
                dups.iter().map(|k| keys::pretty(k)).collect::<Vec<_>>().join(", ")
            ));
        }
        let used: Vec<&str> = keys::binding_names().iter().map(|n| self.vals.key(n)).collect();
        for (k, why) in keys::RESERVED {
            if used.contains(&k) {
                notes.push(format!("{} clashes with Lichii ({}).", keys::pretty(k), why));
            }
        }
        let mut parts = Vec::new();
        if !notes.is_empty() {
            parts.push((notes.join(" "), Tone::Info));
        }
        if self.nothing_to_apply() {
            parts.push((APPLIED.to_string(), Tone::Ok));
        } else if self.unsaved() {
            parts.push(("Changes not applied yet.".to_string(), Tone::Pending));
        } else if self.applied.0 && self.applied.1 {
            let why = self.icons_issue().unwrap_or("Press Apply to update the in-game key icons.");
            parts.push((why.to_string(), Tone::Pending));
        } else if lichii::is_data_folder(&self.s.str("root")) {
            parts.push(("Not applied to Lichii yet.".to_string(), Tone::Pending));
        }
        self.message = parts;
    }

    fn duplicate_keys(&self) -> Vec<String> {
        let mut seen: std::collections::BTreeMap<&str, usize> = Default::default();
        for n in keys::binding_names() {
            let k = self.vals.key(n);
            if k != "none" {
                *seen.entry(k).or_default() += 1;
            }
        }
        seen.into_iter().filter(|(_, c)| *c > 1).map(|(k, _)| k.to_string()).collect()
    }

    /// Once a second: the agent's status, Lichii's own hide-pointer setting, the installed patch.
    fn refresh(&mut self) {
        self.last_refresh = Some(Instant::now());
        let status = std::fs::read_to_string(store::status_path())
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok());
        let field = |k: &str| status.as_ref().and_then(|s| s.get(k)).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let agent = if store::agent_window().is_null() {
            ("stopped".to_string(), "Mouse capture is not running".to_string(), String::new(), String::new())
        } else {
            (field("state"), field("text"), field("panning"), field("panning_help"))
        };
        if !agent.2.is_empty() && agent.2 != self.agent.2 && !self.agent.0.is_empty() {
            self.log.push(if agent.3.is_empty() { agent.2.clone() } else { agent.3.clone() });
        }
        self.agent = agent;
        let root = self.s.str("root");
        if lichii::is_data_folder(&root) {
            self.sync_hide_cursor(&root);
            self.patch_text = Some(match lichii::installed_patch_state(&root) {
                PatchState::Ours(speed) => (true, format!("Camera patch with the aiming/riding fix is installed (speed {speed:.1}).")),
                PatchState::Other => (
                    false,
                    "A different camera patch is installed (probably the original mod's app ran Apply). \
                     Click Apply to put ours back."
                        .to_string(),
                ),
                PatchState::Missing => (false, "No camera patch installed yet. Click Apply.".to_string()),
            });
        }
        let before = self.applied;
        self.installed = if lichii::is_data_folder(&root) { lichii::installed(&root) } else { Default::default() };
        self.check_applied();
        if self.applied != before && self.job.is_none() {
            self.update_message(None);
        }
    }

    /// Compares the current values with what Lichii has (the check marks and the Apply button follow it).
    fn check_applied(&mut self) {
        let root = self.s.str("root");
        self.applied = if lichii::is_data_folder(&root) {
            let (controls, camera) = lichii::applied(&root, &self.vals);
            (controls, camera, self.pictures_current())
        } else {
            (false, false, false)
        };
    }

    /// Everything is saved and in Lichii: Apply has nothing to do.
    fn nothing_to_apply(&self) -> bool {
        !self.unsaved() && self.applied.0 && self.applied.1 && self.applied.2
    }

    /// A step's check mark: left with Next since its last change, or already in Lichii as it is now.
    fn step_done(&self, i: usize) -> bool {
        match i {
            PAGE_CONTROLS => self.next_done[i] || self.applied.0,
            PAGE_CAMERA => self.next_done[i] || self.applied.1,
            _ => self.nothing_to_apply(),
        }
    }

    /// Follow Lichii's own "Hide mouse on inactivity" if it was changed in Lichii after our Apply.
    fn sync_hide_cursor(&mut self, root: &str) {
        let lichii_on = hides_idle_pointer(root);
        let mut saved = self.saved.clone();
        if lichii_on == saved.hide_cursor {
            return;
        }
        let old = saved.hide_cursor;
        saved.hide_cursor = lichii_on;
        self.save_values(saved);
        if self.vals.hide_cursor == old {
            self.vals.hide_cursor = lichii_on;
        }
        self.say(
            &format!(
                "\"Hide mouse on inactivity\" was turned {} in Lichii; the Hide the mouse pointer option follows it.",
                if lichii_on { "on" } else { "off" }
            ),
            Tone::Info,
        );
    }

    fn apply(&mut self) {
        let root = self.s.str("root");
        if !lichii::is_data_folder(&root) {
            self.show("Pick Lichii's data folder first (App page).");
            self.page = PAGE_APP;
            return;
        }
        if !self.lichii_closed("these") {
            return;
        }
        let v = self.vals.clone();
        let mut lines = Vec::new();
        if let Err(e) = lichii::apply(&root, &v, &mut |l| lines.push(l)) {
            self.show(&format!("Apply failed, nothing more was changed:\n{e}"));
            return;
        }
        self.log.extend(lines);
        self.remove_result = None;
        self.save_values(v);
        self.refresh();
        if self.pictures_current() {
            self.update_message(None);
        } else {
            self.run_icons();
        }
    }

    /// Why the installed in-game key icons need Apply although the keys did not change (None: they do not).
    fn icons_issue(&self) -> Option<&'static str> {
        self.s.icon_keys()?;
        if !icons::installed(&self.s.str("root")) {
            Some("The in-game key icons are missing from Lichii. Press Apply to put them back.")
        } else if self.s.icons_version() != icons::DRAWING_VERSION {
            Some("This version updates the in-game key icons. Press Apply to install them.")
        } else {
            None
        }
    }

    /// The installed key icons show the current keys and were made by this version's drawings.
    fn pictures_current(&self) -> bool {
        self.s.icon_keys().as_ref() == Some(&self.vals.icon_keys())
            && self.s.icons_version() == icons::DRAWING_VERSION
            && icons::installed(&self.s.str("root"))
    }

    /// A message in a dialog.
    fn show(&mut self, text: &str) {
        self.dialog = Some(Dialog { text: text.to_string(), confirm: None });
    }

    /// Something on a step changed: its Next no longer counts (its check mark returns if the value goes back).
    fn touched(&mut self, page: usize) {
        if page < STEPS {
            self.next_done[page] = false;
        }
        self.check_applied();
        self.update_message(None);
    }

    /// Draws the in-game key icons for the current keys on a background thread (see `poll_job`).
    fn run_icons(&mut self) {
        if self.job.is_some() {
            return;
        }
        let root = self.s.str("root");
        let nup = self.s.str("nup");
        if !std::path::Path::new(&nup).is_file() && !crate::gamefiles::cache_ready() {
            self.say("Keys applied. Pick the game file, then press Apply again to make the in-game key icons.", Tone::Pending);
            self.page = PAGE_ICONS;
            return;
        }
        if lichii::emulator_running().is_some() {
            self.show("Close Lichii first: it keeps the game files open.");
            return;
        }
        let want = self.vals.icon_keys();
        let keys = want.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let r = icons::build(&root, &nup, &keys, &mut |l| {
                let _ = tx.send(JobMsg::Log(l));
            });
            let _ = tx.send(JobMsg::Done(r));
        });
        self.job = Some((rx, want));
        self.say("Updating the in-game key icons\u{2026}", Tone::Pending);
        self.page = PAGE_ICONS;
    }

    fn poll_job(&mut self, ctx: &egui::Context) {
        let Some((rx, _)) = &self.job else { return };
        let mut done = None;
        loop {
            match rx.try_recv() {
                Ok(JobMsg::Log(l)) => self.log.push(l),
                Ok(JobMsg::Done(r)) => {
                    done = Some(r);
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    done = Some(Err("the drawing stopped".into()));
                    break;
                }
            }
        }
        let Some(r) = done else {
            ctx.request_repaint_after(Duration::from_millis(100));
            return;
        };
        let (_, want) = self.job.take().unwrap();
        match r {
            Ok(()) => {
                self.s.set_icon_keys(&want);
                self.s.save();
                self.check_applied();
                self.update_message(None);
            }
            Err(e) => {
                self.log.push(format!("ERROR: {e}"));
                self.say("Keys applied, but the in-game key icons could not be made (see the log).", Tone::Error);
            }
        }
        ctx.request_repaint();
    }

    fn choose_root(&mut self) {
        let Some(p) = rfd::FileDialog::new().set_title("Lichii data folder (has config, load and nand inside)").pick_folder() else {
            return;
        };
        let mut p = p.to_string_lossy().into_owned();
        let user = std::path::Path::new(&p).join("user");
        if !lichii::is_data_folder(&p) && user.is_dir() {
            p = user.to_string_lossy().into_owned();
        }
        if !lichii::is_data_folder(&p) {
            self.show(
                "That folder has no config\\qt-config.ini. In Lichii: File > Open Lichii Folders > Root Data Folder shows the right one.",
            );
            return;
        }
        self.s.set_str("root", &p);
        self.s.save();
        store::tell_agent(store::CMD_RELOAD);
        self.refresh();
    }

    fn choose_nup(&mut self) {
        if let Some(p) = rfd::FileDialog::new().set_title("Game file (.nup)").add_filter("Game file", &["nup"]).pick_file() {
            self.s.set_str("nup", &p.to_string_lossy());
            self.s.save();
        }
    }

    // ---------------- drawing
    fn sidebar(&mut self, ui: &mut Ui) {
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.add_space(18.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(bold("Green Hat Kid").size(15.5).color(Color32::WHITE));
                ui.label(bold("Keyboard & Mouse").size(15.5).color(Color32::WHITE));
                ui.add_space(4.0);
                ui.label(RichText::new("Plus \u{00b7} native mouse camera").size(12.0).color(QUIET));
            });
        });
        ui.add_space(14.0);
        for (i, name) in PAGES.iter().enumerate() {
            if i == STEPS {
                ui.add_space(10.0); // the App page is not a step
            }
            let selected = self.page == i;
            let done = i < STEPS && self.step_done(i);
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 38.0), Sense::click());
            let r = rect.shrink2(Vec2::new(8.0, 2.0));
            if selected {
                ui.painter().rect_filled(r, 6.0, NAV_SEL);
            } else if resp.hovered() {
                ui.painter().rect_filled(r, 6.0, NAV_HOVER);
            }
            let color = if selected { Color32::WHITE } else if done { SUBTLE } else { NAV_TEXT };
            let text = ui.painter().text(
                egui::pos2(r.left() + 13.0, r.center().y),
                egui::Align2::LEFT_CENTER,
                *name,
                FontId::proportional(14.0),
                color,
            );
            if done {
                ui.painter().text(
                    egui::pos2(text.right() + 8.0, r.center().y),
                    egui::Align2::LEFT_CENTER,
                    "\u{2713}",
                    FontId::new(14.0, bold_family()),
                    GREEN_HI,
                );
            }
            if resp.clicked() {
                self.page = i;
            }
        }
        // the capture's state, and below it a short mouse panning line (the full text on its "?")
        ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
            ui.add_space(14.0);
            let w = ui.available_width() - 28.0;
            ui.spacing_mut().item_spacing.y = 4.0;
            if !self.agent.2.is_empty() {
                let help = self.agent.3.clone();
                let q = if help.is_empty() { 0.0 } else { 20.0 };
                let galley = ui.painter().layout(self.agent.2.clone(), FontId::proportional(12.0), QUIET, w - q);
                let size = Vec2::new(galley.size().x + q, galley.size().y.max(16.0));
                let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
                ui.painter().galley(egui::pos2(rect.left(), rect.center().y - galley.size().y / 2.0), galley, QUIET);
                if !help.is_empty() {
                    let mark = egui::Rect::from_center_size(egui::pos2(rect.right() - 8.0, rect.center().y), Vec2::splat(16.0));
                    help_mark(ui, mark, resp, 8.0, BUTTON_LINE, TEXT, |ui| {
                        ui.set_max_width(360.0);
                        ui.label(help);
                    });
                }
            }
            let c = match self.agent.0.as_str() {
                "active" => Color32::from_rgb(0x4f, 0xc3, 0x5f),
                "paused" => Color32::from_rgb(0xd9, 0xa4, 0x3a),
                "stopped" => ERROR,
                _ => QUIET,
            };
            let mut job = egui::text::LayoutJob::default();
            let fmt = |color| egui::TextFormat { font_id: FontId::proportional(13.5), color, ..Default::default() };
            job.append("\u{25cf} ", 0.0, fmt(c));
            job.append(&self.agent.1, 0.0, fmt(NAV_TEXT));
            job.wrap.max_width = w;
            job.halign = Align::Center;
            let galley = ui.painter().layout_job(job);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, galley.size().y), Sense::hover());
            ui.painter().galley(egui::pos2(rect.center().x, rect.top()), galley, NAV_TEXT);
        });
    }

    fn warning(&mut self, ui: &mut Ui) {
        let tray = self.s.flag("tray", true);
        let text = if tray {
            "\u{26a0}  Keep this open while you play. Closing the window will minimize it to the tray."
        } else {
            "\u{26a0}  Keep this open while you play. Closing the window will quit the app and stop the mouse capture."
        };
        Frame::new()
            .fill(WARN_BG)
            .stroke(Stroke::new(1.0, WARN_LINE))
            .corner_radius(8)
            .inner_margin(Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.add(egui::Label::new(RichText::new(text).color(WARN_TEXT)).wrap());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
                        help_mark(ui, rect, resp, 10.0, WARN_LINE, Color32::from_rgb(0xff, 0xf2, 0xcc), |ui| {
                            ui.set_max_width(440.0);
                            ui.label(bold("Why the app must stay open").color(Color32::WHITE));
                            ui.label("Lichii only tells the game where the pointer is on the game picture. On its own, the camera would stop turning as soon as the pointer reaches the edge of the picture.");
                            ui.label("While the game is in front, this app:");
                            ui.label("\u{2022} moves the pointer back toward the middle by an exact distance that the camera patch knows to ignore, so the camera can turn forever;");
                            ui.label("\u{2022} switches off Windows pointer acceleration and sets the pointer speed so one mouse step is one pixel of the game picture, which makes the camera speed exact (1:1).");
                            ui.label("Both stop the moment another window comes to the front, and your Windows mouse settings are never saved or changed for good.");
                            ui.label("If the app is closed, the camera hits an invisible wall at the screen edge and the speed setting stops being exact. Close the window to the tray instead of quitting.");
                        });
                    });
                });
            });
    }

    fn page_header(ui: &mut Ui, title: &str, sub: Option<&str>) {
        ui.label(RichText::new(title).size(20.0).color(TEXT));
        if let Some(s) = sub {
            ui.label(RichText::new(s).color(SUBTLE));
        }
        ui.add_space(4.0);
    }

    fn key_row(&mut self, ui: &mut Ui, name: &'static str, label: &'static str, hint: &str, dups: &[String]) {
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(Vec2::new(150.0, 30.0), Layout::left_to_right(Align::Center), |ui| {
                ui.set_min_width(150.0);
                ui.label(bold(label));
            });
            // what is left after this column's spacing and the 120-pixel key button (and its spacing)
            let rest = ui.available_width() - 120.0 - 2.0 * ui.spacing().item_spacing.x;
            ui.allocate_ui_with_layout(Vec2::new(rest.max(40.0), 30.0), Layout::left_to_right(Align::Center), |ui| {
                ui.set_min_width(rest.max(40.0));
                ui.add(egui::Label::new(RichText::new(hint).color(SUBTLE)).truncate());
            });
            let current = self.vals.key(name).to_string();
            let dup = dups.contains(&current);
            let btn = egui::Button::new(bold(keys::pretty(&current)).color(TEXT))
                .corner_radius(6)
                .min_size(Vec2::new(120.0, 30.0));
            let mut r = add_look(ui, true, btn, key_look(dup));
            if dup {
                r = r.on_hover_text("Also used by another control");
            }
            if r.clicked() {
                self.key_dialog = Some(KeyDialog { name, label, opened: Instant::now(), mods: ui.input(|i| i.modifiers) });
            }
        });
    }

    fn page_controls(&mut self, ui: &mut Ui) {
        Self::page_header(ui, "Controls", Some("Click a key to change it: press the new key or click a mouse button."));
        let dups = self.duplicate_keys();
        let mut v = self.vals.walk_speed;
        let mut changed = false;
        card(ui, "Movement", None, |ui| {
            for (n, label) in keys::MOVEMENT {
                self.key_row(ui, n, label, "", &dups);
            }
            ui.add_space(4.0);
            ui.label(bold("Slow walk speed"));
            ui.label(RichText::new("How fast the character moves while the slow-walk key is held (0.1 = very slow, 1.0 = no change).").color(SUBTLE));
            changed = slider_box(ui, "walk", &mut v, 0.1, 1.0, 0.05, 2);
        });
        if changed {
            self.vals.walk_speed = v;
            self.touched(PAGE_CONTROLS);
        }
        for (group, rows) in keys::GROUPS {
            card(ui, group, None, |ui| {
                for (n, label, hint) in rows {
                    self.key_row(ui, n, label, hint, &dups);
                }
            });
        }
    }

    fn page_camera(&mut self, ui: &mut Ui) {
        Self::page_header(ui, "Camera", Some("The mouse turns the camera directly, 1:1, like a PC game: also while aiming and riding horses."));
        let mut v = self.vals.clone();
        let mut changed = false;
        card(ui, "Speed", None, |ui| {
            ui.add_enabled_ui(!v.split_axes, |ui| {
                changed |= slider_box(ui, "sens", &mut v.mouse_sens, 0.1, 10.0, 0.1, 1);
            });
            let was = v.split_axes;
            if toggle(ui, &mut v.split_axes, "Different speeds for left/right and up/down") {
                changed = true;
                if v.split_axes && !was {
                    v.mouse_sens_x = v.mouse_sens;
                    v.mouse_sens_y = v.mouse_sens;
                }
            }
            if v.split_axes {
                ui.label("Left / right");
                changed |= slider_box(ui, "sensx", &mut v.mouse_sens_x, 0.1, 10.0, 0.1, 1);
                ui.label("Up / down");
                changed |= slider_box(ui, "sensy", &mut v.mouse_sens_y, 0.1, 10.0, 0.1, 1);
            }
        });
        card(ui, "Options", None, |ui| {
            changed |= toggle(ui, &mut v.invert_y, "Invert up/down (moving the mouse up looks down)");
            changed |= toggle(ui, &mut v.hide_cursor, "Hide the mouse pointer over the game");
            ui.label(RichText::new("The app turns on Lichii's mouse panning for you (what Ctrl+F9 does) when the game starts, and again if the pointer keeps showing while you play. Click the game picture once after starting; Alt+Tab always gives the normal mouse back.").color(SUBTLE));
        });
        if changed {
            self.vals = v;
            self.touched(PAGE_CAMERA);
        }
    }

    fn page_icons(&mut self, ui: &mut Ui) {
        Self::page_header(ui, "In-game key icons", Some("The game shows your keys instead of the gamepad buttons: in texts, on the HUD, on the ocarina song sheet and on the Basic Controls screen. Apply updates them whenever you change a key."));
        let nup = self.s.str("nup");
        let mut pick = false;
        card(ui, "Game file", Some("Needed once to make the key icons (read on this PC, never uploaded)."), |ui| {
            pick = path_row(ui, &nup, "Not found: pick the game file (.nup)");
        });
        if pick {
            self.choose_nup();
        }
        let want = self.vals.icon_keys();
        let status = match self.s.icon_keys() {
            None => ("Not installed yet: Apply makes them.".to_string(), Tone::Info),
            Some(_) if self.pictures_current() => ("\u{2713} The icons match your keys.".to_string(), Tone::Ok),
            Some(k) if k == want => (
                self.icons_issue().unwrap_or("Press Apply to update the in-game key icons.").to_string(),
                Tone::Info,
            ),
            Some(_) => ("Your keys changed: Apply to update the icons.".to_string(), Tone::Info),
        };
        card(ui, "Status", None, |ui| {
            ui.label(RichText::new(&status.0).color(if status.1 == Tone::Ok { OK_TEXT } else { TEXT }));
        });
        Frame::new().fill(SIDE).stroke(Stroke::new(1.0, CARD_LINE)).corner_radius(8).inner_margin(Margin::same(10)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            egui::ScrollArea::vertical().id_salt("log").max_height(160.0).auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui| {
                ui.set_min_height(160.0);
                for l in &self.log {
                    ui.label(RichText::new(l).monospace().color(NAV_TEXT));
                }
            });
        });
    }

    fn page_app(&mut self, ui: &mut Ui) {
        Self::page_header(ui, "App", None);
        let mut tray = self.s.flag("tray", true);
        let mut auto = self.autostart;
        card(ui, "Behaviour", None, |ui| {
            if toggle(ui, &mut tray, "Closing the window keeps the app running in the tray") {
                self.s.set_flag("tray", tray);
                self.s.save();
            }
            if toggle(ui, &mut auto, "Start with Windows (hidden in the tray)") {
                match settings::set_autostart(auto) {
                    Ok(()) => self.autostart = auto,
                    Err(e) => {
                        self.show(&format!("Could not change the Windows start-up setting: {e}"));
                    }
                }
            }
        });
        let root = self.s.str("root");
        let mut pick = false;
        card(ui, "Lichii", None, |ui| {
            pick = path_row(ui, &root, "Lichii data folder not found");
            if let Some((ok, text)) = &self.patch_text {
                ui.horizontal_wrapped(|ui| {
                    let (sym, color) = if *ok { ("\u{2713}", OK_TEXT) } else { ("\u{2715}", ERROR) };
                    ui.label(bold(sym).color(color));
                    ui.label(text);
                });
            }
        });
        if pick {
            self.choose_root();
        }
        let inst = self.installed;
        let busy = self.job.is_some();
        let result = self.remove_result.clone();
        let mut remove = false;
        card(
            ui,
            "Remove the mod",
            Some("Puts Lichii back as it was before the first Apply: Player 1's keys and mouse setting, and no controller profile, camera patch or in-game key icons from this app. Your settings here stay, so Apply can install it again."),
            |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let button = egui::Button::new(bold("Remove the mod").color(Color32::WHITE))
                        .corner_radius(6)
                        .min_size(Vec2::new(0.0, 30.0));
                    let can = inst.any() && !busy;
                    let r = add_look(ui, can, button, RED_LOOK);
                    let r = if inst.any() { r } else { r.on_disabled_hover_text("Nothing to remove: the mod is not in Lichii.") };
                    remove = r.clicked();
                    match &result {
                        Some((ok, text)) => {
                            ui.add(egui::Label::new(bold(text).color(if *ok { OK_TEXT } else { ERROR })).wrap());
                        }
                        None if !inst.any() => {
                            ui.label(RichText::new("The mod is not in Lichii.").color(SUBTLE));
                        }
                        None => {}
                    }
                });
            },
        );
        if remove {
            self.dialog = Some(Dialog {
                text: "Remove the mod from Lichii? Player 1's keys and mouse setting go back to how they were before the first Apply, and the controller profile, the camera patch and the in-game key icons are deleted. Your settings in this app stay.".into(),
                confirm: Some(Confirm::Remove),
            });
        }
        card(ui, "About", None, |ui| {
            ui.label(format!("{APP} {VERSION}"));
            ui.label("Based on lichii-keyboard-mouse by exgde/joaobyss (MIT): key mapping, icon tools and the native camera. Remade in Rust as one program, with a new settings window, the tray, a reworked mouse capture, the camera while aiming and riding horses, and in-game key icons drawn for your keys.");
        });
    }

    fn bottom_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let step = self.page < STEPS;
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if step {
                    let last = self.page == STEPS - 1;
                    let text = if last { "Apply" } else { "Next" };
                    let main = egui::Button::new(bold(text).color(Color32::WHITE))
                        .corner_radius(6)
                        .min_size(Vec2::new(80.0, 32.0));
                    let enabled = self.job.is_none() && !(last && self.nothing_to_apply());
                    let r = add_look(ui, enabled, main, GREEN_LOOK);
                    let r = if last && self.nothing_to_apply() { r.on_disabled_hover_text("Nothing to apply: Lichii already has these settings.") } else { r };
                    if r.clicked() {
                        if last {
                            self.apply();
                        } else {
                            self.next_done[self.page] = true;
                            self.page += 1;
                        }
                    }
                    if self.page == PAGE_CONTROLS || self.page == PAGE_CAMERA {
                        let page = self.page;
                        let at = self.page_at_defaults(page);
                        let defaults = egui::Button::new("Defaults").corner_radius(6).min_size(Vec2::new(76.0, 32.0));
                        let r = ui.add_enabled(!at, defaults);
                        let r = if at { r.on_disabled_hover_text("This page already has the default settings.") } else { r };
                        if r.clicked() {
                            let what = if page == PAGE_CONTROLS { "the keys and the slow walk speed" } else { "the camera settings" };
                            self.dialog = Some(Dialog {
                                text: format!("Put {what} back to the defaults? Nothing changes in Lichii until you press Apply."),
                                confirm: Some(Confirm::Defaults(page)),
                            });
                        }
                    }
                }
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    for (text, tone) in &self.message {
                        let mut t = RichText::new(text).color(tone_color(*tone));
                        if *tone != Tone::Info {
                            t = t.family(bold_family());
                        }
                        ui.add(egui::Label::new(t).wrap());
                    }
                });
            });
        });
    }

    /// The window's own dialogs: a message with OK, or a question with Yes / No.
    fn dialog(&mut self, ctx: &egui::Context) {
        let Some(d) = &self.dialog else { return };
        let (text, confirm) = (d.text.clone(), d.confirm);
        let mut answer: Option<bool> = None;
        Modal::new(Id::unique("message-dialog"))
            .frame(dialog_frame(Margin::symmetric(26, 22)))
            .show(ctx, |ui| {
                ui.set_width(400.0);
                ui.add(egui::Label::new(RichText::new(&text).color(TEXT)).wrap());
                ui.add_space(12.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let primary = |ui: &mut Ui, t: &str| {
                        let b = egui::Button::new(bold(t).color(Color32::WHITE)).corner_radius(6).min_size(Vec2::new(76.0, 30.0));
                        add_look(ui, true, b, GREEN_LOOK)
                    };
                    if confirm.is_some() {
                        if ui.add(egui::Button::new("No").corner_radius(6).min_size(Vec2::new(76.0, 30.0))).clicked() {
                            answer = Some(false);
                        }
                        if primary(ui, "Yes").clicked() {
                            answer = Some(true);
                        }
                    } else if primary(ui, "OK").clicked() {
                        answer = Some(true);
                    }
                });
            });
        ctx.input(|i| {
            if i.key_pressed(Key::Escape) {
                answer = Some(false);
            } else if i.key_pressed(Key::Enter) && confirm.is_none() {
                answer = Some(true);
            }
        });
        let Some(yes) = answer else { return };
        self.dialog = None;
        match (confirm, yes) {
            (Some(Confirm::CloseDuringJob), true) => {
                self.job_close_ok = true;
                self.close_again = true;
            }
            (Some(Confirm::Defaults(page)), true) => self.reset_page(page),
            (Some(Confirm::Remove), true) => self.remove_mod(),
            (Some(Confirm::Quit), true) => {
                self.quit_confirmed = true;
                store::tell_agent(store::CMD_QUIT);
                self.exit_now = true;
            }
            _ => {}
        }
    }

    fn key_dialog(&mut self, ctx: &egui::Context) {
        let Some(d) = &self.key_dialog else { return };
        let (name, label, opened, prev) = (d.name, d.label, d.opened, d.mods);
        let mut result: Option<Option<String>> = None; // Some(None) = cancel, Some(Some(k)) = chosen
        if opened.elapsed() > Duration::from_millis(150) {
            ctx.input(|i| {
                for e in &i.events {
                    match e {
                        egui::Event::Key { key, pressed: true, .. } => {
                            result = Some(match key {
                                Key::Escape => None,
                                Key::Backspace | Key::Delete => Some("none".to_string()),
                                k => key_name_of(*k).map(String::from),
                            })
                            .filter(|r| r.is_some() || *key == Key::Escape);
                        }
                        egui::Event::PointerButton { button, pressed: true, .. } => {
                            result = Some(Some(
                                match button {
                                    PointerButton::Primary => "mouse_left",
                                    PointerButton::Secondary => "mouse_right",
                                    PointerButton::Middle => "mouse_middle",
                                    PointerButton::Extra1 => "mouse_back",
                                    PointerButton::Extra2 => "mouse_forward",
                                }
                                .to_string(),
                            ));
                        }
                        _ => {}
                    }
                }
                // modifier keys alone give no key event: watch them change
                let m = i.modifiers;
                if result.is_none() {
                    if m.shift && !prev.shift {
                        result = Some(Some("shift".into()));
                    } else if m.ctrl && !prev.ctrl {
                        result = Some(Some("ctrl".into()));
                    } else if m.alt && !prev.alt {
                        result = Some(Some("alt".into()));
                    }
                }
            });
        }
        if let Some(d) = &mut self.key_dialog {
            d.mods = ctx.input(|i| i.modifiers);
        }
        Modal::new(Id::unique("key-dialog"))
            .frame(dialog_frame(Margin::symmetric(28, 24)))
            .show(ctx, |ui| {
                ui.set_width(360.0);
                ui.vertical_centered(|ui| {
                    ui.label(bold(label).size(15.0));
                    ui.label(RichText::new("Press a key or click a mouse button in this window.\nBackspace clears it, Esc cancels.").color(SUBTLE));
                });
            });
        if let Some(r) = result {
            self.key_dialog = None;
            if let Some(k) = r {
                self.vals.keys.insert(name.to_string(), k);
                self.touched(PAGE_CONTROLS);
            }
        }
    }

    /// The page's settings are the defaults (your layout) already.
    fn page_at_defaults(&self, page: usize) -> bool {
        let mut v = self.vals.clone();
        page_defaults(&mut v, page);
        v == self.vals
    }

    /// Puts one page's settings back to the defaults (nothing is applied yet).
    fn reset_page(&mut self, page: usize) {
        page_defaults(&mut self.vals, page);
        self.touched(page);
    }

    /// Removes the mod from Lichii and says whether everything went.
    fn remove_mod(&mut self) {
        let root = self.s.str("root");
        if !lichii::is_data_folder(&root) {
            self.show("Pick Lichii's data folder first.");
            return;
        }
        if !self.lichii_closed("this") {
            return;
        }
        let mut lines = Vec::new();
        let r = lichii::remove(&root, &mut |l| lines.push(l));
        self.log.extend(lines);
        self.s.clear_icon_keys();
        self.s.save();
        store::tell_agent(store::CMD_RELOAD);
        self.next_done = [false; STEPS];
        self.refresh();
        let left = self.installed.any();
        self.remove_result = Some(match (&r, left) {
            (Ok(()), false) => (true, "\u{2713} Removed: Lichii is back as it was before the mod.".to_string()),
            (Err(e), _) => {
                self.log.push(format!("ERROR: {e}"));
                (false, "\u{2715} Not everything could be removed (see the log on the In-game key icons page).".to_string())
            }
            (Ok(()), true) => (false, "\u{2715} Something of the mod is still in Lichii. Try again with Lichii closed.".to_string()),
        });
        self.update_message(None);
    }

    /// The tray's Quit, chosen while this window is open: ask here, and close with the app on Yes.
    fn ask_quit(&mut self) {
        let text = if self.job.is_some() {
            "Quit? The mouse capture stops until the app is opened again, and the in-game key icons being made are made again on the next Apply."
        } else {
            "Quit? The mouse capture stops until the app is opened again."
        };
        self.dialog = Some(Dialog { text: text.into(), confirm: Some(Confirm::Quit) });
    }

    /// The window's close button: true when the window may close now. A question shows a dialog instead.
    fn close_requested(&mut self) -> bool {
        if self.quit_confirmed {
            return true;
        }
        if self.job.is_some() && !self.job_close_ok {
            self.dialog = Some(Dialog {
                text: "The in-game key icons are still being made. Close anyway? Apply makes them again next time.".into(),
                confirm: Some(Confirm::CloseDuringJob),
            });
            return false;
        }
        if self.s.flag("tray", true) {
            // closing ends this window only; the agent keeps the capture and the tray icon
            if !self.s.flag("tray_hint_shown", false) && store::tell_agent(store::CMD_TRAY_HINT) {
                self.s.set_flag("tray_hint_shown", true);
                self.s.save();
            }
            return true;
        }
        self.ask_quit();
        false
    }

    fn logic(&mut self, ctx: &egui::Context) {
        if self.last_refresh.is_none() {
            self.refresh();
        }
        self.poll_job(ctx);
    }

    /// When the once-a-second status check is due (the window waits for it between frames).
    fn next_refresh(&self) -> Instant {
        self.last_refresh.map_or_else(Instant::now, |t| t + Duration::from_secs(1))
    }

    /// The once-a-second status check, between frames: true when something on screen changed, so the window
    /// is only drawn again then (an idle window costs next to nothing).
    fn tick(&mut self) -> bool {
        let snapshot =
            |a: &App| (a.agent.clone(), a.patch_text.clone(), a.log.len(), a.message.clone(), a.vals.clone(), a.applied, a.installed);
        let before = snapshot(self);
        self.refresh();
        before != snapshot(self)
    }

    fn ui(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        egui::Panel::left("nav")
            .exact_size(220.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::new().fill(SIDE))
            .show(ui, |ui| self.sidebar(ui));
        egui::Panel::bottom("bottom")
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::new().fill(BG).inner_margin(Margin { left: 28, right: 28, top: 8, bottom: 16 }))
            .show(ui, |ui| self.bottom_bar(ui));
        egui::CentralPanel::no_frame()
            .frame(Frame::new().fill(BG).inner_margin(Margin { left: 28, right: 20, top: 20, bottom: 4 }))
            .show(ui, |ui| {
                self.warning(ui);
                ui.add_space(10.0);
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.set_width(ui.available_width() - 10.0);
                    match self.page {
                        PAGE_CONTROLS => self.page_controls(ui),
                        PAGE_CAMERA => self.page_camera(ui),
                        PAGE_ICONS => self.page_icons(ui),
                        _ => self.page_app(ui),
                    }
                });
            });
        self.key_dialog(&ctx);
        self.dialog(&ctx);
    }
}

/// The title bar in the window's own colours (Windows 11 draws it; older Windows keeps its dark title bar).
fn title_bar_colors(window: &winit::window::Window) {
    use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR};
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let Ok(handle) = window.window_handle() else { return };
    let RawWindowHandle::Win32(h) = handle.as_raw() else { return };
    let colorref = |c: Color32| c.r() as u32 | (c.g() as u32) << 8 | (c.b() as u32) << 16;
    for (attr, color) in [(DWMWA_CAPTION_COLOR, SIDE), (DWMWA_TEXT_COLOR, NAV_TEXT), (DWMWA_BORDER_COLOR, CARD_LINE)] {
        let v = colorref(color);
        unsafe {
            DwmSetWindowAttribute(h.hwnd.get() as _, attr as u32, &v as *const u32 as _, 4);
        }
    }
}

fn window_icon() -> Option<winit::window::Icon> {
    // the same drawing as the exe's icon: green rounded square, white mouse
    let n = 64usize;
    let mut rgba = vec![0u8; n * n * 4];
    let inside_rr = |x: f32, y: f32, x0: f32, y0: f32, x1: f32, y1: f32, r: f32| {
        let cx = x.clamp(x0 + r, x1 - r);
        let cy = y.clamp(y0 + r, y1 - r);
        (x - cx).powi(2) + (y - cy).powi(2) <= r * r
    };
    for y in 0..n {
        for x in 0..n {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let i = (y * n + x) * 4;
            let mut c = None;
            if inside_rr(fx, fy, 2.0, 2.0, 62.0, 62.0, 14.0) {
                c = Some([0x3f, 0x8f, 0x4f]);
            }
            if inside_rr(fx, fy, 19.0, 12.0, 45.0, 52.0, 13.0) {
                c = Some([0xf2, 0xf2, 0xf2]);
                if ((fx - 32.0).abs() < 1.5 && fy < 27.0) || ((fy - 27.0).abs() < 1.5) {
                    c = Some([0x3f, 0x8f, 0x4f]);
                }
            }
            if let Some([r, g, b]) = c {
                rgba[i..i + 4].copy_from_slice(&[r, g, b, 255]);
            }
        }
    }
    winit::window::Icon::from_rgba(rgba, n as u32, n as u32).ok()
}

/// Set by the message hook when the agent asks this window to ask about quitting.
static ASK_QUIT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The window, its input and the CPU drawing: what eframe would do, without a graphics driver.
struct Runner {
    ctx: egui::Context,
    app: Option<App>,
    window: Option<Rc<winit::window::Window>>,
    state: Option<egui_winit::State>,
    surface: Option<softbuffer::Surface<Rc<winit::window::Window>, Rc<winit::window::Window>>>,
    renderer: egui_software_backend::EguiSoftwareRender,
    next_paint: Option<Instant>,
}

impl Runner {
    fn paint(&mut self) {
        let (Some(window), Some(state), Some(app), Some(surface)) =
            (self.window.clone(), self.state.as_mut(), self.app.as_mut(), self.surface.as_mut())
        else {
            return;
        };
        let raw = state.take_egui_input(&window);
        let out = self.ctx.run_ui(raw, |ui| {
            app.logic(ui.ctx());
            app.ui(ui);
        });
        state.handle_platform_output(&window, out.platform_output);
        let delay = out.viewport_output.get(&egui::ViewportId::ROOT).map(|v| v.repaint_delay).unwrap_or(Duration::MAX);
        self.next_paint = Instant::now().checked_add(delay);
        let size = window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else { return };
        let prims = self.ctx.tessellate(out.shapes, out.pixels_per_point);
        if surface.resize(w, h).is_err() {
            return;
        }
        let Ok(mut buffer) = surface.buffer_mut() else { return };
        let bg = (BG.r() as u32) << 16 | (BG.g() as u32) << 8 | BG.b() as u32;
        buffer.fill(bg);
        // softbuffer pixels are 0x00RRGGBB, i.e. bytes B, G, R, 0: the renderer writes in that (Bgra) order
        let pixels: &mut [[u8; 4]] =
            unsafe { std::slice::from_raw_parts_mut(buffer.as_mut_ptr() as *mut [u8; 4], buffer.len()) };
        let mut target = egui_software_backend::BufferMutRef::new(pixels, w.get() as usize, h.get() as usize);
        self.renderer.render(&mut target, &prims, &out.textures_delta, out.pixels_per_point);
        let _ = buffer.present();
    }
}

impl winit::application::ApplicationHandler for Runner {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = winit::window::Window::default_attributes()
            .with_title(APP)
            .with_inner_size(winit::dpi::LogicalSize::new(980.0, 700.0))
            .with_min_inner_size(winit::dpi::LogicalSize::new(860.0, 600.0))
            .with_theme(Some(winit::window::Theme::Dark))
            .with_window_icon(window_icon());
        let Ok(window) = event_loop.create_window(attrs) else {
            event_loop.exit();
            return;
        };
        title_bar_colors(&window);
        let window = Rc::new(window);
        let Ok(context) = softbuffer::Context::new(window.clone()) else {
            event_loop.exit();
            return;
        };
        self.surface = softbuffer::Surface::new(&context, window.clone()).ok();
        self.state = Some(egui_winit::State::new(
            self.ctx.clone(),
            egui::ViewportId::ROOT,
            &*window,
            Some(window.scale_factor() as f32),
            Some(winit::window::Theme::Dark),
            None,
        ));
        self.app = Some(App::new(&self.ctx));
        self.window = Some(window.clone());
        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        use winit::event::WindowEvent;
        let Some(window) = self.window.clone() else { return };
        match &event {
            WindowEvent::CloseRequested => {
                if self.app.as_mut().is_none_or(|a| a.close_requested()) {
                    event_loop.exit();
                } else {
                    window.request_redraw(); // show the question
                }
                return;
            }
            WindowEvent::RedrawRequested => {
                self.paint();
                if let Some(app) = self.app.as_mut() {
                    if app.close_again {
                        app.close_again = false;
                        if app.close_requested() {
                            app.exit_now = true;
                        }
                        window.request_redraw();
                    }
                    if app.exit_now {
                        event_loop.exit();
                    }
                }
                return;
            }
            _ => {}
        }
        if let Some(state) = self.state.as_mut() {
            if state.on_window_event(&window, &event).repaint {
                window.request_redraw();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        use winit::event_loop::ControlFlow;
        let now = Instant::now();
        let mut redraw = false;
        if ASK_QUIT.swap(false, std::sync::atomic::Ordering::Relaxed) {
            if let (Some(app), Some(w)) = (self.app.as_mut(), &self.window) {
                app.ask_quit();
                w.set_minimized(false);
                w.focus_window();
                redraw = true;
            }
        }
        if self.next_paint.is_some_and(|t| t <= now) {
            self.next_paint = None;
            redraw = true;
        }
        let mut wake = self.next_paint;
        if let Some(app) = self.app.as_mut() {
            if app.next_refresh() <= now && app.tick() {
                redraw = true;
            }
            let r = app.next_refresh();
            wake = Some(wake.map_or(r, |w| w.min(r)));
        }
        if redraw {
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }
        event_loop.set_control_flow(wake.map_or(ControlFlow::Wait, ControlFlow::WaitUntil));
    }
}

pub fn run() -> i32 {
    // one settings window: a second start brings the open one to the front
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows_sys::Win32::System::Threading::CreateMutexW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{SetForegroundWindow, ShowWindow, SW_RESTORE};
    let name = store::wide("GreenHatKid-KBM-Plus-settings-mutex");
    unsafe {
        CreateMutexW(std::ptr::null(), 0, name.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            if let Some(h) = store::settings_window() {
                ShowWindow(h, SW_RESTORE);
                SetForegroundWindow(h);
            }
            return 0;
        }
    }
    // the agent's request (the tray's Quit) arrives as a posted message: note it, the loop then shows the question
    let control = store::control_message();
    let mut builder = winit::event_loop::EventLoop::builder();
    {
        use winit::platform::windows::EventLoopBuilderExtWindows;
        builder.with_msg_hook(move |msg| {
            let m = unsafe { &*(msg as *const windows_sys::Win32::UI::WindowsAndMessaging::MSG) };
            if m.message == control && m.wParam == store::CMD_ASK_QUIT {
                ASK_QUIT.store(true, std::sync::atomic::Ordering::Relaxed);
                return true;
            }
            false
        });
    }
    let Ok(event_loop) = builder.build() else { return 1 };
    let mut runner = Runner {
        ctx: egui::Context::default(),
        app: None,
        window: None,
        state: None,
        surface: None,
        renderer: egui_software_backend::EguiSoftwareRender::new(egui_software_backend::ColorFieldOrder::Bgra),
        next_paint: None,
    };
    match event_loop.run_app(&mut runner) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}
