use crate::{
    errors::{AppError, Result},
    web::Client,
};
use ring::digest::{Context, SHA256};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};
use ts_rs::TS;

const LATEST_RELEASE: &str = "https://api.github.com/repos/DiegoFernandoLojanTenesaca/xyra/releases/latest";
const GITHUB_JSON: &str = "application/vnd.github+json";
const INSTALLER_EXTENSION: &str = ".exe";
const SHA256_PREFIX: &str = "sha256:";
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);
const CHUNK_BYTES: usize = 64 * 1024;
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// A published version newer than the running one, with the installer to get it.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Release {
    pub version: String,
    pub notes_url: String,
    #[serde(skip)]
    installer_url: String,
    #[serde(skip)]
    size: u64,
    #[serde(skip)]
    sha256: Option<String>,
}

#[derive(Deserialize)]
struct ReleaseAnswer {
    tag_name: String,
    html_url: String,
    assets: Vec<AssetAnswer>,
}

#[derive(Deserialize)]
struct AssetAnswer {
    name: String,
    browser_download_url: String,
    size: u64,
    digest: Option<String>,
}

fn version_numbers(version: &str) -> Vec<u64> {
    version.trim_start_matches('v').split('.').map(|part| part.parse().unwrap_or(0)).collect()
}

fn is_newer(candidate: &str, current: &str) -> bool {
    version_numbers(candidate) > version_numbers(current)
}

fn newer_release(answer: ReleaseAnswer, current: &str) -> Option<Release> {
    let installer = answer.assets.into_iter().find(|asset| asset.name.to_lowercase().ends_with(INSTALLER_EXTENSION))?;
    is_newer(&answer.tag_name, current).then(|| Release {
        version: answer.tag_name.trim_start_matches('v').to_string(),
        notes_url: answer.html_url,
        installer_url: installer.browser_download_url,
        size: installer.size,
        sha256: installer.digest.and_then(|digest| digest.strip_prefix(SHA256_PREFIX).map(str::to_lowercase)),
    })
}

/// The latest GitHub release when it is newer than this app and ships an installer.
pub fn check(http: &Client) -> Result<Option<Release>> {
    let text = http.get(LATEST_RELEASE).header(reqwest::header::ACCEPT, GITHUB_JSON).send()?.error_for_status()?.text()?;
    let answer: ReleaseAnswer = serde_json::from_str(&text).map_err(|e| AppError::Network(e.to_string()))?;
    Ok(newer_release(answer, CURRENT_VERSION))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Downloads the installer into `dir`, reporting the share done (0-1), and checks it against the published SHA-256.
pub fn download(http: &Client, release: &Release, dir: &Path, mut progress: impl FnMut(f64)) -> Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let path = dir.join(format!("xyra-{}{INSTALLER_EXTENSION}", release.version));
    let mut response = http.get(&release.installer_url).timeout(DOWNLOAD_TIMEOUT).send()?.error_for_status()?;
    let mut file = File::create(&path)?;
    let mut hash = Context::new(&SHA256);
    let mut buffer = vec![0; CHUNK_BYTES];
    let mut received = 0u64;
    loop {
        let read = response.read(&mut buffer).map_err(|e| AppError::Network(e.to_string()))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])?;
        hash.update(&buffer[..read]);
        received += read as u64;
        progress(received as f64 / release.size.max(1) as f64);
    }
    file.flush()?;
    let intact = received == release.size && release.sha256.as_deref().is_none_or(|expected| hex(hash.finish().as_ref()) == expected);
    if !intact {
        fs::remove_file(&path)?;
        return Err(AppError::BadDownload);
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn answer(tag: &str, asset: &str) -> ReleaseAnswer {
        serde_json::from_value(json!({
            "tag_name": tag,
            "html_url": "https://github.com/DiegoFernandoLojanTenesaca/xyra/releases/tag/0.3.0",
            "assets": [
                { "name": "notes.txt", "browser_download_url": "https://example.com/notes.txt", "size": 1, "digest": null },
                { "name": asset, "browser_download_url": "https://example.com/installer", "size": 3, "digest": "sha256:ABC" }
            ]
        }))
        .unwrap()
    }

    #[test]
    fn offers_only_newer_releases_with_an_installer() {
        assert!(is_newer("0.10.0", "0.9.9"));
        assert!(is_newer("v0.3.0", "0.2.0"));
        assert!(!is_newer("0.2.0", "0.2.0"));
        assert!(!is_newer("v0.1.0", "0.2.0"));
        let release = newer_release(answer("0.3.0", "xyra-0.3.0.exe"), "0.2.0").unwrap();
        assert_eq!(
            (release.version.as_str(), release.installer_url.as_str(), release.sha256.as_deref()),
            ("0.3.0", "https://example.com/installer", Some("abc"))
        );
        assert!(newer_release(answer("0.3.0", "xyra-0.3.0.zip"), "0.2.0").is_none());
        assert!(newer_release(answer("0.2.0", "xyra-0.2.0.exe"), "0.2.0").is_none());
    }
}
