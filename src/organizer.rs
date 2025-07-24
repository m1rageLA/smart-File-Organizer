use crate::{
    errors::OrganizerError,
    history::{HistoryManager, MovedFile},
    rules::RuleEngine,
};
use chrono::Utc;
use log::{error, info, warn};
use parking_lot::Mutex;
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct OrganizerConfig {
    pub src_dir: PathBuf,
    pub dst_dir: PathBuf,
    pub dry_run: bool,
    pub overwrite: bool,
}

pub struct Organizer<R: RuleEngine + 'static> {
    config: OrganizerConfig,
    rules: Arc<R>,
    history: Arc<HistoryManager>,
    cancel: Arc<AtomicBool>,
    last_error: Arc<Mutex<Option<OrganizerError>>>,
}

impl<R: RuleEngine + 'static> Organizer<R> {
    pub fn new(config: OrganizerConfig, rules: R, history: HistoryManager) -> Self {
        Self {
            config,
            rules: Arc::new(rules),
            history: Arc::new(history),
            cancel: Arc::new(AtomicBool::new(false)),
            last_error: Arc::new(Mutex::new(None)),
        }
    }

    pub fn cancel_handle(&self) -> Arc<AtomicBool> {
        self.cancel.clone()
    }

    pub fn last_error(&self) -> Option<String> {
        self.last_error.lock().as_ref().map(|e| e.to_string())
    }

    pub fn organize(&self) -> Result<(), OrganizerError> {
        if self.config.src_dir == self.config.dst_dir {
            warn!("Source and destination folders are the same, using nested subfolders.");
        }

        for entry in WalkDir::new(&self.config.src_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if self.cancel.load(Ordering::Relaxed) {
                warn!("Operation cancelled by user");
                break;
            }
            let path = entry.path();
            if path.is_dir() {
                continue;
            }

            if let Err(e) = self.process_file(path) {
                error!("Failed to process {:?}: {}", path, e);
                *self.last_error.lock() = Some(e);
            }
        }
        Ok(())
    }

    fn process_file(&self, path: &Path) -> Result<(), OrganizerError> {
        let rel_path = path.strip_prefix(&self.config.src_dir).unwrap_or(path);
        let target_subdir = self.rules.classify(path);
        let target_dir = self.config.dst_dir.join(target_subdir);
        fs::create_dir_all(&target_dir)?;

        let file_name = rel_path.file_name().ok_or_else(|| {
            OrganizerError::Other(format!("Cannot extract filename from {:?}", rel_path))
        })?;

        let mut target_path = target_dir.join(file_name);

        // resolve conflicts
        if target_path.exists() && !self.config.overwrite {
            target_path = self.resolve_conflict(&target_path)?;
        }

        info!("Move: {:?} -> {:?}", path, target_path);

        if !self.config.dry_run {
            move_file(path, &target_path)?;
            self.history.push(MovedFile {
                from: path.to_path_buf(),
                to: target_path.clone(),
                time: Utc::now(),
            })?;
        }
        Ok(())
    }

    fn resolve_conflict(&self, target: &Path) -> Result<PathBuf, OrganizerError> {
        let stem = target
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("file");
        let ext = target.extension().and_then(|s| s.to_str()).unwrap_or("");
        for i in 1.. {
            let candidate = if ext.is_empty() {
                target.with_file_name(format!("{}_({})", stem, i))
            } else {
                target.with_file_name(format!("{}_({}).{}", stem, i, ext))
            };
            if !candidate.exists() {
                return Ok(candidate);
            }
        }
        Err(OrganizerError::Other(
            "Unable to resolve name conflict".into(),
        ))
    }

    pub fn undo_last(&self) -> Result<(), OrganizerError> {
        if let Some(mov) = self.history.pop_last()? {
            info!("Undo: {:?} -> {:?}", mov.to, mov.from);
            if mov.to.exists() {
                move_file(&mov.to, &mov.from)?;
            } else {
                warn!("Destination file missing: {:?}", mov.to);
            }
        } else {
            warn!("Nothing to undo");
        }
        Ok(())
    }

    pub fn undo_all(&self) -> Result<(), OrganizerError> {
        let mut moves = self.history.take_all()?;
        moves.reverse();
        for mov in moves {
            info!("Undo: {:?} -> {:?}", mov.to, mov.from);
            if mov.to.exists() {
                move_file(&mov.to, &mov.from)?;
            } else {
                warn!("Destination file missing: {:?}", mov.to);
            }
        }
        Ok(())
    }
}

fn move_file(from: &Path, to: &Path) -> io::Result<()> {
    match fs::rename(from, to) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
            fs::copy(from, to)?;
            fs::remove_file(from)?;
            Ok(())
        }
        Err(e) => Err(e),
    }
}
