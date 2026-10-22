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
pub const THEME_SYSTEM: &str = "";
pub const THEME_DARK: &str = "dark";
pub const THEME_LIGHT: &str = "light";
pub const THEME_FLAT: &str = "flat";
pub const THEME_MATERIAL: &str = "material";
pub const THEME_GLASS: &str = "glass";

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
    ThemeTitle,
    ThemeSystem,
    ThemeDark,
    ThemeLight,
    ThemeFlat,
    ThemeMaterial,
    ThemeGlass,
    TiltLabel,
    TapeLabel,
    TapeWidthLabel,
    TapeHeightLabel,
    PolaroidLabel,
    CaptionLabel,
    PeekOneLabel,
    PeekTwoLabel,
    PeekDelayLabel,
    PeekScaleLabel,
    PeekScaleMax,
    HistoryButton,
    DialogClose,
    PanelOpen,
    PanelClose,
    MenuExit,
    MenuGallery,
    CopyImage,
    MagnifyImage,
    OpenWith,
    DeleteImage,
    DeleteConfirm,
    DeleteFailed,
    EditImage,
    EditorSave,
    EditorCancel,
    EditorSelect,
    EditorRect,
    EditorEllipse,
    EditorPolygon,
    EditorArrow,
    EditorText,
    EditorStroke,
    EditorFill,
    EditorWidth,
    EditorHead,
    EditorHeadSize,
    EditorFont,
    EditorFontSize,
    EditorColor,
    EditorSize,
    EditorHint,
    EditorUnsaved,
    EditorDiscard,
    EditorHeadTriangle,
    EditorHeadChevron,
    EditorHeadBarbed,
    CopyFailed,
    SaveFailed,
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

pub fn normalize_theme(value: &str) -> &'static str {
    match value.trim().to_ascii_lowercase().as_str() {
        "dark" => THEME_DARK,
        "light" => THEME_LIGHT,
        "flat" => THEME_FLAT,
        "material" => THEME_MATERIAL,
        "glass" => THEME_GLASS,
        _ => THEME_SYSTEM,
    }
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
        TextId::MenuShortcut => "Snapshot shortcut",
        TextId::MenuOpenLog => "Open log",
        TextId::MenuVersion => "Version",
        TextId::MenuLanguage => "Language",
        TextId::ThemeTitle => "Theme",
        TextId::ThemeSystem => "System",
        TextId::ThemeDark => "Dark",
        TextId::ThemeLight => "Light",
        TextId::ThemeFlat => "Flat",
        TextId::ThemeMaterial => "Material",
        TextId::ThemeGlass => "Glass",
        TextId::TiltLabel => "Tilt",
        TextId::TapeLabel => "Tape",
        TextId::TapeWidthLabel => "Length",
        TextId::TapeHeightLabel => "Height",
        TextId::PolaroidLabel => "Polaroid mode",
        TextId::CaptionLabel => "Date size",
        TextId::PeekOneLabel => "Hover zoom",
        TextId::PeekTwoLabel => "Second zoom",
        TextId::PeekDelayLabel => "Wait (ms)",
        TextId::PeekScaleLabel => "Scale",
        TextId::PeekScaleMax => "Max",
        TextId::HistoryButton => "History",
        TextId::DialogClose => "Close",
        TextId::PanelOpen => "Show",
        TextId::PanelClose => "Hide",
        TextId::MenuExit => "Exit",
        TextId::MenuGallery => "Snap Gallery",
        TextId::CopyImage => "Copy image",
        TextId::MagnifyImage => "Magnify image",
        TextId::OpenWith => "Open with default program...",
        TextId::DeleteImage => "Delete image...",
        TextId::DeleteConfirm => "Delete the selected shots?",
        TextId::DeleteFailed => "Could not delete the shot.",
        TextId::EditImage => "Edit image...",
        TextId::EditorSave => "Save",
        TextId::EditorCancel => "Cancel",
        TextId::EditorSelect => "Select",
        TextId::EditorRect => "Rectangle",
        TextId::EditorEllipse => "Ellipse",
        TextId::EditorPolygon => "Polygon",
        TextId::EditorArrow => "Arrow",
        TextId::EditorText => "Text",
        TextId::EditorStroke => "Stroke",
        TextId::EditorFill => "Fill",
        TextId::EditorWidth => "Width",
        TextId::EditorHead => "Head",
        TextId::EditorHeadSize => "Head size",
        TextId::EditorFont => "Font",
        TextId::EditorFontSize => "Type size",
        TextId::EditorColor => "Color",
        TextId::EditorSize => "Size",
        TextId::EditorHint => {
            "Ctrl+S saves. Enter finishes text. Ctrl+Enter adds a line."
        }
        TextId::EditorUnsaved => "Save changes?",
        TextId::EditorDiscard => "Don't save",
        TextId::EditorHeadTriangle => "Triangle",
        TextId::EditorHeadChevron => "Chevron",
        TextId::EditorHeadBarbed => "Barbed",
        TextId::CopyFailed => "Could not copy the image.",
        TextId::SaveFailed => "Could not save the image.",
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
        TextId::MenuShortcut => "스냅샷 단축키",
        TextId::MenuOpenLog => "로그 열기",
        TextId::MenuVersion => "버전",
        TextId::MenuLanguage => "언어",
        TextId::ThemeTitle => "테마",
        TextId::ThemeSystem => "시스템",
        TextId::ThemeDark => "다크",
        TextId::ThemeLight => "라이트",
        TextId::ThemeFlat => "Flat",
        TextId::ThemeMaterial => "Material",
        TextId::ThemeGlass => "글래스",
        TextId::TiltLabel => "기울기",
        TextId::TapeLabel => "테이프",
        TextId::TapeWidthLabel => "길이",
        TextId::TapeHeightLabel => "높이",
        TextId::PolaroidLabel => "폴라로이드 모드",
        TextId::CaptionLabel => "날짜 크기",
        TextId::PeekOneLabel => "호버 확대",
        TextId::PeekTwoLabel => "2단계 확대",
        TextId::PeekDelayLabel => "대기(ms)",
        TextId::PeekScaleLabel => "배율",
        TextId::PeekScaleMax => "최대",
        TextId::HistoryButton => "변경 내용",
        TextId::DialogClose => "닫기",
        TextId::PanelOpen => "펼치기",
        TextId::PanelClose => "접기",
        TextId::MenuExit => "종료",
        TextId::MenuGallery => "스냅 갤러리",
        TextId::CopyImage => "이미지 복사",
        TextId::MagnifyImage => "돋보기 이미지",
        TextId::OpenWith => "연결 프로그램으로 열기...",
        TextId::DeleteImage => "이미지 삭제...",
        TextId::DeleteConfirm => "선택한 캡처를 삭제할까?",
        TextId::DeleteFailed => "캡처를 삭제하지 못했다.",
        TextId::EditImage => "이미지 편집...",
        TextId::EditorSave => "저장",
        TextId::EditorCancel => "취소",
        TextId::EditorSelect => "선택",
        TextId::EditorRect => "네모",
        TextId::EditorEllipse => "원",
        TextId::EditorPolygon => "다각형",
        TextId::EditorArrow => "화살표",
        TextId::EditorText => "텍스트",
        TextId::EditorStroke => "외곽선",
        TextId::EditorFill => "채우기",
        TextId::EditorWidth => "굵기",
        TextId::EditorHead => "화살촉",
        TextId::EditorHeadSize => "화살촉 크기",
        TextId::EditorFont => "글꼴",
        TextId::EditorFontSize => "글자 크기",
        TextId::EditorColor => "색",
        TextId::EditorSize => "크기",
        TextId::EditorHint => {
            "Ctrl+S로 저장. Enter는 텍스트 종료, Ctrl+Enter는 줄바꿈."
        }
        TextId::EditorUnsaved => "변경한 내용을 저장할까?",
        TextId::EditorDiscard => "저장 안 함",
        TextId::EditorHeadTriangle => "삼각",
        TextId::EditorHeadChevron => "꺾쇠",
        TextId::EditorHeadBarbed => "가시",
        TextId::CopyFailed => "이미지를 클립보드에 넣지 못했다.",
        TextId::SaveFailed => "이미지를 저장하지 못했다.",
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
