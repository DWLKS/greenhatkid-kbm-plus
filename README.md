# Green Hat Kid Keyboard & Mouse Plus

Keyboard and mouse controls for the Green Hat Kid game on the Lichii emulator, with a 1:1 mouse camera that also works while aiming and riding horses. It's one small Windows program with nothing to install.

## Features

- **Your keys:** every button can be a keyboard key or a mouse button, plus a slow-walk key with an adjustable speed.
- **Mouse camera:** the mouse turns the camera directly, like a PC game. There's no stick curve and no top speed, and it keeps working while aiming and riding horses.
- **In-game key icons:** the game shows your keys instead of gamepad buttons in its texts, on the HUD, on the ocarina song sheet and on the Basic Controls screen, which is laid out again for your keys. They are made from your own game file on your PC; nothing from the game ships with the app.
- **Mouse panning:** the app turns on Lichii's mouse panning when the game starts, so the pointer stays hidden while you play.
- **Remove the mod:** one button puts Lichii back exactly as it was before.
- **Light on resources:** a 5 MB exe that runs from the tray. The background part uses under 2 MB of memory and only watches the mouse while the game is in front.

## Use

1. Download the zip from the Releases page and unzip it anywhere.
2. Close Lichii.
3. Start `GreenHatKid-KBM-Plus.exe`. It finds Lichii's data folder by itself; if it doesn't, set the folder on the App page.
4. Go through the steps with **Next** (Controls, Camera, In-game key icons), then press **Apply**. The first time, Apply needs your game file (`.nup`), which is read on your PC only.
5. Start Lichii and play. Keep the app running in the tray: it captures the mouse while the game is in front and gives it back on Alt+Tab.

Windows may warn about an unknown publisher the first time, because the program isn't code-signed. Choose **More info**, then **Run anyway**.

## What it changes in Lichii

- A controller profile named `GreenHatKid-KBM-Plus`, Player 1's keys and the real-mouse setting. Lichii's settings file is backed up once, before the first change, as `config\qt-config.ini.pre-greenhatkid-kbm-plus`.
- The mouse camera patch: `load\0400486024DEC000\MouseCamera-Native`.
- The in-game key icons: `load\0400486024DEC000\ButtonPrompts-KeyboardMouse`.

The app's own settings and its cache of the game files it needs are in `%LOCALAPPDATA%\GreenHatKid-KBM-Plus`.

## How the mouse camera works

The camera patch (`patch/`) runs inside the game. It reads the mouse through the game's own input functions and turns each mouse step into a fixed angle. Lichii only reports where the pointer is on the game picture, so while the game is in front the app does two things:

- It moves the pointer back toward the middle by an exact distance, which the patch knows to ignore.
- It switches off Windows pointer acceleration and sets the pointer speed so that one mouse step is one pixel of the picture.

Both stop as soon as another window comes to the front. Windows' own mouse settings are never saved or changed for good.

## Building

You need Windows 10 or 11 (x64) and Rust 1.88 or newer (MSVC toolchain).

```
cd native
cargo build --release
```

The exe is `native/target/release/GreenHatKid-KBM-Plus.exe`. It links the C runtime statically, so it needs no Visual C++ redistributable.

To make a release zip, with Python 3 and `cargo` on PATH:

```
python tools/make_notices.py
python native/release/make_zip.py
```

The first script writes `native/LICENSES.txt`; the second writes `out/GreenHatKid-KBM-Plus-<version>.zip`.

### Generated files in the repository

- **`patch/prebuilt/*.ips`, the camera patch.** It is built from `patch/cave.c` and `patch/hooks.S` with `python patch/build_native_camera_ips.py`, which needs LLVM (`clang`, `ld.lld`, `llvm-objcopy`, `llvm-nm`).
- **`native/res/basic-controls.bin.zst`, the keyboard and mouse pictures of the Basic Controls screen.** To regenerate it:
  ```
  pip install -r tools/requirements.txt
  python tools/make_basic_controls.py "%LOCALAPPDATA%\GreenHatKid-KBM-Plus\game-cache"
  ```
  This needs the game-cache folder the app fills from your game file.

## Layout

| Folder | What it holds |
| --- | --- |
| `native/src` | The app: settings window (`ui.rs`), background agent and tray (`agent.rs`), mouse capture (`capture.rs`), mouse panning (`panning.rs`), Lichii settings (`lichii.rs`), in-game key icons (`icons.rs`, `font.rs`, `textures.rs`), Basic Controls screen (`guides.rs`, `layout.rs`) |
| `native/res` | Icon, manifest, version info and the Basic Controls pictures |
| `native/release` | The release README and zip script |
| `patch` | The camera patch's source and the built patch |
| `tools` | Scripts for the license notices and the Basic Controls pictures |

## Credits and license

Parts of this software are based on lichii-keyboard-mouse by exgde/joaobyss (MIT): the key mapping, the key icon tools and art, the mouse capture and the native camera patch. This project remakes it in Rust and builds on it with:

- **One program:** a single 5 MB exe replaces the settings tool and the separate mouse capture helper, with nothing to install.
- **A new settings window:** dark and step by step (Controls, Camera, In-game key icons, then Apply). Check marks show what Lichii already has. You set a key by pressing it, keys used twice get a warning, and each page has its own Defaults.
- **Tray:** closing the window keeps the app running in the tray, and it can start with Windows.
- **Reworked mouse capture:** it only watches the mouse while the game is in front and handles every mouse report at once. It follows Lichii's aspect ratio, notices when the game stops, and puts Windows' pointer settings back even after a crash.
- **The mouse camera while aiming and riding horses.**
- **In-game key icons made inside the app** from your own game file.
- **A Basic Controls screen laid out for your keys:** a bigger keyboard and mouse, labels placed so their lines cross as little as possible, each label on its own background, and Reset Camera as a label of its own.
- **Automatic mouse panning.**
- **Remove the mod**, which puts Lichii back as it was.

MIT License, see [LICENSE](LICENSE). The original mod's copyright notice is kept with this software: [third_party/original/LICENSE](third_party/original/LICENSE) in the source, section 2 of `LICENSES.txt` in the release.
