use std::fs;
use std::path::{Path, PathBuf};

pub const APP_NAME: &str = "AgSnapTool";
pub const LEGACY_APP_NAME: &str = "SnapDrag";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CLEANUP_INTERVAL_MS: u32 = 3_600_000;
pub const DEFAULT_RETENTION_DAYS: i32 = 7;
pub const MINIMUM_RETENTION_DAYS: i32 = 1;
pub const MAXIMUM_RETENTION_DAYS: i32 = 3650;
pub const LOGGED_TEXT_LIMIT: usize = 40;
pub const MINIMUM_ELEMENT_EDGE: i32 = 8;
const AREA_NUMERATOR: i64 = 17;
const AREA_DENOMINATOR: i64 = 20;
const MAX_SHOT_SUFFIX: i32 = 100;
const SHOT_PREFIX: &str = "Screenshot-";
const SHOT_EXTENSION: &str = ".png";
pub const HC_ACTION: i32 = 0;
pub const WM_KEY_DOWN: u32 = 0x0100;
pub const WM_KEY_UP: u32 = 0x0101;
pub const WM_SYS_KEY_DOWN: u32 = 0x0104;
pub const WM_SYS_KEY_UP: u32 = 0x0105;
pub const WM_LEFT_BUTTON_DOWN: u32 = 0x0201;
pub const WM_RIGHT_BUTTON_DOWN: u32 = 0x0204;
pub const WM_RIGHT_BUTTON_UP: u32 = 0x0205;
pub const WM_APP_BEGIN_DRAG: u32 = 0x8001;
pub const WM_APP_CAPTURE_WINDOW: u32 = 0x8002;
pub const VK_MENU: u32 = 0x12;
const HDROP_HEADER_SIZE: usize = 20;
const DIB_HEADER_SIZE: usize = 40;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Modifiers {
    pub control: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Modifiers {
    pub const DEFAULT: Self = Self {
        control: true,
        shift: true,
        alt: true,
    };

    pub fn has_any(self) -> bool {
        self.control || self.shift || self.alt
    }

    pub fn matches(self, control: bool, shift: bool, alt: bool) -> bool {
        if !self.has_any() {
            return false;
        }
        if self.control && !control {
            return false;
        }
        if self.shift && !shift {
            return false;
        }
        if self.alt && !alt {
            return false;
        }
        true
    }

    pub fn should_suppress_alt(self, control: bool, shift: bool) -> bool {
        if !self.alt || (!self.control && !self.shift) {
            return false;
        }
        if self.control && !control {
            return false;
        }
        if self.shift && !shift {
            return false;
        }
        true
    }

    pub fn format(self) -> String {
        let mut text = String::new();
        append_token(&mut text, self.control, "Ctrl");
        append_token(&mut text, self.shift, "Shift");
        append_token(&mut text, self.alt, "Alt");
        text
    }

    pub fn parse(text: &str) -> Self {
        let mut modifiers = Self {
            control: false,
            shift: false,
            alt: false,
        };
        for part in text.split('+') {
            let token = part.trim();
            if token.eq_ignore_ascii_case("Ctrl")
                || token.eq_ignore_ascii_case("Control")
            {
                modifiers.control = true;
            } else if token.eq_ignore_ascii_case("Shift") {
                modifiers.shift = true;
            } else if token.eq_ignore_ascii_case("Alt")
                || token.eq_ignore_ascii_case("Menu")
            {
                modifiers.alt = true;
            }
        }
        modifiers
    }
}

fn append_token(text: &mut String, include: bool, token: &str) {
    if !include {
        return;
    }
    if !text.is_empty() {
        text.push('+');
    }
    text.push_str(token);
}

pub fn read_modifiers(path: &Path) -> Modifiers {
    let Ok(text) = fs::read_to_string(path) else {
        return Modifiers::DEFAULT;
    };
    let parsed = Modifiers::parse(&text);
    if parsed.has_any() {
        parsed
    } else {
        Modifiers::DEFAULT
    }
}

pub fn write_modifiers(
    path: &Path,
    modifiers: Modifiers,
) -> std::io::Result<()> {
    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory)?;
    }
    fs::write(path, modifiers.format())
}

pub fn normalize_days(days: i32) -> i32 {
    if days < MINIMUM_RETENTION_DAYS {
        return DEFAULT_RETENTION_DAYS;
    }
    if days > MAXIMUM_RETENTION_DAYS {
        return MAXIMUM_RETENTION_DAYS;
    }
    days
}

pub fn normalize_day_text(text: &str) -> i32 {
    match text.trim().parse::<i32>() {
        Ok(days) if text.trim().chars().all(|ch| ch.is_ascii_digit()) => {
            normalize_days(days)
        }
        _ => DEFAULT_RETENTION_DAYS,
    }
}

pub fn read_retention(path: &Path) -> i32 {
    fs::read_to_string(path)
        .map(|text| normalize_day_text(&text))
        .unwrap_or(DEFAULT_RETENTION_DAYS)
}

pub fn write_retention(path: &Path, days: i32) -> std::io::Result<()> {
    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory)?;
    }
    fs::write(path, normalize_days(days).to_string())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CivilTime {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl CivilTime {
    pub fn day_folder(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    pub fn stamp(self) -> String {
        format!(
            "{:04}{:02}{:02}-{:02}{:02}{:02}",
            self.year,
            self.month,
            self.day,
            self.hour,
            self.minute,
            self.second
        )
    }
}

pub fn is_day_folder(name: &str) -> bool {
    parse_day_folder(name).is_some()
}

pub fn parse_day_folder(name: &str) -> Option<(i32, u32, u32)> {
    let bytes = name.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let year: i32 = name[0..4].parse().ok()?;
    let month: u32 = name[5..7].parse().ok()?;
    let day: u32 = name[8..10].parse().ok()?;
    if !(1..=12).contains(&month) || !valid_day(year, month, day) {
        return None;
    }
    Some((year, month, day))
}

fn valid_day(year: i32, month: u32, day: u32) -> bool {
    let max = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => 0,
    };
    (1..=max).contains(&day)
}

fn is_leap(year: i32) -> bool {
    let year = year as u32;
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub fn create_shot_path(
    when: CivilTime,
    shots_root: &Path,
) -> std::io::Result<PathBuf> {
    let directory = shots_root.join(when.day_folder());
    fs::create_dir_all(&directory)?;
    let stamp = when.stamp();
    let first = directory.join(format!("{SHOT_PREFIX}{stamp}{SHOT_EXTENSION}"));
    if !first.exists() {
        return Ok(first);
    }
    for suffix in 2..MAX_SHOT_SUFFIX {
        let candidate = directory
            .join(format!("{SHOT_PREFIX}{stamp}-{suffix}{SHOT_EXTENSION}"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(std::io::Error::other(
        "같은 초의 캡처 파일 이름을 만들지 못했다.",
    ))
}

pub fn delete_older_than(
    utc_now: std::time::SystemTime,
    directory: &Path,
    max_age: std::time::Duration,
    today: (i32, u32, u32),
) -> usize {
    if !directory.is_dir() {
        return 0;
    }
    let mut deleted_parents = Vec::new();
    let deleted =
        delete_shot_files(utc_now, directory, max_age, &mut deleted_parents);
    delete_empty_day_folders(directory, &deleted_parents, today);
    deleted
}

fn delete_shot_files(
    utc_now: std::time::SystemTime,
    directory: &Path,
    max_age: std::time::Duration,
    deleted_parents: &mut Vec<PathBuf>,
) -> usize {
    let mut deleted = 0usize;
    let mut pending = vec![directory.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries = match fs::read_dir(&current) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if !is_shot_file(&path) || !is_stale(&path, utc_now, max_age) {
                continue;
            }
            if fs::remove_file(&path).is_ok() {
                deleted += 1;
                if let Some(parent) = path.parent() {
                    deleted_parents.push(parent.to_path_buf());
                }
            }
        }
    }
    deleted
}

fn is_shot_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name.starts_with(SHOT_PREFIX)
        && name.to_ascii_lowercase().ends_with(SHOT_EXTENSION)
}

fn is_stale(
    path: &Path,
    utc_now: std::time::SystemTime,
    max_age: std::time::Duration,
) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    let Ok(written) = metadata.modified() else {
        return false;
    };
    utc_now
        .duration_since(written)
        .is_ok_and(|age| age >= max_age)
}

fn delete_empty_day_folders(
    directory: &Path,
    deleted_parents: &[PathBuf],
    today: (i32, u32, u32),
) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir()
            || !can_remove_day_folder(&path, deleted_parents, today)
        {
            continue;
        }
        let Ok(mut children) = fs::read_dir(&path) else {
            continue;
        };
        if children.next().is_none() {
            let _ = fs::remove_dir(&path);
        }
    }
}

fn can_remove_day_folder(
    directory: &Path,
    deleted_parents: &[PathBuf],
    today: (i32, u32, u32),
) -> bool {
    let Some(name) = directory.file_name().and_then(|name| name.to_str())
    else {
        return false;
    };
    let Some(day) = parse_day_folder(name) else {
        return false;
    };
    deleted_parents.iter().any(|parent| parent == directory) || day < today
}

pub fn choose_bounds(
    window: crate::rect::Rect,
    element: Option<crate::rect::Rect>,
) -> crate::rect::Rect {
    match element {
        Some(item) if is_tighter(window, item) => item,
        _ => window,
    }
}

fn is_tighter(window: crate::rect::Rect, item: crate::rect::Rect) -> bool {
    if item.width() <= MINIMUM_ELEMENT_EDGE
        || item.height() <= MINIMUM_ELEMENT_EDGE
    {
        return false;
    }
    if !fits_inside(window, item) {
        return false;
    }
    let window_area = i64::from(window.width()) * i64::from(window.height());
    if window_area <= 0 {
        return false;
    }
    let item_area = i64::from(item.width()) * i64::from(item.height());
    item_area * AREA_DENOMINATOR < window_area * AREA_NUMERATOR
}

fn fits_inside(outer: crate::rect::Rect, inner: crate::rect::Rect) -> bool {
    inner.left >= outer.left
        && inner.top >= outer.top
        && inner.right <= outer.right
        && inner.bottom <= outer.bottom
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClickChoice {
    pub x: i32,
    pub y: i32,
    pub whole_window: bool,
    pub is_double: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClickObservation {
    pub ready: Option<ClickChoice>,
    pub is_waiting: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ClickSeries {
    pending: bool,
    origin_x: i32,
    origin_y: i32,
    whole_window: bool,
}

impl ClickSeries {
    pub fn observe_down(
        &mut self,
        x: i32,
        y: i32,
        whole_window: bool,
        rectangle_width: i32,
        rectangle_height: i32,
    ) -> ClickObservation {
        if !self.pending {
            self.begin(x, y, whole_window);
            return ClickObservation {
                ready: None,
                is_waiting: true,
            };
        }
        if whole_window == self.whole_window
            && is_inside(
                self.origin_x,
                self.origin_y,
                x,
                y,
                rectangle_width,
                rectangle_height,
            )
        {
            let choice = ClickChoice {
                x: self.origin_x,
                y: self.origin_y,
                whole_window: self.whole_window,
                is_double: true,
            };
            self.pending = false;
            return ClickObservation {
                ready: Some(choice),
                is_waiting: false,
            };
        }
        let single = ClickChoice {
            x: self.origin_x,
            y: self.origin_y,
            whole_window: self.whole_window,
            is_double: false,
        };
        self.begin(x, y, whole_window);
        ClickObservation {
            ready: Some(single),
            is_waiting: true,
        }
    }

    pub fn expire(&mut self) -> ClickObservation {
        if !self.pending {
            return ClickObservation {
                ready: None,
                is_waiting: false,
            };
        }
        let single = ClickChoice {
            x: self.origin_x,
            y: self.origin_y,
            whole_window: self.whole_window,
            is_double: false,
        };
        self.pending = false;
        ClickObservation {
            ready: Some(single),
            is_waiting: false,
        }
    }

    fn begin(&mut self, x: i32, y: i32, whole_window: bool) {
        self.pending = true;
        self.origin_x = x;
        self.origin_y = y;
        self.whole_window = whole_window;
    }
}

pub fn is_inside(
    origin_x: i32,
    origin_y: i32,
    x: i32,
    y: i32,
    rectangle_width: i32,
    rectangle_height: i32,
) -> bool {
    (x - origin_x).abs() < rectangle_width / 2
        && (y - origin_y).abs() < rectangle_height / 2
}

pub fn should_swallow_alt(
    message: u32,
    virtual_key: u32,
    shortcut: Modifiers,
    control_down: bool,
    shift_down: bool,
) -> bool {
    if virtual_key != VK_MENU || !is_key_transition(message) {
        return false;
    }
    shortcut.should_suppress_alt(control_down, shift_down)
}

fn is_key_transition(message: u32) -> bool {
    message == WM_KEY_DOWN
        || message == WM_KEY_UP
        || message == WM_SYS_KEY_DOWN
        || message == WM_SYS_KEY_UP
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseDecision {
    Pass,
    Swallow,
    Post { message: u32, x: i32, y: i32 },
}

pub fn decide_mouse(
    code: i32,
    message: u32,
    modifiers_down: bool,
    busy: bool,
    point: Option<(i32, i32)>,
) -> MouseDecision {
    if code < HC_ACTION || !is_gesture_message(message) {
        return MouseDecision::Pass;
    }
    if message == WM_RIGHT_BUTTON_UP {
        return if modifiers_down {
            MouseDecision::Swallow
        } else {
            MouseDecision::Pass
        };
    }
    if busy || !modifiers_down {
        return MouseDecision::Pass;
    }
    let Some((x, y)) = point else {
        return MouseDecision::Pass;
    };
    let posted = if message == WM_RIGHT_BUTTON_DOWN {
        WM_APP_CAPTURE_WINDOW
    } else {
        WM_APP_BEGIN_DRAG
    };
    MouseDecision::Post {
        message: posted,
        x,
        y,
    }
}

pub fn is_gesture_message(message: u32) -> bool {
    message == WM_LEFT_BUTTON_DOWN
        || message == WM_RIGHT_BUTTON_DOWN
        || message == WM_RIGHT_BUTTON_UP
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowHit {
    pub handle: isize,
    pub bounds: crate::rect::Rect,
    pub class_name: String,
    pub text: String,
}

pub fn format_gesture(x: i32, y: i32, hit: Option<&WindowHit>) -> String {
    let Some(found) = hit else {
        return format!("gesture miss point={x},{y}");
    };
    let text = sanitize(&found.text);
    format!(
        "gesture hwnd=0x{:X} class={} text={} rect={},{},{},{} point={x},{y}",
        found.handle as usize,
        found.class_name,
        text,
        found.bounds.left,
        found.bounds.top,
        found.bounds.width(),
        found.bounds.height()
    )
}

fn sanitize(text: &str) -> String {
    let one_line: String = text
        .replace("\r\n", " ")
        .replace(['\r', '\n'], " ")
        .trim()
        .chars()
        .take(LOGGED_TEXT_LIMIT)
        .collect();
    one_line
}

#[derive(Clone, Debug, Default)]
pub struct BalloonShot {
    path: Option<PathBuf>,
}

impl BalloonShot {
    pub fn remember(&mut self, path: Option<PathBuf>) {
        self.path = path;
    }

    pub fn path_to_open(&self) -> Option<&Path> {
        let path = self.path.as_deref()?;
        if path.as_os_str().is_empty() || !path.is_file() {
            return None;
        }
        Some(path)
    }
}

pub fn hdrop_blob(path: &str) -> Vec<u8> {
    let mut text: Vec<u16> = path.encode_utf16().collect();
    text.push(0);
    text.push(0);
    let mut result = vec![0u8; HDROP_HEADER_SIZE + text.len() * 2];
    result[..4].copy_from_slice(&(HDROP_HEADER_SIZE as i32).to_le_bytes());
    result[16..20].copy_from_slice(&1i32.to_le_bytes());
    for (index, unit) in text.iter().enumerate() {
        let offset = HDROP_HEADER_SIZE + index * 2;
        result[offset..offset + 2].copy_from_slice(&unit.to_le_bytes());
    }
    result
}

pub fn dib_from_top_down(
    pixels: &[u8],
    width: i32,
    height: i32,
) -> Result<Vec<u8>, ()> {
    let stride = width as usize * 4;
    if width <= 0 || height <= 0 || pixels.len() != stride * height as usize {
        return Err(());
    }
    let mut result = vec![0u8; DIB_HEADER_SIZE + pixels.len()];
    result[..4].copy_from_slice(&(DIB_HEADER_SIZE as i32).to_le_bytes());
    result[4..8].copy_from_slice(&width.to_le_bytes());
    result[8..12].copy_from_slice(&height.to_le_bytes());
    result[12..14].copy_from_slice(&1i16.to_le_bytes());
    result[14..16].copy_from_slice(&32i16.to_le_bytes());
    for row in 0..height as usize {
        let source = (height as usize - 1 - row) * stride;
        let destination = DIB_HEADER_SIZE + row * stride;
        result[destination..destination + stride]
            .copy_from_slice(&pixels[source..source + stride]);
    }
    Ok(result)
}

pub const OWNED_FORMAT_NAMES: [&str; 3] = ["CF_HDROP", "PNG", "CF_DIB"];

pub fn rejects_file_group_descriptor(name: &str) -> bool {
    !OWNED_FORMAT_NAMES.iter().any(|owned| *owned == name)
}

pub fn first_heading(history: &str) -> String {
    for line in history.split('\n') {
        let trimmed = line.trim().trim_end_matches('\r');
        if !trimmed.starts_with("## ") {
            continue;
        }
        let Some(date_start) = trimmed.find(" (") else {
            return trimmed.to_string();
        };
        return trimmed[..date_start].to_string();
    }
    String::new()
}

pub fn read_history_text(path: &Path, missing: &str) -> String {
    let Ok(text) = fs::read_to_string(path) else {
        return missing.to_string();
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        missing.to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn settings_dir() -> PathBuf {
    local_app_dir().join(APP_NAME)
}

pub fn legacy_settings_dir() -> PathBuf {
    local_app_dir().join(LEGACY_APP_NAME)
}

pub fn resolve_settings_file(name: &str) -> PathBuf {
    let current = settings_dir().join(name);
    if current.is_file() {
        return current;
    }
    let legacy = legacy_settings_dir().join(name);
    if legacy.is_file() {
        return legacy;
    }
    current
}

fn local_app_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct GalleryShot {
    pub path: String,
    pub day: String,
    pub modified_ms: u128,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Gallery {
    pub days: Vec<String>,
    pub shots: Vec<GalleryShot>,
}

pub fn list_gallery(root: &Path) -> Gallery {
    let mut shots = Vec::new();
    collect_shots(root, &mut shots);
    shots.sort_by(|left, right| {
        right
            .modified_ms
            .cmp(&left.modified_ms)
            .then_with(|| right.path.cmp(&left.path))
    });
    let mut days: Vec<String> =
        shots.iter().map(|shot| shot.day.clone()).collect();
    days.sort();
    days.dedup();
    days.reverse();
    Gallery { days, shots }
}

fn collect_shots(directory: &Path, shots: &mut Vec<GalleryShot>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_shots(&path, shots);
            continue;
        }
        if !is_shot_file(&path) {
            continue;
        }
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        let modified_ms = modified
            .duration_since(std::time::UNIX_EPOCH)
            .map(|age| age.as_millis())
            .unwrap_or(0);
        let day = path
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
            .filter(|name| is_day_folder(name))
            .unwrap_or("unknown")
            .to_string();
        shots.push(GalleryShot {
            path: path.display().to_string(),
            day,
            modified_ms,
        });
    }
}

pub fn shots_directory() -> PathBuf {
    std::env::temp_dir().join("shots")
}
