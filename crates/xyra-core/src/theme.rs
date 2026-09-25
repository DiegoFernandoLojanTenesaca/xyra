use serde_json::Value;
use std::sync::OnceLock;

fn tokens() -> &'static Value {
    static TOKENS: OnceLock<Value> = OnceLock::new();
    TOKENS.get_or_init(|| serde_json::from_str(include_str!("../../../design/tokens.json")).expect("valid tokens.json"))
}

const MISSING_COLOR: u32 = 0xff00ff;

pub fn token(path: &str) -> Option<&'static str> {
    path.split('.').try_fold(tokens(), |node, part| node.get(part)).and_then(Value::as_str)
}

/// Color token ("color.accent", "quality.excellent"…) as 0xRRGGBB.
pub fn color(path: &str) -> u32 {
    token(path).and_then(|hex| u32::from_str_radix(hex.trim_start_matches('#'), 16).ok()).unwrap_or(MISSING_COLOR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_color_tokens() {
        assert_eq!(color("color.accent"), 0xe5132b);
        assert_eq!(color("quality.excellent"), 0xff3448);
        assert_eq!(color("missing.token"), MISSING_COLOR);
        assert_eq!(token("font.native"), Some("Bahnschrift"));
    }
}
