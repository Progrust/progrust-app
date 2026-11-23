//! ファイルの書き込み（spec/architecture/data-locations.md「書き込みの規則」）。
//!
//! 一時ファイルに全部を書いてから、元のファイルの名前にリネームする。同じファイルシステムの中のリネームは
//! 一度に入れ替わるので、書き込みの途中でプロセスが止まっても、元のファイルは書き込む前のまま残る。

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{AppDataDir, StorageError};

impl AppDataDir {
    /// `{app_data_dir}` からの相対パス `relative` のファイルに、文字列（Markdown など）を書く。
    pub fn write_text(
        &self,
        relative: impl AsRef<Path>,
        contents: &str,
    ) -> Result<(), StorageError> {
        let path = self.path(relative)?;
        let lock = self.lock_file(&path);
        let _guard = lock.hold();
        write_atomic(&path, contents.as_bytes())
    }

    /// `{app_data_dir}` からの相対パス `relative` のファイルに、`value` を JSON で書く。
    pub fn write_json<T: Serialize + ?Sized>(
        &self,
        relative: impl AsRef<Path>,
        value: &T,
    ) -> Result<(), StorageError> {
        let path = self.path(relative)?;
        let bytes = to_json_bytes(&path, value)?;
        let lock = self.lock_file(&path);
        let _guard = lock.hold();
        write_atomic(&path, &bytes)
    }
}

/// 書き込む JSON。人が開いても読めるように、字下げして、最後に改行を付ける。
pub(super) fn to_json_bytes<T: Serialize + ?Sized>(
    path: &Path,
    value: &T,
) -> Result<Vec<u8>, StorageError> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|source| StorageError::Serialize {
        path: path.to_path_buf(),
        source,
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// `path` に `bytes` を、一時ファイルとリネームで書く。呼ぶ側が `path` のロックを持っていること。
pub(super) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), StorageError> {
    let temp = write_temp(path, bytes)?;
    fs::rename(&temp, path).map_err(|source| {
        let _ = fs::remove_file(&temp);
        StorageError::io(path, source)
    })
}

/// 一時ファイルに `bytes` を書き、ディスクに書き出して、一時ファイルのパスを返す。
/// 要るディレクトリは作る。失敗したときは、一時ファイルを消す。
pub(super) fn write_temp(path: &Path, bytes: &[u8]) -> Result<PathBuf, StorageError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| StorageError::io(parent, source))?;
    }
    let temp = temp_path(path);
    let written = File::create(&temp).and_then(|mut file| {
        file.write_all(bytes)?;
        file.sync_all()
    });
    match written {
        Ok(()) => Ok(temp),
        Err(source) => {
            let _ = fs::remove_file(&temp);
            Err(StorageError::io(&temp, source))
        }
    }
}

/// `path` の一時ファイルのパス。同じディレクトリに置き（リネームが同じファイルシステムの中で済むように）、
/// 名前を `.` で始めて `.tmp` で終える（`drafts/` などで `.json` のファイルを並べたときに混ざらないように）。
/// 名前にプロセス ID を入れ、同じファイルを書くプロセスがほかにあっても一時ファイルが重ならないようにする。
/// 同じプロセスの中では、同じファイルの書き込みはロックで直列化しているので、名前は 1 つでよい。
pub(super) fn temp_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.with_file_name(format!(".{name}.{}.tmp", std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exists(path: &Path) -> std::io::Result<bool> {
        path.try_exists()
    }

    #[test]
    fn writes_text_and_creates_directories() {
        let temp = tempfile::tempdir().unwrap();
        let dir = AppDataDir::new(temp.path());
        dir.write_text("records/r1.md", "# 記録\n").unwrap();
        assert_eq!(
            fs::read_to_string(temp.path().join("records/r1.md")).unwrap(),
            "# 記録\n"
        );
    }

    #[test]
    fn writes_json_with_indentation_and_trailing_newline() {
        let temp = tempfile::tempdir().unwrap();
        let dir = AppDataDir::new(temp.path());
        dir.write_json("settings.json", &serde_json::json!({ "schema_version": 1 }))
            .unwrap();
        assert_eq!(
            fs::read_to_string(temp.path().join("settings.json")).unwrap(),
            "{\n  \"schema_version\": 1\n}\n"
        );
    }

    #[test]
    fn replaces_the_existing_file_and_leaves_no_temp_file() {
        let temp = tempfile::tempdir().unwrap();
        let dir = AppDataDir::new(temp.path());
        dir.write_text("analysis/analysis.md", "古い").unwrap();
        dir.write_text("analysis/analysis.md", "新しい").unwrap();
        let path = temp.path().join("analysis/analysis.md");
        assert_eq!(fs::read_to_string(&path).unwrap(), "新しい");
        assert!(!exists(&temp_path(&path)).unwrap());
        let names: Vec<_> = fs::read_dir(temp.path().join("analysis"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, ["analysis.md"]);
    }

    /// 一時ファイルを書いた後、リネームの前にプロセスが止まった状態。元のファイルは書き込む前のまま読め、
    /// 次の書き込みは、残った一時ファイルがあっても成功する。
    #[test]
    fn a_write_stopped_before_the_rename_leaves_the_original_file() {
        let temp = tempfile::tempdir().unwrap();
        let dir = AppDataDir::new(temp.path());
        dir.write_text("progress.json", "元の内容").unwrap();
        let path = dir.path("progress.json").unwrap();

        let leftover = write_temp(&path, "途中の内容".as_bytes()).unwrap();
        assert!(exists(&leftover).unwrap());
        assert_eq!(fs::read_to_string(&path).unwrap(), "元の内容");

        dir.write_text("progress.json", "次の内容").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "次の内容");
        assert!(!exists(&leftover).unwrap());
    }

    #[test]
    fn temp_files_are_hidden_and_not_json_or_markdown() {
        let path = Path::new("/data/drafts/ch01-01.json");
        let temp = temp_path(path);
        assert_eq!(temp.parent(), path.parent());
        let name = temp.file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with(".ch01-01.json."), "{name}");
        assert!(name.ends_with(".tmp"), "{name}");
    }

    #[test]
    fn a_failed_write_reports_io_error_and_keeps_the_original() {
        let temp = tempfile::tempdir().unwrap();
        let dir = AppDataDir::new(temp.path());
        // `drafts` をファイルにしておくと、その下のファイルは書けない。
        dir.write_text("drafts", "ファイル").unwrap();
        let error = dir.write_text("drafts/ch01-01.json", "{}").unwrap_err();
        assert!(matches!(error, StorageError::Io { .. }), "{error:?}");
        assert_eq!(
            fs::read_to_string(temp.path().join("drafts")).unwrap(),
            "ファイル"
        );
    }
}
