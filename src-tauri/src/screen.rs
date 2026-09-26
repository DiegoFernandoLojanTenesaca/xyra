use std::{ffi::c_void, mem::size_of, ptr::null_mut};
use windows::{
    Globalization::Language,
    Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap},
    Media::Ocr::OcrEngine,
    Storage::Streams::DataWriter,
    Win32::{
        Graphics::Gdi::{
            BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC,
            MONITOR_DEFAULTTONEAREST, MonitorFromWindow, ReleaseDC, SRCCOPY, SelectObject,
        },
        System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize},
        UI::WindowsAndMessaging::{FindWindowW, GetForegroundWindow, GetSystemMetrics, IsIconic, SM_CXSCREEN, SM_CYSCREEN},
    },
    core::{HSTRING, Result},
};
use xyra_core::cards::OcrLine;

const GAME_WINDOW_TITLE: &str = "League of Legends (TM) Client";
const FALLBACK_OCR_LANGUAGE: &str = "en";

/// Initializes WinRT on the calling thread for the OCR.
pub fn init_thread() -> Result<()> {
    unsafe { RoInitialize(RO_INIT_MULTITHREADED) }
}

pub fn primary_size() -> (i32, i32) {
    unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) }
}

/// BGRA pixels of a rectangle of the primary monitor.
pub fn capture(x: i32, y: i32, width: i32, height: i32) -> Result<Vec<u8>> {
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
        let pixels = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0).and_then(|bitmap| {
            let previous = SelectObject(dc, bitmap.into());
            let copied = BitBlt(dc, 0, 0, width, height, Some(screen), x, y, SRCCOPY)
                .map(|()| std::slice::from_raw_parts(bits as *const u8, (width * height * 4) as usize).to_vec());
            SelectObject(dc, previous);
            DeleteObject(bitmap.into()).ok()?;
            copied
        });
        DeleteDC(dc).ok()?;
        ReleaseDC(None, screen);
        pixels
    }
}

pub struct Ocr {
    engine: OcrEngine,
    pub language: String,
}

impl Ocr {
    /// OCR in the client's language, or in the Windows user's.
    pub fn new(client_locale: &str) -> Option<Ocr> {
        let full = client_locale.replace('_', "-");
        let short = full.split('-').next().unwrap_or(FALLBACK_OCR_LANGUAGE).to_string();
        for tag in [full, short] {
            let Ok(language) = Language::CreateLanguage(&HSTRING::from(tag.as_str())) else { continue };
            if OcrEngine::IsLanguageSupported(&language).unwrap_or(false)
                && let Ok(engine) = OcrEngine::TryCreateFromLanguage(&language)
            {
                return Some(Ocr { engine, language: tag });
            }
        }
        let engine = OcrEngine::TryCreateFromUserProfileLanguages().ok()?;
        let language = engine.RecognizerLanguage().ok()?.LanguageTag().ok()?.to_string();
        Some(Ocr { engine, language })
    }

    pub fn read(&self, bgra: &[u8], width: i32, height: i32) -> Result<Vec<OcrLine>> {
        let writer = DataWriter::new()?;
        writer.WriteBytes(bgra)?;
        let bitmap = SoftwareBitmap::CreateCopyFromBuffer(&writer.DetachBuffer()?, BitmapPixelFormat::Bgra8, width, height)?;
        let result = self.engine.RecognizeAsync(&bitmap)?.join()?;
        let mut lines = Vec::new();
        for line in result.Lines()? {
            let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
            for word in line.Words()? {
                let rect = word.BoundingRect()?;
                x0 = x0.min(rect.X as f64);
                x1 = x1.max((rect.X + rect.Width) as f64);
                y0 = y0.min(rect.Y as f64);
                y1 = y1.max((rect.Y + rect.Height) as f64);
            }
            if x0 < x1 {
                lines.push(OcrLine { text: line.Text()?.to_string(), x0, x1, y0, y1 });
            }
        }
        Ok(lines)
    }
}

/// Whether the game window shows on screen: it has the focus, or the focus is on another monitor.
pub fn is_game_visible() -> bool {
    unsafe {
        let Ok(game) = FindWindowW(None, &HSTRING::from(GAME_WINDOW_TITLE)) else { return false };
        if IsIconic(game).as_bool() {
            return false;
        }
        let foreground = GetForegroundWindow();
        foreground == game || MonitorFromWindow(foreground, MONITOR_DEFAULTTONEAREST) != MonitorFromWindow(game, MONITOR_DEFAULTTONEAREST)
    }
}
