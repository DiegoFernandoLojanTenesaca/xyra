use serde::Serialize;
use std::fmt;
use ts_rs::TS;

/// Failure reasons the UI renders as `errors:<code>`.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(tag = "code", content = "detail", rename_all = "camelCase")]
pub enum AppError {
    ClientClosed,
    GameInProgress,
    NoFreeRunePage,
    NotInChampSelect,
    NoSummoner,
    NoData,
    EmptyCatalog,
    BadDownload,
    Network(String),
    OpggFormat(String),
    Client(String),
    ClientFormat(String),
    Storage(String),
    Platform(String),
}

pub type Result<T> = std::result::Result<T, AppError>;

impl AppError {
    pub fn client(error: impl fmt::Display) -> AppError {
        AppError::Client(error.to_string())
    }

    pub fn platform(error: impl fmt::Display) -> AppError {
        AppError::Platform(error.to_string())
    }

    pub fn opgg_format(source: &str, error: impl fmt::Display) -> AppError {
        AppError::OpggFormat(format!("{source}: {error}"))
    }

    pub fn client_format(source: &str, error: impl fmt::Display) -> AppError {
        AppError::ClientFormat(format!("{source}: {error}"))
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for AppError {}

impl From<reqwest::Error> for AppError {
    fn from(error: reqwest::Error) -> AppError {
        AppError::Network(error.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> AppError {
        AppError::Storage(error.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> AppError {
        AppError::Storage(error.to_string())
    }
}
