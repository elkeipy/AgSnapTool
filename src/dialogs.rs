use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CreateWindowExW, DefWindowProcW, DestroyWindow,
    DispatchMessageW, GWLP_WNDPROC, GetMessageW, GetWindowTextW, HMENU,
    IsDialogMessageW, IsWindow, RegisterClassW, SW_SHOW, SetWindowLongPtrW,
    SetWindowTextW, ShowWindow, TranslateMessage, WINDOW_STYLE, WM_CLOSE,
    WM_COMMAND, WM_DESTROY, WM_KEYDOWN, WNDCLASSW, WS_BORDER, WS_CAPTION,
    WS_CHILD, WS_EX_DLGMODALFRAME, WS_SYSMENU, WS_THICKFRAME, WS_VISIBLE,
    WS_VSCROLL,
};
use windows::core::PCWSTR;

use crate::logic::normalize_days;
use crate::strings::{TextId, UiLanguage, text};

const EDIT_ID: isize = 200;
const OK_ID: usize = 1;
const CANCEL_ID: usize = 2;
const DIALOG_CLASS: &str = "AgSnapDialog";

struct PromptState {
    accepted: bool,
    days: i32,
}

pub fn prompt_days(
    owner: HWND,
    language: UiLanguage,
    days: i32,
) -> Option<i32> {
    let class = wide(DIALOG_CLASS);
    register(&class);
    let title = wide(text(language, TextId::RetentionTitle));
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            PCWSTR(class.as_ptr()),
            PCWSTR(title.as_ptr()),
            WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            200,
            200,
            280,
            160,
            Some(owner),
            None,
            Some(instance()),
            None,
        )
        .ok()?
    };
    let state = Box::new(PromptState {
        accepted: false,
        days: normalize_days(days),
    });
    let state_ptr = Box::into_raw(state);
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
            hwnd,
            windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
            state_ptr as isize,
        );
        let label = wide(text(language, TextId::RetentionLabel));
        let ok = wide(text(language, TextId::RetentionOk));
        let cancel = wide(text(language, TextId::RetentionCancel));
        let _ = CreateWindowExW(
            Default::default(),
            windows::core::w!("STATIC"),
            PCWSTR(label.as_ptr()),
            WS_CHILD | WS_VISIBLE,
            12,
            14,
            80,
            18,
            Some(hwnd),
            None,
            Some(instance()),
            None,
        );
        let edit = CreateWindowExW(
            Default::default(),
            windows::core::w!("EDIT"),
            PCWSTR::null(),
            WINDOW_STYLE(
                WS_CHILD.0
                    | WS_VISIBLE.0
                    | WS_BORDER.0
                    | windows::Win32::UI::WindowsAndMessaging::ES_NUMBER as u32,
            ),
            12,
            36,
            72,
            22,
            Some(hwnd),
            Some(HMENU(EDIT_ID as *mut _)),
            Some(instance()),
            None,
        )
        .unwrap_or_default();
        let value = wide(&normalize_days(days).to_string());
        let _ = SetWindowTextW(edit, PCWSTR(value.as_ptr()));
        let _ = CreateWindowExW(
            Default::default(),
            windows::core::w!("BUTTON"),
            PCWSTR(ok.as_ptr()),
            WS_CHILD | WS_VISIBLE,
            100,
            80,
            75,
            24,
            Some(hwnd),
            Some(HMENU(OK_ID as *mut _)),
            Some(instance()),
            None,
        );
        let _ = CreateWindowExW(
            Default::default(),
            windows::core::w!("BUTTON"),
            PCWSTR(cancel.as_ptr()),
            WS_CHILD | WS_VISIBLE,
            184,
            80,
            75,
            24,
            Some(hwnd),
            Some(HMENU(CANCEL_ID as *mut _)),
            Some(instance()),
            None,
        );
        let _ = EnableWindow(owner, false);
        let _ = ShowWindow(hwnd, SW_SHOW);
        modal_loop(hwnd);
        let _ = EnableWindow(owner, true);
    }
    let state = unsafe { Box::from_raw(state_ptr) };
    if state.accepted {
        Some(state.days)
    } else {
        None
    }
}

pub fn show_history(owner: HWND, title: &str, history: &str) -> Option<HWND> {
    let class = wide("AgSnapHistory");
    register_history(&class);
    let title_wide = wide(title);
    let hwnd = unsafe {
        CreateWindowExW(
            Default::default(),
            PCWSTR(class.as_ptr()),
            PCWSTR(title_wide.as_ptr()),
            WS_CAPTION | WS_SYSMENU | WS_THICKFRAME | WS_VISIBLE,
            180,
            120,
            540,
            460,
            Some(owner),
            None,
            Some(instance()),
            None,
        )
        .ok()?
    };
    let body = wide(history);
    unsafe {
        let edit = CreateWindowExW(
            Default::default(),
            windows::core::w!("EDIT"),
            PCWSTR(body.as_ptr()),
            WINDOW_STYLE(
                WS_CHILD.0
                    | WS_VISIBLE.0
                    | WS_VSCROLL.0
                    | windows::Win32::UI::WindowsAndMessaging::ES_MULTILINE
                        as u32
                    | windows::Win32::UI::WindowsAndMessaging::ES_READONLY
                        as u32
                    | windows::Win32::UI::WindowsAndMessaging::ES_AUTOVSCROLL
                        as u32,
            ),
            12,
            12,
            500,
            390,
            Some(hwnd),
            None,
            Some(instance()),
            None,
        )
        .unwrap_or_default();
        let previous = SetWindowLongPtrW(
            edit,
            GWLP_WNDPROC,
            edit_proc as *const () as isize,
        );
        let _ = SetWindowLongPtrW(
            edit,
            windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
            previous,
        );
        let _ = ShowWindow(hwnd, SW_SHOW);
    }
    Some(hwnd)
}

fn modal_loop(hwnd: HWND) {
    unsafe {
        let mut message =
            windows::Win32::UI::WindowsAndMessaging::MSG::default();
        while IsWindow(Some(hwnd)).as_bool() {
            let status = GetMessageW(&mut message, None, 0, 0);
            if status.0 == 0 || status.0 == -1 {
                break;
            }
            if IsDialogMessageW(hwnd, &message).as_bool() {
                continue;
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

unsafe extern "system" fn prompt_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_COMMAND {
        let id = wparam.0 & 0xffff;
        if id == OK_ID {
            accept(hwnd);
            return LRESULT(0);
        }
        if id == CANCEL_ID {
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
            return LRESULT(0);
        }
    }
    if message == WM_CLOSE {
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
        return LRESULT(0);
    }
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

fn accept(hwnd: HWND) {
    let mut buffer = [0u16; 32];
    let edit = child(hwnd, EDIT_ID);
    let length = unsafe { GetWindowTextW(edit, &mut buffer) };
    let text_value = String::from_utf16_lossy(&buffer[..length as usize]);
    let days = normalize_days(text_value.trim().parse().unwrap_or(0));
    unsafe {
        let pointer = windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
            hwnd,
            windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
        ) as *mut PromptState;
        if !pointer.is_null() {
            (*pointer).accepted = true;
            (*pointer).days = days;
        }
        let _ = DestroyWindow(hwnd);
    }
}

unsafe extern "system" fn history_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_KEYDOWN && wparam.0 == 0x1b {
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
        return LRESULT(0);
    }
    if message == WM_DESTROY {
        crate::win::clear_version_window();
    }
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

unsafe extern "system" fn edit_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_KEYDOWN && wparam.0 == 0x1b {
        let parent =
            unsafe { windows::Win32::UI::WindowsAndMessaging::GetParent(hwnd) };
        if let Ok(parent) = parent {
            unsafe {
                let _ = DestroyWindow(parent);
            }
        }
        return LRESULT(0);
    }
    let previous = unsafe {
        windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
            hwnd,
            windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
        )
    };
    type Proc = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;
    let call: Proc = unsafe { std::mem::transmute(previous) };
    unsafe { call(hwnd, message, wparam, lparam) }
}

fn child(parent: HWND, id: isize) -> HWND {
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::GetDlgItem(
            Some(parent),
            id as i32,
        )
        .unwrap_or_default()
    }
}

fn register(class_name: &[u16]) {
    unsafe {
        let class = WNDCLASSW {
            lpfnWndProc: Some(prompt_proc),
            hInstance: instance(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH(
                windows::Win32::Graphics::Gdi::GetStockObject(
                    windows::Win32::Graphics::Gdi::WHITE_BRUSH,
                )
                .0,
            ),
            ..Default::default()
        };
        RegisterClassW(&class);
    }
}

fn register_history(class_name: &[u16]) {
    unsafe {
        let class = WNDCLASSW {
            lpfnWndProc: Some(history_proc),
            hInstance: instance(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH(
                windows::Win32::Graphics::Gdi::GetStockObject(
                    windows::Win32::Graphics::Gdi::WHITE_BRUSH,
                )
                .0,
            ),
            ..Default::default()
        };
        RegisterClassW(&class);
    }
}

fn instance() -> windows::Win32::Foundation::HINSTANCE {
    unsafe { GetModuleHandleW(None).unwrap_or_default().into() }
}

fn wide(text_value: &str) -> Vec<u16> {
    text_value
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect()
}

#[allow(dead_code)]
fn _create(value: CREATESTRUCTW) -> CREATESTRUCTW {
    value
}
