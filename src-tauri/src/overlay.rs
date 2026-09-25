use std::{
    mem::size_of,
    ptr::null_mut,
    sync::mpsc::{self, Sender},
    thread,
    time::Duration,
};
use windows::{
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM},
        Graphics::{
            Direct2D::{
                Common::{D2D_RECT_F, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_FIGURE_BEGIN_FILLED, D2D1_FIGURE_END_CLOSED, D2D1_PIXEL_FORMAT},
                D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT, D2D1_RENDER_TARGET_PROPERTIES,
                D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE, D2D1_ROUNDED_RECT, D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE, D2D1CreateFactory,
                ID2D1DCRenderTarget, ID2D1Factory,
            },
            DirectWrite::{
                DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_SEMI_CONDENSED, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT, DWRITE_FONT_WEIGHT_BOLD,
                DWRITE_FONT_WEIGHT_EXTRA_BOLD, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_TEXT_METRICS, DWriteCreateFactory, IDWriteFactory, IDWriteTextLayout,
            },
            Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
            Gdi::{
                AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC,
                DeleteObject, GetDC, HBITMAP, HDC, HGDIOBJ, ReleaseDC, SelectObject,
            },
        },
        System::{LibraryLoader::GetModuleHandleW, Threading::GetCurrentThreadId},
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, HWND_TOPMOST, KillTimer, MSG, PostThreadMessageW, RegisterClassW, SW_HIDE,
            SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SetTimer, SetWindowPos, ShowWindow, ULW_ALPHA, UpdateLayeredWindow, WM_APP, WM_TIMER,
            WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
        },
    },
    core::{HSTRING, Result, w},
};
use windows_numerics::Vector2;
use xyra_core::{
    cards::Card,
    config::{Config, LabelStyle},
    i18n, theme,
};

/// Card measurements taken from real captures at 1920x1200, scaled by the screen height.
const REFERENCE_HEIGHT: f32 = 1200.0;
const CARD_HALF_WIDTH: f32 = 175.0;
const CARD_ABOVE_NAME: f32 = 285.0;
const CARD_BELOW_NAME: f32 = 290.0;
const CARD_RADIUS: f32 = 16.0;
const FRAME_GAP: f32 = 5.0;
const FRAME_GLOW: [(f32, f32); 3] = [(16.0, 0.10), (10.0, 0.18), (6.0, 0.30)];
const FRAME_WIDTH: f32 = 3.0;
const PLATE_HEIGHT: f32 = 36.0;
const TAB_HEIGHT: f32 = 30.0;
const BADGE_RADIUS: f32 = 34.0;
const BADGE_INSET: f32 = 14.0;
const BADGE_PLATE_HEIGHT: f32 = 28.0;
const BADGE_PLATE_GAP: f32 = 16.0;
const RIBBON_HEIGHT: f32 = 38.0;
const RIBBON_BOTTOM: f32 = 62.0;
const RIBBON_INSET: f32 = 22.0;
const RIBBON_CUT: f32 = 10.0;
const MEDAL_RADIUS: f32 = 32.0;
const FOCUS_PLATE_HEIGHT: f32 = 34.0;
const FOCUS_DIM_PLATE_HEIGHT: f32 = 28.0;
const FOCUS_DIM: f32 = 0.62;
const TEXT_LARGE: f32 = 16.0;
const TEXT_MEDIUM: f32 = 15.0;
const TEXT_SMALL: f32 = 14.0;
const TEXT_TINY: f32 = 13.0;
const PANEL_ALPHA: f32 = 0.94;
const RIBBON_ALPHA: f32 = 0.95;
const PLATE_PADDING: f32 = 13.0;
const PLATE_GEM_INSET: f32 = 5.0;
const PLATE_GEM_PADDING: f32 = 7.0;
const PLATE_GEM_GAP: f32 = 9.0;
const PLATE_GEM_ROOM: f32 = 8.0;
const PLATE_CUT_RATIO: f32 = 0.3;
const PLATE_STROKE: f32 = 2.0;
const GEM_STROKE: f32 = 3.0;
const GEM_TEXT_RATIO: f32 = 0.95;
const MEDAL_LIFT: f32 = 2.0;
const PODIUM_PLACES: u32 = 3;
const SHADOW: [(f32, f32); 2] = [(4.0, 0.18), (2.0, 0.28)];
const FADE_STEPS: [u8; 4] = [70, 140, 205, 255];
const FADE_STEP: Duration = Duration::from_millis(25);
const MAX_LAYOUT: (f32, f32) = (4000.0, 400.0);
const DEMO_TIMER: usize = 1;
const DEMO_DURATION_MS: u32 = 5000;

/// Texts drawn over the cards, in one language.
pub struct OverlayTexts {
    language: &'static str,
}

impl OverlayTexts {
    pub fn new(language: &'static str) -> OverlayTexts {
        OverlayTexts { language }
    }

    fn get(&self, key: &str) -> String {
        i18n::t(self.language, key)
    }
}

fn d2d_color(rgb: u32, alpha: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F { r: ((rgb >> 16) & 0xff) as f32 / 255.0, g: ((rgb >> 8) & 0xff) as f32 / 255.0, b: (rgb & 0xff) as f32 / 255.0, a: alpha }
}

fn blend(alpha: u8) -> BLENDFUNCTION {
    BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: alpha, AlphaFormat: AC_SRC_ALPHA as u8 }
}

fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> D2D_RECT_F {
    D2D_RECT_F { left: x0, top: y0, right: x1, bottom: y1 }
}

unsafe extern "system" fn window_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

struct Span<'a> {
    text: &'a str,
    size: f32,
    weight: DWRITE_FONT_WEIGHT,
    color: u32,
}

fn span(text: &str, size: f32, weight: DWRITE_FONT_WEIGHT, color: u32) -> Span<'_> {
    Span { text, size, weight, color }
}

/// Click-through, never-focused layered window drawn with Direct2D.
pub struct Overlay {
    hwnd: HWND,
    d2d: ID2D1Factory,
    dwrite: IDWriteFactory,
    font: HSTRING,
    width: i32,
    height: i32,
}

impl Overlay {
    pub fn new(width: i32, height: i32) -> Result<Overlay> {
        unsafe {
            let instance = GetModuleHandleW(None)?;
            let class = WNDCLASSW { lpfnWndProc: Some(window_proc), hInstance: instance.into(), lpszClassName: w!("XyraOverlay"), ..Default::default() };
            RegisterClassW(&class);
            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
                w!("XyraOverlay"),
                w!("Xyra overlay"),
                WS_POPUP,
                0,
                0,
                width,
                height,
                None,
                None,
                Some(instance.into()),
                None,
            )?;
            let d2d: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let dwrite: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            let font = HSTRING::from(theme::token("font.native").unwrap_or_default());
            Ok(Overlay { hwnd, d2d, dwrite, font, width, height })
        }
    }

    pub fn hide(&self) {
        unsafe {
            let _was_visible = ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    /// Draws the labels into a screen-sized bitmap and hands `use_bitmap` the screen DC, the bitmap DC and its premultiplied BGRA pixels.
    fn with_drawing<R>(
        &self,
        cards: &[Card],
        champion: &str,
        config: &Config,
        texts: &OverlayTexts,
        use_bitmap: impl FnOnce(HDC, HDC, &[u8]) -> R,
    ) -> Result<R> {
        unsafe {
            let screen = GetDC(None);
            let dc = CreateCompatibleDC(Some(screen));
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: self.width,
                    biHeight: -self.height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits = null_mut();
            let bitmap: HBITMAP = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0)?;
            let previous = SelectObject(dc, HGDIOBJ(bitmap.0));
            let drawn = self.draw(dc, cards, champion, config, texts);
            let result = drawn.map(|_| use_bitmap(screen, dc, std::slice::from_raw_parts(bits as *const u8, (self.width * self.height * 4) as usize)));
            SelectObject(dc, previous);
            DeleteObject(HGDIOBJ(bitmap.0)).ok()?;
            DeleteDC(dc).ok()?;
            ReleaseDC(None, screen);
            result
        }
    }

    /// Shows the labels above the game without taking focus, with a short fade.
    pub fn show(&self, cards: &[Card], champion: &str, config: &Config, texts: &OverlayTexts) -> Result<()> {
        let updated = self.with_drawing(cards, champion, config, texts, |screen, dc, _| unsafe {
            UpdateLayeredWindow(
                self.hwnd,
                Some(screen),
                Some(&POINT { x: 0, y: 0 }),
                Some(&SIZE { cx: self.width, cy: self.height }),
                Some(dc),
                Some(&POINT { x: 0, y: 0 }),
                COLORREF(0),
                Some(&blend(0)),
                ULW_ALPHA,
            )
            .is_ok()
        })?;
        if updated {
            unsafe {
                SetWindowPos(self.hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOSIZE | SWP_NOMOVE | SWP_NOACTIVATE | SWP_SHOWWINDOW)?;
                for alpha in FADE_STEPS {
                    UpdateLayeredWindow(self.hwnd, None, None, None, None, None, COLORREF(0), Some(&blend(alpha)), ULW_ALPHA)?;
                    thread::sleep(FADE_STEP);
                }
            }
        }
        Ok(())
    }

    /// The labels as premultiplied BGRA pixels, without showing anything (for the style previews).
    pub fn render(&self, cards: &[Card], champion: &str, config: &Config, texts: &OverlayTexts) -> Result<Vec<u8>> {
        self.with_drawing(cards, champion, config, texts, |_, _, pixels| pixels.to_vec())
    }

    fn layout(&self, text: &str, size: f32, weight: DWRITE_FONT_WEIGHT) -> Result<(IDWriteTextLayout, f32, f32)> {
        unsafe {
            let format = self.dwrite.CreateTextFormat(&self.font, None, weight, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_SEMI_CONDENSED, size, w!(""))?;
            let utf16: Vec<u16> = text.encode_utf16().collect();
            let layout = self.dwrite.CreateTextLayout(&utf16, &format, MAX_LAYOUT.0, MAX_LAYOUT.1)?;
            let mut metrics = DWRITE_TEXT_METRICS::default();
            layout.GetMetrics(&mut metrics)?;
            Ok((layout, metrics.widthIncludingTrailingWhitespace, metrics.height))
        }
    }

    fn draw(&self, dc: HDC, cards: &[Card], champion: &str, config: &Config, texts: &OverlayTexts) -> Result<()> {
        unsafe {
            let properties = D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED },
                dpiX: 96.0,
                dpiY: 96.0,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };
            let target: ID2D1DCRenderTarget = self.d2d.CreateDCRenderTarget(&properties)?;
            target.BindDC(dc, &RECT { left: 0, top: 0, right: self.width, bottom: self.height })?;
            target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
            target.BeginDraw();
            target.Clear(Some(&d2d_color(0, 0.0)));
            let painter = Painter { target: &target, overlay: self, unit: self.height as f32 / REFERENCE_HEIGHT };
            for card in cards {
                painter.card(card, champion, config, texts)?;
            }
            target.EndDraw(None, None)
        }
    }
}

/// An angular plate: where, how tall at the reference height, what it says and how it looks.
struct Plate<'a> {
    center: (f32, f32),
    height: f32,
    spans: &'a [Span<'a>],
    background: (u32, f32),
    stroke: Option<u32>,
    gem: Option<(&'a str, u32)>,
}

struct Painter<'a> {
    target: &'a ID2D1DCRenderTarget,
    overlay: &'a Overlay,
    unit: f32,
}

/// Rectangle with its top-right and bottom-left corners cut diagonally, like the app panels.
fn cut_corners(r: D2D_RECT_F, cut: f32) -> [(f32, f32); 6] {
    [(r.left, r.top), (r.right - cut, r.top), (r.right, r.top + cut), (r.right, r.bottom), (r.left + cut, r.bottom), (r.left, r.bottom - cut)]
}

fn diamond(cx: f32, cy: f32, r: f32) -> [(f32, f32); 4] {
    [(cx, cy - r), (cx + r, cy), (cx, cy + r), (cx - r, cy)]
}

struct Palette {
    accent: u32,
    accent_bright: u32,
    ink: u32,
    panel: u32,
    white: u32,
    subtle: u32,
    muted: u32,
    warning: u32,
    plate_border: u32,
}

fn palette() -> Palette {
    Palette {
        accent: theme::color("color.accent"),
        accent_bright: theme::color("color.accentBright"),
        ink: theme::color("color.textOnLight"),
        panel: theme::color("color.panel"),
        white: theme::color("color.white"),
        subtle: theme::color("color.textSubtle"),
        muted: theme::color("color.textMuted"),
        warning: theme::color("color.warning"),
        plate_border: theme::color("color.line"),
    }
}

impl Painter<'_> {
    fn fill_rounded(&self, r: D2D_RECT_F, radius: f32, rgb: u32, alpha: f32) -> Result<()> {
        unsafe {
            let brush = self.target.CreateSolidColorBrush(&d2d_color(rgb, alpha), None)?;
            self.target.FillRoundedRectangle(&D2D1_ROUNDED_RECT { rect: r, radiusX: radius, radiusY: radius }, &brush);
        }
        Ok(())
    }

    fn stroke_rounded(&self, r: D2D_RECT_F, radius: f32, rgb: u32, alpha: f32, width: f32) -> Result<()> {
        unsafe {
            let brush = self.target.CreateSolidColorBrush(&d2d_color(rgb, alpha), None)?;
            self.target.DrawRoundedRectangle(&D2D1_ROUNDED_RECT { rect: r, radiusX: radius, radiusY: radius }, &brush, width, None);
        }
        Ok(())
    }

    fn polygon(&self, points: &[(f32, f32)], rgb: u32, alpha: f32, stroke: Option<(u32, f32)>) -> Result<()> {
        unsafe {
            let geometry = self.overlay.d2d.CreatePathGeometry()?;
            let sink = geometry.Open()?;
            sink.BeginFigure(Vector2 { X: points[0].0, Y: points[0].1 }, D2D1_FIGURE_BEGIN_FILLED);
            for &(x, y) in &points[1..] {
                sink.AddLine(Vector2 { X: x, Y: y });
            }
            sink.EndFigure(D2D1_FIGURE_END_CLOSED);
            sink.Close()?;
            self.target.FillGeometry(&geometry, &self.target.CreateSolidColorBrush(&d2d_color(rgb, alpha), None)?, None);
            if let Some((color, width)) = stroke {
                self.target.DrawGeometry(&geometry, &self.target.CreateSolidColorBrush(&d2d_color(color, 1.0), None)?, width, None);
            }
        }
        Ok(())
    }

    fn shadow(&self, points: &[(f32, f32)], k: f32) -> Result<()> {
        for (offset, alpha) in SHADOW {
            let moved: Vec<(f32, f32)> = points.iter().map(|&(x, y)| (x, y + offset * k)).collect();
            self.polygon(&moved, 0, alpha, None)?;
        }
        Ok(())
    }

    /// Draws `span` vertically centered on `cy`, starting at `x` (or centered on `x`). Returns its width.
    fn text(&self, x: f32, cy: f32, span: &Span, centered: bool) -> Result<f32> {
        let (layout, width, height) = self.overlay.layout(span.text, span.size, span.weight)?;
        let x = if centered { x - width / 2.0 } else { x };
        unsafe {
            let brush = self.target.CreateSolidColorBrush(&d2d_color(span.color, 1.0), None)?;
            self.target.DrawTextLayout(Vector2 { X: x, Y: cy - height / 2.0 }, &layout, &brush, D2D1_DRAW_TEXT_OPTIONS_NONE);
        }
        Ok(width)
    }

    fn ink_for(&self, background: u32, colors: &Palette) -> u32 {
        if [colors.accent, colors.accent_bright, colors.muted].contains(&background) { colors.white } else { colors.ink }
    }

    fn gem(&self, (cx, cy): (f32, f32), r: f32, label: &str, background: u32, stroke: Option<(u32, f32)>, colors: &Palette) -> Result<()> {
        self.polygon(&diamond(cx, cy, r), background, 1.0, stroke)?;
        self.text(cx, cy, &span(label, r * GEM_TEXT_RATIO, DWRITE_FONT_WEIGHT_EXTRA_BOLD, self.ink_for(background, colors)), true)?;
        Ok(())
    }

    /// Angular plate centered on `plate.center` with an optional tier gem and a row of spans.
    fn plate(&self, plate: Plate, k: f32, colors: &Palette) -> Result<()> {
        let (cx, cy) = plate.center;
        let h = plate.height * k;
        let gem_radius = h / 2.0 - PLATE_GEM_INSET * k;
        let widths: Vec<f32> = plate.spans.iter().map(|s| self.overlay.layout(s.text, s.size, s.weight).map(|l| l.1)).collect::<Result<_>>()?;
        let gem_width = if plate.gem.is_some() { 2.0 * gem_radius + PLATE_GEM_ROOM * k } else { 0.0 };
        let width = widths.iter().sum::<f32>() + 2.0 * PLATE_PADDING * k + gem_width;
        let shape = cut_corners(rect(cx - width / 2.0, cy - h / 2.0, cx + width / 2.0, cy + h / 2.0), h * PLATE_CUT_RATIO);
        self.shadow(&shape, k)?;
        self.polygon(&shape, plate.background.0, plate.background.1, plate.stroke.map(|s| (s, PLATE_STROKE * k)))?;
        let mut x = cx - width / 2.0 + PLATE_PADDING * k;
        if let Some((label, color)) = plate.gem {
            let gem_x = cx - width / 2.0 + PLATE_GEM_PADDING * k + gem_radius;
            self.gem((gem_x, cy), gem_radius, label, color, None, colors)?;
            x = gem_x + gem_radius + PLATE_GEM_GAP * k;
        }
        for (s, w) in plate.spans.iter().zip(widths) {
            self.text(x, cy, s, false)?;
            x += w;
        }
        Ok(())
    }

    fn highlight_frame(&self, x0: f32, y0: f32, x1: f32, y1: f32, k: f32, colors: &Palette) -> Result<()> {
        let frame = rect(x0 - FRAME_GAP * k, y0 - FRAME_GAP * k, x1 + FRAME_GAP * k, y1 + FRAME_GAP * k);
        for (width, alpha) in FRAME_GLOW {
            self.stroke_rounded(frame, CARD_RADIUS * k, colors.accent, alpha, width * k)?;
        }
        self.stroke_rounded(frame, CARD_RADIUS * k, colors.accent_bright, 1.0, FRAME_WIDTH * k)
    }

    fn card(&self, card: &Card, champion: &str, config: &Config, texts: &OverlayTexts) -> Result<()> {
        let colors = palette();
        let k = self.unit * config.scale as f32;
        let x = card.x as f32;
        let y = card.y as f32 + config.offset_y as f32 * self.unit;
        let (x0, y0, x1, y1) = (x - CARD_HALF_WIDTH * k, y - CARD_ABOVE_NAME * k, x + CARD_HALF_WIDTH * k, y + CARD_BELOW_NAME * k);
        let quality_key = i18n::variant_key(card.quality);
        let quality = texts.get(&format!("common:quality.{quality_key}"));
        let quality_color = theme::color(&format!("quality.{quality_key}"));
        let for_champion = format!(" {}", i18n::t_with(texts.language, "overlay:forChampion", &[("champion", champion)]));
        let best_pick = texts.get("overlay:bestPick");
        let (bold, regular) = (DWRITE_FONT_WEIGHT_BOLD, DWRITE_FONT_WEIGHT_NORMAL);
        let body = (colors.panel, PANEL_ALPHA);
        let stroke = Some(if card.best { colors.accent_bright } else { colors.plate_border });
        let plate_spans = || [span(&quality, TEXT_LARGE * k, bold, quality_color), span(&for_champion, TEXT_SMALL * k, regular, colors.subtle)];
        let below = (x, y1 + FRAME_GAP * k);

        match config.label_style {
            LabelStyle::Badge => {
                if card.best {
                    self.highlight_frame(x0, y0, x1, y1, k, &colors)?;
                }
                let (cx, cy, r) = (x1 - BADGE_INSET * k, y0 + BADGE_INSET * k, BADGE_RADIUS * k);
                self.shadow(&diamond(cx, cy, r), k)?;
                let ring = if card.best { colors.accent_bright } else { colors.ink };
                self.gem((cx, cy), r, &card.grade, quality_color, Some((ring, GEM_STROKE * k)), &colors)?;
                let (background, ink) = if card.best { ((colors.accent, 1.0), colors.white) } else { (body, quality_color) };
                let spans = [span(&quality, TEXT_SMALL * k, bold, ink)];
                self.plate(
                    Plate { center: (cx, cy + r + BADGE_PLATE_GAP * k), height: BADGE_PLATE_HEIGHT, spans: &spans, background, stroke: None, gem: None },
                    k,
                    &colors,
                )?;
            }
            LabelStyle::Ribbon => {
                if card.best {
                    self.highlight_frame(x0, y0, x1, y1, k, &colors)?;
                }
                let (h, cy) = (RIBBON_HEIGHT * k, y1 - RIBBON_BOTTOM * k);
                let background = if card.best { colors.accent } else { quality_color };
                let shape = cut_corners(rect(x0 + RIBBON_INSET * k, cy - h / 2.0, x1 - RIBBON_INSET * k, cy + h / 2.0), RIBBON_CUT * k);
                self.shadow(&shape, k)?;
                self.polygon(&shape, background, RIBBON_ALPHA, None)?;
                let main = if card.best { format!("{best_pick} · {quality}") } else { quality.clone() };
                let main_width = self.overlay.layout(&main, TEXT_MEDIUM * k, bold)?.1;
                let rest_width = self.overlay.layout(&for_champion, TEXT_TINY * k, regular)?.1;
                let start = x - (main_width + rest_width) / 2.0;
                let ink = self.ink_for(background, &colors);
                self.text(start, cy, &span(&main, TEXT_MEDIUM * k, bold, ink), false)?;
                self.text(start + main_width, cy, &span(&for_champion, TEXT_TINY * k, regular, ink), false)?;
            }
            LabelStyle::Podium => {
                if card.best {
                    self.highlight_frame(x0, y0, x1, y1, k, &colors)?;
                }
                let medal = theme::color(&format!("podium.{}", if card.rank <= PODIUM_PLACES { card.rank.to_string() } else { "other".into() }));
                let (r, cy) = (MEDAL_RADIUS * k, y0 - MEDAL_LIFT * k);
                self.shadow(&diamond(x, cy, r), k)?;
                self.gem((x, cy), r, &card.rank.to_string(), medal, Some((colors.ink, GEM_STROKE * k)), &colors)?;
                let spans = plate_spans();
                self.plate(
                    Plate { center: below, height: PLATE_HEIGHT, spans: &spans, background: body, stroke, gem: Some((&card.grade, quality_color)) },
                    k,
                    &colors,
                )?;
            }
            LabelStyle::Focus => {
                if card.best {
                    self.highlight_frame(x0, y0, x1, y1, k, &colors)?;
                    let pick_this = texts.get("overlay:pickThis");
                    let detail = format!(" · {}", quality.to_lowercase());
                    let spans = [span(&pick_this, TEXT_MEDIUM * k, bold, colors.white), span(&detail, TEXT_TINY * k, regular, colors.white)];
                    self.plate(
                        Plate { center: below, height: FOCUS_PLATE_HEIGHT, spans: &spans, background: (colors.accent, 1.0), stroke: None, gem: None },
                        k,
                        &colors,
                    )?;
                } else {
                    self.fill_rounded(rect(x0, y0, x1, y1), BADGE_INSET * k, 0, FOCUS_DIM)?;
                    let spans = [span(&quality, TEXT_TINY * k, bold, quality_color)];
                    let gem = Some((card.grade.as_str(), quality_color));
                    self.plate(Plate { center: below, height: FOCUS_DIM_PLATE_HEIGHT, spans: &spans, background: body, stroke: None, gem }, k, &colors)?;
                }
            }
            LabelStyle::Plate => {
                if card.best {
                    self.highlight_frame(x0, y0, x1, y1, k, &colors)?;
                    let spans = [span(&best_pick, TEXT_MEDIUM * k, bold, colors.white)];
                    let tab =
                        Plate { center: (x, y0 - FRAME_GAP * k), height: TAB_HEIGHT, spans: &spans, background: (colors.accent, 1.0), stroke: None, gem: None };
                    self.plate(tab, k, &colors)?;
                }
                let reroll = format!("  {}", texts.get("common:reroll"));
                let mut spans: Vec<Span> = plate_spans().into_iter().collect();
                if card.reroll {
                    spans.push(span(&reroll, TEXT_SMALL * k, bold, colors.warning));
                }
                self.plate(
                    Plate { center: below, height: PLATE_HEIGHT, spans: &spans, background: body, stroke, gem: Some((&card.grade, quality_color)) },
                    k,
                    &colors,
                )?;
            }
        }
        Ok(())
    }
}

enum Command {
    Show(Labels),
    Demo(Labels),
    Hide,
}

/// What to draw: the rated cards, for whom, how and in which language.
pub struct Labels {
    pub cards: Vec<Card>,
    pub champion: String,
    pub config: Config,
    pub language: &'static str,
}

/// The overlay on its own thread, which sleeps in its message loop until it gets a command.
pub struct OverlayHandle {
    commands: Sender<Command>,
    thread: u32,
}

impl OverlayHandle {
    pub fn spawn(width: i32, height: i32, report: impl Fn(windows::core::Error) + Send + 'static) -> Result<OverlayHandle> {
        let (commands, received) = mpsc::channel::<Command>();
        let (ready, started) = mpsc::channel();
        thread::spawn(move || {
            let created = Overlay::new(width, height).map(|overlay| (overlay, unsafe { GetCurrentThreadId() }));
            let overlay = match created {
                Ok((overlay, thread)) if ready.send(Ok(thread)).is_ok() => overlay,
                Ok(_) => return,
                Err(error) => return ready.send(Err(error)).unwrap_or_default(),
            };
            let mut message = MSG::default();
            while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
                let outcome = match message.message {
                    WM_APP => received.try_iter().try_for_each(|command| overlay.apply(command)),
                    WM_TIMER => overlay.end_demo(),
                    _ => {
                        unsafe { DispatchMessageW(&message) };
                        Ok(())
                    }
                };
                if let Err(error) = outcome {
                    report(error);
                }
            }
        });
        let thread = started.recv().map_err(|_| windows::core::Error::from_thread())??;
        Ok(OverlayHandle { commands, thread })
    }

    fn send(&self, command: Command) {
        if self.commands.send(command).is_ok() {
            unsafe { PostThreadMessageW(self.thread, WM_APP, WPARAM(0), LPARAM(0)) }.ok();
        }
    }

    pub fn show(&self, labels: Labels) {
        self.send(Command::Show(labels));
    }

    /// Shows the labels for a few seconds, to try the style outside a game.
    pub fn demo(&self, labels: Labels) {
        self.send(Command::Demo(labels));
    }

    pub fn hide(&self) {
        self.send(Command::Hide);
    }
}

impl Overlay {
    fn apply(&self, command: Command) -> Result<()> {
        match command {
            Command::Show(labels) => {
                self.stop_demo_timer();
                self.show(&labels.cards, &labels.champion, &labels.config, &OverlayTexts::new(labels.language))
            }
            Command::Demo(labels) => {
                self.show(&labels.cards, &labels.champion, &labels.config, &OverlayTexts::new(labels.language))?;
                match unsafe { SetTimer(Some(self.hwnd), DEMO_TIMER, DEMO_DURATION_MS, None) } {
                    0 => Err(windows::core::Error::from_thread()),
                    _ => Ok(()),
                }
            }
            Command::Hide => {
                self.stop_demo_timer();
                self.hide();
                Ok(())
            }
        }
    }

    fn end_demo(&self) -> Result<()> {
        self.hide();
        unsafe { KillTimer(Some(self.hwnd), DEMO_TIMER) }
    }

    /// Cancels the timer that ends a demo.
    fn stop_demo_timer(&self) {
        let _no_timer_running = unsafe { KillTimer(Some(self.hwnd), DEMO_TIMER) };
    }
}
