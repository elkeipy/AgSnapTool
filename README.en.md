<p align="center">
  <img src="icons/icon.png" width="88" alt="AgSnapTool">
</p>

<p align="center">
  <a href="README.md">한국어</a>
  &nbsp;·&nbsp;
  <b>English</b>
</p>

<h1 align="center">AgSnapTool</h1>

<p align="center">
  <strong>Hold a modifier and click. That control becomes a PNG.</strong><br>
  It stays in the Windows tray. Shots open in a Polaroid gallery.
</p>

<p align="center">
  <a href="#build-and-run"><b>Build</b></a>
  &nbsp;·&nbsp;
  <a href="#snap-gallery">Gallery</a>
  &nbsp;·&nbsp;
  <a href="#capture">Capture</a>
  &nbsp;·&nbsp;
  <a href="#files">Files</a>
</p>

<p align="center"><sub>2.7.17 · Windows x64 · no administrator</sub></p>

| Capture | Browse | Edit |
| --- | --- | --- |
| Left click is a control, drag is a region, right click is a window. It lands on the clipboard as a file. | Polaroid, tape and pins, hover zoom, tags and memo, dates. | Magnifier, shape editing, OCR markdown. Themes apply only to the gallery shell. |

## Requirements

- Windows x64
- WebView2
- [Rust](https://rustup.rs/) 1.85 or newer. This repo was checked on 1.99

DPI is Per-Monitor V2. Do not run as administrator.

## Build and run

```powershell
cargo run --bin agsnaptool
cargo build --release --bin agsnaptool
```

The release binary is `target\release\agsnaptool.exe`. There is no console window. The build copies `CHANGELOG.md` next to the exe.

Starting the app does not open the gallery. Closing the window leaves the process in the tray. Quit is on the tray menu. Only one instance runs. A second launch opens the existing window.

## Snap gallery

Open it from a left click on the tray, or from "Snap Gallery".

### Find

The first item in the date list is All. The rest are days that have shots, newest first. All shows the newest shots first. A date leaves only that day.

Search looks at tags, memo, time, and date. Press `#` in the gallery to put `#` in the search box and move the cursor there.

Only what fits on screen is attached. Scrolling down loads the next batch. Ctrl+wheel changes the card size.

### Cards

Polaroid mode is the default. The time at the bottom includes milliseconds. Tags and memo sit above the date when present. Tape is yellow, red, or blue per shot: two pieces on the corners, or one piece at the top center. Length is the setting ±10 per shot. Height stays as set. The ends look torn, and the tape sticks farther toward the wall than the photo. With hover zoom on, the card grows after the configured wait, at the configured scale. Scale starts at 1.2. Second zoom can be turned on separately. Second zoom does not tilt.

Turning Polaroid mode off removes tilt and tape.

### Selection and actions

A click selects. Ctrl selects several. Shift selects a range.

| Input | Result |
| --- | --- |
| Double-click | Magnifier |
| Shift+double-click | Magnifier window |
| Ctrl+C, "Copy(C)" | Puts the selected PNGs on the clipboard. Ctrl+V in Explorer creates the files. One shot also gets `PNG` and `CF_DIB` |
| "Tags/Memo(T)..." | Tags and memo. Several shots get the same text. `#` in the tag field picks an existing tag. `#ag` leaves only tags that start with ag |
| "Copy MD(M)" | Puts the original image and the OCR text on the clipboard as Markdown |
| "Save MD(S)" | Writes a `.md` in the same folder. If Korean OCR is missing, it says how to install it |
| "Magnifier(V)..." | Shows that image large. The wheel zooms. After zooming, drag with the left button |
| "Edit(E)..." | Draws shapes and text in another window. Color, stroke, arrow head, and type size keep the last value as the next default |
| Delete, "Delete(D)..." | Deletes the selected files |
| "Open with default program..." | Opens that file with its associated program |

Menu shortcuts are the letter in parentheses. Esc closes the menu, the magnifier, and dialogs.

### Settings

Change them in the right pane. Show and Hide fold it. Children of a turned-off item stay hidden.

| Item | Detail |
| --- | --- |
| Shortcut | Ctrl, Shift, and Alt used for capture. The default is `Ctrl+Shift+Alt` |
| Delete old shots | When on, deletes `Screenshot-*.png` past the retention days |
| Language | System, Korean, English |
| Theme | System, Dark, Light, Flat, Material, Glass, Skeuomorphism, Neumorphism, Claymorphism, Neo-brutalism, Liquid Glass, 3D Icons. System, Dark, and Light are Apple-style. They apply only to the top bar and settings. Polaroid photos stay as they are |
| Polaroid mode | Date size, tilt, decoration (tape or pin), tape length and height, pin size and height. The pin sits at the top center in red, blue, or green |
| Hover zoom / Second zoom | Wait (ms) and scale. Turning the first stage off also hides the second |
| History | `CHANGELOG.md` |

## Capture

Clicks and region drags are not delivered to the target app. The clipboard gets a real PNG file (`CF_HDROP`), `PNG` bytes, and `CF_DIB`.

| Input | Result |
| --- | --- |
| Left click | Swallows down only. Captures the deepest visible control under the cursor |
| Left drag | Captures the chosen rectangle. Only the outside is dimmed. The rectangle stays clear. Up is swallowed in the hook. On move, the dim window covers the target app |
| Right click | Swallows down and up. Captures the window frame without its shadow |
| Double-click | Captures one shot and opens it with no notification |
| Single click | Shows only a notification. Pressing it opens the magnifier window |

The UI Automation rectangle for a left click is used only when it sits inside the window, each side is longer than 8, and the area is under 85% (`17/20`) of the window.

## Files

```text
%LOCALAPPDATA%\AgSnapTool\shots\yyyy-MM-dd\Screenshot-yyyyMMdd-HHmmss.png
%LOCALAPPDATA%\AgSnapTool\settings.json
%LOCALAPPDATA%\AgSnapTool\notes.json
%LOCALAPPDATA%\AgSnapTool\gesture.log
```

Settings live in one `settings.json`. Tags and memo live in `notes.json`. Saving a shot stores the window title as the default tag.

The same second gets a suffix from `-2`. Shots are always saved. With "Delete old shots" on, age is the number of days between the shot date and today. Only `Screenshot-*.png` at least that many days old are deleted at startup and every hour. Files that used to live in `%TEMP%\shots` are moved into the app folder, then judged by the same rule. A file is not deleted right after it is placed on the clipboard.

## Tests

```powershell
cargo test
```

## Version

The `version` in `Cargo.toml` matches the number on the first heading of `CHANGELOG.md`. The current version is 2.7.17.
