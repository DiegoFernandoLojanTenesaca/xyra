//! Capa de etiquetas nativa: una ventana transparente siempre encima, dibujada con Direct2D/DirectWrite
//! y mostrada con UpdateLayeredWindow (transparencia por píxel). No usa navegador: mientras juegas solo
//! existe esta ventana, que deja pasar los clics, nunca toma el foco y no sale en Alt+Tab.
//! Vive en el hilo del motor, que es quien le procesa los mensajes.
use xyra_core::{cartas::Carta, config::Config};
use std::{mem::size_of, ptr::null_mut, thread, time::Duration};
use windows_numerics::Vector2;
use windows::{
    core::{w, Result},
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM},
        Graphics::{
            Direct2D::{
                Common::{
                    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_FIGURE_BEGIN_FILLED, D2D1_FIGURE_END_CLOSED, D2D1_PIXEL_FORMAT,
                    D2D_RECT_F,
                },
                D2D1CreateFactory, ID2D1DCRenderTarget, ID2D1Factory, D2D1_DRAW_TEXT_OPTIONS_NONE,
                D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT, D2D1_RENDER_TARGET_PROPERTIES,
                D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE, D2D1_ROUNDED_RECT,
                D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE,
            },
            DirectWrite::{
                DWriteCreateFactory, IDWriteFactory, IDWriteTextLayout, DWRITE_FACTORY_TYPE_SHARED,
                DWRITE_FONT_STRETCH_SEMI_CONDENSED, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT, DWRITE_FONT_WEIGHT_BOLD,
                DWRITE_TEXT_METRICS, DWRITE_FONT_WEIGHT_EXTRA_BOLD, DWRITE_FONT_WEIGHT_NORMAL,
            },
            Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
            Gdi::{
                CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject, AC_SRC_ALPHA,
                AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ,
            },
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DispatchMessageW, PeekMessageW, RegisterClassW, SetWindowPos, ShowWindow,
            TranslateMessage, UpdateLayeredWindow, HWND_TOPMOST, MSG, PM_REMOVE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
            SWP_SHOWWINDOW, SW_HIDE, ULW_ALPHA, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
            WS_EX_TRANSPARENT, WS_POPUP,
        },
    },
};

// Mismos colores que la interfaz (src/lib/tema.css): negro y carmesí; el rojo marca la mejor carta.
const ROJO: u32 = 0xe5132b;
const ROJO_2: u32 = 0xff3448;
const NEGRO: u32 = 0x070707;
const PANEL: u32 = 0x0d0d0e;
const BLANCO: u32 = 0xffffff;
const SUAVE: u32 = 0xc9c9ce;
const GRIS: u32 = 0x8a8a90;
const AVISO: u32 = 0xffb020;
const COLOR_TIER: [u32; 4] = [0xff3448, 0xff7a88, 0xececef, 0xa3a3a9];

/// Textos de las etiquetas.
pub struct Textos {
    pub tiers: [&'static str; 4],
    pub mala: &'static str,
    pub sin_datos: &'static str,
    pub para: &'static str,
    pub mejor: &'static str,
    pub elige: &'static str,
    pub cambiala: &'static str,
}

pub const ES: Textos = Textos {
    tiers: ["EXCELENTE", "MUY BUENA", "BUENA", "REGULAR"],
    mala: "MALA",
    sin_datos: "POCO USADA",
    para: "para",
    mejor: "MEJOR OPCIÓN",
    elige: "ELIGE ESTA",
    cambiala: "cámbiala",
};

pub const EN: Textos = Textos {
    tiers: ["EXCELLENT", "GREAT", "GOOD", "OKAY"],
    mala: "BAD",
    sin_datos: "RARE PICK",
    para: "for",
    mejor: "BEST PICK",
    elige: "PICK THIS",
    cambiala: "reroll",
};

/// (palabra, color, letra) de un tier de OP.GG.
pub fn calidad(t: &Textos, tier: Option<u8>) -> (&'static str, u32, &'static str) {
    const LETRAS: [&str; 7] = ["S", "A", "B", "C", "D", "E", "F"];
    match tier {
        None => (t.sin_datos, GRIS, "?"),
        Some(n) if (n as usize) < 4 => (t.tiers[n as usize], COLOR_TIER[n as usize], LETRAS[n as usize]),
        Some(n) => (t.mala, GRIS, LETRAS[(n as usize).min(6)]),
    }
}

fn color(rgb: u32, a: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: ((rgb >> 16) & 0xff) as f32 / 255.0,
        g: ((rgb >> 8) & 0xff) as f32 / 255.0,
        b: (rgb & 0xff) as f32 / 255.0,
        a,
    }
}

/// Transparencia de toda la capa (0 = invisible, 255 = opaca) sobre los píxeles ya premultiplicados.
fn mezcla(alfa: u8) -> BLENDFUNCTION {
    BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: alfa, AlphaFormat: AC_SRC_ALPHA as u8 }
}

fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> D2D_RECT_F {
    D2D_RECT_F { left: x0, top: y0, right: x1, bottom: y1 }
}

unsafe extern "system" fn procedimiento(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    DefWindowProcW(h, m, w, l)
}

/// Un trozo de texto de una píldora.
struct Trozo<'a> {
    texto: &'a str,
    tam: f32,
    peso: DWRITE_FONT_WEIGHT,
    color: u32,
}

fn trozo(texto: &str, tam: f32, peso: DWRITE_FONT_WEIGHT, color: u32) -> Trozo<'_> {
    Trozo { texto, tam, peso, color }
}

pub struct Capa {
    hwnd: HWND,
    d2d: ID2D1Factory,
    dw: IDWriteFactory,
    ancho: i32,
    alto: i32,
}

impl Capa {
    pub fn nueva(ancho: i32, alto: i32) -> Result<Capa> {
        unsafe {
            let instancia = GetModuleHandleW(None)?;
            let clase = WNDCLASSW {
                lpfnWndProc: Some(procedimiento),
                hInstance: instancia.into(),
                lpszClassName: w!("XyraCapa"),
                ..Default::default()
            };
            RegisterClassW(&clase);
            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
                w!("XyraCapa"),
                w!("Xyra capa"),
                WS_POPUP,
                0,
                0,
                ancho,
                alto,
                None,
                None,
                Some(instancia.into()),
                None,
            )?;
            let d2d: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let dw: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            Ok(Capa { hwnd, d2d, dw, ancho, alto })
        }
    }

    /// Procesa los mensajes de la ventana (hay que llamarlo seguido desde el hilo que la creó).
    pub fn bombear(&self) {
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    pub fn ocultar(&self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    /// Dibuja las etiquetas en un mapa de bits del tamaño de la pantalla y se lo pasa a `usar`
    /// (DC de la pantalla, DC del mapa y sus píxeles BGRA premultiplicados). Libera todo al terminar.
    fn con_dibujo<R>(&self, cartas: &[Carta], campeon: &str, config: &Config, t: &Textos, usar: impl FnOnce(HDC, HDC, &[u8]) -> R) -> Result<R> {
        unsafe {
            let pantalla = GetDC(None);
            let dc = CreateCompatibleDC(Some(pantalla));
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: self.ancho,
                    biHeight: -self.alto,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits = null_mut();
            let bmp: HBITMAP = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0)?;
            let viejo = SelectObject(dc, HGDIOBJ(bmp.0));
            let dibujado = self.dibujar(dc, cartas, campeon, config, t);
            let r = dibujado.map(|_| usar(pantalla, dc, std::slice::from_raw_parts(bits as *const u8, (self.ancho * self.alto * 4) as usize)));
            SelectObject(dc, viejo);
            let _ = DeleteObject(HGDIOBJ(bmp.0));
            let _ = DeleteDC(dc);
            ReleaseDC(None, pantalla);
            r
        }
    }

    /// Dibuja las etiquetas y muestra la capa por encima del juego sin quitarle el foco, con un fundido corto.
    pub fn mostrar(&self, cartas: &[Carta], campeon: &str, config: &Config, t: &Textos) -> Result<()> {
        let ok = self.con_dibujo(cartas, campeon, config, t, |pantalla, dc, _| unsafe {
            UpdateLayeredWindow(
                self.hwnd,
                Some(pantalla),
                Some(&POINT { x: 0, y: 0 }),
                Some(&SIZE { cx: self.ancho, cy: self.alto }),
                Some(dc),
                Some(&POINT { x: 0, y: 0 }),
                COLORREF(0),
                Some(&mezcla(0)),
                ULW_ALPHA,
            )
            .is_ok()
        })?;
        // una ventana en capas que nunca se dibujó es invisible: si algo falló no se muestra nada
        if ok {
            unsafe {
                let _ = SetWindowPos(self.hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOSIZE | SWP_NOMOVE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
                for alfa in [70u8, 140, 205, 255] {
                    let _ = UpdateLayeredWindow(self.hwnd, None, None, None, None, None, COLORREF(0), Some(&mezcla(alfa)), ULW_ALPHA);
                    thread::sleep(Duration::from_millis(25));
                }
            }
        }
        Ok(())
    }

    /// Las etiquetas como imagen (BGRA premultiplicado), sin mostrar nada: para las vistas previas de los estilos.
    pub fn imagen(&self, cartas: &[Carta], campeon: &str, config: &Config, t: &Textos) -> Result<Vec<u8>> {
        self.con_dibujo(cartas, campeon, config, t, |_, _, px| px.to_vec())
    }

    fn texto(&self, s: &str, tam: f32, peso: DWRITE_FONT_WEIGHT) -> Result<(IDWriteTextLayout, f32, f32)> {
        unsafe {
            let formato = self.dw.CreateTextFormat(
                w!("Bahnschrift"),
                None,
                peso,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_SEMI_CONDENSED,
                tam,
                w!(""),
            )?;
            let ancho_ancho: Vec<u16> = s.encode_utf16().collect();
            let layout = self.dw.CreateTextLayout(&ancho_ancho, &formato, 4000.0, 400.0)?;
            let mut m = DWRITE_TEXT_METRICS::default();
            layout.GetMetrics(&mut m)?;
            Ok((layout, m.widthIncludingTrailingWhitespace, m.height))
        }
    }

    fn dibujar(&self, dc: HDC, cartas: &[Carta], campeon: &str, config: &Config, t: &Textos) -> Result<()> {
        unsafe {
            let propiedades = D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED },
                dpiX: 96.0,
                dpiY: 96.0,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };
            let rt: ID2D1DCRenderTarget = self.d2d.CreateDCRenderTarget(&propiedades)?;
            rt.BindDC(dc, &RECT { left: 0, top: 0, right: self.ancho, bottom: self.alto })?;
            rt.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
            rt.BeginDraw();
            rt.Clear(Some(&color(0, 0.0)));
            let mut l = Lienzo { rt: &rt, capa: self, base: self.alto as f32 / 1200.0 };
            for c in cartas {
                l.carta(c, campeon, config, t)?;
            }
            rt.EndDraw(None, None)
        }
    }
}

/// Dibujo de una carta. Medidas tomadas de capturas reales a 1920x1200 y escaladas con el alto de la pantalla.
struct Lienzo<'a> {
    rt: &'a ID2D1DCRenderTarget,
    capa: &'a Capa,
    base: f32,
}

/// Rectángulo con dos esquinas cortadas en diagonal (arriba a la derecha y abajo a la izquierda), como en la interfaz.
fn corte(r: D2D_RECT_F, c: f32) -> [(f32, f32); 6] {
    [(r.left, r.top), (r.right - c, r.top), (r.right, r.top + c), (r.right, r.bottom), (r.left + c, r.bottom), (r.left, r.bottom - c)]
}

fn rombo(cx: f32, cy: f32, r: f32) -> [(f32, f32); 4] {
    [(cx, cy - r), (cx + r, cy), (cx, cy + r), (cx - r, cy)]
}

/// Texto blanco sobre el rojo y el gris; oscuro sobre los colores claros.
fn tinta(fondo: u32) -> u32 {
    if matches!(fondo, ROJO | ROJO_2 | GRIS) {
        BLANCO
    } else {
        NEGRO
    }
}

impl Lienzo<'_> {
    fn relleno(&self, r: D2D_RECT_F, radio: f32, rgb: u32, a: f32) -> Result<()> {
        unsafe {
            let pincel = self.rt.CreateSolidColorBrush(&color(rgb, a), None)?;
            self.rt.FillRoundedRectangle(&D2D1_ROUNDED_RECT { rect: r, radiusX: radio, radiusY: radio }, &pincel);
        }
        Ok(())
    }

    fn borde(&self, r: D2D_RECT_F, radio: f32, rgb: u32, a: f32, grosor: f32) -> Result<()> {
        unsafe {
            let pincel = self.rt.CreateSolidColorBrush(&color(rgb, a), None)?;
            self.rt.DrawRoundedRectangle(&D2D1_ROUNDED_RECT { rect: r, radiusX: radio, radiusY: radio }, &pincel, grosor, None);
        }
        Ok(())
    }

    /// Polígono relleno, con borde opcional (color, grosor).
    fn poligono(&self, puntos: &[(f32, f32)], rgb: u32, a: f32, borde: Option<(u32, f32)>) -> Result<()> {
        unsafe {
            let g = self.capa.d2d.CreatePathGeometry()?;
            let s = g.Open()?;
            s.BeginFigure(Vector2 { X: puntos[0].0, Y: puntos[0].1 }, D2D1_FIGURE_BEGIN_FILLED);
            for &(x, y) in &puntos[1..] {
                s.AddLine(Vector2 { X: x, Y: y });
            }
            s.EndFigure(D2D1_FIGURE_END_CLOSED);
            s.Close()?;
            self.rt.FillGeometry(&g, &self.rt.CreateSolidColorBrush(&color(rgb, a), None)?, None);
            if let Some((b, grosor)) = borde {
                self.rt.DrawGeometry(&g, &self.rt.CreateSolidColorBrush(&color(b, 1.0), None)?, grosor, None);
            }
        }
        Ok(())
    }

    /// Sombra dura desplazada hacia abajo (sin filtros: barato de dibujar).
    fn sombra(&self, puntos: &[(f32, f32)], k: f32) -> Result<()> {
        for (d, a) in [(4.0, 0.18), (2.0, 0.28)] {
            let movido: Vec<(f32, f32)> = puntos.iter().map(|&(x, y)| (x, y + d * k)).collect();
            self.poligono(&movido, 0, a, None)?;
        }
        Ok(())
    }

    /// Texto centrado verticalmente en `cy`, empezando en `x` (o centrado en `x` si `centrar`). Devuelve el ancho.
    fn escribir(&self, x: f32, cy: f32, t: &Trozo, centrar: bool) -> Result<f32> {
        let (layout, ancho, alto) = self.capa.texto(t.texto, t.tam, t.peso)?;
        let x = if centrar { x - ancho / 2.0 } else { x };
        unsafe {
            let pincel = self.rt.CreateSolidColorBrush(&color(t.color, 1.0), None)?;
            self.rt.DrawTextLayout(Vector2 { X: x, Y: cy - alto / 2.0 }, &layout, &pincel, D2D1_DRAW_TEXT_OPTIONS_NONE);
        }
        Ok(ancho)
    }

    /// Rombo con la letra del tier (o un número) centrado en (cx, cy).
    fn gema(&self, cx: f32, cy: f32, r: f32, texto: &str, fondo: u32, borde: Option<(u32, f32)>) -> Result<()> {
        self.poligono(&rombo(cx, cy, r), fondo, 1.0, borde)?;
        self.escribir(cx, cy, &trozo(texto, r * 0.95, DWRITE_FONT_WEIGHT_EXTRA_BOLD, tinta(fondo)), true)?;
        Ok(())
    }

    /// Placa angular centrada en (cx, cy): rombo opcional con la letra del tier + trozos de texto.
    #[allow(clippy::too_many_arguments)]
    fn placa(&self, cx: f32, cy: f32, k: f32, alto: f32, trozos: &[Trozo], fondo: (u32, f32), borde: Option<u32>, gema: Option<(&str, u32)>) -> Result<()> {
        let h = alto * k;
        let rg = (alto - 10.0) * k / 2.0;
        let anchos: Vec<f32> = trozos.iter().map(|t| self.capa.texto(t.texto, t.tam, t.peso).map(|x| x.1)).collect::<Result<_>>()?;
        let ancho = anchos.iter().sum::<f32>() + 26.0 * k + if gema.is_some() { 2.0 * rg + 8.0 * k } else { 0.0 };
        let forma = corte(rect(cx - ancho / 2.0, cy - h / 2.0, cx + ancho / 2.0, cy + h / 2.0), h * 0.3);
        self.sombra(&forma, k)?;
        self.poligono(&forma, fondo.0, fondo.1, borde.map(|b| (b, 2.0 * k)))?;
        let mut x = cx - ancho / 2.0 + 13.0 * k;
        if let Some((letra, c)) = gema {
            x = cx - ancho / 2.0 + 7.0 * k;
            self.gema(x + rg, cy, rg, letra, c, None)?;
            x += 2.0 * rg + 9.0 * k;
        }
        for (t, w) in trozos.iter().zip(anchos) {
            self.escribir(x, cy, t, false)?;
            x += w;
        }
        Ok(())
    }

    /// Marco rojo con brillo alrededor de la carta elegida (sigue la forma redondeada de la carta del juego).
    fn marco(&self, x0: f32, y0: f32, x1: f32, y1: f32, k: f32) -> Result<()> {
        let m = rect(x0 - 5.0 * k, y0 - 5.0 * k, x1 + 5.0 * k, y1 + 5.0 * k);
        for (grosor, a) in [(16.0, 0.10), (10.0, 0.18), (6.0, 0.30)] {
            self.borde(m, 16.0 * k, ROJO, a, grosor * k)?;
        }
        self.borde(m, 16.0 * k, ROJO_2, 1.0, 3.0 * k)
    }

    fn carta(&mut self, c: &Carta, campeon: &str, config: &Config, t: &Textos) -> Result<()> {
        let k = self.base * config.escala as f32;
        let x = c.x as f32;
        let y = c.y as f32 + config.offset_y as f32 * self.base; // pie del nombre de la carta
        let (x0, y0, x1, y1) = (x - 175.0 * k, y - 285.0 * k, x + 175.0 * k, y + 290.0 * k);
        let (palabra, col, letra) = calidad(t, c.tier);
        let para = format!(" {} {campeon}", t.para);
        let b = DWRITE_FONT_WEIGHT_BOLD;
        let n = DWRITE_FONT_WEIGHT_NORMAL;
        let texto_placa = || [trozo(palabra, 16.0 * k, b, col), trozo(&para, 14.0 * k, n, SUAVE)];
        let cuerpo = (PANEL, 0.94);
        let borde = Some(if c.mejor { ROJO_2 } else { 0x2a2a2e });
        let mejor = format!("★ {}", t.mejor);

        match config.estilo.as_str() {
            "insignia" => {
                if c.mejor {
                    self.marco(x0, y0, x1, y1, k)?;
                }
                let (cx, cy, r) = (x1 - 14.0 * k, y0 + 14.0 * k, 34.0 * k);
                self.sombra(&rombo(cx, cy, r), k)?;
                self.gema(cx, cy, r, letra, col, Some((if c.mejor { ROJO_2 } else { NEGRO }, 3.0 * k)))?;
                let etiqueta = if c.mejor { format!("★ {palabra}") } else { palabra.to_string() };
                let (fondo, tinta_txt) = if c.mejor { ((ROJO, 1.0), BLANCO) } else { (cuerpo, col) };
                self.placa(cx, cy + r + 16.0 * k, k, 28.0, &[trozo(&etiqueta, 14.0 * k, b, tinta_txt)], fondo, None, None)?;
            }
            "cinta" => {
                if c.mejor {
                    self.marco(x0, y0, x1, y1, k)?;
                }
                let (h, cy) = (38.0 * k, y1 - 62.0 * k);
                let fondo = if c.mejor { ROJO } else { col };
                let forma = corte(rect(x0 + 22.0 * k, cy - h / 2.0, x1 - 22.0 * k, cy + h / 2.0), 10.0 * k);
                self.sombra(&forma, k)?;
                self.poligono(&forma, fondo, 0.95, None)?;
                let principal = if c.mejor { format!("{mejor} · {palabra}") } else { palabra.to_string() };
                let a = self.capa.texto(&principal, 15.0 * k, b)?.1;
                let s = self.capa.texto(&para, 13.0 * k, n)?.1;
                let inicio = x - (a + s) / 2.0;
                self.escribir(inicio, cy, &trozo(&principal, 15.0 * k, b, tinta(fondo)), false)?;
                self.escribir(inicio + a, cy, &trozo(&para, 13.0 * k, n, tinta(fondo)), false)?;
            }
            "podio" => {
                if c.mejor {
                    self.marco(x0, y0, x1, y1, k)?;
                }
                // 1.º rojo, 2.º blanco, 3.º gris
                let medalla = match c.puesto {
                    1 => ROJO_2,
                    2 => 0xececef,
                    3 => 0xa3a3a9,
                    _ => GRIS,
                };
                let (r, cy) = (32.0 * k, y0 - 2.0 * k);
                self.sombra(&rombo(x, cy, r), k)?;
                self.gema(x, cy, r, &c.puesto.to_string(), medalla, Some((NEGRO, 3.0 * k)))?;
                self.placa(x, y1 + 5.0 * k, k, 36.0, &texto_placa(), cuerpo, borde, Some((letra, col)))?;
            }
            "enfoque" => {
                if c.mejor {
                    self.marco(x0, y0, x1, y1, k)?;
                    let fino = format!(" · {}", palabra.to_lowercase());
                    let trozos = [trozo("★ ", 15.0 * k, b, BLANCO), trozo(t.elige, 15.0 * k, b, BLANCO), trozo(&fino, 13.0 * k, n, BLANCO)];
                    self.placa(x, y1 + 5.0 * k, k, 34.0, &trozos, (ROJO, 1.0), None, None)?;
                } else {
                    self.relleno(rect(x0, y0, x1, y1), 14.0 * k, 0, 0.62)?;
                    self.placa(x, y1 + 5.0 * k, k, 28.0, &[trozo(palabra, 13.0 * k, b, col)], cuerpo, None, Some((letra, col)))?;
                }
            }
            _ => {
                // "placa": placa bajo cada carta; la mejor con marco rojo y pestaña arriba
                if c.mejor {
                    self.marco(x0, y0, x1, y1, k)?;
                    self.placa(x, y0 - 5.0 * k, k, 30.0, &[trozo(&mejor, 15.0 * k, b, BLANCO)], (ROJO, 1.0), None, None)?;
                }
                let cambiar = format!("  ↻ {}", t.cambiala);
                let mut trozos: Vec<Trozo> = texto_placa().into_iter().collect();
                if c.cambiar {
                    trozos.push(trozo(&cambiar, 14.0 * k, b, AVISO));
                }
                self.placa(x, y1 + 5.0 * k, k, 36.0, &trozos, cuerpo, borde, Some((letra, col)))?;
            }
        }
        Ok(())
    }
}
