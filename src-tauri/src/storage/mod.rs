//! 学習者データ（`{app_data_dir}`）のファイルの読み書き（spec/architecture/data-locations.md）。
//!
//! - 書き込みは、同じディレクトリの一時ファイルに書いてからリネームする（[`write`]）。
//! - 同じファイルへの読み書きは、[`AppDataDir`] ごとのファイル単位のロックで直列化する。
//! - `schema_version` を持つ JSON は [`AppDataDir::read_versioned_json`] で読む。読み込めないときは
//!   `.broken-{日時}` を付けて退避し、初期状態で作り直す（[`versioned`]）。
//! - 下書き・試行ログのように、無い状態が初期状態であるファイルは [`AppDataDir::read_optional_versioned_json`]
//!   で読む。読み込めないときは退避し、作り直さずに、無いものとして扱う。

mod error;
mod versioned;
mod write;

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tauri::{AppHandle, Manager, Runtime};

pub use error::StorageError;
pub use versioned::Loaded;

/// 学習者データのディレクトリ。ファイルの読み書きは、すべてこれを通して行う。
///
/// アプリでは [`AppDataDir::from_app`] で Tauri の `app_data_dir()` から作り、Tauri の状態（`manage`）に
/// 1 つだけ置く。テストでは [`AppDataDir::new`] にテストごとの一時ディレクトリを渡す。
/// 作るときにはディレクトリを作らない。書き込むときに、要るディレクトリを作る。
#[derive(Debug)]
pub struct AppDataDir {
    root: PathBuf,
    /// ファイルのパスごとのロック。同じファイルへの読み書きを直列化する。
    locks: Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>,
}

impl AppDataDir {
    /// `root` を学習者データのディレクトリとして使う。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            locks: Mutex::new(HashMap::new()),
        }
    }

    /// Tauri の `app_data_dir()` を学習者データのディレクトリとして使う。
    pub fn from_app<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Self> {
        Ok(Self::new(app.path().app_data_dir()?))
    }

    /// 学習者データのディレクトリのパス。
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `{app_data_dir}` からの相対パス（例: `progress.json`、`drafts/ch01-01.json`）を、ファイルのパスにする。
    /// 相対パスは、ディレクトリの外を指せないように、名前（`..` や `/` で始まるものでない）だけでできていなければならない。
    pub fn path(&self, relative: impl AsRef<Path>) -> Result<PathBuf, StorageError> {
        let relative = relative.as_ref();
        let is_plain = relative.components().next().is_some()
            && relative
                .components()
                .all(|component| matches!(component, Component::Normal(_)));
        if !is_plain {
            return Err(StorageError::InvalidPath {
                path: relative.to_path_buf(),
            });
        }
        Ok(self.root.join(relative))
    }

    /// `path` のファイルのロックを取る。ロックを持っている間、同じファイルを読み書きするほかのスレッドは待つ。
    fn lock_file(&self, path: &Path) -> FileLock {
        let lock = Arc::clone(
            lock_ignoring_poison(&self.locks)
                .entry(path.to_path_buf())
                .or_default(),
        );
        FileLock { lock }
    }
}

/// 1 つのファイルのロック。[`FileLock::hold`] で取る。
struct FileLock {
    lock: Arc<Mutex<()>>,
}

impl FileLock {
    fn hold(&self) -> MutexGuard<'_, ()> {
        lock_ignoring_poison(&self.lock)
    }
}

/// ほかのスレッドがロックを持ったままパニックしても、ロックを取る。
/// ロックが守るのはファイルへの操作の順番だけで、壊れて困るメモリ上の値は無い。
fn lock_ignoring_poison<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_joins_relative_paths_under_the_root() {
        let dir = AppDataDir::new("/data");
        assert_eq!(
            dir.path("progress.json").unwrap(),
            Path::new("/data/progress.json")
        );
        assert_eq!(
            dir.path("drafts/ch01-01.json").unwrap(),
            Path::new("/data/drafts/ch01-01.json")
        );
    }

    #[test]
    fn path_rejects_paths_that_leave_the_root() {
        let dir = AppDataDir::new("/data");
        for relative in [
            "",
            "/etc/passwd",
            "../progress.json",
            "drafts/../x.json",
            "./x.json",
        ] {
            assert!(
                matches!(dir.path(relative), Err(StorageError::InvalidPath { .. })),
                "{relative:?} は受け付けない"
            );
        }
    }

    #[test]
    fn the_same_file_shares_one_lock() {
        let dir = AppDataDir::new("/data");
        let path = dir.path("progress.json").unwrap();
        let first = dir.lock_file(&path);
        let second = dir.lock_file(&path);
        assert!(Arc::ptr_eq(&first.lock, &second.lock));
        let other = dir.lock_file(&dir.path("settings.json").unwrap());
        assert!(!Arc::ptr_eq(&first.lock, &other.lock));
    }
}
