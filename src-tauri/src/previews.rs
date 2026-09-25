use crate::{
    engine::demo_cards,
    overlay::{Overlay, OverlayTexts},
};
use std::{collections::HashMap, fs, io::BufReader, path::Path};
use xyra_core::{
    config::{Config, LABEL_STYLES},
    i18n,
};

const PREVIEW_CHAMPION: &str = "Brand";

/// Draws every label style over a capture of the cards and saves `<style>.png`, so the Labels page shows the real overlay.
pub fn render_style_previews(background: &Path, output: &Path, language: &str) -> Result<(), String> {
    let file = fs::File::open(background).map_err(|e| e.to_string())?;
    let mut reader = png::Decoder::new(BufReader::new(file)).read_info().map_err(|e| e.to_string())?;
    let mut pixels = vec![0; reader.output_buffer_size().ok_or("png buffer")?];
    let info = reader.next_frame(&mut pixels).map_err(|e| e.to_string())?;
    let channels = info.color_type.samples();
    let (width, height) = (info.width as i32, info.height as i32);
    let overlay = Overlay::new(width, height).map_err(|e| e.to_string())?;
    let cards = demo_cards(&HashMap::new(), width, height);
    let texts = OverlayTexts::new(i18n::resolve(language));
    fs::create_dir_all(output).map_err(|e| e.to_string())?;
    for style in LABEL_STYLES {
        let config = Config { label_style: style.into(), ..Default::default() };
        let labels = overlay.render(&cards, PREVIEW_CHAMPION, &config, &texts).map_err(|e| e.to_string())?;
        let rgb: Vec<u8> = labels
            .chunks_exact(4)
            .zip(pixels.chunks_exact(channels))
            .flat_map(|(label, below)| {
                let rest = 255 - label[3] as u32;
                [2, 1, 0].map(|i| (label[i] as u32 + below[2 - i] as u32 * rest / 255).min(255) as u8)
            })
            .collect();
        let target = fs::File::create(output.join(format!("{style}.png"))).map_err(|e| e.to_string())?;
        let mut encoder = png::Encoder::new(std::io::BufWriter::new(target), width as u32, height as u32);
        encoder.set_color(png::ColorType::Rgb);
        encoder.write_header().and_then(|mut w| w.write_image_data(&rgb)).map_err(|e| e.to_string())?;
    }
    Ok(())
}
