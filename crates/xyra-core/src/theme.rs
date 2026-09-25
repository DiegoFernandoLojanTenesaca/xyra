use crate::model::Quality;

pub mod tokens {
    include!(concat!(env!("OUT_DIR"), "/tokens.rs"));
}

pub fn quality_color(quality: Quality) -> u32 {
    match quality {
        Quality::Excellent => tokens::QUALITY_EXCELLENT,
        Quality::Great => tokens::QUALITY_GREAT,
        Quality::Good => tokens::QUALITY_GOOD,
        Quality::Fair => tokens::QUALITY_FAIR,
        Quality::Bad => tokens::QUALITY_BAD,
        Quality::RarePick => tokens::QUALITY_RARE_PICK,
    }
}

pub fn podium_color(place: u32) -> u32 {
    match place {
        1 => tokens::PODIUM_1,
        2 => tokens::PODIUM_2,
        3 => tokens::PODIUM_3,
        _ => tokens::PODIUM_OTHER,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_color_tokens_through_aliases() {
        assert_eq!(tokens::COLOR_ACCENT, 0xe5132b);
        assert_eq!(quality_color(Quality::Excellent), tokens::COLOR_ACCENT_BRIGHT);
        assert_eq!(podium_color(2), quality_color(Quality::Good));
        assert_eq!(podium_color(7), tokens::QUALITY_BAD);
        assert_eq!(tokens::COLOR_TEXT_ON_LIGHT, tokens::COLOR_BACKGROUND);
        assert_eq!(tokens::FONT_NATIVE, "Bahnschrift");
    }
}
