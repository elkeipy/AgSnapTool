mod dialogs;
pub mod logic;
pub mod rect;
pub mod strings;
mod win;

pub use logic::*;
pub use rect::Rect;
pub use strings::{
    TextId, UiLanguage, language_from_tag, normalize_culture, normalize_theme,
    tag_from_language, text,
};
pub use win::{
    CaptureNotice, apply_runtime_settings, ask_yes_no_cancel,
    claim_single_instance, copy_shot_files, open_document, prepare_process,
    set_notice_sender, spawn_capture,
};

pub fn run() -> i32 {
    win::run()
}
