//! Pantalla: captura (GDI) y OCR de Windows.
//! Nada de esto toca el proceso del juego: solo se lee la imagen del escritorio y se dibuja una ventana encima.
use xyra_core::cartas::Linea;
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

const TITULO_JUEGO: &str = "League of Legends (TM) Client";

/// El OCR de Windows (WinRT) necesita el hilo inicializado.
pub fn iniciar_hilo() {
    unsafe {
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
    }
}

/// Tamaño del monitor principal en píxeles reales (la app es consciente de DPI por monitor).
pub fn tamano() -> (i32, i32) {
    unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) }
}

/// BGRA de un rectángulo del monitor principal.
pub fn capturar(x: i32, y: i32, ancho: i32, alto: i32) -> Option<Vec<u8>> {
    unsafe {
        let pantalla = GetDC(None);
        let dc = CreateCompatibleDC(Some(pantalla));
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: ancho,
                biHeight: -alto, // de arriba hacia abajo
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut c_void = null_mut();
        let mut datos = None;
        if let Ok(bmp) = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0) {
            let viejo = SelectObject(dc, bmp.into());
            if BitBlt(dc, 0, 0, ancho, alto, Some(pantalla), x, y, SRCCOPY).is_ok() {
                datos = Some(std::slice::from_raw_parts(bits as *const u8, (ancho * alto * 4) as usize).to_vec());
            }
            SelectObject(dc, viejo);
            let _ = DeleteObject(bmp.into());
        }
        let _ = DeleteDC(dc);
        ReleaseDC(None, pantalla);
        datos
    }
}

pub struct Ocr {
    motor: OcrEngine,
    pub idioma: String,
}

impl Ocr {
    /// OCR en el idioma del cliente (los nombres de las cartas salen en ese idioma).
    pub fn nuevo(idioma_cliente: &str) -> Option<Ocr> {
        let completo = idioma_cliente.replace('_', "-");
        let corto = completo.split('-').next().unwrap_or("en").to_string();
        for tag in [completo, corto] {
            let Ok(idioma) = Language::CreateLanguage(&HSTRING::from(tag.as_str())) else { continue };
            if OcrEngine::IsLanguageSupported(&idioma).unwrap_or(false) {
                if let Ok(motor) = OcrEngine::TryCreateFromLanguage(&idioma) {
                    return Some(Ocr { motor, idioma: tag });
                }
            }
        }
        let motor = OcrEngine::TryCreateFromUserProfileLanguages().ok()?;
        let idioma = motor.RecognizerLanguage().ok()?.LanguageTag().ok()?.to_string();
        Some(Ocr { motor, idioma })
    }

    pub fn leer(&self, bgra: &[u8], ancho: i32, alto: i32) -> windows::core::Result<Vec<Linea>> {
        let escritor = DataWriter::new()?;
        escritor.WriteBytes(bgra)?;
        let bmp = SoftwareBitmap::CreateCopyFromBuffer(&escritor.DetachBuffer()?, BitmapPixelFormat::Bgra8, ancho, alto)?;
        let resultado = self.motor.RecognizeAsync(&bmp)?.join()?;
        let mut lineas = Vec::new();
        for l in resultado.Lines()? {
            let (mut x0, mut x1, mut y1) = (f64::MAX, f64::MIN, f64::MIN);
            for p in l.Words()? {
                let r = p.BoundingRect()?;
                x0 = x0.min(r.X as f64);
                x1 = x1.max((r.X + r.Width) as f64);
                y1 = y1.max((r.Y + r.Height) as f64);
            }
            if x0 < x1 {
                lineas.push(Linea { texto: l.Text()?.to_string(), x0, x1, y1 });
            }
        }
        Ok(lineas)
    }
}

/// ¿La ventana al frente es el juego? Se mira el título (no se abre el proceso del juego, que protege Vanguard).
pub fn juego_al_frente() -> bool {
    let mut titulo = [0u16; 64];
    let n = unsafe { GetWindowTextW(GetForegroundWindow(), &mut titulo) };
    String::from_utf16_lossy(&titulo[..n.max(0) as usize]) == TITULO_JUEGO
}
