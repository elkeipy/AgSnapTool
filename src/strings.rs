//! English is the neutral language. Korean overrides it.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiLanguage {
    System,
    Korean,
    English,
}

pub const FOLLOW_SYSTEM: &str = "";
pub const KOREAN: &str = "ko";
pub const ENGLISH: &str = "en";

#[derive(Clone, Copy, Debug)]
pub enum TextId {
    MenuOpenFolder,
    MenuRetention,
    RetentionTitle,
    RetentionLabel,
    RetentionOk,
    RetentionCancel,
    MenuShortcut,
    MenuOpenLog,
    MenuVersion,
    MenuLanguage,
    MenuExit,
    MenuGallery,
    GalleryAll,
    GalleryEmpty,
    LanguageSystem,
    LanguageKorean,
    LanguageEnglish,
    EmptyLog,
    EmptyShortcut,
    ClickHint,
    CopiedNotice,
    OpenShotFailed,
    MissingHistory,
    HookFailure,
}

pub fn text(language: UiLanguage, id: TextId) -> &'static str {
    match resolved(language) {
        UiLanguage::Korean => korean(id),
        _ => english(id),
    }
}

pub fn format_retention(language: UiLanguage, days: i32) -> String {
    text(language, TextId::MenuRetention).replace("{0}", &days.to_string())
}

pub fn format_hint(language: UiLanguage, shortcut: &str) -> String {
    text(language, TextId::ClickHint).replace("{0}", shortcut)
}

pub fn format_hook_failure(language: UiLanguage, code: i32) -> String {
    text(language, TextId::HookFailure).replace("{0}", &code.to_string())
}

pub fn normalize_culture(culture: &str) -> &'static str {
    let token = culture.trim();
    if is_token(token, KOREAN) {
        return KOREAN;
    }
    if is_token(token, ENGLISH) {
        return ENGLISH;
    }
    FOLLOW_SYSTEM
}

pub fn language_from_tag(tag: &str) -> UiLanguage {
    match normalize_culture(tag) {
        KOREAN => UiLanguage::Korean,
        ENGLISH => UiLanguage::English,
        _ => UiLanguage::System,
    }
}

pub fn tag_from_language(language: UiLanguage) -> &'static str {
    match language {
        UiLanguage::System => FOLLOW_SYSTEM,
        UiLanguage::Korean => KOREAN,
        UiLanguage::English => ENGLISH,
    }
}

fn resolved(language: UiLanguage) -> UiLanguage {
    match language {
        UiLanguage::System => system_language(),
        other => other,
    }
}

fn system_language() -> UiLanguage {
    if primary_lang() == LANG_KOREAN {
        UiLanguage::Korean
    } else {
        UiLanguage::English
    }
}

const LANG_KOREAN: u16 = 0x12;

fn primary_lang() -> u16 {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetUserDefaultUILanguage() -> u16;
    }
    unsafe { GetUserDefaultUILanguage() & 0x3ff }
}

fn is_token(value: &str, language: &str) -> bool {
    value.eq_ignore_ascii_case(language)
        || value.get(..language.len() + 1).is_some_and(|prefix| {
            prefix[..language.len()].eq_ignore_ascii_case(language)
                && prefix.as_bytes().get(language.len()) == Some(&b'-')
        })
}

fn english(id: TextId) -> &'static str {
    match id {
        TextId::MenuOpenFolder => "Capture folder",
        TextId::MenuRetention => "Keep {0} days",
        TextId::RetentionTitle => "Keep shots",
        TextId::RetentionLabel => "Days",
        TextId::RetentionOk => "OK",
        TextId::RetentionCancel => "Cancel",
        TextId::MenuShortcut => "Shortcut",
        TextId::MenuOpenLog => "Open log",
        TextId::MenuVersion => "Version",
        TextId::MenuLanguage => "Language",
        TextId::MenuExit => "Exit",
        TextId::MenuGallery => "Gallery",
        TextId::GalleryAll => "All",
        TextId::GalleryEmpty => "No shots yet.",
        TextId::LanguageSystem => "System",
        TextId::LanguageKorean => "Korean",
        TextId::LanguageEnglish => "English",
        TextId::EmptyLog => "No entries yet.",
        TextId::EmptyShortcut => "At least one modifier is required.",
        TextId::ClickHint => "{0} left click / right click is the window",
        TextId::CopiedNotice => "Copied to the clipboard.",
        TextId::OpenShotFailed => "Could not open the shot.",
        TextId::MissingHistory => "No changelog.",
        TextId::HookFailure => "Could not install the mouse hook. Code {0}",
    }
}

fn korean(id: TextId) -> &'static str {
    match id {
        TextId::MenuOpenFolder => "캡처 폴더",
        TextId::MenuRetention => "보관 {0}일",
        TextId::RetentionTitle => "캡처 보관",
        TextId::RetentionLabel => "일수",
        TextId::RetentionOk => "확인",
        TextId::RetentionCancel => "취소",
        TextId::MenuShortcut => "단축키",
        TextId::MenuOpenLog => "로그 열기",
        TextId::MenuVersion => "버전",
        TextId::MenuLanguage => "언어",
        TextId::MenuExit => "종료",
        TextId::MenuGallery => "모아보기",
        TextId::GalleryAll => "전체",
        TextId::GalleryEmpty => "아직 캡처가 없다.",
        TextId::LanguageSystem => "시스템",
        TextId::LanguageKorean => "한국어",
        TextId::LanguageEnglish => "English",
        TextId::EmptyLog => "아직 기록이 없다.",
        TextId::EmptyShortcut => "모디파이어가 하나 이상 필요하다.",
        TextId::ClickHint => "{0} 좌클릭 / 우클릭은 창",
        TextId::CopiedNotice => "클립보드에 넣었다.",
        TextId::OpenShotFailed => "캡처를 열지 못했다.",
        TextId::MissingHistory => "변경 이력이 없다.",
        TextId::HookFailure => "마우스 훅을 설치하지 못했다. 코드 {0}",
    }
}
