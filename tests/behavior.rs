use std::fs;
use std::time::{Duration, SystemTime};

use agsnap::{
    APP_VERSION, BalloonShot, CLEANUP_INTERVAL_MS, CivilTime, ClickSeries,
    DEFAULT_RETENTION_DAYS, HC_ACTION, Modifiers, MouseDecision,
    OWNED_FORMAT_NAMES, Rect, TextId, UiLanguage, VK_MENU, WM_KEY_DOWN,
    WM_KEY_UP, WM_LEFT_BUTTON_DOWN, WM_RIGHT_BUTTON_DOWN, WM_RIGHT_BUTTON_UP,
    WM_SYS_KEY_DOWN, WM_SYS_KEY_UP, WindowHit, choose_bounds, clipboard_path,
    create_shot_path, decide_mouse, delete_older_than, dib_from_top_down,
    first_heading, format_gesture, hdrop_blob, is_day_folder, is_inside,
    list_gallery, normalize_day_text, read_history_text, read_modifiers,
    read_retention, rejects_file_group_descriptor, should_swallow_alt, text,
    write_modifiers, write_retention,
};

const RECTANGLE: i32 = 8;

#[test]
fn element_bounds_follow_the_csharp_rules() {
    let window = Rect::from_xywh(0, 0, 200, 100);
    assert_eq!(choose_bounds(window, None), window);
    let element = Rect::from_xywh(10, 10, 40, 20);
    assert_eq!(choose_bounds(window, Some(element)), element);
    let covered = Rect::from_xywh(0, 0, 100, 100);
    let almost = Rect::from_xywh(0, 0, 95, 95);
    assert_eq!(choose_bounds(covered, Some(almost)), covered);
    let tiny = Rect::from_xywh(1, 1, 8, 8);
    let outside = Rect::from_xywh(-10, 0, 20, 20);
    assert_eq!(choose_bounds(window, Some(tiny)), window);
    assert_eq!(choose_bounds(window, Some(outside)), window);
}

#[test]
fn click_series_waits_then_collapses_a_double() {
    let mut series = ClickSeries::default();
    let first = series.observe_down(10, 20, false, RECTANGLE, RECTANGLE);
    assert!(first.ready.is_none() && first.is_waiting);
    let second = series.observe_down(12, 21, false, RECTANGLE, RECTANGLE);
    assert!(!second.is_waiting);
    assert_eq!(
        second.ready,
        Some(agsnap::ClickChoice {
            x: 10,
            y: 20,
            whole_window: false,
            is_double: true,
        })
    );
}

#[test]
fn quiet_expire_is_a_single_click() {
    let mut series = ClickSeries::default();
    series.observe_down(10, 20, true, RECTANGLE, RECTANGLE);
    let expired = series.expire();
    assert_eq!(
        expired.ready,
        Some(agsnap::ClickChoice {
            x: 10,
            y: 20,
            whole_window: true,
            is_double: false,
        })
    );
    assert!(series.expire().ready.is_none());
}

#[test]
fn second_down_outside_fires_the_first() {
    let mut series = ClickSeries::default();
    series.observe_down(10, 20, false, RECTANGLE, RECTANGLE);
    let observed = series.observe_down(30, 20, false, RECTANGLE, RECTANGLE);
    assert!(observed.is_waiting);
    assert_eq!(observed.ready.unwrap().x, 10);
    assert!(!observed.ready.unwrap().is_double);
    assert_eq!(series.expire().ready.unwrap().x, 30);
}

#[test]
fn other_button_is_not_a_double() {
    let mut series = ClickSeries::default();
    series.observe_down(4, 5, false, RECTANGLE, RECTANGLE);
    let observed = series.observe_down(4, 5, true, RECTANGLE, RECTANGLE);
    assert!(observed.is_waiting);
    assert!(!observed.ready.unwrap().whole_window);
}

#[test]
fn edge_of_the_rectangle_is_outside() {
    assert!(is_inside(0, 0, 3, 3, 8, 8));
    assert!(!is_inside(0, 0, 4, 0, 8, 8));
    assert!(!is_inside(0, 0, 0, 4, 8, 8));
}

#[test]
fn modifiers_round_trip_and_alt_guard() {
    assert_eq!(Modifiers::DEFAULT.format(), "Ctrl+Shift+Alt");
    assert!(Modifiers::DEFAULT.matches(true, true, true));
    assert!(!Modifiers::DEFAULT.matches(true, true, false));
    let empty = Modifiers::parse(" ");
    assert!(!empty.has_any());
    assert!(!empty.matches(false, false, false));
    let directory = std::env::temp_dir().join(format!(
        "agsnap-mod-{}",
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let path = directory.join("modifiers.txt");
    let modifiers = Modifiers {
        control: true,
        shift: false,
        alt: true,
    };
    write_modifiers(&path, modifiers).unwrap();
    let actual = read_modifiers(&path);
    assert_eq!(actual, modifiers);
    assert!(actual.should_suppress_alt(true, false));
    assert!(!actual.should_suppress_alt(false, false));
    let missing = directory.join("missing.txt");
    assert_eq!(read_modifiers(&missing), Modifiers::DEFAULT);
    let _ = fs::remove_dir_all(directory);
    for message in [WM_KEY_DOWN, WM_KEY_UP, WM_SYS_KEY_DOWN, WM_SYS_KEY_UP] {
        assert!(should_swallow_alt(
            message,
            VK_MENU,
            Modifiers::DEFAULT,
            true,
            true
        ));
    }
    assert!(!should_swallow_alt(
        WM_SYS_KEY_DOWN,
        VK_MENU,
        Modifiers::DEFAULT,
        false,
        false
    ));
    assert!(!should_swallow_alt(
        WM_KEY_DOWN,
        0x41,
        Modifiers::DEFAULT,
        true,
        true
    ));
    let alt_only = Modifiers {
        control: false,
        shift: false,
        alt: true,
    };
    assert!(!should_swallow_alt(
        WM_KEY_DOWN,
        VK_MENU,
        alt_only,
        true,
        true
    ));
}

#[test]
fn mouse_hook_decisions_match_the_gate() {
    let down = decide_mouse(
        HC_ACTION,
        WM_LEFT_BUTTON_DOWN,
        true,
        false,
        Some((40000, -20)),
    );
    assert_eq!(
        down,
        MouseDecision::Post {
            message: agsnap::WM_APP_BEGIN_DRAG,
            x: 40000,
            y: -20,
        }
    );
    assert_eq!(
        decide_mouse(HC_ACTION, 0x0200, true, false, Some((1, 2))),
        MouseDecision::Pass
    );
    assert_eq!(
        decide_mouse(
            HC_ACTION,
            WM_LEFT_BUTTON_DOWN,
            false,
            false,
            Some((1, 2))
        ),
        MouseDecision::Pass
    );
    assert_eq!(
        decide_mouse(HC_ACTION, WM_LEFT_BUTTON_DOWN, true, true, Some((1, 2))),
        MouseDecision::Pass
    );
    assert_eq!(
        decide_mouse(
            HC_ACTION,
            WM_RIGHT_BUTTON_DOWN,
            true,
            false,
            Some((40000, -20))
        ),
        MouseDecision::Post {
            message: agsnap::WM_APP_CAPTURE_WINDOW,
            x: 40000,
            y: -20,
        }
    );
    assert_eq!(
        decide_mouse(HC_ACTION, WM_RIGHT_BUTTON_UP, true, false, Some((1, 2))),
        MouseDecision::Swallow
    );
    assert_eq!(
        decide_mouse(HC_ACTION, WM_RIGHT_BUTTON_UP, true, true, Some((1, 2))),
        MouseDecision::Swallow
    );
}

#[test]
fn hdrop_and_dib_layouts() {
    let path = r"C:\shots\Screenshot-20261011-220509.png";
    let actual = hdrop_blob(path);
    assert_eq!(i32::from_le_bytes(actual[0..4].try_into().unwrap()), 20);
    assert_eq!(i32::from_le_bytes(actual[16..20].try_into().unwrap()), 1);
    let text = String::from_utf16_lossy(&bytemuck_u16(&actual[20..]));
    assert!(text.starts_with(&format!("{path}\0\0")));
    let many = agsnap::hdrop_paths(&[r"C:\a.png", r"C:\b.png"]);
    assert_eq!(i32::from_le_bytes(many[16..20].try_into().unwrap()), 1);
    let many_text = String::from_utf16_lossy(&bytemuck_u16(&many[20..]));
    assert!(many_text.starts_with("C:\\a.png\0C:\\b.png\0\0"));
    assert_eq!(
        clipboard_path(std::path::Path::new(r"\\?\C:\shots\a.png")),
        r"C:\shots\a.png"
    );
    assert_eq!(
        clipboard_path(std::path::Path::new(r"\\?\UNC\server\share\a.png")),
        r"\\server\share\a.png"
    );
    let pixels = [
        0u8, 0, 255, 255, 0, 255, 0, 255, 255, 0, 0, 255, 255, 255, 255, 255,
    ];
    let dib = dib_from_top_down(&pixels, 2, 2).unwrap();
    assert_eq!(i32::from_le_bytes(dib[0..4].try_into().unwrap()), 40);
    assert_eq!(i32::from_le_bytes(dib[4..8].try_into().unwrap()), 2);
    assert_eq!(i32::from_le_bytes(dib[8..12].try_into().unwrap()), 2);
    assert_eq!(i16::from_le_bytes(dib[14..16].try_into().unwrap()), 32);
    assert_eq!(dib[40], 255);
    assert_eq!(dib[41], 0);
    assert_eq!(dib[42], 0);
    assert_eq!(dib[48], 0);
    assert_eq!(dib[49], 0);
    assert_eq!(dib[50], 255);
    assert!(rejects_file_group_descriptor("FileGroupDescriptorW"));
    assert_eq!(OWNED_FORMAT_NAMES[1], "PNG");
}

fn bytemuck_u16(bytes: &[u8]) -> Vec<u16> {
    bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect()
}

#[test]
fn shot_names_stay_gregorian() {
    let when = CivilTime {
        year: 2026,
        month: 10,
        day: 11,
        hour: 22,
        minute: 5,
        second: 9,
    };
    let directory = temp_dir("shot");
    let actual = create_shot_path(when, &directory).unwrap();
    assert_eq!(
        actual,
        directory
            .join("2026-10-11")
            .join("Screenshot-20261011-220509.png")
    );
    fs::write(&actual, [1]).unwrap();
    let second = create_shot_path(when, &directory).unwrap();
    assert_eq!(
        second,
        directory
            .join("2026-10-11")
            .join("Screenshot-20261011-220509-2.png")
    );
    assert!(is_day_folder("2026-10-11"));
    assert!(!is_day_folder("2026-02-31"));
    let _ = fs::remove_dir_all(directory);
}

#[test]
fn cleaner_deletes_only_stale_shots() {
    let directory = temp_dir("clean");
    let stale = directory.join("Screenshot-old.png");
    let fresh = directory.join("Screenshot-new.png");
    let other = directory.join("notes.txt");
    fs::write(&stale, [1]).unwrap();
    fs::write(&fresh, [2]).unwrap();
    fs::write(&other, [3]).unwrap();
    let now = SystemTime::now();
    set_age(&stale, now, Duration::from_secs(11 * 60));
    set_age(&fresh, now, Duration::from_secs(60));
    let deleted = delete_older_than(
        now,
        &directory,
        Duration::from_secs(10 * 60),
        (2099, 1, 1),
    );
    assert_eq!(deleted, 1);
    assert!(!stale.exists());
    assert!(fresh.exists());
    assert!(other.exists());
    let _ = fs::remove_dir_all(directory);
}

#[test]
fn cleaner_removes_an_empty_past_day_only() {
    let directory = temp_dir("days");
    let old_day = directory.join("1999-01-01");
    let kept = directory.join("keep");
    fs::create_dir_all(&old_day).unwrap();
    fs::create_dir_all(&kept).unwrap();
    let stale = old_day.join("Screenshot-old.png");
    fs::write(&stale, [1]).unwrap();
    let now = SystemTime::now();
    set_age(&stale, now, Duration::from_secs(8 * 24 * 60 * 60));
    delete_older_than(
        now,
        &directory,
        Duration::from_secs(7 * 24 * 60 * 60),
        (2026, 10, 16),
    );
    assert!(!old_day.exists());
    assert!(kept.exists());
    let today = directory.join("2026-10-16");
    fs::create_dir_all(&today).unwrap();
    delete_older_than(
        now,
        &directory,
        Duration::from_secs(7 * 24 * 60 * 60),
        (2026, 10, 16),
    );
    assert!(today.exists());
    assert_eq!(CLEANUP_INTERVAL_MS, 3_600_000);
    assert_eq!(DEFAULT_RETENTION_DAYS, 7);
    let _ = fs::remove_dir_all(directory);
}

#[test]
fn retention_and_culture_files() {
    let directory = temp_dir("settings");
    let retention = directory.join("retention.txt");
    assert_eq!(read_retention(&retention), DEFAULT_RETENTION_DAYS);
    write_retention(&retention, 14).unwrap();
    assert_eq!(read_retention(&retention), 14);
    assert_eq!(fs::read_to_string(&retention).unwrap(), "14");
    assert_eq!(normalize_day_text(""), 7);
    assert_eq!(normalize_day_text("soon"), 7);
    assert_eq!(normalize_day_text("0"), 7);
    assert_eq!(normalize_day_text("-3"), 7);
    assert_eq!(normalize_day_text("99999"), 3650);
    assert_eq!(normalize_day_text(" 30 "), 30);
    let culture = directory.join("culture.txt");
    assert_eq!(agsnap::strings::normalize_culture(""), "");
    fs::create_dir_all(&directory).unwrap();
    fs::write(&culture, "ko").unwrap();
    assert_eq!(
        agsnap::strings::normalize_culture(
            &fs::read_to_string(&culture).unwrap()
        ),
        "ko"
    );
    assert_eq!(agsnap::strings::normalize_culture("en-US"), "en");
    assert_eq!(agsnap::strings::normalize_culture("fr"), "");
    let _ = fs::remove_dir_all(directory);
}

#[test]
fn strings_fall_back_to_english() {
    assert_eq!(
        text(UiLanguage::Korean, TextId::MenuOpenFolder),
        "캡처 폴더"
    );
    assert_eq!(
        text(UiLanguage::English, TextId::MenuOpenFolder),
        "Capture folder"
    );
    assert_eq!(text(UiLanguage::Korean, TextId::MenuGallery), "스냅 갤러리");
}

#[test]
fn balloon_forgets_a_plain_notice() {
    let directory = temp_dir("balloon");
    let path = directory.join("shot.png");
    fs::create_dir_all(&directory).unwrap();
    fs::write(&path, [1]).unwrap();
    let mut shot = BalloonShot::default();
    shot.remember(Some(path.clone()));
    assert_eq!(shot.path_to_open(), Some(path.as_path()));
    shot.remember(None);
    assert!(shot.path_to_open().is_none());
    shot.remember(Some(directory.join("missing.png")));
    assert!(shot.path_to_open().is_none());
    let _ = fs::remove_dir_all(directory);
}

#[test]
fn gesture_report_stays_on_one_line() {
    assert_eq!(format_gesture(3, 4, None), "gesture miss point=3,4");
    let hit = WindowHit {
        handle: 0x2A,
        bounds: Rect::from_xywh(1, 2, 3, 4),
        class_name: "Button".to_string(),
        text: "A\r\nB".to_string(),
    };
    let actual = format_gesture(8, 9, Some(&hit));
    assert!(!actual.contains('\n'));
    assert!(actual.contains("hwnd=0x2A"));
    assert!(actual.contains("class=Button"));
    assert!(actual.contains("text=A B"));
    assert!(actual.contains("rect=1,2,3,4"));
    assert!(actual.contains("point=8,9"));
    let long = WindowHit {
        handle: 1,
        bounds: Rect::from_xywh(0, 0, 1, 1),
        class_name: "Static".to_string(),
        text: "가".repeat(50),
    };
    let truncated = format_gesture(0, 0, Some(&long));
    assert!(truncated.contains(&format!("text={}", "가".repeat(40))));
    assert!(!truncated.contains(&"가".repeat(41)));
}

#[test]
fn changelog_matches_the_package_version() {
    let executable = std::env::current_exe().unwrap();
    let path = executable.parent().unwrap().join("CHANGELOG.md");
    assert!(path.is_file(), "missing {}", path.display());
    let history = read_history_text(&path, "missing");
    assert_eq!(first_heading(&history), format!("## {APP_VERSION}"));
    assert!(!APP_VERSION.contains('+'));
}

#[test]
fn gallery_lists_newest_shots_and_days() {
    let directory = temp_dir("gallery");
    let older = directory.join("2026-10-01");
    let newer = directory.join("2026-10-16");
    fs::create_dir_all(&older).unwrap();
    fs::create_dir_all(&newer).unwrap();
    let old_shot = older.join("Screenshot-old.png");
    let new_shot = newer.join("Screenshot-new.png");
    let note = newer.join("notes.txt");
    fs::write(&old_shot, [1]).unwrap();
    fs::write(&new_shot, [2]).unwrap();
    fs::write(&note, [3]).unwrap();
    let now = SystemTime::now();
    set_age(&old_shot, now, Duration::from_secs(3600));
    set_age(&new_shot, now, Duration::from_secs(60));
    let gallery = list_gallery(&directory);
    assert_eq!(
        gallery.days,
        vec!["2026-10-16".to_string(), "2026-10-01".to_string()]
    );
    assert_eq!(gallery.shots.len(), 2);
    assert!(gallery.shots[0].path.ends_with("Screenshot-new.png"));
    assert_eq!(gallery.shots[0].day, "2026-10-16");
    let _ = fs::remove_dir_all(directory);
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "agsnap-{name}-{}",
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    directory
}

fn set_age(path: &std::path::Path, now: SystemTime, age: Duration) {
    let written = now.checked_sub(age).unwrap();
    let file = fs::File::options().write(true).open(path).unwrap();
    file.set_modified(written).unwrap();
}
