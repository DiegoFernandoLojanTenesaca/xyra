use std::{ffi::c_void, mem::size_of, ptr::null_mut};
use windows::{
    core::HSTRING,
    Globalization::Language,
    Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap},
    Media::Ocr::OcrEngine,
    Storage::Streams::DataWriter,
    Win32::{
        Graphics::Gdi::{
            BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject, BITMAPINFO,
            BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY,
        },
        System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED},
        UI::WindowsAndMessaging::{GetForegroundWindow, GetSystemMetrics, GetWindowTextW, SM_CXSCREEN, SM_CYSCREEN},
    },
};
use xyra_core::cards::OcrLine;

const GAME_WINDOW_TITLE: &str = "League of Legends (TM) Client";
const FALLBACK_OCR_LANGUAGE: &str = "en";

/// Windows OCR (WinRT) needs the calling thread initialized.
pub fn init_thread() {
    unsafe {
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
    }
}

pub fn primary_size() -> (i32, i32) {
    unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) }
}

/// BGRA pixels of a rectangle of the primary monitor.
pub fn capture(x: i32, y: i32, width: i32, height: i32) -> Option<Vec<u8>> {
    unsafe {
        let screen = GetDC(None);
        let dc = CreateCompatibleDC(Some(screen));
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut c_void = null_mut();
        let mut pixels = None;
        if let Ok(bitmap) = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0) {
            let previous = SelectObject(dc, bitmap.into());
            if BitBlt(dc, 0, 0, width, height, Some(screen), x, y, SRCCOPY).is_ok() {
                pixels = Some(std::slice::from_raw_parts(bits as *const u8, (width * height * 4) as usize).to_vec());
            }
            SelectObject(dc, previous);
            let _ = DeleteObject(bitmap.into());
        }
        let _ = DeleteDC(dc);
        ReleaseDC(None, screen);
        pixels
    }
}

pub struct Ocr {
    engine: OcrEngine,
    pub language: String,
}

impl Ocr {
    /// Card names are shown in the client language, so the OCR uses it too.
    pub fn new(client_locale: &str) -> Option<Ocr> {
        let full = client_locale.replace('_', "-");
        let short = full.split('-').next().unwrap_or(FALLBACK_OCR_LANGUAGE).to_string();
        for tag in [full, short] {
            let Ok(language) = Language::CreateLanguage(&HSTRING::from(tag.as_str())) else { continue };
            if OcrEngine::IsLanguageSupported(&language).unwrap_or(false) {
                if let Ok(engine) = OcrEngine::TryCreateFromLanguage(&language) {
                    return Some(Ocr { engine, language: tag });
                }
            }
        }
        let engine = OcrEngine::TryCreateFromUserProfileLanguages().ok()?;
        let language = engine.RecognizerLanguage().ok()?.LanguageTag().ok()?.to_string();
        Some(Ocr { engine, language })
    }

    pub fn read(&self, bgra: &[u8], width: i32, height: i32) -> windows::core::Result<Vec<OcrLine>> {
        let writer = DataWriter::new()?;
        writer.WriteBytes(bgra)?;
        let bitmap = SoftwareBitmap::CreateCopyFromBuffer(&writer.DetachBuffer()?, BitmapPixelFormat::Bgra8, width, height)?;
        let result = self.engine.RecognizeAsync(&bitmap)?.join()?;
        let mut lines = Vec::new();
        for line in result.Lines()? {
            let (mut x0, mut x1, mut y1) = (f64::MAX, f64::MIN, f64::MIN);
            for word in line.Words()? {
                let rect = word.BoundingRect()?;
                x0 = x0.min(rect.X as f64);
                x1 = x1.max((rect.X + rect.Width) as f64);
                y1 = y1.max((rect.Y + rect.Height) as f64);
            }
            if x0 < x1 {
                lines.push(OcrLine { text: line.Text()?.to_string(), x0, x1, y1 });
            }
        }
        Ok(lines)
    }
}

/// Checks the foreground window title only: the game process is never opened (Vanguard protects it).
pub fn is_game_focused() -> bool {
    let mut title = [0u16; 64];
    let len = unsafe { GetWindowTextW(GetForegroundWindow(), &mut title) };
    String::from_utf16_lossy(&title[..len.max(0) as usize]) == GAME_WINDOW_TITLE
}
