use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

#[derive(Serialize, Deserialize, Default)]
pub struct Snapshot(pub HashMap<String, u64>);

pub fn load_snapshot(path: &Path) -> io::Result<Snapshot> {
    match fs::read(path) {
        Ok(data) => {
            serde_json::from_slice(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Snapshot::default()),
        Err(e) => Err(e),
    }
}

pub fn save_snapshot(path: &Path, snapshot: &Snapshot) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_vec(snapshot).map_err(io::Error::other)?;
    fs::write(path, data)
}

impl Snapshot {
    pub fn get(&self, path: &Path) -> Option<SystemTime> {
        self.0
            .get(path.to_string_lossy().as_ref())
            .map(|secs| SystemTime::UNIX_EPOCH + Duration::from_secs(*secs))
    }

    pub fn insert(&mut self, path: PathBuf, mtime: SystemTime) {
        if let Ok(duration) = mtime.duration_since(SystemTime::UNIX_EPOCH) {
            self.0
                .insert(path.to_string_lossy().into_owned(), duration.as_secs());
        }
    }
}
