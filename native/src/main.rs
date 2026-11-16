//! Green Hat Kid Keyboard & Mouse Plus (native). One exe:
//!   (no arguments)        start the background agent if needed and open the settings window
//!   --tray                start only the background agent (start with Windows)
//!   --settings            the settings window (only runs while open)
//! Checks used while developing:
//!   --selftest-agent <s>                          run the agent for <s> seconds, then quit
//!   --selftest-apply <folder>                     Apply the default settings to a Lichii data folder
//!   --selftest-icons <folder> <nup> [keys.json]   make the in-game key icons into <folder>
//!   --selftest-guides <out.pgm> [keys.json]       the Basic Controls guide lines, as one picture
#![windows_subsystem = "windows"]

mod agent;
mod capture;
mod font;
mod gamefiles;
mod guides;
mod icons;
mod keys;
mod lichii;
mod outline;
mod panning;
mod raster;
mod settings;
mod store;
mod textures;
mod ui;

/// The keys of a keys.json file (icon key -> key name), or the defaults when there is none or it cannot be read.
fn keys_file(path: Option<&String>) -> std::collections::BTreeMap<String, String> {
    path.and_then(|f| std::fs::read_to_string(f).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| keys::Values::default().icon_keys())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("--selftest-agent") => {
            let secs = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(3);
            agent::run(false, Some(secs))
        }
        Some("--settings") => ui::run(),
        // apply the default values (the user's layout) to the Lichii folder given
        Some("--selftest-apply") => match args.get(1) {
            Some(root) => match lichii::apply(root, &keys::Values::default(), &mut |_| {}) {
                Ok(()) => 0,
                Err(_) => 1,
            },
            None => 2,
        },
        // make the in-game key icons into a folder laid out like Lichii's (load\...)
        Some("--selftest-icons") => match args.get(1) {
            Some(out) => {
                let nup = args.get(2).map(String::as_str).unwrap_or("");
                let keys = keys_file(args.get(3));
                let mut log = String::new();
                let t = std::time::Instant::now();
                let r = icons::build(out, nup, &keys, &mut |l| log.push_str(&format!("{:.2}s {l}\n", t.elapsed().as_secs_f64())));
                if let Err(e) = &r {
                    log.push_str(&format!("ERROR: {e}\n"));
                }
                let _ = std::fs::write(std::path::Path::new(out).join("selftest-icons.log"), log);
                if r.is_ok() { 0 } else { 1 }
            }
            None => 2,
        },
        // check: the Basic Controls guide sheets for keys.json (or the defaults), all five in one PGM picture
        Some("--selftest-guides") => match args.get(1) {
            Some(out) => {
                let keys = keys_file(args.get(2));
                let (w, h) = (1555, 1511);
                let t = std::time::Instant::now();
                let sheets = guides::draw(&keys, [(w, h); 5]);
                let ms = t.elapsed().as_millis();
                let mut px = vec![0u8; w * h];
                for s in &sheets {
                    for (p, v) in px.iter_mut().zip(s) {
                        *p = (*p).max(*v);
                    }
                }
                let mut file = format!("P5\n# {ms} ms\n{w} {h}\n255\n").into_bytes();
                file.extend(px);
                if std::fs::write(out, file).is_ok() { 0 } else { 1 }
            }
            None => 2,
        },
        Some("--tray") => agent::run(false, None),
        _ => agent::run(true, None),
    };
    std::process::exit(code);
}
