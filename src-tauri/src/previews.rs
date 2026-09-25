use crate::{
    engine::demo_cards,
    overlay::{Overlay, OverlayTexts},
};
use std::{collections::HashMap, fs, io::BufReader, path::Path};
use xyra_core::{
    config::{Config, LabelStyle},
    errors::{AppError, Result},
    i18n,
};

const PREVIEW_CHAMPION: &str = "Brand";

/// Draws every label style over a capture of the cards and saves `<style>.png`.
pub fn render_style_previews(background: &Path, output: &Path, language: &str) -> Result<()> {
    let file = fs::File::open(background)?;
    let mut reader = png::Decoder::new(BufReader::new(file)).read_info().map_err(AppError::platform)?;
    let mut pixels = vec![0; reader.output_buffer_size().ok_or_else(|| AppError::platform("png buffer size"))?];
    let info = reader.next_frame(&mut pixels).map_err(AppError::platform)?;
    let channels = info.color_type.samples();
    let (width, height) = (info.width as i32, info.height as i32);
    let overlay = Overlay::new(width, height).map_err(AppError::platform)?;
    let cards = demo_cards(&HashMap::new(), width, height);
    let texts = OverlayTexts::new(i18n::resolve(language));
    fs::create_dir_all(output)?;
    for style in LabelStyle::ALL {
        let config = Config { label_style: style, ..Default::default() };
        let labels = overlay.render(&cards, PREVIEW_CHAMPION, &config, &texts).map_err(AppError::platform)?;
        let rgb: Vec<u8> = labels
            .as_chunks::<4>()
            .0
            .iter()
            .zip(pixels.chunks_exact(channels))
            .flat_map(|(label, below)| {
                let rest = 255 - label[3] as u32;
                [2, 1, 0].map(|i| (label[i] as u32 + below[2 - i] as u32 * rest / 255).min(255) as u8)
            })
            .collect();
        let target = fs::File::create(output.join(format!("{}.png", i18n::variant_key(style))))?;
        let mut encoder = png::Encoder::new(std::io::BufWriter::new(target), width as u32, height as u32);
        encoder.set_color(png::ColorType::Rgb);
        encoder.write_header().and_then(|mut w| w.write_image_data(&rgb)).map_err(AppError::platform)?;
    }
    Ok(())
}
