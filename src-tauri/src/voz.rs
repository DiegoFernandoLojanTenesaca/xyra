//! Aviso por voz con el sintetizador de Windows ("Elige la de la derecha").
use xyra_core::cartas::Carta;
use windows::{
    core::{Result, HSTRING},
    Media::{Core::MediaSource, Playback::MediaPlayer, SpeechSynthesis::SpeechSynthesizer},
};

pub struct Voz {
    sintetizador: SpeechSynthesizer,
    reproductor: MediaPlayer,
}

impl Voz {
    pub fn nueva() -> Option<Voz> {
        Some(Voz { sintetizador: SpeechSynthesizer::new().ok()?, reproductor: MediaPlayer::new().ok()? })
    }

    /// `idioma`: "es" o "en". Usa la primera voz instalada de ese idioma.
    pub fn decir(&self, texto: &str, idioma: &str) -> Result<()> {
        for v in SpeechSynthesizer::AllVoices()? {
            if v.Language()?.to_string().to_lowercase().starts_with(idioma) {
                self.sintetizador.SetVoice(&v)?;
                break;
            }
        }
        let audio = self.sintetizador.SynthesizeTextToStreamAsync(&HSTRING::from(texto))?.join()?;
        self.reproductor.SetSource(&MediaSource::CreateFromStream(&audio, &audio.ContentType()?)?)?;
        self.reproductor.Play()
    }
}

/// Frase para la mejor carta según su posición de izquierda a derecha.
pub fn frase(cartas: &[Carta], idioma: &str) -> Option<&'static str> {
    let mut orden: Vec<&Carta> = cartas.iter().collect();
    orden.sort_by(|a, b| a.x.total_cmp(&b.x));
    let i = orden.iter().position(|c| c.mejor)?;
    let lado = if orden.len() == 2 { [0, 2][i] } else { i.min(2) };
    let es = ["Elige la de la izquierda", "Elige la del medio", "Elige la de la derecha"];
    let en = ["Pick the left one", "Pick the middle one", "Pick the right one"];
    Some(if idioma == "es" { es[lado] } else { en[lado] })
}
