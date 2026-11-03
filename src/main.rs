#![windows_subsystem = "windows"]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{LazyLock, Mutex};

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_notification::NotificationExt;

use agsnap::{
    APP_NAME, APP_VERSION, CaptureNotice, Modifiers, TextId, UiLanguage,
    apply_runtime_settings, apply_window_placement, attach_shot_notes,
    claim_single_instance, copy_shot_files, copy_shot_note,
    copy_text_clipboard, delete_library_shots, editor_window_label,
    format_ocr_notice, installed_ocr_tags, is_editor_label, is_lens_label,
    join_markdown_blocks, language_from_tag, lens_window_label,
    library_shot_files, list_gallery, load_notes, load_settings,
    markdown_from_ocr, markdown_sidecar_path, normalize_caption_size,
    normalize_culture, normalize_days, normalize_gallery_zoom,
    normalize_peek_delay, normalize_peek_scale, normalize_peek_two_scale,
    normalize_tape_height, normalize_tape_width, normalize_theme,
    normalize_tilt, ocr_pack_status, open_document, preferred_ocr_tag,
    prepare_process, prepare_shots_directory, read_history_text,
    read_shot_file, recognize_shot_lines, remove_shot_notes, set_notice_sender,
    settings_dir, spawn_capture, tag_from_language, text, update_settings,
    upsert_shot_notes, window_role_from_label, write_edited_shot,
};

static LAST_SHOT: Mutex<Option<String>> = Mutex::new(None);
static EDITOR_SEQ: AtomicU64 = AtomicU64::new(1);
static EDITOR_PATHS: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static LENS_SEQ: AtomicU64 = AtomicU64::new(1);
static LENS_PATHS: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(serde::Serialize)]
struct ShellState {
    language: String,
    version: String,
    delete_old: bool,
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
    md_copy: String,
    md_save: String,
    magnify_image: String,
    open_with: String,
    delete_image: String,
    delete_confirm: String,
    edit_image: String,
    editor_save: String,
    editor_cancel: String,
    editor_select: String,
    editor_rect: String,
    editor_ellipse: String,
    editor_polygon: String,
    editor_arrow: String,
    editor_text: String,
    editor_stroke: String,
    editor_fill: String,
    editor_width: String,
    editor_head: String,
    editor_head_size: String,
    editor_font: String,
    editor_font_size: String,
    editor_color: String,
    editor_size: String,
    editor_hint: String,
    editor_unsaved: String,
    editor_discard: String,
    editor_head_triangle: String,
    editor_head_chevron: String,
    editor_head_barbed: String,
    theme: String,
    theme_label: String,
    theme_system: String,
    theme_dark: String,
    theme_light: String,
    theme_flat: String,
    theme_material: String,
    theme_glass: String,
    theme_skeuo: String,
    theme_neu: String,
    theme_clay: String,
    theme_brutal: String,
    theme_liquid: String,
    theme_icon3d: String,
    panel_open: bool,
    panel_open_label: String,
    panel_close_label: String,
    tilt: i32,
    tilt_label: String,
    tape_visible: bool,
    tape_label: String,
    tape_width: i32,
    tape_width_label: String,
    tape_height: i32,
    tape_height_label: String,
    polaroid: bool,
    polaroid_label: String,
    caption_size: i32,
    caption_label: String,
    peek_one: bool,
    peek_two: bool,
    peek_one_delay: i32,
    peek_two_delay: i32,
    peek_one_scale: String,
    peek_two_scale: String,
    peek_one_label: String,
    peek_two_label: String,
    peek_delay_label: String,
    peek_scale_label: String,
    peek_scale_max: String,
    gallery_zoom: i32,
    history_label: String,
    close_label: String,
    memo_image: String,
    memo_label: String,
    tags_label: String,
    search_placeholder: String,
}

fn main() {
    prepare_process();
    if !claim_single_instance() {
        return;
    }
    let (sender, receiver) = mpsc::channel::<CaptureNotice>();
    set_notice_sender(sender);
    spawn_capture();
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(move |app| {
            install_tray(app)?;
            if let Some(window) = app.get_webview_window("main") {
                restore_main_window(&window);
            }
            watch_notices(app.handle().clone(), receiver);
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                } else if is_lens_label(window.label()) {
                    LENS_PATHS.lock().expect("lens").remove(window.label());
                }
            }
            tauri::WindowEvent::Resized(_) | tauri::WindowEvent::Moved(_) => {
                remember_window_placement(window);
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            shell_state,
            gallery,
            open_shot,
            copy_shots,
            copy_shot_markdown,
            save_shot_markdown,
            delete_shots,
            open_folder,
            open_log,
            set_shortcut,
            set_delete_old,
            set_retention_days,
            set_language,
            set_theme,
            set_panel_open,
            set_tilt,
            set_tape_visible,
            set_tape_width,
            set_tape_height,
            set_polaroid,
            set_caption_size,
            set_peek,
            set_gallery_zoom,
            read_shot,
            save_edited_shot,
            set_shot_note,
            window_role,
            show_editor,
            hide_editor,
            take_editor_path,
            show_lens,
            hide_lens,
            take_lens_path
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
    let clicked = app.clone();
    let _ = app.notification().on_action(move |_| {
        let path = LAST_SHOT.lock().expect("shot").clone();
        let Some(path) = path else {
            return;
        };
        let handle = clicked.clone();
        tauri::async_runtime::spawn(async move {
            let _ = show_lens(handle, path).await;
        });
    });
    std::thread::spawn(move || {
        while let Ok(notice) = receiver.recv() {
            if notice.show_gallery {
                show_gallery(&app);
            }
            if let Some(path) = notice.path.clone() {
                *LAST_SHOT.lock().expect("shot") = Some(path.clone());
                if notice.open_lens {
                    let handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = show_lens(handle, path).await;
                    });
                }
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
    let root = prepare_shots_directory();
    let mut listed = list_gallery(&root);
    attach_shot_notes(&mut listed.shots, &load_notes(), &root);
    listed
}

#[tauri::command]
fn open_shot(path: String) {
    open_document(Path::new(&path));
}

#[derive(serde::Serialize)]
struct MarkdownOutcome {
    notice: String,
    has_korean: bool,
    has_english: bool,
}

fn markdown_for_paths(
    paths: Vec<String>,
) -> Result<(MarkdownOutcome, Vec<(std::path::PathBuf, String)>), String> {
    let tags = installed_ocr_tags();
    let status = ocr_pack_status(&tags);
    let notice = format_ocr_notice(current_language(), &status);
    if !status.can_run {
        return Err(notice);
    }
    let language = preferred_ocr_tag(&tags).ok_or_else(|| notice.clone())?;
    let files = library_shot_files(
        &paths
            .iter()
            .map(std::path::PathBuf::from)
            .collect::<Vec<_>>(),
    );
    if files.is_empty() {
        return Err(text(current_language(), TextId::CopyFailed).to_string());
    }
    let mut pages = Vec::new();
    for path in files {
        let lines = recognize_shot_lines(&path, &language).map_err(|_| {
            text(current_language(), TextId::OcrFailed).to_string()
        })?;
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("shot.png")
            .to_string();
        pages.push((path, markdown_from_ocr(&name, &lines)));
    }
    let outcome = MarkdownOutcome {
        notice,
        has_korean: status.has_korean,
        has_english: status.has_english,
    };
    Ok((outcome, pages))
}

#[tauri::command]
fn copy_shot_markdown(paths: Vec<String>) -> Result<MarkdownOutcome, String> {
    let (outcome, pages) = markdown_for_paths(paths)?;
    let blocks: Vec<String> = pages.into_iter().map(|(_, body)| body).collect();
    if copy_text_clipboard(&join_markdown_blocks(&blocks)) {
        Ok(outcome)
    } else {
        Err(text(current_language(), TextId::CopyFailed).to_string())
    }
}

#[tauri::command]
fn save_shot_markdown(paths: Vec<String>) -> Result<MarkdownOutcome, String> {
    let (outcome, pages) = markdown_for_paths(paths)?;
    for (path, body) in pages {
        std::fs::write(markdown_sidecar_path(&path), body).map_err(|_| {
            text(current_language(), TextId::SaveFailed).to_string()
        })?;
    }
    Ok(outcome)
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
fn delete_shots(
    app: tauri::AppHandle,
    paths: Vec<String>,
) -> Result<usize, String> {
    let root = prepare_shots_directory();
    let files = paths
        .iter()
        .map(std::path::PathBuf::from)
        .collect::<Vec<_>>();
    let _ = remove_shot_notes(&files, &root, &settings_dir());
    let deleted = delete_library_shots(&files, &root).map_err(edit_error)?;
    let _ = app.emit("gallery-refresh", ());
    Ok(deleted)
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
    update_settings(|settings| {
        settings.control = modifiers.control;
        settings.shift = modifiers.shift;
        settings.alt = modifiers.alt;
    })
    .map_err(|error| error.to_string())?;
    apply_runtime_settings();
    Ok(current_shell())
}

#[tauri::command]
fn set_delete_old(enabled: bool) -> Result<ShellState, String> {
    update_settings(|settings| {
        settings.delete_old = enabled;
    })
    .map_err(|error| error.to_string())?;
    apply_runtime_settings();
    Ok(current_shell())
}

#[tauri::command]
fn set_retention_days(days: i32) -> Result<ShellState, String> {
    update_settings(|settings| {
        settings.retention_days = normalize_days(days);
    })
    .map_err(|error| error.to_string())?;
    apply_runtime_settings();
    Ok(current_shell())
}

#[tauri::command]
fn set_language(tag: String) -> Result<ShellState, String> {
    let stored = tag_from_language(language_from_tag(normalize_culture(&tag)));
    update_settings(|settings| {
        settings.language = stored.to_string();
    })
    .map_err(|error| error.to_string())?;
    apply_runtime_settings();
    Ok(current_shell())
}

#[tauri::command]
fn set_theme(tag: String) -> Result<ShellState, String> {
    update_settings(|settings| {
        settings.theme = normalize_theme(&tag).to_string();
    })
    .map_err(|error| error.to_string())?;
    Ok(current_shell())
}

#[tauri::command]
fn set_tilt(degrees: i32) -> Result<ShellState, String> {
    update_settings(|settings| {
        settings.tilt = normalize_tilt(degrees);
    })
    .map_err(|error| error.to_string())?;
    Ok(current_shell())
}

#[tauri::command]
fn set_polaroid(enabled: bool) -> Result<ShellState, String> {
    update_settings(|settings| {
        settings.polaroid = enabled;
    })
    .map_err(|error| error.to_string())?;
    Ok(current_shell())
}

#[tauri::command]
fn set_caption_size(pixels: i32) -> Result<ShellState, String> {
    update_settings(|settings| {
        settings.caption_size = normalize_caption_size(pixels);
    })
    .map_err(|error| error.to_string())?;
    Ok(current_shell())
}

#[tauri::command]
fn set_peek(
    one: bool,
    two: bool,
    one_delay: i32,
    two_delay: i32,
    one_scale: String,
    two_scale: String,
) -> Result<ShellState, String> {
    update_settings(|settings| {
        settings.peek_one = one;
        settings.peek_two = two;
        settings.peek_one_delay = normalize_peek_delay(one_delay);
        settings.peek_two_delay = normalize_peek_delay(two_delay);
        settings.peek_one_scale = normalize_peek_scale(&one_scale).to_string();
        settings.peek_two_scale =
            normalize_peek_two_scale(&one_scale, &two_scale).to_string();
    })
    .map_err(|error| error.to_string())?;
    Ok(current_shell())
}

#[tauri::command]
fn set_gallery_zoom(pixels: i32) -> Result<ShellState, String> {
    update_settings(|settings| {
        settings.gallery_zoom = normalize_gallery_zoom(pixels);
    })
    .map_err(|error| error.to_string())?;
    Ok(current_shell())
}

#[tauri::command]
fn set_tape_visible(visible: bool) -> Result<ShellState, String> {
    update_settings(|settings| {
        settings.tape_visible = visible;
    })
    .map_err(|error| error.to_string())?;
    Ok(current_shell())
}

#[tauri::command]
fn set_tape_width(pixels: i32) -> Result<ShellState, String> {
    update_settings(|settings| {
        settings.tape_width = normalize_tape_width(pixels);
    })
    .map_err(|error| error.to_string())?;
    Ok(current_shell())
}

#[tauri::command]
fn set_tape_height(pixels: i32) -> Result<ShellState, String> {
    update_settings(|settings| {
        settings.tape_height = normalize_tape_height(pixels);
    })
    .map_err(|error| error.to_string())?;
    Ok(current_shell())
}

#[tauri::command]
fn read_shot(path: String) -> Result<Vec<u8>, String> {
    let root = prepare_shots_directory();
    read_shot_file(Path::new(&path), &root).map_err(edit_error)
}

#[tauri::command]
fn save_edited_shot(
    app: tauri::AppHandle,
    source: String,
    png: Vec<u8>,
) -> Result<String, String> {
    let root = prepare_shots_directory();
    let path = write_edited_shot(Path::new(&source), &png, &root)
        .map_err(edit_error)?;
    let _ = copy_shot_note(Path::new(&source), &path, &root, &settings_dir());
    let _ = app.emit("gallery-refresh", ());
    Ok(path.display().to_string())
}

#[tauri::command]
fn set_shot_note(
    paths: Vec<String>,
    memo: String,
    tags: Vec<String>,
) -> Result<agsnap::ShotNote, String> {
    let files: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    upsert_shot_notes(
        &files,
        &prepare_shots_directory(),
        &settings_dir(),
        &memo,
        &tags,
    )
    .map_err(edit_error)
}

#[tauri::command]
fn window_role(window: tauri::WebviewWindow) -> String {
    if is_editor_label(window.label()) {
        "editor".to_string()
    } else if is_lens_label(window.label()) {
        "lens".to_string()
    } else {
        window.label().to_string()
    }
}

#[tauri::command]
async fn show_editor(
    app: tauri::AppHandle,
    path: String,
) -> Result<(), String> {
    let id = EDITOR_SEQ.fetch_add(1, Ordering::Relaxed);
    let label = editor_window_label(id);
    EDITOR_PATHS
        .lock()
        .expect("editor")
        .insert(label.clone(), path);
    let title = format!(
        "{} — {}",
        APP_NAME,
        text(current_language(), TextId::EditImage)
    );
    open_editor_window(&app, &label, &title)
}

fn open_editor_window(
    app: &tauri::AppHandle,
    label: &str,
    title: &str,
) -> Result<(), String> {
    let settings = load_settings();
    let builder = WebviewWindowBuilder::new(
        app,
        label,
        WebviewUrl::App("index.html".into()),
    )
    .title(title)
    .inner_size(
        f64::from(settings.editor_width),
        f64::from(settings.editor_height),
    )
    .min_inner_size(640.0, 420.0)
    .resizable(true)
    .decorations(true)
    .visible(true);
    with_saved_position(builder, settings.editor_x, settings.editor_y)
        .build()
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
async fn hide_editor(window: tauri::WebviewWindow) {
    EDITOR_PATHS.lock().expect("editor").remove(window.label());
    let _ = window.destroy();
}

#[tauri::command]
fn take_editor_path(window: tauri::WebviewWindow) -> Option<String> {
    EDITOR_PATHS.lock().expect("editor").remove(window.label())
}

#[tauri::command]
async fn show_lens(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let id = LENS_SEQ.fetch_add(1, Ordering::Relaxed);
    let label = lens_window_label(id);
    LENS_PATHS.lock().expect("lens").insert(label.clone(), path);
    let title = format!(
        "{} — {}",
        APP_NAME,
        text(current_language(), TextId::MagnifyImage)
    );
    open_lens_window(&app, &label, &title)
}

fn open_lens_window(
    app: &tauri::AppHandle,
    label: &str,
    title: &str,
) -> Result<(), String> {
    let settings = load_settings();
    let builder = WebviewWindowBuilder::new(
        app,
        label,
        WebviewUrl::App("index.html".into()),
    )
    .title(title)
    .inner_size(
        f64::from(settings.lens_width),
        f64::from(settings.lens_height),
    )
    .min_inner_size(480.0, 320.0)
    .resizable(true)
    .decorations(true)
    .visible(true);
    with_saved_position(builder, settings.lens_x, settings.lens_y)
        .build()
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
async fn hide_lens(window: tauri::WebviewWindow) {
    LENS_PATHS.lock().expect("lens").remove(window.label());
    let _ = window.destroy();
}

#[tauri::command]
fn take_lens_path(window: tauri::WebviewWindow) -> Option<String> {
    LENS_PATHS.lock().expect("lens").remove(window.label())
}

fn with_saved_position<'a, M: Manager<tauri::Wry>>(
    builder: WebviewWindowBuilder<'a, tauri::Wry, M>,
    x: Option<i32>,
    y: Option<i32>,
) -> WebviewWindowBuilder<'a, tauri::Wry, M> {
    match (x, y) {
        (Some(left), Some(top)) => {
            builder.position(f64::from(left), f64::from(top))
        }
        _ => builder.center(),
    }
}

fn restore_main_window(window: &tauri::WebviewWindow) {
    let settings = load_settings();
    let _ = window.set_size(tauri::LogicalSize::new(
        f64::from(settings.main_width),
        f64::from(settings.main_height),
    ));
    if let (Some(x), Some(y)) = (settings.main_x, settings.main_y) {
        let _ = window.set_position(tauri::LogicalPosition::new(
            f64::from(x),
            f64::from(y),
        ));
    }
}

fn remember_window_placement(window: &tauri::Window) {
    let Some(role) = window_role_from_label(window.label()) else {
        return;
    };
    if window.is_minimized().unwrap_or(false) {
        return;
    }
    let Ok(scale) = window.scale_factor() else {
        return;
    };
    let Ok(size) = window.inner_size() else {
        return;
    };
    let Ok(position) = window.outer_position() else {
        return;
    };
    let logical_size = size.to_logical::<f64>(scale);
    let logical_position = position.to_logical::<f64>(scale);
    let width = logical_size.width.round() as i32;
    let height = logical_size.height.round() as i32;
    if width < 100 || height < 100 {
        return;
    }
    let x = logical_position.x.round() as i32;
    let y = logical_position.y.round() as i32;
    let _ = update_settings(|settings| {
        apply_window_placement(settings, role, x, y, width, height);
    });
}

fn edit_error(error: agsnap::ShotEditError) -> String {
    let id = match error {
        agsnap::ShotEditError::InvalidImage
        | agsnap::ShotEditError::MissingSource
        | agsnap::ShotEditError::OutsideLibrary => TextId::OpenShotFailed,
        agsnap::ShotEditError::NameExhausted
        | agsnap::ShotEditError::WriteFailed => TextId::SaveFailed,
        agsnap::ShotEditError::DeleteFailed => TextId::DeleteFailed,
    };
    text(current_language(), id).to_string()
}

#[tauri::command]
fn set_panel_open(open: bool) -> Result<ShellState, String> {
    update_settings(|settings| {
        settings.panel_open = open;
    })
    .map_err(|error| error.to_string())?;
    Ok(current_shell())
}

fn current_shell() -> ShellState {
    let settings = load_settings();
    let language = language_from_tag(normalize_culture(&settings.language));
    let modifiers = settings.modifiers();
    ShellState {
        language: tag_from_language(language).to_string(),
        version: APP_VERSION.to_string(),
        delete_old: settings.delete_old,
        retention_days: settings.retention_days,
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
        copy_image: text(language, TextId::MenuCopy).to_string(),
        md_copy: text(language, TextId::MenuMdCopy).to_string(),
        md_save: text(language, TextId::MenuMdSave).to_string(),
        magnify_image: text(language, TextId::MenuLens).to_string(),
        open_with: text(language, TextId::OpenWith).to_string(),
        delete_image: text(language, TextId::MenuDelete).to_string(),
        delete_confirm: text(language, TextId::DeleteConfirm).to_string(),
        edit_image: text(language, TextId::MenuEdit).to_string(),
        editor_save: text(language, TextId::EditorSave).to_string(),
        editor_cancel: text(language, TextId::EditorCancel).to_string(),
        editor_select: text(language, TextId::EditorSelect).to_string(),
        editor_rect: text(language, TextId::EditorRect).to_string(),
        editor_ellipse: text(language, TextId::EditorEllipse).to_string(),
        editor_polygon: text(language, TextId::EditorPolygon).to_string(),
        editor_arrow: text(language, TextId::EditorArrow).to_string(),
        editor_text: text(language, TextId::EditorText).to_string(),
        editor_stroke: text(language, TextId::EditorStroke).to_string(),
        editor_fill: text(language, TextId::EditorFill).to_string(),
        editor_width: text(language, TextId::EditorWidth).to_string(),
        editor_head: text(language, TextId::EditorHead).to_string(),
        editor_head_size: text(language, TextId::EditorHeadSize).to_string(),
        editor_font: text(language, TextId::EditorFont).to_string(),
        editor_font_size: text(language, TextId::EditorFontSize).to_string(),
        editor_color: text(language, TextId::EditorColor).to_string(),
        editor_size: text(language, TextId::EditorSize).to_string(),
        editor_hint: text(language, TextId::EditorHint).to_string(),
        editor_unsaved: text(language, TextId::EditorUnsaved).to_string(),
        editor_discard: text(language, TextId::EditorDiscard).to_string(),
        editor_head_triangle: text(language, TextId::EditorHeadTriangle)
            .to_string(),
        editor_head_chevron: text(language, TextId::EditorHeadChevron)
            .to_string(),
        editor_head_barbed: text(language, TextId::EditorHeadBarbed)
            .to_string(),
        theme: normalize_theme(&settings.theme).to_string(),
        theme_label: text(language, TextId::ThemeTitle).to_string(),
        theme_system: text(language, TextId::ThemeSystem).to_string(),
        theme_dark: text(language, TextId::ThemeDark).to_string(),
        theme_light: text(language, TextId::ThemeLight).to_string(),
        theme_flat: text(language, TextId::ThemeFlat).to_string(),
        theme_material: text(language, TextId::ThemeMaterial).to_string(),
        theme_glass: text(language, TextId::ThemeGlass).to_string(),
        theme_skeuo: text(language, TextId::ThemeSkeuo).to_string(),
        theme_neu: text(language, TextId::ThemeNeu).to_string(),
        theme_clay: text(language, TextId::ThemeClay).to_string(),
        theme_brutal: text(language, TextId::ThemeBrutal).to_string(),
        theme_liquid: text(language, TextId::ThemeLiquid).to_string(),
        theme_icon3d: text(language, TextId::ThemeIcon3d).to_string(),
        panel_open: settings.panel_open,
        panel_open_label: text(language, TextId::PanelOpen).to_string(),
        panel_close_label: text(language, TextId::PanelClose).to_string(),
        tilt: settings.tilt,
        tilt_label: text(language, TextId::TiltLabel).to_string(),
        tape_visible: settings.tape_visible,
        tape_label: text(language, TextId::TapeLabel).to_string(),
        tape_width: settings.tape_width,
        tape_width_label: text(language, TextId::TapeWidthLabel).to_string(),
        tape_height: settings.tape_height,
        tape_height_label: text(language, TextId::TapeHeightLabel).to_string(),
        polaroid: settings.polaroid,
        polaroid_label: text(language, TextId::PolaroidLabel).to_string(),
        caption_size: settings.caption_size,
        caption_label: text(language, TextId::CaptionLabel).to_string(),
        peek_one: settings.peek_one,
        peek_two: settings.peek_two,
        peek_one_delay: settings.peek_one_delay,
        peek_two_delay: settings.peek_two_delay,
        peek_one_scale: settings.peek_one_scale,
        peek_two_scale: settings.peek_two_scale,
        peek_one_label: text(language, TextId::PeekOneLabel).to_string(),
        peek_two_label: text(language, TextId::PeekTwoLabel).to_string(),
        peek_delay_label: text(language, TextId::PeekDelayLabel).to_string(),
        peek_scale_label: text(language, TextId::PeekScaleLabel).to_string(),
        peek_scale_max: text(language, TextId::PeekScaleMax).to_string(),
        gallery_zoom: settings.gallery_zoom,
        history_label: text(language, TextId::HistoryButton).to_string(),
        close_label: text(language, TextId::DialogClose).to_string(),
        memo_image: text(language, TextId::MenuMemo).to_string(),
        memo_label: text(language, TextId::MemoLabel).to_string(),
        tags_label: text(language, TextId::TagsLabel).to_string(),
        search_placeholder: text(language, TextId::SearchPlaceholder)
            .to_string(),
    }
}

fn current_language() -> UiLanguage {
    let settings = load_settings();
    language_from_tag(normalize_culture(&settings.language))
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
