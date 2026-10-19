#![windows_subsystem = "windows"]

use std::path::Path;
use std::sync::Mutex;
use std::sync::mpsc;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
use tauri::{Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use agsnap::{
    APP_NAME, APP_VERSION, CaptureNotice, Modifiers, TextId, UiLanguage,
    apply_runtime_settings, copy_shot_files, language_from_tag, list_gallery,
    normalize_culture, normalize_days, normalize_theme, normalize_tilt,
    open_document, prepare_process, prepare_shots_directory, read_history_text,
    read_modifiers, read_panel_open, read_polaroid, read_retention,
    read_tape_visible, read_tilt, resolve_settings_file, set_notice_sender,
    settings_dir, spawn_capture, tag_from_language, text, write_modifiers,
    write_panel_open, write_polaroid, write_retention, write_tape_visible,
    write_tilt,
};

static LAST_SHOT: Mutex<Option<String>> = Mutex::new(None);

#[derive(serde::Serialize)]
struct ShellState {
    language: String,
    version: String,
    retention_days: i32,
    control: bool,
    shift: bool,
    alt: bool,
    hint: String,
    gallery: String,
    all: String,
    empty: String,
    folder: String,
    retention: String,
    shortcut: String,
    log: String,
    version_label: String,
    language_label: String,
    exit_label: String,
    system: String,
    korean: String,
    english: String,
    days_label: String,
    changelog: String,
    copy_image: String,
    magnify_image: String,
    theme: String,
    theme_label: String,
    theme_system: String,
    theme_dark: String,
    theme_light: String,
    panel_open: bool,
    panel_open_label: String,
    panel_close_label: String,
    tilt: i32,
    tilt_label: String,
    tape_visible: bool,
    tape_label: String,
    polaroid: bool,
    polaroid_label: String,
    history_label: String,
    close_label: String,
}

fn main() {
    prepare_process();
    let (sender, receiver) = mpsc::channel::<CaptureNotice>();
    set_notice_sender(sender);
    spawn_capture();
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(move |app| {
            install_tray(app)?;
            watch_notices(app.handle().clone(), receiver);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            shell_state,
            gallery,
            open_shot,
            copy_shots,
            open_folder,
            open_log,
            set_shortcut,
            set_retention_days,
            set_language,
            set_theme,
            set_panel_open,
            set_tilt,
            set_tape_visible,
            set_polaroid
        ])
        .run(tauri::generate_context!())
        .expect("AgSnapTool");
}

fn install_tray(app: &tauri::App) -> tauri::Result<()> {
    let language = current_language();
    let gallery = MenuItem::with_id(
        app,
        "gallery",
        text(language, TextId::MenuGallery),
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(
        app,
        "quit",
        text(language, TextId::MenuExit),
        true,
        None::<&str>,
    )?;
    let menu = Menu::with_items(app, &[&gallery, &quit])?;
    let icon =
        tauri::image::Image::from_bytes(include_bytes!("../ui/icon.png"))?;
    let tray = tauri::tray::TrayIconBuilder::new()
        .icon(icon)
        .tooltip(format!("{APP_NAME} {APP_VERSION}"))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "gallery" => show_gallery(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_gallery(tray.app_handle());
            }
        })
        .build(app)?;
    let _ = tray;
    Ok(())
}

fn show_gallery(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn watch_notices(
    app: tauri::AppHandle,
    receiver: mpsc::Receiver<CaptureNotice>,
) {
    let _ = app.notification().on_action(|_| {
        let path = LAST_SHOT.lock().expect("shot").clone();
        if let Some(path) = path {
            open_document(Path::new(&path));
        }
    });
    std::thread::spawn(move || {
        while let Ok(notice) = receiver.recv() {
            if let Some(path) = notice.path.clone() {
                *LAST_SHOT.lock().expect("shot") = Some(path);
            }
            let _ = app.emit("gallery-refresh", ());
            if notice.text.is_empty() {
                continue;
            }
            let _ = app
                .notification()
                .builder()
                .title(APP_NAME)
                .body(notice.text)
                .show();
        }
    });
}

#[tauri::command]
fn shell_state() -> ShellState {
    current_shell()
}

#[tauri::command]
fn gallery() -> agsnap::Gallery {
    list_gallery(&prepare_shots_directory())
}

#[tauri::command]
fn open_shot(path: String) {
    open_document(Path::new(&path));
}

#[tauri::command]
fn copy_shots(paths: Vec<String>) -> Result<(), String> {
    let files = paths
        .iter()
        .map(std::path::PathBuf::from)
        .collect::<Vec<_>>();
    if copy_shot_files(&files) {
        Ok(())
    } else {
        Err(text(current_language(), TextId::CopyFailed).to_string())
    }
}

#[tauri::command]
fn open_folder() {
    let directory = prepare_shots_directory();
    let _ = std::fs::create_dir_all(&directory);
    open_document(&directory);
}

#[tauri::command]
fn open_log() {
    let path = settings_dir().join("gesture.log");
    if path.is_file() {
        open_document(&path);
    }
}

#[tauri::command]
fn set_shortcut(
    control: bool,
    shift: bool,
    alt: bool,
) -> Result<ShellState, String> {
    let modifiers = Modifiers {
        control,
        shift,
        alt,
    };
    if !modifiers.has_any() {
        return Err(text(current_language(), TextId::EmptyShortcut).to_string());
    }
    let path = settings_dir().join("modifiers.txt");
    write_modifiers(&path, modifiers).map_err(|error| error.to_string())?;
    apply_runtime_settings();
    Ok(current_shell())
}

#[tauri::command]
fn set_retention_days(days: i32) -> Result<ShellState, String> {
    let path = settings_dir().join("retention.txt");
    write_retention(&path, normalize_days(days))
        .map_err(|error| error.to_string())?;
    apply_runtime_settings();
    Ok(current_shell())
}

#[tauri::command]
fn set_language(tag: String) -> Result<ShellState, String> {
    let path = settings_dir().join("culture.txt");
    if let Some(directory) = path.parent() {
        std::fs::create_dir_all(directory)
            .map_err(|error| error.to_string())?;
    }
    let stored = tag_from_language(language_from_tag(normalize_culture(&tag)));
    std::fs::write(&path, stored).map_err(|error| error.to_string())?;
    apply_runtime_settings();
    Ok(current_shell())
}

#[tauri::command]
fn set_theme(tag: String) -> Result<ShellState, String> {
    let path = settings_dir().join("theme.txt");
    if let Some(directory) = path.parent() {
        std::fs::create_dir_all(directory)
            .map_err(|error| error.to_string())?;
    }
    let stored = normalize_theme(&tag);
    std::fs::write(&path, stored).map_err(|error| error.to_string())?;
    Ok(current_shell())
}

#[tauri::command]
fn set_tilt(degrees: i32) -> Result<ShellState, String> {
    let path = settings_dir().join("tilt.txt");
    write_tilt(&path, normalize_tilt(degrees))
        .map_err(|error| error.to_string())?;
    Ok(current_shell())
}

#[tauri::command]
fn set_polaroid(enabled: bool) -> Result<ShellState, String> {
    let path = settings_dir().join("polaroid.txt");
    write_polaroid(&path, enabled).map_err(|error| error.to_string())?;
    Ok(current_shell())
}

#[tauri::command]
fn set_tape_visible(visible: bool) -> Result<ShellState, String> {
    let path = settings_dir().join("tape.txt");
    write_tape_visible(&path, visible).map_err(|error| error.to_string())?;
    Ok(current_shell())
}

#[tauri::command]
fn set_panel_open(open: bool) -> Result<ShellState, String> {
    let path = settings_dir().join("panel.txt");
    write_panel_open(&path, open).map_err(|error| error.to_string())?;
    Ok(current_shell())
}

fn current_shell() -> ShellState {
    let language = current_language();
    let modifiers = read_modifiers(&resolve_settings_file("modifiers.txt"));
    ShellState {
        language: tag_from_language(language).to_string(),
        version: APP_VERSION.to_string(),
        retention_days: read_retention(&resolve_settings_file("retention.txt")),
        control: modifiers.control,
        shift: modifiers.shift,
        alt: modifiers.alt,
        hint: agsnap::strings::format_hint(language, &modifiers.format()),
        gallery: text(language, TextId::MenuGallery).to_string(),
        all: text(language, TextId::GalleryAll).to_string(),
        empty: text(language, TextId::GalleryEmpty).to_string(),
        folder: text(language, TextId::MenuOpenFolder).to_string(),
        retention: text(language, TextId::RetentionTitle).to_string(),
        shortcut: text(language, TextId::MenuShortcut).to_string(),
        log: text(language, TextId::MenuOpenLog).to_string(),
        version_label: text(language, TextId::MenuVersion).to_string(),
        language_label: text(language, TextId::MenuLanguage).to_string(),
        exit_label: text(language, TextId::MenuExit).to_string(),
        system: text(language, TextId::LanguageSystem).to_string(),
        korean: text(language, TextId::LanguageKorean).to_string(),
        english: text(language, TextId::LanguageEnglish).to_string(),
        days_label: text(language, TextId::RetentionLabel).to_string(),
        changelog: changelog_text(language),
        copy_image: text(language, TextId::CopyImage).to_string(),
        magnify_image: text(language, TextId::MagnifyImage).to_string(),
        theme: normalize_theme(&read_theme()).to_string(),
        theme_label: text(language, TextId::ThemeTitle).to_string(),
        theme_system: text(language, TextId::ThemeSystem).to_string(),
        theme_dark: text(language, TextId::ThemeDark).to_string(),
        theme_light: text(language, TextId::ThemeLight).to_string(),
        panel_open: read_panel_open(&resolve_settings_file("panel.txt")),
        panel_open_label: text(language, TextId::PanelOpen).to_string(),
        panel_close_label: text(language, TextId::PanelClose).to_string(),
        tilt: read_tilt(&resolve_settings_file("tilt.txt")),
        tilt_label: text(language, TextId::TiltLabel).to_string(),
        tape_visible: read_tape_visible(&resolve_settings_file("tape.txt")),
        tape_label: text(language, TextId::TapeLabel).to_string(),
        polaroid: read_polaroid(&resolve_settings_file("polaroid.txt")),
        polaroid_label: text(language, TextId::PolaroidLabel).to_string(),
        history_label: text(language, TextId::HistoryButton).to_string(),
        close_label: text(language, TextId::DialogClose).to_string(),
    }
}

fn read_theme() -> String {
    let path = resolve_settings_file("theme.txt");
    std::fs::read_to_string(path).unwrap_or_default()
}

fn current_language() -> UiLanguage {
    let path = resolve_settings_file("culture.txt");
    let Ok(value) = std::fs::read_to_string(path) else {
        return UiLanguage::System;
    };
    language_from_tag(normalize_culture(&value))
}

fn changelog_text(language: UiLanguage) -> String {
    let beside = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|dir| dir.join("CHANGELOG.md")));
    if let Some(path) = beside {
        let missing = text(language, TextId::MissingHistory);
        let history = read_history_text(&path, missing);
        if history != missing {
            return history;
        }
    }
    include_str!("../CHANGELOG.md").trim().to_string()
}
