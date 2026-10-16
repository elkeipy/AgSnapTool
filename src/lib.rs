mod dialogs;
pub mod logic;
pub mod rect;
pub mod strings;
mod win;

pub use logic::*;
pub use rect::Rect;
pub use strings::{
    TextId, UiLanguage, language_from_tag, normalize_culture,
    tag_from_language, text,
};
pub use win::{
    CaptureNotice, apply_runtime_settings, open_document, prepare_process,
    set_notice_sender, spawn_capture,
};

pub fn run() -> i32 {
    win::run()
}
