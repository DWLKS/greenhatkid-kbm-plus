//! Key names, the controls grouped for the window, the defaults (the user's layout) and Lichii binding strings.
//! Key names follow the original mod's settings file, so its key icon rules understand them unchanged.

use std::collections::BTreeMap;

const MOUSE_BUTTONS: [(&str, u32); 5] =
    [("mouse_left", 0), ("mouse_right", 1), ("mouse_middle", 2), ("mouse_back", 3), ("mouse_forward", 4)];

const NAMED: [(&str, u32); 30] = [
    ("space", 32), ("enter", 16777220), ("tab", 16777217), ("backspace", 16777219), ("esc", 16777216),
    ("insert", 16777222), ("delete", 16777223), ("home", 16777232), ("end", 16777233),
    ("left", 16777234), ("up", 16777235), ("right", 16777236), ("down", 16777237),
    ("pageup", 16777238), ("pagedown", 16777239), ("shift", 16777248), ("ctrl", 16777249), ("alt", 16777251),
    ("capslock", 16777252), ("comma", 44), ("minus", 45), ("period", 46), ("slash", 47), ("semicolon", 59),
    ("equals", 61), ("bracketleft", 91), ("backslash", 92), ("bracketright", 93), ("grave", 96), ("apostrophe", 39),
];

/// Qt key code of a key name (the code Lichii stores), None for unknown names.
fn key_code(name: &str) -> Option<u32> {
    let n = name.to_lowercase();
    if n.len() == 1 {
        let c = n.as_bytes()[0];
        if c.is_ascii_lowercase() {
            return Some(c.to_ascii_uppercase() as u32);
        }
        if c.is_ascii_digit() {
            return Some(c as u32);
        }
    }
    if let Some(f) = n.strip_prefix('f').and_then(|d| d.parse::<u32>().ok()).filter(|d| (1..=12).contains(d)) {
        return Some(16777263 + f);
    }
    NAMED.iter().find(|(k, _)| *k == n).map(|(_, v)| *v)
}

pub fn pretty(name: &str) -> String {
    let n = name.to_lowercase();
    let fixed = match n.as_str() {
        "mouse_left" => "Left click", "mouse_right" => "Right click", "mouse_middle" => "Middle click",
        "mouse_back" => "Mouse back", "mouse_forward" => "Mouse forward", "none" | "" => "(none)",
        "space" => "Space", "shift" => "Shift", "enter" => "Enter", "tab" => "Tab", "backspace" => "Backspace",
        "esc" => "Esc", "capslock" => "Caps Lock", "up" => "↑ Up", "down" => "↓ Down", "left" => "← Left",
        "right" => "→ Right", "pageup" => "Page Up", "pagedown" => "Page Down", "ctrl" => "Ctrl", "alt" => "Alt",
        _ => "",
    };
    if !fixed.is_empty() {
        return fixed.to_string();
    }
    if n.len() == 1 || (n.starts_with('f') && n[1..].parse::<u32>().is_ok()) {
        return n.to_uppercase();
    }
    let mut c = n.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

/// Lichii keys that clash with the emulator's own shortcuts (allowed, but warned about).
pub const RESERVED: [(&str, &str); 3] = [
    ("ctrl", "Ctrl+Q quits Lichii and Ctrl+F9 hides the pointer"),
    ("alt", "Alt opens Lichii's menu bar"),
    ("esc", "Esc leaves fullscreen"),
];

/// A control: (setting, label, what it does in the game).
type Control = (&'static str, &'static str, &'static str);

/// The controls, grouped as in the window.
pub const GROUPS: [(&str, [Control; 6]); 3] = [
    ("Combat", [
        ("button_y", "Attack", "Y: swing the sword"),
        ("button_zl", "Target (hold)", "ZL: lock on to an enemy or look ahead"),
        ("button_zr", "Use item", "ZR: use the selected item"),
        ("button_a", "Action", "A: talk, open, roll; hold to run"),
        ("button_x", "Jump", "X: jump"),
        ("button_b", "Put Away", "B: put weapon/item away, cancel"),
    ]),
    ("Items", [
        ("button_l", "Previous item", "L: quick select items (left)"),
        ("button_r", "Next item", "R: quick select items (right)"),
        ("dpad_up", "Accessory", "D-pad up"),
        ("dpad_left", "Ocarina", "D-pad left"),
        ("dpad_right", "Fairy", "D-pad right"),
        ("dpad_down", "Boots", "D-pad down"),
    ]),
    ("Menus and camera", [
        ("button_plus", "Menu", "+: open the menu"),
        ("button_minus", "Map", "-: open the map"),
        ("button_r3", "Reset camera", "Right stick press: reset camera position"),
        ("button_l3", "Left stick press", "Not used by the game"),
        ("button_home", "Home", "Switch Home button"),
        ("button_screenshot", "Capture", "Switch capture button"),
    ]),
];

pub const MOVEMENT: [(&str, &str); 5] = [
    ("move_up", "Forward"), ("move_down", "Back"), ("move_left", "Left"), ("move_right", "Right"),
    ("walk_key", "Walk slowly (hold)"),
];

pub fn binding_names() -> Vec<&'static str> {
    let mut v: Vec<&str> = GROUPS.iter().flat_map(|(_, rows)| rows.iter().map(|r| r.0)).collect();
    v.extend(MOVEMENT.iter().map(|m| m.0));
    v
}

/// Buttons whose in-game key icons follow the chosen key (same list as the original configurator).
const ICON_KEYS: [&str; 19] = [
    "button_a", "button_b", "button_x", "button_y", "button_l", "button_r", "button_zl", "button_zr", "button_r3",
    "button_plus", "button_minus", "dpad_up", "dpad_left", "dpad_right", "dpad_down", "move_up", "move_down",
    "move_left", "move_right",
];

/// Everything the user can change, as stored under "values" in settings.json.
#[derive(Clone, PartialEq, Debug)]
pub struct Values {
    pub keys: BTreeMap<String, String>,
    pub walk_speed: f64,
    pub mouse_sens: f64,
    pub split_axes: bool,
    pub mouse_sens_x: f64,
    pub mouse_sens_y: f64,
    pub invert_y: bool,
    pub hide_cursor: bool,
}

impl Default for Values {
    fn default() -> Self {
        let keys = [
            ("button_y", "mouse_left"), ("button_a", "shift"), ("button_b", "q"), ("button_x", "space"),
            ("button_zl", "mouse_right"), ("button_zr", "e"), ("button_l", "z"), ("button_r", "x"),
            ("button_r3", "mouse_middle"), ("button_l3", "none"), ("button_plus", "n"), ("button_minus", "m"),
            ("dpad_up", "up"), ("dpad_left", "left"), ("dpad_right", "right"), ("dpad_down", "down"),
            ("button_home", "none"), ("button_screenshot", "none"),
            ("move_up", "w"), ("move_down", "s"), ("move_left", "a"), ("move_right", "d"), ("walk_key", "c"),
        ];
        Values {
            keys: keys.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            walk_speed: 0.5,
            mouse_sens: 2.0,
            split_axes: false,
            mouse_sens_x: 2.0,
            mouse_sens_y: 2.0,
            invert_y: false,
            hide_cursor: true,
        }
    }
}

impl Values {
    pub fn key(&self, name: &str) -> &str {
        self.keys.get(name).map(String::as_str).unwrap_or("none")
    }

    pub fn from_json(v: Option<&serde_json::Value>) -> Values {
        let mut out = Values::default();
        let Some(o) = v.and_then(|v| v.as_object()) else { return out };
        for name in binding_names() {
            if let Some(s) = o.get(name).and_then(|x| x.as_str()) {
                out.keys.insert(name.to_string(), s.to_string());
            }
        }
        let f = |k: &str, d: f64| o.get(k).and_then(|x| x.as_f64()).unwrap_or(d);
        let b = |k: &str, d: bool| o.get(k).and_then(|x| x.as_bool()).unwrap_or(d);
        out.walk_speed = f("walk_speed", out.walk_speed);
        out.mouse_sens = f("mouse_sens", out.mouse_sens);
        out.split_axes = b("split_axes", out.split_axes);
        out.mouse_sens_x = f("mouse_sens_x", out.mouse_sens_x);
        out.mouse_sens_y = f("mouse_sens_y", out.mouse_sens_y);
        out.invert_y = b("invert_y", out.invert_y);
        out.hide_cursor = b("hide_cursor", out.hide_cursor);
        out
    }

    pub fn to_json(&self) -> serde_json::Value {
        let mut o = serde_json::Map::new();
        for (k, v) in &self.keys {
            o.insert(k.clone(), serde_json::Value::String(v.clone()));
        }
        o.insert("walk_speed".into(), self.walk_speed.into());
        o.insert("mouse_sens".into(), self.mouse_sens.into());
        o.insert("split_axes".into(), self.split_axes.into());
        o.insert("mouse_sens_x".into(), self.mouse_sens_x.into());
        o.insert("mouse_sens_y".into(), self.mouse_sens_y.into());
        o.insert("invert_y".into(), self.invert_y.into());
        o.insert("hide_cursor".into(), self.hide_cursor.into());
        serde_json::Value::Object(o)
    }

    /// The keys the in-game key icons depend on (what "icon_keys" in settings.json records).
    pub fn icon_keys(&self) -> BTreeMap<String, String> {
        ICON_KEYS.iter().map(|k| (k.to_string(), self.key(k).to_string())).collect()
    }
}

/// The Lichii binding string for a key name.
pub fn binding(name: &str) -> Result<String, String> {
    let v = name.trim().to_lowercase();
    if v.is_empty() || v == "none" {
        return Ok("engine:keyboard,code:0,toggle:0".into());
    }
    if let Some((_, b)) = MOUSE_BUTTONS.iter().find(|(n, _)| *n == v) {
        return Ok(format!("engine:mouse,button:{b}"));
    }
    key_code(&v).map(|c| format!("engine:keyboard,code:{c},toggle:0")).ok_or(format!("unknown key name: {name}"))
}
