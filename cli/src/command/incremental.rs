use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

#[derive(Serialize, Deserialize, Default, Clone)]
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
    fn normalize(path: &Path) -> String {
        let s = path.to_string_lossy();
        if path.is_absolute() {
            s.into_owned()
        } else if s == "." {
            "./".to_string()
        } else if s.starts_with("./") {
            s.into_owned()
        } else {
            format!("./{}", s)
        }
    }

    pub fn get(&self, path: &Path) -> Option<SystemTime> {
        let key = Self::normalize(path);
        self.0
            .get(&key)
            .map(|secs| SystemTime::UNIX_EPOCH + Duration::from_secs(*secs))
    }

    pub fn insert(&mut self, path: PathBuf, mtime: SystemTime) {
        if let Ok(duration) = mtime.duration_since(SystemTime::UNIX_EPOCH) {
            let key = Self::normalize(&path);
            self.0.insert(key, duration.as_secs());
        }
    }
}
