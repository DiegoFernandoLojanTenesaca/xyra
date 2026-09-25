use windows::{
    core::{Result, HSTRING},
    Media::{Core::MediaSource, Playback::MediaPlayer, SpeechSynthesis::SpeechSynthesizer},
};
use xyra_core::{cards::Card, i18n};

const SIDES: [&str; 3] = ["overlay:voice.left", "overlay:voice.middle", "overlay:voice.right"];

pub struct Voice {
    synthesizer: SpeechSynthesizer,
    player: MediaPlayer,
}

impl Voice {
    pub fn new() -> Option<Voice> {
        Some(Voice { synthesizer: SpeechSynthesizer::new().ok()?, player: MediaPlayer::new().ok()? })
    }

    /// Uses the first installed voice of `language`.
    pub fn speak(&self, text: &str, language: &str) -> Result<()> {
        for voice in SpeechSynthesizer::AllVoices()? {
            if voice.Language()?.to_string().to_lowercase().starts_with(language) {
                self.synthesizer.SetVoice(&voice)?;
                break;
            }
        }
        let audio = self.synthesizer.SynthesizeTextToStreamAsync(&HSTRING::from(text))?.join()?;
        self.player.SetSource(&MediaSource::CreateFromStream(&audio, &audio.ContentType()?)?)?;
        self.player.Play()
    }
}

/// Phrase naming where the best card is, from left to right.
pub fn best_card_phrase(cards: &[Card], language: &str) -> Option<String> {
    let mut ordered: Vec<&Card> = cards.iter().collect();
    ordered.sort_by(|a, b| a.x.total_cmp(&b.x));
    let index = ordered.iter().position(|c| c.best)?;
    let side = if ordered.len() == 2 { [0, 2][index] } else { index.min(2) };
    Some(i18n::t(language, SIDES[side]))
}
