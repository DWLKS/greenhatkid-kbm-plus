//! settings.json as the settings window uses it (unknown fields are kept), and the
//! "start with Windows" entry in the current user's Run key.

use crate::keys::Values;
use crate::store::{settings_path, state_dir};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub struct Settings {
    pub json: Value,
}

impl Settings {
    pub fn load() -> Settings {
        let mut json = std::fs::read_to_string(settings_path())
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .filter(|v| v.is_object())
            .unwrap_or_else(|| json!({}));
        let o = json.as_object_mut().unwrap();
        o.entry("tray").or_insert(json!(true));
        o.entry("root").or_insert(json!(""));
        o.entry("nup").or_insert(json!(""));
        o.entry("icon_keys").or_insert(Value::Null);
        Settings { json }
    }

    pub fn save(&self) {
        let _ = std::fs::create_dir_all(state_dir());
        let tmp = settings_path().with_extension("json.tmp");
        if std::fs::write(&tmp, serde_json::to_string_pretty(&self.json).unwrap_or_default()).is_ok() {
            let _ = std::fs::rename(&tmp, settings_path());
        }
    }

    pub fn values(&self) -> Values {
        Values::from_json(self.json.get("values"))
    }

    pub fn set_values(&mut self, v: &Values) {
        self.json["values"] = v.to_json();
    }

    pub fn str(&self, key: &str) -> String {
        self.json.get(key).and_then(|v| v.as_str()).unwrap_or("").to_string()
    }

    pub fn set_str(&mut self, key: &str, value: &str) {
        self.json[key] = json!(value);
    }

    pub fn flag(&self, key: &str, default: bool) -> bool {
        self.json.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
    }

    pub fn set_flag(&mut self, key: &str, value: bool) {
        self.json[key] = json!(value);
    }

    pub fn icon_keys(&self) -> Option<BTreeMap<String, String>> {
        self.json.get("icon_keys").and_then(|v| v.as_object()).map(|o| {
            o.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect()
        })
    }

    /// The in-game key icons are no longer in Lichii (the mod was removed).
    pub fn clear_icon_keys(&mut self) {
        self.json["icon_keys"] = Value::Null;
    }

    pub fn set_icon_keys(&mut self, keys: &BTreeMap<String, String>) {
        self.json["icon_keys"] = serde_json::to_value(keys).unwrap_or(Value::Null);
        self.json["icons_version"] = json!(crate::icons::DRAWING_VERSION);
    }

    /// The drawing version the installed in-game key icons were made with (0 before versions were recorded).
    pub fn icons_version(&self) -> u64 {
        self.json.get("icons_version").and_then(|v| v.as_u64()).unwrap_or(0)
    }
}

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "GreenHatKid-KBM-Plus";

pub fn autostart_enabled() -> bool {
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    hkcu.open_subkey(RUN_KEY).and_then(|k| k.get_value::<String, _>(RUN_VALUE)).is_ok()
}

pub fn set_autostart(on: bool) -> std::io::Result<()> {
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(RUN_KEY)?;
    if on {
        let exe = std::env::current_exe()?;
        key.set_value(RUN_VALUE, &format!("\"{}\" --tray", exe.display()))
    } else {
        match key.delete_value(RUN_VALUE) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }
}
