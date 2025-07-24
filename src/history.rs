use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use crate::errors::OrganizerError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovedFile {
    pub from: PathBuf,
    pub to: PathBuf,
    pub time: DateTime<Utc>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct History {
    pub moves: Vec<MovedFile>,
}

pub struct HistoryManager {
    path: PathBuf,
}

impl HistoryManager {
    pub fn new(path: PathBuf) -> Self { Self { path } }

    pub fn load(&self) -> Result<History, OrganizerError> {
        if !self.path.exists() {
            return Ok(History::default());
        }
        let data = fs::read_to_string(&self.path)?;
        Ok(serde_json::from_str(&data)?)
    }

    pub fn save(&self, history: &History) -> Result<(), OrganizerError> {
        fs::write(&self.path, serde_json::to_string_pretty(history)?)?;
        Ok(())
    }

    pub fn push(&self, moved: MovedFile) -> Result<(), OrganizerError> {
        let mut history = self.load()?;
        history.moves.push(moved);
        self.save(&history)
    }

    pub fn pop_last(&self) -> Result<Option<MovedFile>, OrganizerError> {
        let mut history = self.load()?;
        let res = history.moves.pop();
        self.save(&history)?;
        Ok(res)
    }

    pub fn take_all(&self) -> Result<Vec<MovedFile>, OrganizerError> {
        let mut history = self.load()?;
        let res = std::mem::take(&mut history.moves);
        self.save(&history)?;
        Ok(res)
    }
}
