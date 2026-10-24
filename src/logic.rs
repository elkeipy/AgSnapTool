use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::strings::{normalize_culture, normalize_theme};

pub const APP_NAME: &str = "AgSnapTool";
pub const INSTANCE_MUTEX_NAME: &str = "Local\\AgSnapTool.single";
pub const MESSAGE_WINDOW_CLASS: &str = "AgSnapMessage";
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
pub const EDITED_MARK: &str = "Modify";
const EDITOR_WINDOW_PREFIX: &str = "editor-";
const LENS_WINDOW_PREFIX: &str = "lens-";
pub const SAVE_PROMPT_YES: i32 = 6;
pub const SAVE_PROMPT_NO: i32 = 7;
const PNG_SIGNATURE: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
const MAX_EDITED_PNG: usize = 40 * 1024 * 1024;
const SECONDS_PER_DAY: u64 = 24 * 60 * 60;
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
pub const WM_APP_SHOW_GALLERY: u32 = 0x8004;
pub const VK_MENU: u32 = 0x12;
const HDROP_HEADER_SIZE: usize = 20;
const DIB_HEADER_SIZE: usize = 40;

/// True for the live gallery or tool windows of this app.
pub fn is_gallery_window_title(title: &str) -> bool {
    title == APP_NAME || title.starts_with(&format!("{APP_NAME} —"))
}

/// True for the hidden capture message window.
pub fn is_message_window_class(class_name: &str) -> bool {
    class_name == MESSAGE_WINDOW_CLASS
}

/// Label for a newly created image editor window.
pub fn editor_window_label(id: u64) -> String {
    format!("{EDITOR_WINDOW_PREFIX}{id}")
}

/// True for the reserved editor window names.
pub fn is_editor_label(label: &str) -> bool {
    label == "editor" || label.starts_with(EDITOR_WINDOW_PREFIX)
}

/// Label for a newly created magnifier window.
pub fn lens_window_label(id: u64) -> String {
    format!("{LENS_WINDOW_PREFIX}{id}")
}

/// True for the reserved magnifier window names.
pub fn is_lens_label(label: &str) -> bool {
    label == "lens" || label.starts_with(LENS_WINDOW_PREFIX)
}

/// Maps a Yes/No/Cancel message-box result.
pub fn save_prompt_choice(result: i32) -> &'static str {
    if result == SAVE_PROMPT_YES {
        "save"
    } else if result == SAVE_PROMPT_NO {
        "discard"
    } else {
        "cancel"
    }
}

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

pub const PANEL_OPEN: &str = "open";
pub const PANEL_CLOSED: &str = "closed";

pub fn read_panel_open(path: &Path) -> bool {
    match fs::read_to_string(path) {
        Ok(text) if text.trim().eq_ignore_ascii_case(PANEL_CLOSED) => false,
        _ => true,
    }
}

pub fn write_panel_open(path: &Path, open: bool) -> std::io::Result<()> {
    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory)?;
    }
    let stored = if open { PANEL_OPEN } else { PANEL_CLOSED };
    fs::write(path, stored)
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

pub const TILT_MIN: i32 = 0;
pub const TILT_MAX: i32 = 15;
pub const DEFAULT_TILT: i32 = 3;
pub const TAPE_WIDTH_MIN: i32 = 24;
pub const TAPE_WIDTH_MAX: i32 = 120;
pub const DEFAULT_TAPE_WIDTH: i32 = 58;
pub const TAPE_HEIGHT_MIN: i32 = 8;
pub const TAPE_HEIGHT_MAX: i32 = 48;
pub const DEFAULT_TAPE_HEIGHT: i32 = 22;
pub const DEFAULT_EDITOR_WIDTH: i32 = 1100;
pub const DEFAULT_EDITOR_HEIGHT: i32 = 760;
pub const MIN_EDITOR_WIDTH: i32 = 640;
pub const MIN_EDITOR_HEIGHT: i32 = 420;
pub const MAX_EDITOR_WIDTH: i32 = 10_000;
pub const MAX_EDITOR_HEIGHT: i32 = 10_000;
pub const DEFAULT_MAIN_WIDTH: i32 = 1120;
pub const DEFAULT_MAIN_HEIGHT: i32 = 780;
pub const MIN_MAIN_WIDTH: i32 = 760;
pub const MIN_MAIN_HEIGHT: i32 = 480;
pub const MAX_MAIN_WIDTH: i32 = 10_000;
pub const MAX_MAIN_HEIGHT: i32 = 10_000;
pub const MIN_LENS_WIDTH: i32 = 480;
pub const MIN_LENS_HEIGHT: i32 = 320;
pub const MAX_LENS_WIDTH: i32 = 10_000;
pub const MAX_LENS_HEIGHT: i32 = 10_000;
pub const DEFAULT_GALLERY_ZOOM: i32 = 220;
pub const MIN_GALLERY_ZOOM: i32 = 140;
pub const MAX_GALLERY_ZOOM: i32 = 400;
pub const DEFAULT_CAPTION_SIZE: i32 = 12;
pub const MIN_CAPTION_SIZE: i32 = 8;
pub const MAX_CAPTION_SIZE: i32 = 36;
pub const DEFAULT_LENS_WIDTH: i32 = 960;
pub const DEFAULT_LENS_HEIGHT: i32 = 720;
pub const PEEK_SCALE_TAGS: &[&str] = &[
    "1.2", "1.3", "1.5", "2.0", "2.5", "3.0", "3.5", "4.0", "max",
];
pub const PEEK_SCALE_MAX: &str = "max";
pub const DEFAULT_PEEK_ONE_SCALE: &str = "1.5";
pub const DEFAULT_PEEK_TWO_SCALE: &str = "max";
pub const DEFAULT_PEEK_DELAY: i32 = 500;
pub const MIN_PEEK_DELAY: i32 = 50;
pub const MAX_PEEK_DELAY: i32 = 5000;
pub const TAPE_SHOWN: &str = "show";
pub const TAPE_HIDDEN: &str = "hide";
pub const POLAROID_ON: &str = "on";
pub const POLAROID_OFF: &str = "off";

pub fn normalize_tilt(degrees: i32) -> i32 {
    degrees.clamp(TILT_MIN, TILT_MAX)
}

pub fn normalize_tape_width(pixels: i32) -> i32 {
    pixels.clamp(TAPE_WIDTH_MIN, TAPE_WIDTH_MAX)
}

pub fn normalize_tape_height(pixels: i32) -> i32 {
    pixels.clamp(TAPE_HEIGHT_MIN, TAPE_HEIGHT_MAX)
}

/// Keeps the editor window width inside the allowed pixel range.
pub fn normalize_editor_width(pixels: i32) -> i32 {
    pixels.clamp(MIN_EDITOR_WIDTH, MAX_EDITOR_WIDTH)
}

/// Keeps the editor window height inside the allowed pixel range.
pub fn normalize_editor_height(pixels: i32) -> i32 {
    pixels.clamp(MIN_EDITOR_HEIGHT, MAX_EDITOR_HEIGHT)
}

/// Keeps the gallery window width inside the allowed pixel range.
pub fn normalize_main_width(pixels: i32) -> i32 {
    pixels.clamp(MIN_MAIN_WIDTH, MAX_MAIN_WIDTH)
}

/// Keeps the gallery window height inside the allowed pixel range.
pub fn normalize_main_height(pixels: i32) -> i32 {
    pixels.clamp(MIN_MAIN_HEIGHT, MAX_MAIN_HEIGHT)
}

/// Keeps the magnifier window width inside the allowed pixel range.
pub fn normalize_lens_width(pixels: i32) -> i32 {
    pixels.clamp(MIN_LENS_WIDTH, MAX_LENS_WIDTH)
}

/// Keeps the magnifier window height inside the allowed pixel range.
pub fn normalize_lens_height(pixels: i32) -> i32 {
    pixels.clamp(MIN_LENS_HEIGHT, MAX_LENS_HEIGHT)
}

/// Saved placement group for a live window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowRole {
    Main,
    Editor,
    Lens,
}

/// Maps a window label to the saved placement group.
pub fn window_role_from_label(label: &str) -> Option<WindowRole> {
    if label == "main" {
        Some(WindowRole::Main)
    } else if is_editor_label(label) {
        Some(WindowRole::Editor)
    } else if is_lens_label(label) {
        Some(WindowRole::Lens)
    } else {
        None
    }
}

/// True when a saved window has both coordinates.
pub fn has_window_position(x: Option<i32>, y: Option<i32>) -> bool {
    x.is_some() && y.is_some()
}

/// True when the next gallery card starts a new day group.
pub fn is_gallery_day_break(previous: Option<&str>, day: &str) -> bool {
    previous != Some(day)
}

/// Keeps the gallery card width inside the allowed pixel range.
pub fn normalize_gallery_zoom(pixels: i32) -> i32 {
    pixels.clamp(MIN_GALLERY_ZOOM, MAX_GALLERY_ZOOM)
}

/// Keeps the polaroid date size inside the allowed pixel range.
pub fn normalize_caption_size(pixels: i32) -> i32 {
    pixels.clamp(MIN_CAPTION_SIZE, MAX_CAPTION_SIZE)
}

/// Keeps the hover delay inside the allowed millisecond range.
pub fn normalize_peek_delay(ms: i32) -> i32 {
    ms.clamp(MIN_PEEK_DELAY, MAX_PEEK_DELAY)
}

/// Returns a known hover scale tag.
pub fn normalize_peek_scale(tag: &str) -> &'static str {
    let trimmed = tag.trim();
    PEEK_SCALE_TAGS
        .iter()
        .copied()
        .find(|item| *item == trimmed)
        .unwrap_or(DEFAULT_PEEK_ONE_SCALE)
}

/// Scale tags that are larger than the first hover step.
pub fn peek_scale_options_after(tag: &str) -> Vec<&'static str> {
    let current = normalize_peek_scale(tag);
    let index = PEEK_SCALE_TAGS
        .iter()
        .position(|item| *item == current)
        .unwrap_or(0);
    PEEK_SCALE_TAGS[index + 1..].to_vec()
}

/// Keeps the second hover scale larger than the first.
pub fn normalize_peek_two_scale(one: &str, two: &str) -> &'static str {
    let one = normalize_peek_scale(one);
    let two = normalize_peek_scale(two);
    let options = peek_scale_options_after(one);
    if options.iter().any(|item| *item == two) {
        return two;
    }
    options.first().copied().unwrap_or(PEEK_SCALE_MAX)
}

pub fn read_tilt(path: &Path) -> i32 {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| text.trim().parse::<i32>().ok())
        .map(normalize_tilt)
        .unwrap_or(DEFAULT_TILT)
}

pub fn write_tilt(path: &Path, degrees: i32) -> std::io::Result<()> {
    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory)?;
    }
    fs::write(path, normalize_tilt(degrees).to_string())
}

pub fn read_tape_visible(path: &Path) -> bool {
    match fs::read_to_string(path) {
        Ok(text) if text.trim().eq_ignore_ascii_case(TAPE_HIDDEN) => false,
        _ => true,
    }
}

pub fn write_tape_visible(path: &Path, visible: bool) -> std::io::Result<()> {
    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory)?;
    }
    let stored = if visible { TAPE_SHOWN } else { TAPE_HIDDEN };
    fs::write(path, stored)
}

pub fn read_polaroid(path: &Path) -> bool {
    match fs::read_to_string(path) {
        Ok(text) if text.trim().eq_ignore_ascii_case(POLAROID_OFF) => false,
        _ => true,
    }
}

pub fn write_polaroid(path: &Path, enabled: bool) -> std::io::Result<()> {
    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory)?;
    }
    let stored = if enabled { POLAROID_ON } else { POLAROID_OFF };
    fs::write(path, stored)
}

pub const SETTINGS_FILE: &str = "settings.json";
pub const NOTES_FILE: &str = "notes.json";
pub const MAX_MEMO_CHARS: usize = 80;
pub const MAX_NOTE_TAGS: usize = 8;
const LEGACY_SETTING_FILES: &[&str] = &[
    "modifiers.txt",
    "retention.txt",
    "culture.txt",
    "theme.txt",
    "panel.txt",
    "tilt.txt",
    "tape.txt",
    "polaroid.txt",
];

fn always_true() -> bool {
    true
}

fn default_retention_days() -> i32 {
    DEFAULT_RETENTION_DAYS
}

fn default_tilt_value() -> i32 {
    DEFAULT_TILT
}

fn default_tape_width() -> i32 {
    DEFAULT_TAPE_WIDTH
}

fn default_tape_height() -> i32 {
    DEFAULT_TAPE_HEIGHT
}

fn default_editor_width() -> i32 {
    DEFAULT_EDITOR_WIDTH
}

fn default_editor_height() -> i32 {
    DEFAULT_EDITOR_HEIGHT
}

fn default_main_width() -> i32 {
    DEFAULT_MAIN_WIDTH
}

fn default_main_height() -> i32 {
    DEFAULT_MAIN_HEIGHT
}

fn default_lens_width() -> i32 {
    DEFAULT_LENS_WIDTH
}

fn default_lens_height() -> i32 {
    DEFAULT_LENS_HEIGHT
}

fn default_gallery_zoom() -> i32 {
    DEFAULT_GALLERY_ZOOM
}

fn default_caption_size() -> i32 {
    DEFAULT_CAPTION_SIZE
}

fn default_peek_delay() -> i32 {
    DEFAULT_PEEK_DELAY
}

fn default_peek_one_scale() -> String {
    DEFAULT_PEEK_ONE_SCALE.to_string()
}

fn default_peek_two_scale() -> String {
    DEFAULT_PEEK_TWO_SCALE.to_string()
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    #[serde(default = "always_true")]
    pub control: bool,
    #[serde(default = "always_true")]
    pub shift: bool,
    #[serde(default = "always_true")]
    pub alt: bool,
    #[serde(default = "always_true", alias = "keep_shots")]
    pub delete_old: bool,
    #[serde(default = "default_retention_days")]
    pub retention_days: i32,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub theme: String,
    #[serde(default = "always_true")]
    pub panel_open: bool,
    #[serde(default = "default_tilt_value")]
    pub tilt: i32,
    #[serde(default = "always_true")]
    pub tape_visible: bool,
    #[serde(default = "default_tape_width")]
    pub tape_width: i32,
    #[serde(default = "default_tape_height")]
    pub tape_height: i32,
    #[serde(default = "always_true")]
    pub polaroid: bool,
    #[serde(default = "default_editor_width")]
    pub editor_width: i32,
    #[serde(default = "default_editor_height")]
    pub editor_height: i32,
    #[serde(default)]
    pub editor_x: Option<i32>,
    #[serde(default)]
    pub editor_y: Option<i32>,
    #[serde(default = "default_main_width")]
    pub main_width: i32,
    #[serde(default = "default_main_height")]
    pub main_height: i32,
    #[serde(default)]
    pub main_x: Option<i32>,
    #[serde(default)]
    pub main_y: Option<i32>,
    #[serde(default = "default_lens_width")]
    pub lens_width: i32,
    #[serde(default = "default_lens_height")]
    pub lens_height: i32,
    #[serde(default)]
    pub lens_x: Option<i32>,
    #[serde(default)]
    pub lens_y: Option<i32>,
    #[serde(default = "default_gallery_zoom")]
    pub gallery_zoom: i32,
    #[serde(default = "default_caption_size")]
    pub caption_size: i32,
    #[serde(default = "always_true")]
    pub peek_one: bool,
    #[serde(default = "always_true")]
    pub peek_two: bool,
    #[serde(default = "default_peek_delay")]
    pub peek_one_delay: i32,
    #[serde(default = "default_peek_delay")]
    pub peek_two_delay: i32,
    #[serde(default = "default_peek_one_scale")]
    pub peek_one_scale: String,
    #[serde(default = "default_peek_two_scale")]
    pub peek_two_scale: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            control: true,
            shift: true,
            alt: true,
            delete_old: true,
            retention_days: DEFAULT_RETENTION_DAYS,
            language: String::new(),
            theme: String::new(),
            panel_open: true,
            tilt: DEFAULT_TILT,
            tape_visible: true,
            tape_width: DEFAULT_TAPE_WIDTH,
            tape_height: DEFAULT_TAPE_HEIGHT,
            polaroid: true,
            editor_width: DEFAULT_EDITOR_WIDTH,
            editor_height: DEFAULT_EDITOR_HEIGHT,
            editor_x: None,
            editor_y: None,
            main_width: DEFAULT_MAIN_WIDTH,
            main_height: DEFAULT_MAIN_HEIGHT,
            main_x: None,
            main_y: None,
            lens_width: DEFAULT_LENS_WIDTH,
            lens_height: DEFAULT_LENS_HEIGHT,
            lens_x: None,
            lens_y: None,
            gallery_zoom: DEFAULT_GALLERY_ZOOM,
            caption_size: DEFAULT_CAPTION_SIZE,
            peek_one: true,
            peek_two: true,
            peek_one_delay: DEFAULT_PEEK_DELAY,
            peek_two_delay: DEFAULT_PEEK_DELAY,
            peek_one_scale: DEFAULT_PEEK_ONE_SCALE.to_string(),
            peek_two_scale: DEFAULT_PEEK_TWO_SCALE.to_string(),
        }
    }
}

impl Settings {
    pub fn modifiers(&self) -> Modifiers {
        let parsed = Modifiers {
            control: self.control,
            shift: self.shift,
            alt: self.alt,
        };
        if parsed.has_any() {
            parsed
        } else {
            Modifiers::DEFAULT
        }
    }

    pub fn normalized(&self) -> Self {
        let modifiers = self.modifiers();
        Self {
            control: modifiers.control,
            shift: modifiers.shift,
            alt: modifiers.alt,
            delete_old: self.delete_old,
            retention_days: normalize_days(self.retention_days),
            language: normalize_culture(&self.language).to_string(),
            theme: normalize_theme(&self.theme).to_string(),
            panel_open: self.panel_open,
            tilt: normalize_tilt(self.tilt),
            tape_visible: self.tape_visible,
            tape_width: normalize_tape_width(self.tape_width),
            tape_height: normalize_tape_height(self.tape_height),
            polaroid: self.polaroid,
            editor_width: normalize_editor_width(self.editor_width),
            editor_height: normalize_editor_height(self.editor_height),
            editor_x: self.editor_x,
            editor_y: self.editor_y,
            main_width: normalize_main_width(self.main_width),
            main_height: normalize_main_height(self.main_height),
            main_x: self.main_x,
            main_y: self.main_y,
            lens_width: normalize_lens_width(self.lens_width),
            lens_height: normalize_lens_height(self.lens_height),
            lens_x: self.lens_x,
            lens_y: self.lens_y,
            gallery_zoom: normalize_gallery_zoom(self.gallery_zoom),
            caption_size: normalize_caption_size(self.caption_size),
            peek_one: self.peek_one,
            peek_two: self.peek_two,
            peek_one_delay: normalize_peek_delay(self.peek_one_delay),
            peek_two_delay: normalize_peek_delay(self.peek_two_delay),
            peek_one_scale: normalize_peek_scale(&self.peek_one_scale)
                .to_string(),
            peek_two_scale: normalize_peek_two_scale(
                &self.peek_one_scale,
                &self.peek_two_scale,
            )
            .to_string(),
        }
    }
}

/// Writes size and position for one window group.
pub fn apply_window_placement(
    settings: &mut Settings,
    role: WindowRole,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    match role {
        WindowRole::Main => {
            settings.main_x = Some(x);
            settings.main_y = Some(y);
            settings.main_width = normalize_main_width(width);
            settings.main_height = normalize_main_height(height);
        }
        WindowRole::Editor => {
            settings.editor_x = Some(x);
            settings.editor_y = Some(y);
            settings.editor_width = normalize_editor_width(width);
            settings.editor_height = normalize_editor_height(height);
        }
        WindowRole::Lens => {
            settings.lens_x = Some(x);
            settings.lens_y = Some(y);
            settings.lens_width = normalize_lens_width(width);
            settings.lens_height = normalize_lens_height(height);
        }
    }
}

pub fn load_settings() -> Settings {
    load_settings_from(&settings_dir(), &legacy_settings_dir())
}

pub fn load_settings_from(current: &Path, legacy: &Path) -> Settings {
    let current_json = current.join(SETTINGS_FILE);
    if current_json.is_file() {
        return parse_settings_file(&current_json).unwrap_or_default();
    }
    let legacy_json = legacy.join(SETTINGS_FILE);
    if legacy_json.is_file() {
        return parse_settings_file(&legacy_json).unwrap_or_default();
    }
    settings_from_legacy(current, legacy)
}

pub fn save_settings(settings: &Settings) -> std::io::Result<()> {
    save_settings_to(&settings_dir(), settings)
}

pub fn save_settings_to(
    directory: &Path,
    settings: &Settings,
) -> std::io::Result<()> {
    fs::create_dir_all(directory)?;
    let stored = settings.normalized();
    let text = serde_json::to_string_pretty(&stored)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    fs::write(directory.join(SETTINGS_FILE), text)?;
    forget_legacy_setting_files(directory);
    Ok(())
}

pub fn update_settings<F>(edit: F) -> std::io::Result<Settings>
where
    F: FnOnce(&mut Settings),
{
    let mut settings = load_settings();
    edit(&mut settings);
    save_settings(&settings)?;
    Ok(settings.normalized())
}

fn parse_settings_file(path: &Path) -> Option<Settings> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Settings>(&text)
        .ok()
        .map(|settings| settings.normalized())
}

fn settings_from_legacy(current: &Path, legacy: &Path) -> Settings {
    let modifiers =
        read_modifiers(&resolve_in(current, legacy, "modifiers.txt"));
    Settings {
        control: modifiers.control,
        shift: modifiers.shift,
        alt: modifiers.alt,
        delete_old: true,
        retention_days: read_retention(&resolve_in(
            current,
            legacy,
            "retention.txt",
        )),
        language: fs::read_to_string(resolve_in(
            current,
            legacy,
            "culture.txt",
        ))
        .unwrap_or_default(),
        theme: fs::read_to_string(resolve_in(current, legacy, "theme.txt"))
            .unwrap_or_default(),
        panel_open: read_panel_open(&resolve_in(current, legacy, "panel.txt")),
        tilt: read_tilt(&resolve_in(current, legacy, "tilt.txt")),
        tape_visible: read_tape_visible(&resolve_in(
            current, legacy, "tape.txt",
        )),
        tape_width: DEFAULT_TAPE_WIDTH,
        tape_height: DEFAULT_TAPE_HEIGHT,
        polaroid: read_polaroid(&resolve_in(current, legacy, "polaroid.txt")),
        editor_width: DEFAULT_EDITOR_WIDTH,
        editor_height: DEFAULT_EDITOR_HEIGHT,
        editor_x: None,
        editor_y: None,
        main_width: DEFAULT_MAIN_WIDTH,
        main_height: DEFAULT_MAIN_HEIGHT,
        main_x: None,
        main_y: None,
        lens_width: DEFAULT_LENS_WIDTH,
        lens_height: DEFAULT_LENS_HEIGHT,
        lens_x: None,
        lens_y: None,
        gallery_zoom: DEFAULT_GALLERY_ZOOM,
        caption_size: DEFAULT_CAPTION_SIZE,
        peek_one: true,
        peek_two: true,
        peek_one_delay: DEFAULT_PEEK_DELAY,
        peek_two_delay: DEFAULT_PEEK_DELAY,
        peek_one_scale: DEFAULT_PEEK_ONE_SCALE.to_string(),
        peek_two_scale: DEFAULT_PEEK_TWO_SCALE.to_string(),
    }
    .normalized()
}

fn resolve_in(current: &Path, legacy: &Path, name: &str) -> PathBuf {
    let path = current.join(name);
    if path.is_file() {
        return path;
    }
    let path = legacy.join(name);
    if path.is_file() {
        return path;
    }
    current.join(name)
}

fn forget_legacy_setting_files(directory: &Path) {
    for name in LEGACY_SETTING_FILES {
        let _ = fs::remove_file(directory.join(name));
    }
}

pub fn caption_clock(file_name: &str, modified_ms: u128) -> String {
    let seconds = taken_text(file_name);
    if seconds.is_empty() {
        return String::new();
    }
    let millis = (modified_ms % 1000) as u32;
    format!("{seconds}.{millis:03}")
}

pub fn taken_text(file_name: &str) -> String {
    let Some(stem) = original_shot_name(file_name)
        .strip_prefix(SHOT_PREFIX)
        .and_then(|name| name.strip_suffix(SHOT_EXTENSION))
    else {
        return String::new();
    };
    let Some((date, rest)) = stem.split_once('-') else {
        return String::new();
    };
    if date.len() != 8 || rest.len() < 6 {
        return String::new();
    }
    if rest.len() > 6 && !rest[6..].starts_with('-') {
        return String::new();
    }
    let time = &rest[..6];
    if !date.bytes().all(|byte| byte.is_ascii_digit())
        || !time.bytes().all(|byte| byte.is_ascii_digit())
    {
        return String::new();
    }
    format!(
        "{}-{}-{} {}:{}:{}",
        &date[0..4],
        &date[4..6],
        &date[6..8],
        &time[0..2],
        &time[2..4],
        &time[4..6]
    )
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

/// Why a shot could not be read or saved as an edit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShotEditError {
    InvalidImage,
    MissingSource,
    OutsideLibrary,
    NameExhausted,
    WriteFailed,
    DeleteFailed,
}

/// Strips `Modify_` / `Modify2_` so captions and names stay on the original.
pub fn original_shot_name(file_name: &str) -> &str {
    let Some((prefix, rest)) = file_name.split_once('_') else {
        return file_name;
    };
    if rest.is_empty() {
        return file_name;
    }
    if prefix == EDITED_MARK {
        return rest;
    }
    let Some(digits) = prefix.strip_prefix(EDITED_MARK) else {
        return file_name;
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return file_name;
    }
    rest
}

/// Next unused `Modify_` name beside the source shot.
pub fn next_edited_shot_path(source: &Path) -> Option<PathBuf> {
    let parent = source.parent()?;
    let name = source.file_name()?.to_str()?;
    let original = original_shot_name(name);
    let first = parent.join(format!("{EDITED_MARK}_{original}"));
    if !first.exists() {
        return Some(first);
    }
    for index in 2..MAX_SHOT_SUFFIX {
        let candidate = parent.join(format!("{EDITED_MARK}{index}_{original}"));
        if !candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

/// PNG files start with this signature.
pub fn is_png(bytes: &[u8]) -> bool {
    bytes.starts_with(PNG_SIGNATURE)
}

/// True when `path` is inside `root` after both are made canonical.
pub fn is_under_directory(path: &Path, root: &Path) -> bool {
    let Ok(path) = path.canonicalize() else {
        return false;
    };
    let Ok(root) = root.canonicalize() else {
        return false;
    };
    path.starts_with(root)
}

/// Reads a PNG from the shots library.
pub fn read_shot_file(
    path: &Path,
    shots_root: &Path,
) -> Result<Vec<u8>, ShotEditError> {
    if !path.is_file() {
        return Err(ShotEditError::MissingSource);
    }
    if !is_shot_file(path) || !is_under_directory(path, shots_root) {
        return Err(ShotEditError::OutsideLibrary);
    }
    let bytes = fs::read(path).map_err(|_| ShotEditError::InvalidImage)?;
    if bytes.len() > MAX_EDITED_PNG || !is_png(&bytes) {
        return Err(ShotEditError::InvalidImage);
    }
    Ok(bytes)
}

/// Writes the edited PNG beside the source with a `Modify` prefix.
pub fn write_edited_shot(
    source: &Path,
    png: &[u8],
    shots_root: &Path,
) -> Result<PathBuf, ShotEditError> {
    if png.len() > MAX_EDITED_PNG || !is_png(png) {
        return Err(ShotEditError::InvalidImage);
    }
    if !source.is_file() {
        return Err(ShotEditError::MissingSource);
    }
    if !is_shot_file(source) || !is_under_directory(source, shots_root) {
        return Err(ShotEditError::OutsideLibrary);
    }
    let destination =
        next_edited_shot_path(source).ok_or(ShotEditError::NameExhausted)?;
    fs::write(&destination, png).map_err(|_| ShotEditError::WriteFailed)?;
    Ok(destination)
}

/// Deletes library shots and empty day folders they left behind.
pub fn delete_library_shots(
    paths: &[PathBuf],
    shots_root: &Path,
) -> Result<usize, ShotEditError> {
    let mut deleted = 0usize;
    let mut parents = Vec::new();
    for path in paths {
        delete_library_shot(path, shots_root)?;
        deleted += 1;
        if let Some(parent) = path.parent() {
            parents.push(parent.to_path_buf());
        }
    }
    for parent in parents {
        remove_empty_day_folder(&parent);
    }
    Ok(deleted)
}

fn delete_library_shot(
    path: &Path,
    shots_root: &Path,
) -> Result<(), ShotEditError> {
    if !path.is_file() {
        return Err(ShotEditError::MissingSource);
    }
    if !is_shot_file(path) || !is_under_directory(path, shots_root) {
        return Err(ShotEditError::OutsideLibrary);
    }
    fs::remove_file(path).map_err(|_| ShotEditError::DeleteFailed)
}

fn remove_empty_day_folder(directory: &Path) {
    let Some(name) = directory.file_name().and_then(|name| name.to_str())
    else {
        return;
    };
    if !is_day_folder(name) {
        return;
    }
    let Ok(mut children) = fs::read_dir(directory) else {
        return;
    };
    if children.next().is_none() {
        let _ = fs::remove_dir(directory);
    }
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
    let deleted = delete_shot_files(
        utc_now,
        directory,
        max_age,
        today,
        &mut deleted_parents,
    );
    delete_empty_day_folders(directory, &deleted_parents, today);
    deleted
}

fn delete_shot_files(
    utc_now: std::time::SystemTime,
    directory: &Path,
    max_age: std::time::Duration,
    today: (i32, u32, u32),
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
            if !is_shot_file(&path)
                || !is_past_retention(&path, utc_now, max_age, today)
            {
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
    original_shot_name(name).starts_with(SHOT_PREFIX)
        && name.to_ascii_lowercase().ends_with(SHOT_EXTENSION)
}

fn is_past_retention(
    path: &Path,
    utc_now: std::time::SystemTime,
    max_age: std::time::Duration,
    today: (i32, u32, u32),
) -> bool {
    let Some(day) = shot_day(path) else {
        return is_stale(path, utc_now, max_age);
    };
    days_before(day, today) >= retention_day_count(max_age)
}

fn shot_day(path: &Path) -> Option<(i32, u32, u32)> {
    path.parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .and_then(parse_day_folder)
}

fn retention_day_count(max_age: std::time::Duration) -> i64 {
    i64::try_from(max_age.as_secs() / SECONDS_PER_DAY).unwrap_or(i64::MAX)
}

fn days_before(earlier: (i32, u32, u32), later: (i32, u32, u32)) -> i64 {
    let later_days = unix_days(later.0, later.1, later.2);
    let earlier_days = unix_days(earlier.0, earlier.1, earlier.2);
    later_days - earlier_days
}

fn unix_days(year: i32, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = (year - era * 400) as u32;
    let month_index = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era =
        year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    i64::from(era) * 146_097 + i64::from(day_of_era) - 719_468
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

const EXTENDED_UNC_PREFIX: &str = r"\\?\UNC\";
const EXTENDED_PATH_PREFIX: &str = r"\\?\";

pub fn clipboard_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix(EXTENDED_UNC_PREFIX) {
        return format!(r"\\{rest}");
    }
    if let Some(rest) = text.strip_prefix(EXTENDED_PATH_PREFIX) {
        return rest.to_string();
    }
    text.into_owned()
}

pub fn hdrop_blob(path: &str) -> Vec<u8> {
    hdrop_paths(&[path])
}

pub fn hdrop_paths(paths: &[&str]) -> Vec<u8> {
    let mut text: Vec<u16> = Vec::new();
    for path in paths {
        text.extend(path.encode_utf16());
        text.push(0);
    }
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

#[derive(
    Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub struct ShotNote {
    #[serde(default)]
    pub memo: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Memo and tags keyed by a library-relative path.
pub type ShotNotes = BTreeMap<String, ShotNote>;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct GalleryShot {
    pub path: String,
    pub day: String,
    pub taken: String,
    pub modified_ms: u128,
    pub memo: String,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Gallery {
    pub days: Vec<String>,
    pub shots: Vec<GalleryShot>,
}

/// Newer day folders first, then newer files inside a day.
pub fn compare_gallery_shots(
    left: &GalleryShot,
    right: &GalleryShot,
) -> std::cmp::Ordering {
    right
        .day
        .cmp(&left.day)
        .then_with(|| right.modified_ms.cmp(&left.modified_ms))
        .then_with(|| right.path.cmp(&left.path))
}

pub fn list_gallery(root: &Path) -> Gallery {
    let mut shots = Vec::new();
    collect_shots(root, &mut shots);
    shots.sort_by(compare_gallery_shots);
    let mut days: Vec<String> =
        shots.iter().map(|shot| shot.day.clone()).collect();
    days.sort();
    days.dedup();
    days.reverse();
    Gallery { days, shots }
}

/// Relative key used in `notes.json`.
pub fn shot_note_key(path: &Path, shots_root: &Path) -> Option<String> {
    let relative = match (path.canonicalize(), shots_root.canonicalize()) {
        (Ok(full), Ok(root)) => full.strip_prefix(root).ok()?.to_path_buf(),
        _ => path.strip_prefix(shots_root).ok()?.to_path_buf(),
    };
    Some(relative.to_string_lossy().replace('\\', "/"))
}

/// Trims and caps a memo line.
pub fn normalize_memo(text: &str) -> String {
    text.trim().chars().take(MAX_MEMO_CHARS).collect()
}

/// Lowercases, drops blanks, and keeps a few unique hash tags.
pub fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut stored = Vec::new();
    for tag in tags {
        let item = normalize_tag(tag);
        if item.is_empty() || stored.iter().any(|have| have == &item) {
            continue;
        }
        stored.push(item);
        if stored.len() == MAX_NOTE_TAGS {
            break;
        }
    }
    stored
}

/// Unique hash tags from many shots, sorted.
pub fn catalog_tags(tags: &[String]) -> Vec<String> {
    let mut stored = Vec::new();
    for tag in tags {
        let item = normalize_tag(tag);
        if item.is_empty() || stored.iter().any(|have| have == &item) {
            continue;
        }
        stored.push(item);
    }
    stored.sort();
    stored
}

/// Prefix after `#` when the draft is a tag query.
pub fn tag_query_prefix(query: &str) -> Option<String> {
    let item = query.trim();
    let rest = item.strip_prefix('#')?;
    Some(rest.trim().to_lowercase())
}

/// Tags that start with the typed `#` prefix.
pub fn filter_tag_suggestions(tags: &[String], query: &str) -> Vec<String> {
    let Some(needle) = tag_query_prefix(query) else {
        return Vec::new();
    };
    catalog_tags(tags)
        .into_iter()
        .filter(|tag| tag.trim_start_matches('#').starts_with(&needle))
        .collect()
}

fn normalize_tag(tag: &str) -> String {
    let mut item = tag.trim().to_lowercase();
    loop {
        let Some(rest) = item.strip_prefix('#') else {
            break;
        };
        item = rest.trim().to_string();
    }
    if item.is_empty() {
        String::new()
    } else {
        format!("#{item}")
    }
}

/// True when the gallery filter should keep this shot.
pub fn shot_matches_query(shot: &GalleryShot, query: &str) -> bool {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return true;
    }
    if shot.memo.to_lowercase().contains(&needle) {
        return true;
    }
    if shot.tags.iter().any(|tag| tag.contains(&needle)) {
        return true;
    }
    if shot.taken.to_lowercase().contains(&needle) {
        return true;
    }
    shot.day.to_lowercase().contains(&needle)
}

/// Writes memo and tags onto listed shots.
pub fn attach_shot_notes(
    shots: &mut [GalleryShot],
    notes: &ShotNotes,
    shots_root: &Path,
) {
    for shot in shots {
        let Some(key) = shot_note_key(Path::new(&shot.path), shots_root) else {
            continue;
        };
        let Some(note) = notes.get(&key) else {
            continue;
        };
        shot.memo = note.memo.clone();
        shot.tags = note.tags.clone();
    }
}

pub fn load_notes() -> ShotNotes {
    load_notes_from(&settings_dir())
}

pub fn load_notes_from(directory: &Path) -> ShotNotes {
    let text = fs::read_to_string(directory.join(NOTES_FILE))
        .unwrap_or_else(|_| "{}".to_string());
    serde_json::from_str(&text).unwrap_or_default()
}

pub fn save_notes_to(
    directory: &Path,
    notes: &ShotNotes,
) -> std::io::Result<()> {
    fs::create_dir_all(directory)?;
    let text = serde_json::to_string_pretty(notes)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    fs::write(directory.join(NOTES_FILE), text)
}

/// Saves or clears one shot note.
pub fn upsert_shot_note(
    path: &Path,
    shots_root: &Path,
    notes_dir: &Path,
    memo: &str,
    tags: &[String],
) -> Result<ShotNote, ShotEditError> {
    upsert_shot_notes(&[path.to_path_buf()], shots_root, notes_dir, memo, tags)
}

/// Saves or clears the same note on several shots.
pub fn upsert_shot_notes(
    paths: &[PathBuf],
    shots_root: &Path,
    notes_dir: &Path,
    memo: &str,
    tags: &[String],
) -> Result<ShotNote, ShotEditError> {
    if paths.is_empty() {
        return Err(ShotEditError::MissingSource);
    }
    let note = ShotNote {
        memo: normalize_memo(memo),
        tags: normalize_tags(tags),
    };
    let mut notes = load_notes_from(notes_dir);
    for path in paths {
        if !path.is_file() || !is_shot_file(path) {
            return Err(ShotEditError::MissingSource);
        }
        if !is_under_directory(path, shots_root) {
            return Err(ShotEditError::OutsideLibrary);
        }
        let key = shot_note_key(path, shots_root)
            .ok_or(ShotEditError::OutsideLibrary)?;
        if note.memo.is_empty() && note.tags.is_empty() {
            notes.remove(&key);
        } else {
            notes.insert(key, note.clone());
        }
    }
    save_notes_to(notes_dir, &notes).map_err(|_| ShotEditError::WriteFailed)?;
    Ok(note)
}

/// Copies a note when an edited file is written beside the source.
pub fn copy_shot_note(
    source: &Path,
    destination: &Path,
    shots_root: &Path,
    notes_dir: &Path,
) -> std::io::Result<()> {
    let Some(from) = shot_note_key(source, shots_root) else {
        return Ok(());
    };
    let Some(into) = shot_note_key(destination, shots_root) else {
        return Ok(());
    };
    let mut notes = load_notes_from(notes_dir);
    let Some(note) = notes.get(&from).cloned() else {
        return Ok(());
    };
    notes.insert(into, note);
    save_notes_to(notes_dir, &notes)
}

/// Drops notes for deleted library files.
pub fn remove_shot_notes(
    paths: &[PathBuf],
    shots_root: &Path,
    notes_dir: &Path,
) -> std::io::Result<()> {
    let mut notes = load_notes_from(notes_dir);
    let mut changed = false;
    for path in paths {
        if let Some(key) = shot_note_key(path, shots_root) {
            changed |= notes.remove(&key).is_some();
        }
    }
    if changed {
        save_notes_to(notes_dir, &notes)?;
    }
    Ok(())
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
        let taken = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| caption_clock(name, modified_ms))
            .unwrap_or_default();
        shots.push(GalleryShot {
            path: path.display().to_string(),
            day,
            taken,
            modified_ms,
            memo: String::new(),
            tags: Vec::new(),
        });
    }
}

pub fn shots_directory() -> PathBuf {
    settings_dir().join("shots")
}

pub fn legacy_shots_directory() -> PathBuf {
    std::env::temp_dir().join("shots")
}

pub fn prepare_shots_directory() -> PathBuf {
    let current = shots_directory();
    adopt_legacy_shots(&legacy_shots_directory(), &current);
    current
}

pub fn adopt_legacy_shots(legacy: &Path, current: &Path) {
    if !legacy.is_dir() || legacy == current {
        return;
    }
    let Ok(entries) = fs::read_dir(legacy) else {
        return;
    };
    for entry in entries.flatten() {
        adopt_legacy_entry(&entry.path(), current);
    }
    let _ = fs::remove_dir(legacy);
}

fn adopt_legacy_entry(source: &Path, current: &Path) {
    let Some(name) = source.file_name().and_then(|name| name.to_str()) else {
        return;
    };
    if source.is_dir() && is_day_folder(name) {
        move_day_folder(source, &current.join(name));
        let _ = fs::remove_dir(source);
        return;
    }
    if is_shot_file(source) {
        move_file_if_absent(source, &current.join(name));
    }
}

fn move_day_folder(source: &Path, destination: &Path) {
    let Ok(entries) = fs::read_dir(source) else {
        return;
    };
    let _ = fs::create_dir_all(destination);
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name() else {
            continue;
        };
        if path.is_dir() {
            continue;
        }
        move_file_if_absent(&path, &destination.join(name));
    }
}

/// Installed Windows OCR language packs that this app cares about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OcrPackStatus {
    pub has_korean: bool,
    pub has_english: bool,
    pub can_run: bool,
}

/// True when a WinOCR tag belongs to a language prefix.
pub fn is_ocr_language_tag(tag: &str, prefix: &str) -> bool {
    let item = tag.trim().to_ascii_lowercase();
    let prefix = prefix.trim().to_ascii_lowercase();
    item == prefix || item.starts_with(&format!("{prefix}-"))
}

/// Reads which OCR packs are present from WinOCR language tags.
pub fn ocr_pack_status(tags: &[String]) -> OcrPackStatus {
    let has_korean = tags.iter().any(|tag| is_ocr_language_tag(tag, "ko"));
    let has_english = tags.iter().any(|tag| is_ocr_language_tag(tag, "en"));
    OcrPackStatus {
        has_korean,
        has_english,
        can_run: !tags.is_empty(),
    }
}

/// Picks Korean, then English, then whatever WinOCR offered.
pub fn preferred_ocr_tag(tags: &[String]) -> Option<String> {
    tags.iter()
        .find(|tag| is_ocr_language_tag(tag, "ko"))
        .or_else(|| tags.iter().find(|tag| is_ocr_language_tag(tag, "en")))
        .cloned()
        .or_else(|| tags.first().cloned())
}

/// Builds a markdown page with the original image and the OCR lines.
pub fn markdown_from_ocr(image_name: &str, lines: &[String]) -> String {
    let mut body = format!("![]({image_name})\n");
    let text = lines
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if text.is_empty() {
        return body;
    }
    body.push('\n');
    body.push_str(&text);
    body.push('\n');
    body
}

/// Joins several shot markdown pages.
pub fn join_markdown_blocks(blocks: &[String]) -> String {
    blocks.join("\n\n")
}

/// Writes the sidecar markdown next to the PNG.
pub fn markdown_sidecar_path(image: &Path) -> PathBuf {
    image.with_extension("md")
}

/// Explains missing OCR packs and how to install them.
pub fn format_ocr_notice(
    language: crate::strings::UiLanguage,
    status: &OcrPackStatus,
) -> String {
    use crate::strings::{TextId, text};
    let mut lines = Vec::new();
    if !status.can_run {
        lines.push(text(language, TextId::OcrMissingAll).to_string());
        lines.push(text(language, TextId::OcrInstallHow).to_string());
        return lines.join("\n\n");
    }
    if !status.has_korean {
        lines.push(text(language, TextId::OcrMissingKorean).to_string());
    }
    if !status.has_english {
        lines.push(text(language, TextId::OcrEnglishOptional).to_string());
    }
    if !lines.is_empty() {
        lines.push(text(language, TextId::OcrInstallHow).to_string());
    }
    lines.join("\n\n")
}

fn move_file_if_absent(source: &Path, destination: &Path) {
    if destination.exists() {
        return;
    }
    if let Some(parent) = destination.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if fs::rename(source, destination).is_ok() {
        return;
    }
    if fs::copy(source, destination).is_ok() {
        let _ = fs::remove_file(source);
    }
}
