mod dialogs;
pub mod logic;
pub mod rect;
pub mod strings;
mod win;

pub use logic::*;
pub use rect::Rect;
pub use strings::{TextId, UiLanguage, text};

pub fn run() -> i32 {
    win::run()
}
