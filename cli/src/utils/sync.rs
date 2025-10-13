use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};

#[derive(Default)]
struct PathLockRegistry {
    inner: Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>,
}

static REGISTRY: OnceLock<PathLockRegistry> = OnceLock::new();

pub(crate) fn with_path_lock<F, T>(path: &Path, f: F) -> T
where
    F: FnOnce() -> T,
{
    REGISTRY
        .get_or_init(PathLockRegistry::default)
        .with_lock(path, f)
}

impl PathLockRegistry {
    fn with_lock<F, T>(&self, path: &Path, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let path_buf = path.to_path_buf();
        let lock = {
            let mut map = self
                .inner
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            map.entry(path_buf.clone())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };
        let guard = lock.lock().unwrap_or_else(|poison| poison.into_inner());
        let result = f();
        drop(guard);
        let mut map = self
            .inner
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if Arc::strong_count(&lock) == 1 {
            if let Some(current) = map.get(&path_buf) {
                if Arc::ptr_eq(current, &lock) {
                    map.remove(&path_buf);
                }
            }
        }
        result
    }
}
