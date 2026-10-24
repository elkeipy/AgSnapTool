use std::path::Path;
use std::time::Duration;

use windows::Globalization::Language;
use windows::Graphics::Imaging::{
    BitmapDecoder, BitmapPixelFormat, SoftwareBitmap,
};
use windows::Media::Ocr::{OcrEngine, OcrResult};
use windows::Storage::Streams::IRandomAccessStream;
use windows::Storage::{FileAccessMode, StorageFile};
use windows::core::{HSTRING, RuntimeType};
use windows_future::{AsyncStatus, IAsyncOperation};

/// Language tags Windows OCR can use on this PC.
pub fn installed_ocr_tags() -> Vec<String> {
    collect_ocr_tags().unwrap_or_default()
}

/// Reads text lines from a PNG with the chosen WinOCR language.
pub fn recognize_shot_lines(
    path: &Path,
    language: &str,
) -> Result<Vec<String>, i32> {
    recognize_lines(path, language).map_err(|error| error.code().0)
}

fn collect_ocr_tags() -> windows::core::Result<Vec<String>> {
    let langs = OcrEngine::AvailableRecognizerLanguages()?;
    let count = langs.Size()?;
    let mut tags = Vec::new();
    for index in 0..count {
        let lang = langs.GetAt(index)?;
        tags.push(lang.LanguageTag()?.to_string());
    }
    Ok(tags)
}

fn recognize_lines(
    path: &Path,
    language: &str,
) -> windows::core::Result<Vec<String>> {
    let tag = Language::CreateLanguage(&HSTRING::from(language))?;
    let engine = OcrEngine::TryCreateFromLanguage(&tag)?;
    let bitmap = software_bitmap_from_png(path)?;
    let result: OcrResult = wait_async(engine.RecognizeAsync(&bitmap)?)?;
    let lines = result.Lines()?;
    let mut stored = Vec::new();
    let count = lines.Size()?;
    for index in 0..count {
        let line = lines.GetAt(index)?;
        let item = line.Text()?.to_string();
        if !item.trim().is_empty() {
            stored.push(item);
        }
    }
    Ok(stored)
}

fn software_bitmap_from_png(
    path: &Path,
) -> windows::core::Result<SoftwareBitmap> {
    let file: StorageFile = wait_async(StorageFile::GetFileFromPathAsync(
        &winrt_file_path(path)?,
    )?)?;
    let stream: IRandomAccessStream =
        wait_async(file.OpenAsync(FileAccessMode::Read)?)?;
    let decoder: BitmapDecoder =
        wait_async(BitmapDecoder::CreateAsync(&stream)?)?;
    let bitmap: SoftwareBitmap = wait_async(decoder.GetSoftwareBitmapAsync()?)?;
    SoftwareBitmap::Convert(&bitmap, BitmapPixelFormat::Gray8)
}

fn wait_async<T: RuntimeType + 'static>(
    operation: IAsyncOperation<T>,
) -> windows::core::Result<T> {
    loop {
        if operation.Status()? == AsyncStatus::Started {
            std::thread::sleep(Duration::from_millis(2));
            continue;
        }
        return operation.GetResults();
    }
}

fn winrt_file_path(path: &Path) -> windows::core::Result<HSTRING> {
    let full = path
        .canonicalize()
        .map_err(|_| windows::core::Error::empty())?;
    let mut text = full.to_string_lossy().into_owned();
    if let Some(rest) = text.strip_prefix(r"\\?\") {
        text = rest.to_string();
    }
    Ok(HSTRING::from(text))
}
