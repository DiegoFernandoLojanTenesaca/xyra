use std::{fs, path::PathBuf};
use xyra_core::league::Installation;

const RIOT_METADATA: &str = r"C:\ProgramData\Riot Games\Metadata\league_of_legends.live\league_of_legends.live.product_settings.yaml";
const DEFAULT_INSTALL_DIR: &str = r"C:\Riot Games\League of Legends";
const DEFAULT_LOCALE: &str = "en_US";
const INSTALL_DIR_KEY: &str = "product_install_full_path:";
const LOCALE_KEY: &str = "locale:";

fn quoted(line: &str) -> Option<String> {
    let start = line.find('"')? + 1;
    let length = line[start..].find('"')?;
    Some(line[start..start + length].to_string())
}

/// Where League is installed and its language, from the Riot Client metadata.
pub fn find() -> Installation {
    let text = fs::read_to_string(RIOT_METADATA).unwrap_or_default();
    let dir = text.lines().find(|line| line.starts_with(INSTALL_DIR_KEY)).and_then(quoted);
    let locale = text.lines().find(|line| line.starts_with(' ') && line.trim_start().starts_with(LOCALE_KEY)).and_then(quoted);
    Installation { dir: PathBuf::from(dir.unwrap_or_else(|| DEFAULT_INSTALL_DIR.into())), locale: locale.unwrap_or_else(|| DEFAULT_LOCALE.into()) }
}
