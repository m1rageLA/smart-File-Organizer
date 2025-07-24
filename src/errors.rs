use thiserror::Error;
use std::path::PathBuf;

#[derive(Error, Debug)]
pub enum OrganizerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serde error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Permission denied for path: {0}")]
    PermissionDenied(PathBuf),

    #[error("File already exists at destination: {0}")]
    DestinationExists(PathBuf),

    #[error("Other error: {0}")]
    Other(String),
}
