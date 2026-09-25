use crate::{
    catalog::Catalog,
    config::Config,
    errors::{AppError, Result},
    profile::Profile,
    stats::StoredGame,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use ts_rs::TS;

const CONFIG: &str = "config.json";
const STATS: &str = "stats.json";
const CATALOG: &str = "catalog.json";
const PROFILE: &str = "profile.json";
const LOG: &str = "xyra.log";
const SCREENSHOTS: &str = "screenshots";
const TEMPORARY_EXTENSION: &str = "tmp";
const CORRUPT_EXTENSION: &str = "corrupt";
const MAX_LOG_BYTES: u64 = 1_000_000;
const LEGACY_CACHES: [&str; 2] = ["catalogo.json", "perfil.json"];
const LEGACY_SCREENSHOTS: &str = "capturas";
const LEGACY_AUTO_LANGUAGE: &str = "auto";
/// Version 0.1.0 stored dates cut to minutes, without the UTC mark.
const LEGACY_DATE_LENGTH: usize = 16;
const LEGACY_DATE_SUFFIX: &str = ":00.000Z";

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum DataFolder {
    Data,
    Screenshots,
    Log,
}

#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct DataUsage {
    pub folder: String,
    #[ts(type = "number")]
    pub bytes: u64,
}

/// Every file Xyra keeps, inside the app data folder.
pub struct Storage {
    dir: PathBuf,
}

impl Storage {
    pub fn open(dir: PathBuf) -> Result<Storage> {
        fs::create_dir_all(&dir)?;
        let storage = Storage { dir };
        storage.migrate_legacy_files()?;
        if fs::metadata(storage.file(LOG)).is_ok_and(|m| m.len() > MAX_LOG_BYTES) {
            fs::remove_file(storage.file(LOG))?;
        }
        Ok(storage)
    }

    fn migrate_legacy_files(&self) -> Result<()> {
        for cache in LEGACY_CACHES.map(|name| self.file(name)).iter().filter(|path| path.exists()) {
            fs::remove_file(cache)?;
        }
        let (legacy, screenshots) = (self.file(LEGACY_SCREENSHOTS), self.file(SCREENSHOTS));
        if legacy.exists() && !screenshots.exists() {
            fs::rename(legacy, screenshots)?;
        }
        Ok(())
    }

    fn file(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    /// Path of `folder`, creating the screenshots folder the first time.
    pub fn folder(&self, folder: DataFolder) -> Result<PathBuf> {
        let path = match folder {
            DataFolder::Data => self.dir.clone(),
            DataFolder::Screenshots => self.file(SCREENSHOTS),
            DataFolder::Log => self.file(LOG),
        };
        if folder == DataFolder::Screenshots {
            fs::create_dir_all(&path)?;
        }
        Ok(path)
    }

    /// None when the file does not exist; an unreadable file is renamed to `.corrupt`.
    fn read<T: DeserializeOwned>(&self, name: &str) -> Result<Option<T>> {
        let path = self.file(name);
        let Ok(text) = fs::read_to_string(&path) else { return Ok(None) };
        serde_json::from_str(&text).map(Some).map_err(|error| match fs::rename(&path, path.with_extension(CORRUPT_EXTENSION)) {
            Ok(()) => AppError::Storage(format!("{name}: {error}")),
            Err(rename) => AppError::Storage(format!("{name}: {error}; {rename}")),
        })
    }

    /// Reads a cache, deleting it when its format is outdated.
    fn read_cache<T: DeserializeOwned>(&self, name: &str) -> Result<Option<T>> {
        let path = self.file(name);
        let Ok(text) = fs::read_to_string(&path) else { return Ok(None) };
        match serde_json::from_str(&text) {
            Ok(value) => Ok(Some(value)),
            Err(_) => Ok(fs::remove_file(&path).map(|()| None)?),
        }
    }

    fn write<T: Serialize>(&self, name: &str, value: &T) -> Result<()> {
        let path = self.file(name);
        let temporary = path.with_extension(TEMPORARY_EXTENSION);
        fs::write(&temporary, serde_json::to_string_pretty(value)?)?;
        Ok(fs::rename(temporary, path)?)
    }

    pub fn load_config(&self) -> Result<Config> {
        let mut config: Config = self.read(CONFIG)?.unwrap_or_default();
        if config.language.as_deref() == Some(LEGACY_AUTO_LANGUAGE) {
            config.language = None;
        }
        Ok(config)
    }

    pub fn save_config(&self, config: &Config) -> Result<()> {
        self.write(CONFIG, config)
    }

    pub fn load_games(&self) -> Result<Vec<StoredGame>> {
        let mut games: Vec<StoredGame> = self.read(STATS)?.unwrap_or_default();
        for game in games.iter_mut().filter(|g| g.date.len() == LEGACY_DATE_LENGTH) {
            game.date.push_str(LEGACY_DATE_SUFFIX);
        }
        Ok(games)
    }

    pub fn save_games(&self, games: &[StoredGame]) -> Result<()> {
        self.write(STATS, &games)
    }

    pub fn load_catalog(&self) -> Result<Option<Catalog>> {
        self.read_cache(CATALOG)
    }

    pub fn save_catalog(&self, catalog: &Catalog) -> Result<()> {
        self.write(CATALOG, catalog)
    }

    pub fn load_profile(&self) -> Result<Option<Profile>> {
        self.read_cache(PROFILE)
    }

    pub fn save_profile(&self, profile: &Profile) -> Result<()> {
        self.write(PROFILE, profile)
    }

    /// Where the data lives and the bytes its files use.
    pub fn usage(&self) -> DataUsage {
        fn size(path: &Path) -> u64 {
            fs::read_dir(path)
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| entry.metadata().map_or(0, |m| if m.is_dir() { size(&entry.path()) } else { m.len() }))
                .sum()
        }
        DataUsage { folder: self.dir.to_string_lossy().into(), bytes: size(&self.dir) }
    }

    /// Deletes the games, profile, screenshots and log; settings and the game data catalog stay.
    pub fn delete_personal_data(&self) -> Result<()> {
        for path in [STATS, PROFILE, LOG].map(|name| self.file(name)).iter().filter(|path| path.exists()) {
            fs::remove_file(path)?;
        }
        let screenshots = self.file(SCREENSHOTS);
        if screenshots.exists() {
            fs::remove_dir_all(screenshots)?;
        }
        Ok(())
    }

    pub fn screenshot_path(&self) -> Result<PathBuf> {
        let dir = self.folder(DataFolder::Screenshots)?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
        Ok(dir.join(format!("{now}.png")))
    }

    /// Appends a failure a human must look at to the log.
    pub fn log_error(&self, context: &str, error: impl std::fmt::Display) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let written = OpenOptions::new().create(true).append(true).open(self.file(LOG)).and_then(|mut log| writeln!(log, "{now} {context}: {error}"));
        if let Err(log_error) = written {
            eprintln!("{context}: {error} ({log_error})");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LabelStyle;

    fn storage(name: &str) -> Storage {
        let dir = std::env::temp_dir().join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).unwrap();
        }
        Storage::open(dir).unwrap()
    }

    #[test]
    fn migrates_version_one_files() {
        let storage = storage("xyra-storage-legacy");
        fs::write(storage.file(CONFIG), r#"{"estilo":"cinta","voz":true,"idioma":"auto","auto_runas":true}"#).unwrap();
        fs::write(storage.file(STATS), r#"[{"game_id":1,"fecha":"2026-09-01T20:15","campeon":103,"modo":"KIWI","victoria":true,"aumentos":[]}]"#).unwrap();
        fs::write(storage.file("perfil.json"), "{}").unwrap();
        let storage = Storage::open(storage.dir.clone()).unwrap();
        let config = storage.load_config().unwrap();
        assert_eq!((config.label_style, config.voice, config.language, config.auto_import_runes), (LabelStyle::Ribbon, true, None, true));
        assert_eq!(storage.load_games().unwrap()[0].date, "2026-09-01T20:15:00.000Z");
        assert!(!storage.file("perfil.json").exists());
        fs::write(storage.file(CATALOG), r#"{"rarity":{"1":"kUnknown"}}"#).unwrap();
        assert!(storage.load_catalog().unwrap().is_none());
        assert!(!storage.file(CATALOG).exists());
        fs::remove_dir_all(&storage.dir).unwrap();
    }

    #[test]
    fn sets_aside_unreadable_files_instead_of_overwriting_them() {
        let storage = storage("xyra-storage-corrupt");
        fs::write(storage.file(STATS), "[{broken").unwrap();
        assert!(matches!(storage.load_games(), Err(AppError::Storage(_))));
        assert!(storage.file(STATS).with_extension(CORRUPT_EXTENSION).exists());
        assert!(storage.load_games().unwrap().is_empty());
        storage.save_games(&[]).unwrap();
        assert!(storage.usage().bytes > 0);
        storage.delete_personal_data().unwrap();
        assert!(!storage.file(STATS).exists());
        fs::remove_dir_all(&storage.dir).unwrap();
    }
}
