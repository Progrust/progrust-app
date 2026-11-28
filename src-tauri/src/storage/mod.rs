//! 学習者データ（`{app_data_dir}`）のファイルの読み書き（spec/architecture/data-locations.md）。
//!
//! - 書き込みは、同じディレクトリの一時ファイルに書いてからリネームする（[`write`]）。
//! - 同じファイルへの読み書きは、[`AppDataDir`] ごとのファイル単位のロックで直列化する。
//! - `schema_version` を持つ JSON は [`AppDataDir::read_versioned_json`] で読む。読み込めないときは
//!   `.broken-{日時}` を付けて退避し、初期状態で作り直す（[`versioned`]）。古い版を変換するファイルは、
//!   変換の関数を渡して [`AppDataDir::read_migrated_json`] で読む。
//! - 下書き・試行ログのように、無い状態が初期状態であるファイルは [`AppDataDir::read_optional_versioned_json`]
//!   で読む。読み込めないときは退避し、作り直さずに、無いものとして扱う。
//! - `{app_data_dir}` の外にもありうるファイル（作業領域の `prepared.json` など）は、パスを渡して
//!   [`write_json_file`] で書き、読んだ内容を [`parse_versioned_json`] で読む。

mod error;
mod versioned;
mod write;

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tauri::{AppHandle, Manager, Runtime};

pub use error::StorageError;
pub use versioned::{Loaded, Migration, parse_versioned_json};
pub use write::write_json_file;

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

    /// `{app_data_dir}` からの相対パス `relative` のディレクトリにある `.json` のファイルの、拡張子を除いた名前を、
    /// 名前の順に返す。`.` で始まる名前のファイル（一時ファイルなど）と、退避したファイル（`.json.broken-…`）は含めない。
    /// ディレクトリが無いときは空を返す（ディレクトリは作らない）。
    pub fn list_json_stems(&self, relative: impl AsRef<Path>) -> Result<Vec<String>, StorageError> {
        let dir = self.path(relative)?;
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(StorageError::io(&dir, error)),
        };
        let mut stems = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| StorageError::io(&dir, error))?;
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            if name.starts_with('.') {
                continue;
            }
            let Some(stem) = name.strip_suffix(".json") else {
                continue;
            };
            let is_file = entry
                .file_type()
                .map_err(|error| StorageError::io(entry.path(), error))?
                .is_file();
            if is_file && !stem.is_empty() {
                stems.push(stem.to_string());
            }
        }
        stems.sort();
        Ok(stems)
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
    fn list_json_stems_returns_only_json_files_in_name_order() {
        let temp = tempfile::tempdir().unwrap();
        let dir = AppDataDir::new(temp.path());
        assert_eq!(
            dir.list_json_stems("attempts").unwrap(),
            Vec::<String>::new()
        );
        assert!(!temp.path().join("attempts").exists());

        for name in [
            "b.json",
            "a.json",
            ".a.json.123.tmp",
            ".hidden.json",
            "c.json.broken-20261123T142500",
            "notes.md",
        ] {
            dir.write_text(format!("attempts/{name}"), "{}").unwrap();
        }
        std::fs::create_dir(temp.path().join("attempts/d.json")).unwrap();
        assert_eq!(dir.list_json_stems("attempts").unwrap(), ["a", "b"]);
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
