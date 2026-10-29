use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{
    AtomicBool, AtomicI32, AtomicIsize, AtomicU8, Ordering,
};
use std::sync::mpsc;

use windows::Win32::Foundation::{
    COLORREF, CloseHandle, ERROR_ALREADY_EXISTS, GENERIC_READ, HANDLE, HWND,
    LPARAM, LRESULT, POINT, RECT, WPARAM,
};
use windows::Win32::Graphics::Dwm::{
    DWMWA_EXTENDED_FRAME_BOUNDS, DwmFlush, DwmGetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::{
    BITMAPINFO, BITMAPINFOHEADER, BeginPaint, BitBlt, CAPTUREBLT, CombineRgn,
    CreateBitmap, CreateCompatibleDC, CreateDIBSection, CreateRectRgn,
    CreateSolidBrush, DIB_RGB_COLORS, DeleteDC, DeleteObject, EndPaint,
    FillRect, GetDC, HBITMAP, HDC, PAINTSTRUCT, RGN_DIFF, ReleaseDC, SRCCOPY,
    SelectObject, SetWindowRgn,
};
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_ContainerFormatPng,
    GUID_WICPixelFormat32bppBGRA, GUID_WICPixelFormat32bppPBGRA,
    IWICBitmapEncoder, IWICBitmapFrameEncode, IWICFormatConverter,
    IWICImagingFactory, IWICStream, WICBitmapDitherTypeNone,
    WICBitmapEncoderNoCache, WICBitmapPaletteTypeCustom,
    WICDecodeMetadataCacheOnDemand,
};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, CoCreateInstance, STGC_DEFAULT,
};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW,
    SetClipboardData,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{
    GMEM_MOVEABLE, GMEM_ZEROINIT, GlobalAlloc, GlobalLock, GlobalUnlock,
};
use windows::Win32::System::Ole::{CF_DIB, CF_HDROP, CF_UNICODETEXT};
use windows::Win32::System::Ole::{
    OleFlushClipboard, OleInitialize as StartOle,
};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement,
};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetDoubleClickTime, VIRTUAL_KEY, VK_CONTROL, VK_MENU,
    VK_SHIFT,
};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_INFO, NIM_ADD, NIM_DELETE,
    NIM_MODIFY, NIM_SETVERSION, NIN_BALLOONUSERCLICK, NOTIFYICON_VERSION,
    NOTIFYICONDATAW, Shell_NotifyIconW, ShellExecuteW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CWP_SKIPINVISIBLE, CWP_SKIPTRANSPARENT, CallNextHookEx,
    ChildWindowFromPointEx, CreateIconIndirect, CreatePopupMenu,
    CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, FindWindowW, GA_ROOT, GetAncestor, GetClassNameW,
    GetClientRect, GetCursorPos, GetMessageW, GetSystemMetrics, GetWindowRect,
    GetWindowTextW, HHOOK, HICON, HMENU, HWND_TOPMOST, ICONINFO, IDC_CROSS,
    IDI_APPLICATION, KBDLLHOOKSTRUCT, KillTimer, LWA_ALPHA, LoadCursorW,
    MB_ICONERROR, MB_ICONQUESTION, MB_OK, MB_YESNOCANCEL, MF_CHECKED, MF_POPUP,
    MF_SEPARATOR, MF_STRING, MF_UNCHECKED, MSLLHOOKSTRUCT, MessageBoxW,
    PostMessageW, PostQuitMessage, RegisterClassW, SM_CXDOUBLECLK, SM_CXDRAG,
    SM_CXVIRTUALSCREEN, SM_CYDOUBLECLK, SM_CYDRAG, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_HIDE, SW_SHOWNOACTIVATE,
    SW_SHOWNORMAL, SWP_NOACTIVATE, SWP_SHOWWINDOW,
    SetForegroundWindow, SetLayeredWindowAttributes, SetTimer, SetWindowPos,
    SetWindowsHookExW, ShowWindow, TPM_RIGHTBUTTON, TrackPopupMenu,
    TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL, WH_MOUSE_LL,
    WINDOW_EX_STYLE, WM_COMMAND, WM_CONTEXTMENU, WM_DESTROY, WM_ERASEBKGND,
    WM_PAINT, WM_RBUTTONUP, WM_TIMER, WNDCLASSW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
    WS_POPUP, WindowFromPoint,
};
use windows::core::BOOL;
use windows::core::{PCWSTR, w};

use crate::logic::{
    APP_NAME, APP_VERSION, BalloonShot, CLEANUP_INTERVAL_MS, CivilTime,
    ClickSeries, DEFAULT_RETENTION_DAYS, INSTANCE_MUTEX_NAME,
    MESSAGE_WINDOW_CLASS, Modifiers, MouseDecision, WM_APP_BEGIN_DRAG,
    WM_APP_CAPTURE_WINDOW, WM_APP_REGION_CANCEL, WM_APP_REGION_MOVE,
    WM_APP_REGION_UP, WM_APP_SHOW_GALLERY, WindowHit, clipboard_path,
    decide_mouse, delete_older_than, dib_from_top_down, format_gesture,
    hdrop_blob, hdrop_paths, is_drag_threshold, load_settings,
    prepare_shots_directory, rect_from_points, save_settings, settings_dir,
    shots_directory,
};
use crate::rect::Rect;
use crate::strings::{
    TextId, UiLanguage, format_hint, format_hook_failure, format_retention,
    language_from_tag, normalize_culture, tag_from_language, text,
};

const TRAY_MESSAGE: u32 = 0x8003;
const TIMER_CLICK: usize = 1;
const REGION_CLASS: PCWSTR = w!("AgSnapRegion");
const OVERLAY_ALPHA: u8 = 96;
const FRAME_COLOR: u32 = 0x00_FF_C8_00;
const FRAME_THICK: i32 = 2;
const TIMER_CLEAN: usize = 2;
const TRAY_ID: u32 = 1;
const CLASS_NAME: PCWSTR = w!("AgSnapMessage");
const MAX_EDGE: i32 = 16000;
const OPAQUE_ALPHA: u8 = 255;
const GENERIC_READ_WRITE: u32 = 0xC000_0000;
const CHANGED_MODE: i32 = 0x8001_0106u32 as i32;
const ICON_SIZE: i32 = 32;
const ID_FOLDER: usize = 100;
const ID_RETENTION: usize = 101;
const ID_LOG: usize = 102;
const ID_VERSION: usize = 103;
const ID_EXIT: usize = 104;
const ID_CTRL: usize = 110;
const ID_SHIFT: usize = 111;
const ID_ALT: usize = 112;
const ID_LANG_SYSTEM: usize = 120;
const ID_LANG_KO: usize = 121;
const ID_LANG_EN: usize = 122;
const BIT_CONTROL: u8 = 1;
const BIT_SHIFT: u8 = 2;
const BIT_ALT: u8 = 4;

static WINDOW: AtomicIsize = AtomicIsize::new(0);
static MOUSE_HOOK: AtomicIsize = AtomicIsize::new(0);
static KEY_HOOK: AtomicIsize = AtomicIsize::new(0);
static BUSY: AtomicI32 = AtomicI32::new(0);
static REQUIRED: AtomicU8 = AtomicU8::new(BIT_CONTROL | BIT_SHIFT | BIT_ALT);
static LANGUAGE: AtomicU8 = AtomicU8::new(0);
static DAYS: AtomicI32 = AtomicI32::new(DEFAULT_RETENTION_DAYS);
static DELETE_OLD: AtomicBool = AtomicBool::new(true);
static INSTANCE_MUTEX: AtomicIsize = AtomicIsize::new(0);
static SESSION: Mutex<Option<Session>> = Mutex::new(None);
static USE_TAURI_UI: AtomicBool = AtomicBool::new(false);
static LEFT_GESTURE: AtomicBool = AtomicBool::new(false);
static REGION_MODE: AtomicBool = AtomicBool::new(false);
static NOTICE_TX: Mutex<Option<mpsc::Sender<CaptureNotice>>> = Mutex::new(None);

pub struct CaptureNotice {
    pub text: String,
    pub path: Option<String>,
    pub open_lens: bool,
    pub show_gallery: bool,
}

struct Session {
    series: ClickSeries,
    balloon: BalloonShot,
    log_path: std::path::PathBuf,
    version_window: isize,
    region: Option<RegionPick>,
}

#[derive(Clone, Copy)]
struct RegionPick {
    origin_x: i32,
    origin_y: i32,
    current_x: i32,
    current_y: i32,
    overlay: isize,
    dimmer: isize,
}

pub fn prepare_process() {
    let _ = enable_per_monitor_v2();
    let _ = start_ole();
}

/// False when another AgSnapTool process already owns the session.
pub fn claim_single_instance() -> bool {
    let name = wide(INSTANCE_MUTEX_NAME);
    let created = unsafe { CreateMutexW(None, false, PCWSTR(name.as_ptr())) };
    let Ok(handle) = created else {
        return true;
    };
    if last_error() == ERROR_ALREADY_EXISTS.0 as i32 {
        let _ = unsafe { CloseHandle(handle) };
        ask_running_instance_to_show();
        return false;
    }
    INSTANCE_MUTEX.store(handle.0 as isize, Ordering::Relaxed);
    true
}

fn ask_running_instance_to_show() {
    let class = wide(MESSAGE_WINDOW_CLASS);
    let Ok(hwnd) = (unsafe { FindWindowW(PCWSTR(class.as_ptr()), None) })
    else {
        return;
    };
    let _ = unsafe {
        PostMessageW(Some(hwnd), WM_APP_SHOW_GALLERY, WPARAM(0), LPARAM(0))
    };
}

pub fn set_notice_sender(sender: mpsc::Sender<CaptureNotice>) {
    *NOTICE_TX.lock().expect("notice") = Some(sender);
    USE_TAURI_UI.store(true, Ordering::Relaxed);
}

pub fn spawn_capture() {
    std::thread::Builder::new()
        .name("agsnap-capture".into())
        .spawn(|| {
            let code = run();
            if code != 0 {
                eprintln!("capture stopped {code}");
            }
        })
        .expect("capture thread");
}

/// Shows a Yes / No / Cancel prompt. Returns the raw Win32 result.
pub fn ask_yes_no_cancel(
    owner: Option<isize>,
    title: &str,
    message: &str,
) -> i32 {
    let body = wide(message);
    let caption = wide(title);
    let hwnd = owner.map(|value| HWND(value as *mut _));
    unsafe {
        MessageBoxW(
            hwnd,
            PCWSTR(body.as_ptr()),
            PCWSTR(caption.as_ptr()),
            MB_YESNOCANCEL | MB_ICONQUESTION,
        )
        .0
    }
}

pub fn open_document(path: &Path) {
    open_path(path);
}

pub fn apply_runtime_settings() {
    let settings = load_settings();
    store_modifiers(settings.modifiers());
    DAYS.store(settings.retention_days, Ordering::Relaxed);
    DELETE_OLD.store(settings.delete_old, Ordering::Relaxed);
    store_language(load_language());
}

pub fn run() -> i32 {
    let language = load_language();
    store_language(language);
    if let Err(error) = enable_per_monitor_v2() {
        show_error(&format_hook_failure(current_language(), error));
        return error;
    }
    if let Err(error) = start_ole() {
        show_error(&format_hook_failure(current_language(), error));
        return error;
    }
    let settings = load_settings();
    store_modifiers(settings.modifiers());
    DAYS.store(settings.retention_days, Ordering::Relaxed);
    DELETE_OLD.store(settings.delete_old, Ordering::Relaxed);
    let _ = delete_stale();
    let hwnd = match create_message_window() {
        Ok(hwnd) => hwnd,
        Err(error) => {
            show_error(&format_hook_failure(current_language(), error));
            return error;
        }
    };
    WINDOW.store(hwnd.0 as isize, Ordering::Relaxed);
    {
        let mut session = SESSION.lock().expect("session");
        *session = Some(Session {
            series: ClickSeries::default(),
            balloon: BalloonShot::default(),
            log_path: settings_dir().join("gesture.log"),
            version_window: 0,
            region: None,
        });
    }
    if let Some(parent) = settings_dir().parent() {
        let _ = std::fs::create_dir_all(settings_dir());
        let _ = parent;
    }
    let _ = std::fs::create_dir_all(settings_dir());
    unsafe {
        let _ = SetTimer(Some(hwnd), TIMER_CLEAN, CLEANUP_INTERVAL_MS, None);
    }
    if let Err(error) = install_hooks() {
        destroy_session(hwnd);
        show_error(&format_hook_failure(current_language(), error));
        return error;
    }
    if !USE_TAURI_UI.load(Ordering::Relaxed)
        && let Err(error) = add_tray(hwnd)
    {
        remove_hooks();
        destroy_session(hwnd);
        show_error(&format_hook_failure(current_language(), error));
        return error;
    }
    pump();
    remove_tray(hwnd);
    remove_hooks();
    destroy_session(hwnd);
    0
}

fn pump() {
    unsafe {
        let mut message =
            windows::Win32::UI::WindowsAndMessaging::MSG::default();
        loop {
            let status = GetMessageW(&mut message, None, 0, 0);
            if status.0 == 0 || status.0 == -1 {
                break;
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

fn enable_per_monitor_v2() -> Result<(), i32> {
    const ACCESS_DENIED: i32 = 0x8007_0005u32 as i32;
    unsafe {
        match SetProcessDpiAwarenessContext(
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        ) {
            Ok(()) => Ok(()),
            Err(error) if error.code().0 == ACCESS_DENIED => Ok(()),
            Err(error) => Err(error.code().0),
        }
    }
}

fn start_ole() -> Result<(), i32> {
    unsafe {
        match StartOle(None) {
            Ok(()) => Ok(()),
            Err(error) if error.code().0 == CHANGED_MODE => Ok(()),
            Err(error) => Err(error.code().0),
        }
    }
}

fn create_message_window() -> Result<HWND, i32> {
    unsafe {
        let instance =
            GetModuleHandleW(None).map_err(|error| error.code().0)?;
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance.into(),
            lpszClassName: CLASS_NAME,
            hCursor: LoadCursorW(None, IDI_APPLICATION).unwrap_or_default(),
            ..Default::default()
        };
        RegisterClassW(&class);
        let hwnd = CreateWindowExW(
            WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
            CLASS_NAME,
            PCWSTR(wide(APP_NAME).as_ptr()),
            WS_POPUP,
            -32000,
            -32000,
            1,
            1,
            None,
            None,
            Some(instance.into()),
            None,
        )
        .map_err(|error| error.code().0)?;
        Ok(hwnd)
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_APP_SHOW_GALLERY => {
            publish_show_gallery();
            LRESULT(0)
        }
        WM_APP_BEGIN_DRAG | WM_APP_CAPTURE_WINDOW => {
            note_down(
                wparam.0 as i32,
                lparam.0 as i32,
                message == WM_APP_CAPTURE_WINDOW,
            );
            LRESULT(0)
        }
        value if value == WM_APP_REGION_MOVE => {
            on_region_move(wparam.0 as i32, lparam.0 as i32);
            LRESULT(0)
        }
        value if value == WM_APP_REGION_UP => {
            on_region_up(wparam.0 as i32, lparam.0 as i32);
            LRESULT(0)
        }
        value if value == WM_APP_REGION_CANCEL => {
            cancel_region();
            LRESULT(0)
        }
        WM_TIMER => on_timer(hwnd, wparam.0),
        value if value == TRAY_MESSAGE => on_tray(hwnd, lparam.0 as u32),
        WM_COMMAND => {
            on_command(hwnd, (wparam.0 & 0xffff) as usize);
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn on_timer(hwnd: HWND, id: usize) -> LRESULT {
    if id == TIMER_CLICK {
        unsafe {
            let _ = KillTimer(Some(hwnd), TIMER_CLICK);
        }
        let choice = {
            let mut guard = SESSION.lock().expect("session");
            guard.as_mut().map(|session| session.series.expire().ready)
        };
        if let Some(Some(choice)) = choice {
            handle_gesture(choice);
        }
    } else if id == TIMER_CLEAN {
        let _ = delete_stale();
    }
    LRESULT(0)
}

fn note_down(x: i32, y: i32, whole_window: bool) {
    let (width, height, time) = double_click_metrics();
    let observed = {
        let mut guard = SESSION.lock().expect("session");
        let Some(session) = guard.as_mut() else {
            return;
        };
        session
            .series
            .observe_down(x, y, whole_window, width, height)
    };
    let hwnd = current_window();
    unsafe {
        let _ = KillTimer(Some(hwnd), TIMER_CLICK);
        if observed.is_waiting {
            let _ = SetTimer(Some(hwnd), TIMER_CLICK, time, None);
        }
    }
    if let Some(choice) = observed.ready {
        handle_gesture(choice);
    }
}

fn double_click_metrics() -> (i32, i32, u32) {
    unsafe {
        let width = GetSystemMetrics(SM_CXDOUBLECLK);
        let height = GetSystemMetrics(SM_CYDOUBLECLK);
        let mut time = GetDoubleClickTime();
        if time < 1 {
            time = 1;
        }
        (width, height, time)
    }
}

fn handle_gesture(choice: crate::logic::ClickChoice) {
    if REGION_MODE.load(Ordering::Relaxed) {
        return;
    }
    LEFT_GESTURE.store(false, Ordering::Relaxed);
    BUSY.fetch_add(1, Ordering::Relaxed);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        write_hit(choice);
    }));
    if result.is_err() {
        eprintln!("gesture panicked");
    }
    BUSY.fetch_sub(1, Ordering::Relaxed);
}

fn write_hit(choice: crate::logic::ClickChoice) {
    let Some(hit) = locate_window(choice.x, choice.y) else {
        write_log(&format_gesture(choice.x, choice.y, None));
        return;
    };
    let target = if choice.whole_window {
        root_hit(hit)
    } else {
        hit
    };
    let Some(shot) =
        save_shot(&target, choice.x, choice.y, choice.whole_window)
    else {
        write_log(&format_gesture(choice.x, choice.y, Some(&target)));
        return;
    };
    let copied = copy_shot(&shot);
    let mut captured = target;
    captured.bounds = shot.bounds;
    let scope = if choice.whole_window {
        "window"
    } else {
        "control"
    };
    write_log(&format!(
        "{} scope={scope} action=clipboard file={}",
        format_gesture(choice.x, choice.y, Some(&captured)),
        shot.path.display()
    ));
    if !copied {
        return;
    }
    if choice.is_double {
        if !publish_lens(&shot.path) {
            open_path(&shot.path);
        }
        return;
    }
    show_shot(text(current_language(), TextId::CopiedNotice), &shot.path);
}

struct SavedShot {
    path: std::path::PathBuf,
    width: i32,
    height: i32,
    pixels: Vec<u8>,
    bounds: Rect,
}

fn save_shot(
    hit: &WindowHit,
    x: i32,
    y: i32,
    whole_window: bool,
) -> Option<SavedShot> {
    let window = bounds_for_window(HWND(hit.handle as *mut _));
    let bounds = if whole_window {
        window
    } else {
        crate::logic::choose_bounds(window, read_element(x, y))
    };
    if bounds.width() <= 0 || bounds.height() <= 0 {
        return None;
    }
    let path =
        crate::logic::create_shot_path(local_now(), &prepare_shots_directory())
            .ok()?;
    let pixels = capture_png(&bounds, &path).ok()?;
    Some(SavedShot {
        path,
        width: bounds.width(),
        height: bounds.height(),
        pixels,
        bounds,
    })
}

fn on_region_move(x: i32, y: i32) {
    if REGION_MODE.load(Ordering::Relaxed) {
        update_region_point(x, y);
        return;
    }
    if !LEFT_GESTURE.load(Ordering::Relaxed) {
        return;
    }
    let origin = {
        let guard = SESSION.lock().expect("session");
        guard
            .as_ref()
            .and_then(|session| session.series.pending_origin())
    };
    let Some((origin_x, origin_y)) = origin else {
        return;
    };
    let (drag_width, drag_height) = drag_metrics();
    if !is_drag_threshold(origin_x, origin_y, x, y, drag_width, drag_height) {
        return;
    }
    begin_region(origin_x, origin_y, x, y);
}

fn begin_region(origin_x: i32, origin_y: i32, x: i32, y: i32) {
    discard_pending_click();
    {
        let mut guard = SESSION.lock().expect("session");
        if let Some(session) = guard.as_mut() {
            session.region = Some(RegionPick {
                origin_x,
                origin_y,
                current_x: x,
                current_y: y,
                overlay: 0,
                dimmer: 0,
            });
        }
    }
    REGION_MODE.store(true, Ordering::Relaxed);
    let Ok(dimmer) = create_dimmer() else {
        cancel_region();
        return;
    };
    let Ok(overlay) = create_frame() else {
        hide_and_destroy_overlay(dimmer.0 as isize);
        cancel_region();
        return;
    };
    let pick = {
        let mut guard = SESSION.lock().expect("session");
        let Some(session) = guard.as_mut() else {
            hide_and_destroy_overlay(overlay.0 as isize);
            hide_and_destroy_overlay(dimmer.0 as isize);
            clear_region_state();
            return;
        };
        let Some(region) = session.region.as_mut() else {
            hide_and_destroy_overlay(overlay.0 as isize);
            hide_and_destroy_overlay(dimmer.0 as isize);
            clear_region_state();
            return;
        };
        region.overlay = overlay.0 as isize;
        region.dimmer = dimmer.0 as isize;
        *region
    };
    sync_frame_window(pick);
}

fn update_region_point(x: i32, y: i32) {
    let pick = {
        let mut guard = SESSION.lock().expect("session");
        let Some(session) = guard.as_mut() else {
            return;
        };
        let Some(region) = session.region.as_mut() else {
            return;
        };
        region.current_x = x;
        region.current_y = y;
        *region
    };
    sync_frame_window(pick);
}

fn on_region_up(x: i32, y: i32) {
    if REGION_MODE.load(Ordering::Relaxed) {
        update_region_point(x, y);
        finish_region();
        return;
    }
    LEFT_GESTURE.store(false, Ordering::Relaxed);
}

fn finish_region() {
    let pick = take_region();
    clear_region_state();
    let Some(pick) = pick else {
        return;
    };
    hide_and_destroy_overlay(pick.overlay);
    hide_and_destroy_overlay(pick.dimmer);
    let bounds = rect_from_points(
        pick.origin_x,
        pick.origin_y,
        pick.current_x,
        pick.current_y,
    );
    if bounds.width() <= 0 || bounds.height() <= 0 {
        return;
    }
    BUSY.fetch_add(1, Ordering::Relaxed);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        write_region(bounds);
    }));
    if result.is_err() {
        eprintln!("region panicked");
    }
    BUSY.fetch_sub(1, Ordering::Relaxed);
}

fn write_region(bounds: Rect) {
    let Some(shot) = save_region(bounds) else {
        write_log(&format!(
            "gesture region miss rect={},{},{},{}",
            bounds.left,
            bounds.top,
            bounds.width(),
            bounds.height()
        ));
        return;
    };
    let copied = copy_shot(&shot);
    write_log(&format!(
        "gesture region rect={},{},{},{} action=clipboard file={}",
        bounds.left,
        bounds.top,
        bounds.width(),
        bounds.height(),
        shot.path.display()
    ));
    if copied {
        show_shot(text(current_language(), TextId::CopiedNotice), &shot.path);
    }
}

fn save_region(bounds: Rect) -> Option<SavedShot> {
    if bounds.width() <= 0 || bounds.height() <= 0 {
        return None;
    }
    let path =
        crate::logic::create_shot_path(local_now(), &prepare_shots_directory())
            .ok()?;
    let pixels = capture_png(&bounds, &path).ok()?;
    Some(SavedShot {
        path,
        width: bounds.width(),
        height: bounds.height(),
        pixels,
        bounds,
    })
}

fn cancel_region() {
    discard_pending_click();
    let pick = take_region();
    clear_region_state();
    if let Some(pick) = pick {
        hide_and_destroy_overlay(pick.overlay);
        hide_and_destroy_overlay(pick.dimmer);
    }
}

fn take_region() -> Option<RegionPick> {
    SESSION
        .lock()
        .expect("session")
        .as_mut()
        .and_then(|session| session.region.take())
}

fn clear_region_state() {
    LEFT_GESTURE.store(false, Ordering::Relaxed);
    REGION_MODE.store(false, Ordering::Relaxed);
}

fn discard_pending_click() {
    let hwnd = current_window();
    unsafe {
        let _ = KillTimer(Some(hwnd), TIMER_CLICK);
    }
    if let Some(session) = SESSION.lock().expect("session").as_mut() {
        session.series.discard();
    }
}

fn drag_metrics() -> (i32, i32) {
    unsafe { (GetSystemMetrics(SM_CXDRAG), GetSystemMetrics(SM_CYDRAG)) }
}

fn virtual_screen() -> Rect {
    unsafe {
        Rect::from_xywh(
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    }
}

fn create_dimmer() -> Result<HWND, i32> {
    let screen = virtual_screen();
    if screen.is_empty() {
        return Err(-1);
    }
    let hwnd = create_region_window(
        WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        screen,
    )?;
    unsafe {
        let _ = SetLayeredWindowAttributes(
            hwnd,
            COLORREF(0),
            OVERLAY_ALPHA,
            LWA_ALPHA,
        );
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
    Ok(hwnd)
}

fn create_frame() -> Result<HWND, i32> {
    create_region_window(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT,
        Rect::from_xywh(0, 0, 1, 1),
    )
}

fn create_region_window(
    ex_style: WINDOW_EX_STYLE,
    bounds: Rect,
) -> Result<HWND, i32> {
    register_region_class()?;
    unsafe {
        let instance =
            GetModuleHandleW(None).map_err(|error| error.code().0)?;
        CreateWindowExW(
            ex_style,
            REGION_CLASS,
            PCWSTR(wide(APP_NAME).as_ptr()),
            WS_POPUP,
            bounds.left,
            bounds.top,
            bounds.width().max(1),
            bounds.height().max(1),
            None,
            None,
            Some(instance.into()),
            None,
        )
        .map_err(|error| error.code().0)
    }
}

fn register_region_class() -> Result<(), i32> {
    unsafe {
        let instance =
            GetModuleHandleW(None).map_err(|error| error.code().0)?;
        let class = WNDCLASSW {
            lpfnWndProc: Some(overlay_proc),
            hInstance: instance.into(),
            lpszClassName: REGION_CLASS,
            hCursor: LoadCursorW(None, IDC_CROSS).unwrap_or_default(),
            ..Default::default()
        };
        RegisterClassW(&class);
        Ok(())
    }
}

unsafe extern "system" fn overlay_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_PAINT => {
            paint_overlay(hwnd);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn paint_overlay(hwnd: HWND) {
    unsafe {
        let mut paint = PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut paint);
        fill_host(hdc, hwnd);
        let _ = EndPaint(hwnd, &paint);
    }
}

fn fill_host(hdc: HDC, hwnd: HWND) {
    let is_frame = SESSION
        .lock()
        .expect("session")
        .as_ref()
        .and_then(|session| session.region)
        .is_some_and(|pick| pick.overlay == hwnd.0 as isize);
    let color = if is_frame { FRAME_COLOR } else { 0 };
    unsafe {
        let mut client = RECT::default();
        let _ = GetClientRect(hwnd, &mut client);
        let brush = CreateSolidBrush(COLORREF(color));
        let _ = FillRect(hdc, &client, brush);
        let _ = DeleteObject(brush.into());
    }
}

fn sync_frame_window(pick: RegionPick) {
    if pick.overlay == 0 {
        return;
    }
    let selected = rect_from_points(
        pick.origin_x,
        pick.origin_y,
        pick.current_x,
        pick.current_y,
    );
    let hwnd = HWND(pick.overlay as *mut _);
    let width = selected.width().max(1);
    let height = selected.height().max(1);
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            selected.left,
            selected.top,
            width,
            height,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
    }
    apply_frame_shape(hwnd, width, height);
}

fn apply_frame_shape(hwnd: HWND, width: i32, height: i32) {
    unsafe {
        let outer = CreateRectRgn(0, 0, width, height);
        if width > FRAME_THICK * 2 && height > FRAME_THICK * 2 {
            let inner = CreateRectRgn(
                FRAME_THICK,
                FRAME_THICK,
                width - FRAME_THICK,
                height - FRAME_THICK,
            );
            let _ = CombineRgn(Some(outer), Some(outer), Some(inner), RGN_DIFF);
            let _ = DeleteObject(inner.into());
        }
        let _ = SetWindowRgn(hwnd, Some(outer), true);
    }
}

fn hide_and_destroy_overlay(handle: isize) {
    if handle == 0 {
        return;
    }
    let hwnd = HWND(handle as *mut _);
    unsafe {
        let _ = ShowWindow(hwnd, SW_HIDE);
        let _ = DestroyWindow(hwnd);
    }
}

fn delete_stale() -> usize {
    if !DELETE_OLD.load(Ordering::Relaxed) {
        return 0;
    }
    let days = DAYS.load(Ordering::Relaxed).max(1) as u64;
    delete_older_than(
        std::time::SystemTime::now(),
        &prepare_shots_directory(),
        std::time::Duration::from_secs(days * 24 * 60 * 60),
        local_today(),
    )
}

fn current_window() -> HWND {
    HWND(WINDOW.load(Ordering::Relaxed) as *mut _)
}

fn current_language() -> UiLanguage {
    match LANGUAGE.load(Ordering::Relaxed) {
        1 => UiLanguage::Korean,
        2 => UiLanguage::English,
        _ => UiLanguage::System,
    }
}

fn store_language(language: UiLanguage) {
    let value = match language {
        UiLanguage::System => 0,
        UiLanguage::Korean => 1,
        UiLanguage::English => 2,
    };
    LANGUAGE.store(value, Ordering::Relaxed);
}

fn store_modifiers(modifiers: Modifiers) {
    let mut bits = 0u8;
    if modifiers.control {
        bits |= BIT_CONTROL;
    }
    if modifiers.shift {
        bits |= BIT_SHIFT;
    }
    if modifiers.alt {
        bits |= BIT_ALT;
    }
    REQUIRED.store(bits, Ordering::Relaxed);
}

fn load_modifiers() -> Modifiers {
    let bits = REQUIRED.load(Ordering::Relaxed);
    Modifiers {
        control: bits & BIT_CONTROL != 0,
        shift: bits & BIT_SHIFT != 0,
        alt: bits & BIT_ALT != 0,
    }
}

fn modifiers_down() -> bool {
    load_modifiers().matches(
        key_down(VK_CONTROL),
        key_down(VK_SHIFT),
        key_down(VK_MENU),
    )
}

fn key_down(key: VIRTUAL_KEY) -> bool {
    unsafe { GetAsyncKeyState(i32::from(key.0)) < 0 }
}

fn install_hooks() -> Result<(), i32> {
    unsafe {
        let mouse = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), None, 0)
            .map_err(|error| error.code().0)?;
        MOUSE_HOOK.store(mouse.0 as isize, Ordering::Relaxed);
        let keyboard =
            SetWindowsHookExW(WH_KEYBOARD_LL, Some(key_proc), None, 0)
                .map_err(|error| error.code().0)?;
        KEY_HOOK.store(keyboard.0 as isize, Ordering::Relaxed);
    }
    Ok(())
}

fn remove_hooks() {
    unsafe {
        let mouse = MOUSE_HOOK.swap(0, Ordering::Relaxed);
        if mouse != 0 {
            let _ = UnhookWindowsHookEx(HHOOK(mouse as *mut _));
        }
        let keyboard = KEY_HOOK.swap(0, Ordering::Relaxed);
        if keyboard != 0 {
            let _ = UnhookWindowsHookEx(HHOOK(keyboard as *mut _));
        }
    }
}

unsafe extern "system" fn mouse_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let point = hook_point(lparam);
    let decision = decide_mouse(
        code,
        wparam.0 as u32,
        modifiers_down(),
        BUSY.load(Ordering::Relaxed) > 0,
        LEFT_GESTURE.load(Ordering::Relaxed),
        REGION_MODE.load(Ordering::Relaxed),
        point,
    );
    match decision {
        MouseDecision::Pass => {
            call_next(MOUSE_HOOK.load(Ordering::Relaxed), code, wparam, lparam)
        }
        MouseDecision::Swallow => LRESULT(1),
        MouseDecision::Post { message, x, y } => {
            deliver_hook_post(message, x, y, true, code, wparam, lparam)
        }
        MouseDecision::Forward { message, x, y } => {
            deliver_hook_post(message, x, y, false, code, wparam, lparam)
        }
    }
}

fn deliver_hook_post(
    message: u32,
    x: i32,
    y: i32,
    swallow: bool,
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_APP_BEGIN_DRAG {
        LEFT_GESTURE.store(true, Ordering::Relaxed);
    }
    let posted = unsafe {
        PostMessageW(
            Some(current_window()),
            message,
            WPARAM(x as isize as usize),
            LPARAM(y as isize),
        )
    };
    if posted.is_ok() && swallow {
        return LRESULT(1);
    }
    if posted.is_err() && message == WM_APP_BEGIN_DRAG {
        LEFT_GESTURE.store(false, Ordering::Relaxed);
    }
    call_next(MOUSE_HOOK.load(Ordering::Relaxed), code, wparam, lparam)
}

fn hook_point(lparam: LPARAM) -> Option<(i32, i32)> {
    if lparam.0 == 0 {
        return None;
    }
    unsafe {
        let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);
        Some((info.pt.x, info.pt.y))
    }
}

unsafe extern "system" fn key_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 && lparam.0 != 0 {
        let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        if crate::logic::should_swallow_escape(
            wparam.0 as u32,
            info.vkCode,
            REGION_MODE.load(Ordering::Relaxed),
        ) {
            unsafe {
                let _ = PostMessageW(
                    Some(current_window()),
                    WM_APP_REGION_CANCEL,
                    WPARAM(0),
                    LPARAM(0),
                );
            }
            return LRESULT(1);
        }
        let swallow = crate::logic::should_swallow_alt(
            wparam.0 as u32,
            info.vkCode,
            load_modifiers(),
            key_down(VK_CONTROL),
            key_down(VK_SHIFT),
        );
        if swallow {
            return LRESULT(1);
        }
    }
    call_next(KEY_HOOK.load(Ordering::Relaxed), code, wparam, lparam)
}

fn call_next(
    hook: isize,
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { CallNextHookEx(Some(HHOOK(hook as *mut _)), code, wparam, lparam) }
}

fn locate_window(x: i32, y: i32) -> Option<WindowHit> {
    unsafe {
        let window = WindowFromPoint(POINT { x, y });
        if window.0.is_null() {
            return None;
        }
        let deepest = walk_children(window, x, y);
        Some(describe(deepest))
    }
}

fn walk_children(window: HWND, x: i32, y: i32) -> HWND {
    let mut current = window;
    for _ in 0..32 {
        let child = visible_child(current, x, y);
        if child.0.is_null() || child == current {
            return current;
        }
        current = child;
    }
    current
}

fn visible_child(parent: HWND, x: i32, y: i32) -> HWND {
    unsafe {
        let mut point = POINT { x, y };
        if !windows::Win32::Graphics::Gdi::ScreenToClient(parent, &mut point)
            .as_bool()
        {
            return HWND::default();
        }
        ChildWindowFromPointEx(
            parent,
            point,
            CWP_SKIPINVISIBLE | CWP_SKIPTRANSPARENT,
        )
    }
}

fn root_hit(hit: WindowHit) -> WindowHit {
    unsafe {
        let root = GetAncestor(HWND(hit.handle as *mut _), GA_ROOT);
        if root.0.is_null() || root.0 as isize == hit.handle {
            return hit;
        }
        describe(root)
    }
}

fn describe(window: HWND) -> WindowHit {
    WindowHit {
        handle: window.0 as isize,
        bounds: window_rect(window),
        class_name: read_class(window),
        text: read_text(window),
    }
}

fn window_rect(window: HWND) -> Rect {
    unsafe {
        let mut rectangle = RECT::default();
        if GetWindowRect(window, &mut rectangle).is_err() {
            return Rect::default();
        }
        Rect {
            left: rectangle.left,
            top: rectangle.top,
            right: rectangle.right,
            bottom: rectangle.bottom,
        }
    }
}

fn read_class(window: HWND) -> String {
    let mut buffer = [0u16; 256];
    let length = unsafe { GetClassNameW(window, &mut buffer) };
    utf16_prefix(&buffer, length as usize)
}

fn read_text(window: HWND) -> String {
    let mut buffer = [0u16; 256];
    let length = unsafe { GetWindowTextW(window, &mut buffer) };
    utf16_prefix(&buffer, length as usize)
}

fn utf16_prefix(buffer: &[u16], length: usize) -> String {
    String::from_utf16_lossy(&buffer[..length.min(buffer.len())])
}

fn bounds_for_window(window: HWND) -> Rect {
    unsafe {
        let root = GetAncestor(window, GA_ROOT);
        if root == window {
            let mut rectangle = RECT::default();
            if DwmGetWindowAttribute(
                window,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                &mut rectangle as *mut RECT as *mut _,
                std::mem::size_of::<RECT>() as u32,
            )
            .is_ok()
                && rectangle.right > rectangle.left
                && rectangle.bottom > rectangle.top
            {
                return Rect {
                    left: rectangle.left,
                    top: rectangle.top,
                    right: rectangle.right,
                    bottom: rectangle.bottom,
                };
            }
        }
    }
    window_rect(window)
}

fn read_element(x: i32, y: i32) -> Option<Rect> {
    unsafe {
        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
                .ok()?;
        let element: IUIAutomationElement =
            automation.ElementFromPoint(POINT { x, y }).ok()?;
        let rectangle = element.CurrentBoundingRectangle().ok()?;
        let width = rectangle.right - rectangle.left;
        let height = rectangle.bottom - rectangle.top;
        if width <= 0 || height <= 0 {
            return None;
        }
        Some(Rect {
            left: rectangle.left,
            top: rectangle.top,
            right: rectangle.right,
            bottom: rectangle.bottom,
        })
    }
}

fn capture_png(bounds: &Rect, path: &Path) -> Result<Vec<u8>, i32> {
    if bounds.width() <= 0
        || bounds.height() <= 0
        || bounds.width() > MAX_EDGE
        || bounds.height() > MAX_EDGE
    {
        return Err(-1);
    }
    if let Some(directory) = path.parent() {
        std::fs::create_dir_all(directory).map_err(|_| -1)?;
    }
    let pixels = copy_screen(bounds)?;
    write_png(path, bounds.width(), bounds.height(), &pixels)?;
    Ok(pixels)
}

fn copy_screen(bounds: &Rect) -> Result<Vec<u8>, i32> {
    unsafe {
        let _ = DwmFlush();
        let screen = GetDC(None);
        if screen.0.is_null() {
            return Err(last_error());
        }
        let copied = copy_from_screen(screen, bounds);
        let _ = ReleaseDC(None, screen);
        copied
    }
}

fn copy_from_screen(screen: HDC, bounds: &Rect) -> Result<Vec<u8>, i32> {
    unsafe {
        let memory = CreateCompatibleDC(Some(screen));
        if memory.0.is_null() {
            return Err(last_error());
        }
        let mut bits = std::ptr::null_mut();
        let section = create_section(screen, bounds, &mut bits)?;
        let previous = SelectObject(memory, section.into());
        let blitted = BitBlt(
            memory,
            0,
            0,
            bounds.width(),
            bounds.height(),
            Some(screen),
            bounds.left,
            bounds.top,
            SRCCOPY | CAPTUREBLT,
        );
        let pixels = if blitted.is_ok() {
            Ok(read_pixels(bits, bounds.width(), bounds.height()))
        } else {
            Err(last_error())
        };
        if !previous.is_invalid() {
            let _ = SelectObject(memory, previous);
        }
        let _ = DeleteObject(section.into());
        let _ = DeleteDC(memory);
        pixels
    }
}

fn create_section(
    screen: HDC,
    bounds: &Rect,
    bits: &mut *mut core::ffi::c_void,
) -> Result<HBITMAP, i32> {
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: bounds.width(),
            biHeight: -bounds.height(),
            biPlanes: 1,
            biBitCount: 32,
            ..Default::default()
        },
        ..Default::default()
    };
    unsafe {
        CreateDIBSection(Some(screen), &info, DIB_RGB_COLORS, bits, None, 0)
            .map_err(|error| error.code().0)
    }
}

fn read_pixels(
    bits: *mut core::ffi::c_void,
    width: i32,
    height: i32,
) -> Vec<u8> {
    let length = width as usize * height as usize * 4;
    let mut pixels = vec![0u8; length];
    unsafe {
        std::ptr::copy_nonoverlapping(
            bits as *const u8,
            pixels.as_mut_ptr(),
            length,
        );
    }
    for index in (3..pixels.len()).step_by(4) {
        pixels[index] = OPAQUE_ALPHA;
    }
    pixels
}

fn write_png(
    path: &Path,
    width: i32,
    height: i32,
    pixels: &[u8],
) -> Result<(), i32> {
    unsafe {
        let factory: IWICImagingFactory = CoCreateInstance(
            &CLSID_WICImagingFactory,
            None,
            CLSCTX_INPROC_SERVER,
        )
        .map_err(|error| error.code().0)?;
        let stream = factory.CreateStream().map_err(|error| error.code().0)?;
        let wide = wide_path(path);
        stream
            .InitializeFromFilename(PCWSTR(wide.as_ptr()), GENERIC_READ_WRITE)
            .map_err(|error| error.code().0)?;
        let encoder = factory
            .CreateEncoder(&GUID_ContainerFormatPng, std::ptr::null())
            .map_err(|error| error.code().0)?;
        encoder
            .Initialize(&stream, WICBitmapEncoderNoCache)
            .map_err(|error| error.code().0)?;
        let mut frame = None;
        let mut options = None;
        encoder
            .CreateNewFrame(&mut frame, &mut options)
            .map_err(|error| error.code().0)?;
        let frame = frame.ok_or(-1)?;
        write_frame(&frame, width, height, pixels)?;
        encoder.Commit().map_err(|error| error.code().0)?;
        stream
            .Commit(STGC_DEFAULT)
            .map_err(|error| error.code().0)?;
        let _drop_frame: IWICBitmapFrameEncode = frame;
        let _drop_encoder: IWICBitmapEncoder = encoder;
        let _drop_stream: IWICStream = stream;
    }
    Ok(())
}

fn write_frame(
    frame: &IWICBitmapFrameEncode,
    width: i32,
    height: i32,
    pixels: &[u8],
) -> Result<(), i32> {
    unsafe {
        frame.Initialize(None).map_err(|error| error.code().0)?;
        frame
            .SetSize(width as u32, height as u32)
            .map_err(|error| error.code().0)?;
        frame
            .SetResolution(96.0, 96.0)
            .map_err(|error| error.code().0)?;
        let mut format = GUID_WICPixelFormat32bppBGRA;
        frame
            .SetPixelFormat(&mut format)
            .map_err(|error| error.code().0)?;
        if format != GUID_WICPixelFormat32bppBGRA
            && format != GUID_WICPixelFormat32bppPBGRA
        {
            return Err(-1);
        }
        frame
            .WritePixels(height as u32, (width as u32) * 4, pixels)
            .map_err(|error| error.code().0)?;
        frame.Commit().map_err(|error| error.code().0)?;
    }
    Ok(())
}

const MAX_DIB_BYTES: usize = 64 * 1024 * 1024;
const DROP_EFFECT_COPY: u32 = 1;

pub fn library_shot_files(
    paths: &[std::path::PathBuf],
) -> Vec<std::path::PathBuf> {
    let Some(root) = canonical_directory(&shots_directory()) else {
        return Vec::new();
    };
    shot_files(&root, paths)
}

pub fn copy_text_clipboard(text: &str) -> bool {
    let mut units: Vec<u16> = text.encode_utf16().collect();
    units.push(0);
    unsafe {
        if OpenClipboard(None).is_err() {
            return false;
        }
        if EmptyClipboard().is_err() {
            let _ = CloseClipboard();
            return false;
        }
        let bytes = std::slice::from_raw_parts(
            units.as_ptr().cast::<u8>(),
            units.len() * 2,
        );
        let stored = put_format(CF_UNICODETEXT.0 as u32, bytes);
        let closed = CloseClipboard().is_ok();
        stored && closed
    }
}

pub fn copy_shot_files(paths: &[std::path::PathBuf]) -> bool {
    let Some(root) = canonical_directory(&shots_directory()) else {
        return false;
    };
    let allowed = shot_files(&root, paths);
    if allowed.is_empty() {
        return false;
    }
    store_shot_files(&allowed)
}

fn canonical_directory(path: &Path) -> Option<std::path::PathBuf> {
    let full = path.canonicalize().ok()?;
    if full.is_dir() { Some(full) } else { None }
}

fn shot_files(
    root: &Path,
    paths: &[std::path::PathBuf],
) -> Vec<std::path::PathBuf> {
    paths
        .iter()
        .filter_map(|path| canonical_shot(root, path))
        .collect()
}

fn canonical_shot(root: &Path, path: &Path) -> Option<std::path::PathBuf> {
    let full = path.canonicalize().ok()?;
    if full.is_file() && full.starts_with(root) {
        Some(full)
    } else {
        None
    }
}

fn store_shot_files(paths: &[std::path::PathBuf]) -> bool {
    let labels: Vec<String> =
        paths.iter().map(|path| clipboard_path(path)).collect();
    let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
    let hdrop = hdrop_paths(&refs);
    let single = paths.first().filter(|_| paths.len() == 1);
    let png = single.and_then(|path| std::fs::read(path).ok());
    let dib = single.and_then(|path| decode_png_dib(path));
    write_file_clipboard(Some(&hdrop), png.as_deref(), dib.as_deref())
}

fn write_file_clipboard(
    hdrop: Option<&[u8]>,
    png: Option<&[u8]>,
    dib: Option<&[u8]>,
) -> bool {
    let png_format =
        unsafe { RegisterClipboardFormatW(PCWSTR(wide("PNG").as_ptr())) };
    let effect_format = unsafe {
        RegisterClipboardFormatW(PCWSTR(wide("Preferred DropEffect").as_ptr()))
    };
    if png_format == 0 || effect_format == 0 {
        return false;
    }
    let effect = DROP_EFFECT_COPY.to_le_bytes();
    unsafe {
        if OpenClipboard(None).is_err() {
            return false;
        }
        if EmptyClipboard().is_err() {
            let _ = CloseClipboard();
            return false;
        }
        let mut stored = true;
        if let Some(drop) = hdrop {
            stored = put_format(CF_HDROP.0 as u32, drop)
                && put_format(effect_format, &effect);
        }
        if let Some(png) = png {
            stored = stored && put_format(png_format, png);
        }
        if let Some(dib) = dib {
            stored = stored && put_format(CF_DIB.0 as u32, dib);
        }
        let closed = CloseClipboard().is_ok();
        if stored && closed {
            let _ = OleFlushClipboard();
            return true;
        }
        false
    }
}

fn decode_png_dib(path: &Path) -> Option<Vec<u8>> {
    let pixels = decode_png_bgra(path).ok()?;
    dib_from_top_down(&pixels.bytes, pixels.width, pixels.height).ok()
}

struct BgraBitmap {
    width: i32,
    height: i32,
    bytes: Vec<u8>,
}

fn decode_png_bgra(path: &Path) -> Result<BgraBitmap, i32> {
    unsafe {
        let factory: IWICImagingFactory = CoCreateInstance(
            &CLSID_WICImagingFactory,
            None,
            CLSCTX_INPROC_SERVER,
        )
        .map_err(|error| error.code().0)?;
        let wide = wide_path(path);
        let decoder = factory
            .CreateDecoderFromFilename(
                PCWSTR(wide.as_ptr()),
                None,
                GENERIC_READ,
                WICDecodeMetadataCacheOnDemand,
            )
            .map_err(|error| error.code().0)?;
        let frame = decoder.GetFrame(0).map_err(|error| error.code().0)?;
        let converter = factory
            .CreateFormatConverter()
            .map_err(|error| error.code().0)?;
        converter
            .Initialize(
                &frame,
                &GUID_WICPixelFormat32bppBGRA,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeCustom,
            )
            .map_err(|error| error.code().0)?;
        let mut width = 0u32;
        let mut height = 0u32;
        converter
            .GetSize(&mut width, &mut height)
            .map_err(|error| error.code().0)?;
        let bytes = read_converter(&converter, width, height)?;
        Ok(BgraBitmap {
            width: width as i32,
            height: height as i32,
            bytes,
        })
    }
}

fn read_converter(
    source: &IWICFormatConverter,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, i32> {
    let stride = width.checked_mul(4).ok_or(-1)?;
    let length = stride.checked_mul(height).ok_or(-1)?;
    if length as usize > MAX_DIB_BYTES {
        return Err(-1);
    }
    let mut bytes = vec![0u8; length as usize];
    unsafe {
        source
            .CopyPixels(std::ptr::null(), stride, &mut bytes)
            .map_err(|error| error.code().0)?;
    }
    Ok(bytes)
}

fn copy_shot(shot: &SavedShot) -> bool {
    let Ok(png) = std::fs::read(&shot.path) else {
        return false;
    };
    let Ok(dib) = dib_from_top_down(&shot.pixels, shot.width, shot.height)
    else {
        return false;
    };
    let hdrop = hdrop_blob(&clipboard_path(&shot.path));
    write_file_clipboard(Some(&hdrop), Some(&png), Some(&dib))
}

fn put_format(format: u32, bytes: &[u8]) -> bool {
    unsafe {
        let flags = GMEM_MOVEABLE | GMEM_ZEROINIT;
        let Ok(handle) = GlobalAlloc(flags, bytes.len()) else {
            return false;
        };
        let locked = GlobalLock(handle);
        if locked.is_null() {
            return false;
        }
        std::ptr::copy_nonoverlapping(
            bytes.as_ptr(),
            locked as *mut u8,
            bytes.len(),
        );
        let _ = GlobalUnlock(handle);
        SetClipboardData(format, Some(HANDLE(handle.0))).is_ok()
    }
}

fn write_log(line: &str) {
    let path = {
        let guard = SESSION.lock().expect("session");
        guard.as_ref().map(|session| session.log_path.clone())
    };
    let Some(path) = path else {
        return;
    };
    if let Some(directory) = path.parent() {
        let _ = std::fs::create_dir_all(directory);
    }
    let stamped = format!("{} {line}\r\n", local_stamp());
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = file.write_all(stamped.as_bytes());
    }
}

fn show_shot(text_value: &str, path: &Path) {
    if publish_notice(text_value, Some(path)) {
        return;
    }
    {
        let mut guard = SESSION.lock().expect("session");
        if let Some(session) = guard.as_mut() {
            session.balloon.remember(Some(path.to_path_buf()));
        }
    }
    show_balloon(text_value);
}

fn show_notice(text_value: &str) {
    if publish_notice(text_value, None) {
        return;
    }
    {
        let mut guard = SESSION.lock().expect("session");
        if let Some(session) = guard.as_mut() {
            session.balloon.remember(None);
        }
    }
    show_balloon(text_value);
}

fn publish_notice(text_value: &str, path: Option<&Path>) -> bool {
    send_notice(text_value, path, false, false)
}

fn publish_lens(path: &Path) -> bool {
    send_notice("", Some(path), true, false)
}

fn publish_show_gallery() -> bool {
    send_notice("", None, false, true)
}

fn send_notice(
    text_value: &str,
    path: Option<&Path>,
    open_lens: bool,
    show_gallery: bool,
) -> bool {
    let guard = NOTICE_TX.lock().expect("notice");
    let Some(sender) = guard.as_ref() else {
        return false;
    };
    sender
        .send(CaptureNotice {
            text: text_value.to_string(),
            path: path.map(|item| item.display().to_string()),
            open_lens,
            show_gallery,
        })
        .is_ok()
}

fn show_balloon(body: &str) {
    let mut data = tray_data(current_window());
    data.uFlags = NIF_INFO;
    copy_units(&mut data.szInfoTitle, APP_NAME);
    copy_units(&mut data.szInfo, body);
    data.dwInfoFlags = NIIF_INFO;
    data.Anonymous.uTimeout = 10_000;
    unsafe {
        let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
    }
}

fn open_remembered() {
    let path = {
        let guard = SESSION.lock().expect("session");
        guard.as_ref().and_then(|session| {
            session
                .balloon
                .path_to_open()
                .map(|path| path.to_path_buf())
        })
    };
    let Some(path) = path else {
        return;
    };
    if publish_lens(&path) {
        return;
    }
    open_path(&path);
}

fn open_path(path: &Path) {
    if !path.exists() {
        return;
    }
    let path_wide = wide_path(path);
    let operation = wide("open");
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(operation.as_ptr()),
            PCWSTR(path_wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if (result.0 as isize) <= 32 {
        show_notice(
            &text(current_language(), TextId::OpenShotFailed).to_string(),
        );
    }
}

fn add_tray(hwnd: HWND) -> Result<(), i32> {
    let mut data = tray_data(hwnd);
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    data.uCallbackMessage = TRAY_MESSAGE;
    data.hIcon = tray_icon();
    copy_units(&mut data.szTip, &format!("{APP_NAME} {APP_VERSION}"));
    unsafe {
        if !Shell_NotifyIconW(NIM_ADD, &data).as_bool() {
            return Err(last_error());
        }
        data.Anonymous.uVersion = NOTIFYICON_VERSION;
        data.uFlags = windows::Win32::UI::Shell::NOTIFY_ICON_DATA_FLAGS(0);
        let _ = Shell_NotifyIconW(NIM_SETVERSION, &data);
    }
    Ok(())
}

fn remove_tray(hwnd: HWND) {
    let data = tray_data(hwnd);
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &data);
    }
}

fn tray_data(hwnd: HWND) -> NOTIFYICONDATAW {
    let mut data = NOTIFYICONDATAW::default();
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = TRAY_ID;
    data
}

fn on_tray(hwnd: HWND, event: u32) -> LRESULT {
    if event == NIN_BALLOONUSERCLICK {
        open_remembered();
    } else if event == WM_RBUTTONUP || event == WM_CONTEXTMENU {
        show_menu(hwnd);
    }
    LRESULT(0)
}

fn show_menu(hwnd: HWND) {
    let language = current_language();
    let modifiers = load_modifiers();
    let days = DAYS.load(Ordering::Relaxed);
    unsafe {
        let menu = CreatePopupMenu().unwrap_or_default();
        let shortcut = CreatePopupMenu().unwrap_or_default();
        let languages = CreatePopupMenu().unwrap_or_default();
        append(menu, 0, &format_hint(language, &modifiers.format()), false);
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        append(
            menu,
            ID_FOLDER,
            text(language, TextId::MenuOpenFolder),
            true,
        );
        append(menu, ID_RETENTION, &format_retention(language, days), true);
        check_item(shortcut, ID_CTRL, "Ctrl", modifiers.control);
        check_item(shortcut, ID_SHIFT, "Shift", modifiers.shift);
        check_item(shortcut, ID_ALT, "Alt", modifiers.alt);
        let _ = AppendMenuW(
            menu,
            MF_POPUP,
            shortcut.0 as usize,
            PCWSTR(wide(text(language, TextId::MenuShortcut)).as_ptr()),
        );
        append(menu, ID_LOG, text(language, TextId::MenuOpenLog), true);
        append(menu, ID_VERSION, text(language, TextId::MenuVersion), true);
        let stored = current_language();
        check_item(
            languages,
            ID_LANG_SYSTEM,
            text(language, TextId::LanguageSystem),
            matches!(stored, UiLanguage::System),
        );
        check_item(
            languages,
            ID_LANG_KO,
            text(language, TextId::LanguageKorean),
            matches!(stored, UiLanguage::Korean),
        );
        check_item(
            languages,
            ID_LANG_EN,
            text(language, TextId::LanguageEnglish),
            matches!(stored, UiLanguage::English),
        );
        let _ = AppendMenuW(
            menu,
            MF_POPUP,
            languages.0 as usize,
            PCWSTR(wide(text(language, TextId::MenuLanguage)).as_ptr()),
        );
        append(menu, ID_EXIT, text(language, TextId::MenuExit), true);
        let mut cursor = POINT::default();
        let _ = GetCursorPos(&mut cursor);
        let _ = SetForegroundWindow(hwnd);
        let _ = TrackPopupMenu(
            menu,
            TPM_RIGHTBUTTON,
            cursor.x,
            cursor.y,
            None,
            hwnd,
            None,
        );
        let _ = PostMessageW(Some(hwnd), 0, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
    }
}

fn append(menu: HMENU, id: usize, caption: &str, enabled: bool) {
    let flags = if enabled {
        MF_STRING
    } else {
        MF_STRING | windows::Win32::UI::WindowsAndMessaging::MF_GRAYED
    };
    let wide_caption = wide(caption);
    unsafe {
        let _ = AppendMenuW(menu, flags, id, PCWSTR(wide_caption.as_ptr()));
    }
}

fn check_item(menu: HMENU, id: usize, caption: &str, checked: bool) {
    let flags = if checked {
        MF_STRING | MF_CHECKED
    } else {
        MF_STRING | MF_UNCHECKED
    };
    let wide_caption = wide(caption);
    unsafe {
        let _ = AppendMenuW(menu, flags, id, PCWSTR(wide_caption.as_ptr()));
    }
}

fn on_command(hwnd: HWND, id: usize) {
    match id {
        ID_FOLDER => open_folder(),
        ID_RETENTION => edit_retention(hwnd),
        ID_LOG => open_log(),
        ID_VERSION => show_version(hwnd),
        ID_EXIT => unsafe {
            let _ = DestroyWindow(hwnd);
        },
        ID_CTRL | ID_SHIFT | ID_ALT => toggle_modifier(id),
        ID_LANG_SYSTEM => save_language(UiLanguage::System),
        ID_LANG_KO => save_language(UiLanguage::Korean),
        ID_LANG_EN => save_language(UiLanguage::English),
        _ => {}
    }
}

fn toggle_modifier(id: usize) {
    let mut modifiers = load_modifiers();
    match id {
        ID_CTRL => modifiers.control = !modifiers.control,
        ID_SHIFT => modifiers.shift = !modifiers.shift,
        ID_ALT => modifiers.alt = !modifiers.alt,
        _ => return,
    }
    if !modifiers.has_any() {
        show_notice(text(current_language(), TextId::EmptyShortcut));
        return;
    }
    store_modifiers(modifiers);
    let mut settings = load_settings();
    settings.control = modifiers.control;
    settings.shift = modifiers.shift;
    settings.alt = modifiers.alt;
    let _ = save_settings(&settings);
    refresh_tip();
}

fn save_language(language: UiLanguage) {
    store_language(language);
    let mut settings = load_settings();
    settings.language = tag_from_language(language).to_string();
    let _ = save_settings(&settings);
}

fn open_folder() {
    let directory = prepare_shots_directory();
    let _ = std::fs::create_dir_all(&directory);
    open_path_any(&directory);
}

fn open_log() {
    let path = settings_dir().join("gesture.log");
    if !path.is_file() {
        show_notice(text(current_language(), TextId::EmptyLog));
        return;
    }
    open_path_any(&path);
}

fn open_path_any(path: &Path) {
    let wide_path = wide_path(path);
    unsafe {
        let _ = ShellExecuteW(
            None,
            PCWSTR(wide("open").as_ptr()),
            PCWSTR(wide_path.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
    }
}

fn edit_retention(owner: HWND) {
    let current = DAYS.load(Ordering::Relaxed);
    let Some(days) =
        crate::dialogs::prompt_days(owner, current_language(), current)
    else {
        return;
    };
    let stored = crate::logic::normalize_days(days);
    DAYS.store(stored, Ordering::Relaxed);
    let mut settings = load_settings();
    settings.retention_days = stored;
    let _ = save_settings(&settings);
    let _ = delete_stale();
}

fn show_version(owner: HWND) {
    let already = {
        let guard = SESSION.lock().expect("session");
        guard
            .as_ref()
            .map(|session| session.version_window != 0)
            .unwrap_or(false)
    };
    if already {
        let hwnd = SESSION
            .lock()
            .expect("session")
            .as_ref()
            .map(|session| HWND(session.version_window as *mut _));
        if let Some(hwnd) = hwnd {
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd);
            }
        }
        return;
    }
    let history = history_text();
    let hwnd = crate::dialogs::show_history(
        owner,
        &format!("{APP_NAME} {APP_VERSION}"),
        &history,
    );
    if let Some(hwnd) = hwnd {
        if let Some(session) = SESSION.lock().expect("session").as_mut() {
            session.version_window = hwnd.0 as isize;
        }
    }
}

pub fn clear_version_window() {
    if let Some(session) = SESSION.lock().expect("session").as_mut() {
        session.version_window = 0;
    }
}

fn history_text() -> String {
    let beside = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|dir| dir.join("CHANGELOG.md")));
    let missing = text(current_language(), TextId::MissingHistory);
    if let Some(path) = beside {
        let text_value = crate::logic::read_history_text(&path, missing);
        if text_value != missing {
            return text_value;
        }
    }
    include_str!("../CHANGELOG.md").trim().to_string()
}

fn refresh_tip() {
    let mut data = tray_data(current_window());
    data.uFlags = NIF_TIP;
    copy_units(&mut data.szTip, &format!("{APP_NAME} {APP_VERSION}"));
    unsafe {
        let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
    }
}

fn load_language() -> UiLanguage {
    let settings = load_settings();
    language_from_tag(normalize_culture(&settings.language))
}

fn destroy_session(hwnd: HWND) {
    cancel_region();
    unsafe {
        let _ = KillTimer(Some(hwnd), TIMER_CLICK);
        let _ = KillTimer(Some(hwnd), TIMER_CLEAN);
        if !hwnd.0.is_null() {
            let _ = DestroyWindow(hwnd);
        }
    }
    *SESSION.lock().expect("session") = None;
}

fn tray_icon() -> HICON {
    unsafe {
        let mut bits = std::ptr::null_mut();
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: ICON_SIZE,
                biHeight: -ICON_SIZE,
                biPlanes: 1,
                biBitCount: 32,
                ..Default::default()
            },
            ..Default::default()
        };
        let color =
            CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0)
                .unwrap_or_default();
        if !bits.is_null() {
            draw_mark(bits as *mut u8);
        }
        let mask = CreateBitmap(ICON_SIZE, ICON_SIZE, 1, 1, None);
        let icon = ICONINFO {
            fIcon: BOOL(1),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask,
            hbmColor: color,
        };
        let created = CreateIconIndirect(&icon).unwrap_or_default();
        let _ = DeleteObject(color.into());
        let _ = DeleteObject(mask.into());
        created
    }
}

fn draw_mark(bits: *mut u8) {
    let size = ICON_SIZE as usize;
    for y in 0..size {
        for x in 0..size {
            let offset = (y * size + x) * 4;
            let (blue, green, red, alpha) = pixel(x as i32, y as i32);
            unsafe {
                *bits.add(offset) = blue;
                *bits.add(offset + 1) = green;
                *bits.add(offset + 2) = red;
                *bits.add(offset + 3) = alpha;
            }
        }
    }
}

fn pixel(x: i32, y: i32) -> (u8, u8, u8, u8) {
    let center = ICON_SIZE / 2;
    let radius = (ICON_SIZE - 4) / 2;
    let dx = x - center;
    let dy = y - center;
    if dx * dx + dy * dy > radius * radius {
        return (0, 0, 0, 0);
    }
    let arm = ICON_SIZE / 5;
    if (x - center).abs() <= 1 && y >= arm && y < ICON_SIZE - arm {
        return (255, 255, 255, 255);
    }
    if (y - center).abs() <= 1 && x >= arm && x < ICON_SIZE - arm {
        return (255, 255, 255, 255);
    }
    (235, 99, 36, 255)
}

fn local_now() -> CivilTime {
    let time = local_system_time();
    CivilTime {
        year: i32::from(time.wYear),
        month: u32::from(time.wMonth),
        day: u32::from(time.wDay),
        hour: u32::from(time.wHour),
        minute: u32::from(time.wMinute),
        second: u32::from(time.wSecond),
    }
}

fn local_today() -> (i32, u32, u32) {
    let time = local_now();
    (time.year, time.month, time.day)
}

fn local_stamp() -> String {
    let time = local_system_time();
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        time.wYear,
        time.wMonth,
        time.wDay,
        time.wHour,
        time.wMinute,
        time.wSecond,
        time.wMilliseconds
    )
}

fn local_system_time() -> windows::Win32::Foundation::SYSTEMTIME {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetLocalTime(time: *mut windows::Win32::Foundation::SYSTEMTIME);
    }
    unsafe {
        let mut time = windows::Win32::Foundation::SYSTEMTIME::default();
        GetLocalTime(&mut time);
        time
    }
}

fn show_error(text_value: &str) {
    let body = wide(text_value);
    let title = wide(APP_NAME);
    unsafe {
        let _ = MessageBoxW(
            None,
            PCWSTR(body.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONERROR,
        );
    }
}

fn wide(text_value: &str) -> Vec<u16> {
    text_value
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect()
}

fn wide_path(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn copy_units(dest: &mut [u16], text_value: &str) {
    dest.fill(0);
    for (index, unit) in
        text_value.encode_utf16().take(dest.len() - 1).enumerate()
    {
        dest[index] = unit;
    }
}

fn last_error() -> i32 {
    unsafe { windows::Win32::Foundation::GetLastError().0 as i32 }
}

#[cfg(test)]
mod tests {
    use super::write_png;

    #[test]
    fn writes_one_pixel_png() {
        let _ = super::start_ole();
        let path = std::env::temp_dir().join("agsnap-one-pixel.png");
        let pixels = [0u8, 0, 255, 255];
        write_png(&path, 1, 1, &pixels).expect("wic png");
        let bytes = std::fs::read(&path).expect("png bytes");
        assert!(bytes.starts_with(b"\x89PNG"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn file_drop_keeps_a_plain_path() {
        use windows::Win32::Foundation::HGLOBAL;
        use windows::Win32::System::DataExchange::GetClipboardData;
        use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
        use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};
        let _ = super::start_ole();
        let directory = crate::logic::shots_directory().join("2026-10-17");
        std::fs::create_dir_all(&directory).expect("day");
        let path = directory.join("Screenshot-paste-check.png");
        let pixels = [0u8, 0, 255, 255];
        super::write_png(&path, 1, 1, &pixels).expect("png");
        assert!(super::copy_shot_files(&[path.clone()]));
        let files = clipboard_files();
        let expected = crate::logic::clipboard_path(
            &path.canonicalize().expect("canonical"),
        );
        assert_eq!(files, vec![expected]);
        assert!(!files[0].starts_with(r"\\?\"));
        assert_eq!(clipboard_drop_effect(), super::DROP_EFFECT_COPY);
        let _ = std::fs::remove_file(&path);
        fn clipboard_files() -> Vec<String> {
            unsafe {
                super::OpenClipboard(None).expect("open");
                let handle =
                    GetClipboardData(super::CF_HDROP.0 as u32).expect("hdrop");
                let drop = HDROP(handle.0);
                let count = DragQueryFileW(drop, 0xFFFF_FFFF, None);
                let mut files = Vec::new();
                for index in 0..count {
                    let length = DragQueryFileW(drop, index, None);
                    let mut buffer = vec![0u16; length as usize + 1];
                    DragQueryFileW(drop, index, Some(&mut buffer));
                    let end = buffer
                        .iter()
                        .position(|unit| *unit == 0)
                        .unwrap_or(buffer.len());
                    files.push(String::from_utf16_lossy(&buffer[..end]));
                }
                let _ = super::CloseClipboard();
                files
            }
        }
        fn clipboard_drop_effect() -> u32 {
            unsafe {
                super::OpenClipboard(None).expect("open");
                let format = super::RegisterClipboardFormatW(super::PCWSTR(
                    super::wide("Preferred DropEffect").as_ptr(),
                ));
                let handle = GetClipboardData(format).expect("effect");
                let locked = GlobalLock(HGLOBAL(handle.0));
                let value = std::ptr::read_unaligned(locked as *const u32);
                let _ = GlobalUnlock(HGLOBAL(handle.0));
                let _ = super::CloseClipboard();
                value
            }
        }
    }
}
