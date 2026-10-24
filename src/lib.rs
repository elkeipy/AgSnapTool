mod dialogs;
pub mod logic;
mod ocr;
pub mod rect;
pub mod strings;
mod win;

pub use logic::*;
pub use ocr::{installed_ocr_tags, recognize_shot_lines};
pub use rect::Rect;
pub use strings::{
    TextId, UiLanguage, language_from_tag, normalize_culture, normalize_theme,
    tag_from_language, text,
};
pub use win::{
    CaptureNotice, apply_runtime_settings, ask_yes_no_cancel,
    claim_single_instance, copy_shot_files, copy_text_clipboard,
    library_shot_files, open_document, prepare_process, set_notice_sender,
    spawn_capture,
};

pub fn run() -> i32 {
    win::run()
}
