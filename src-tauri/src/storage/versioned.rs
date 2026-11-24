//! `schema_version` を持つ JSON の読み込み（spec/architecture/data-locations.md「書き込みの規則」）。
//!
//! 読み込めないファイルは `{元の名前}.broken-{YYYYMMDDTHHMMSS}`（現地時刻）に退避する。`settings.json` などは
//! 初期状態で作り直し（[`AppDataDir::read_versioned_json`]）、下書き・試行ログは作り直さずに、無いものとして扱う
//! （[`AppDataDir::read_optional_versioned_json`]）。同じ秒に 2 回退避するときは、後のほうの名前の最後に `-2`、`-3`… を付け、前に退避したファイルを上書きしない。

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use chrono::{Datelike, Local, NaiveDateTime, Timelike};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::write::{to_json_bytes, write_atomic};
use super::{AppDataDir, StorageError};

/// [`AppDataDir::read_versioned_json`] で読んだ値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded<T> {
    pub value: T,
    /// 読み込めないファイルを退避したときの、退避先のパス（[`AppDataDir::read_versioned_json`] では、
    /// 初期状態で作り直している）。退避しなかったとき（読めたとき、ファイルが無かったとき）は `None`。
    pub broken_backup: Option<PathBuf>,
}

impl AppDataDir {
    /// `{app_data_dir}` からの相対パス `relative` の、`schema_version` を持つ JSON を読む。
    ///
    /// - ファイルが無いときは、`initial` の値（初期状態）を返す。ファイルは作らない。
    /// - 読み込めないとき（JSON として読めない、`T` の形に合わない、`schema_version` が無いか
    ///   `schema_version` と違う）は、ファイルを退避し、初期状態を書いて、それを返す。
    ///   退避先のパスは [`Loaded::broken_backup`] に入る。
    /// - ファイルの読み書きそのものに失敗したとき（権限が無いなど）は、退避せずに [`StorageError::Io`] を返す。
    ///
    /// `initial` の値は、JSON にしたときに `schema_version` を持たなければならない（作り直したファイルを、
    /// 次に読めるように）。持たないときは [`StorageError::InitialStateWithoutSchemaVersion`] を返す。
    pub fn read_versioned_json<T: Serialize + DeserializeOwned>(
        &self,
        relative: impl AsRef<Path>,
        schema_version: u64,
        initial: impl FnOnce() -> T,
    ) -> Result<Loaded<T>, StorageError> {
        self.read_versioned_json_at(
            relative,
            schema_version,
            initial,
            Local::now().naive_local(),
        )
    }

    /// `{app_data_dir}` からの相対パス `relative` の、`schema_version` を持つ JSON を読む。
    /// 下書き・試行ログのように、無い状態が初期状態であるファイルに使う。
    ///
    /// - ファイルが無いときは `None` を返す。ファイルは作らない。
    /// - 読み込めないとき（[`AppDataDir::read_versioned_json`] と同じ条件）は、ファイルを退避し、
    ///   作り直さずに `None` を返す。退避先のパスは [`Loaded::broken_backup`] に入る。
    /// - ファイルの読み込みそのものに失敗したとき（権限が無いなど）は、退避せずに [`StorageError::Io`] を返す。
    pub fn read_optional_versioned_json<T: DeserializeOwned>(
        &self,
        relative: impl AsRef<Path>,
        schema_version: u64,
    ) -> Result<Loaded<Option<T>>, StorageError> {
        self.read_optional_versioned_json_at(relative, schema_version, Local::now().naive_local())
    }

    /// [`AppDataDir::read_optional_versioned_json`] の、退避の名前に付ける日時 `now` を渡せるもの。
    fn read_optional_versioned_json_at<T: DeserializeOwned>(
        &self,
        relative: impl AsRef<Path>,
        schema_version: u64,
        now: NaiveDateTime,
    ) -> Result<Loaded<Option<T>>, StorageError> {
        let path = self.path(relative)?;
        let lock = self.lock_file(&path);
        let _guard = lock.hold();

        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(Loaded {
                    value: None,
                    broken_backup: None,
                });
            }
            Err(error) => return Err(StorageError::io(&path, error)),
        };
        if let Some(value) = parse_versioned_json(&bytes, schema_version) {
            return Ok(Loaded {
                value: Some(value),
                broken_backup: None,
            });
        }

        let backup = broken_path(&path, now)?;
        fs::rename(&path, &backup).map_err(|error| StorageError::io(&path, error))?;
        Ok(Loaded {
            value: None,
            broken_backup: Some(backup),
        })
    }

    /// [`AppDataDir::read_versioned_json`] の、退避の名前に付ける日時 `now` を渡せるもの。
    fn read_versioned_json_at<T: Serialize + DeserializeOwned>(
        &self,
        relative: impl AsRef<Path>,
        schema_version: u64,
        initial: impl FnOnce() -> T,
        now: NaiveDateTime,
    ) -> Result<Loaded<T>, StorageError> {
        let path = self.path(relative)?;
        // 退避と作り直しの間に、ほかのスレッドが同じファイルを読み書きしないように、全体をロックの中で行う。
        let lock = self.lock_file(&path);
        let _guard = lock.hold();

        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                let value = initial();
                initial_json(&path, &value, schema_version)?;
                return Ok(Loaded {
                    value,
                    broken_backup: None,
                });
            }
            Err(error) => return Err(StorageError::io(&path, error)),
        };
        if let Some(value) = parse_versioned_json(&bytes, schema_version) {
            return Ok(Loaded {
                value,
                broken_backup: None,
            });
        }

        let value = initial();
        let initial_bytes = initial_json(&path, &value, schema_version)?;
        let backup = broken_path(&path, now)?;
        fs::rename(&path, &backup).map_err(|error| StorageError::io(&path, error))?;
        write_atomic(&path, &initial_bytes)?;
        Ok(Loaded {
            value,
            broken_backup: Some(backup),
        })
    }
}

/// `schema_version` を持つ JSON のファイルの内容 `bytes` を読む。読み込めないとき（JSON として読めない、
/// `T` の形に合わない、`schema_version` が無いか `schema_version` と違う）は `None`。退避はしない。
pub fn parse_versioned_json<T: DeserializeOwned>(bytes: &[u8], schema_version: u64) -> Option<T> {
    let json: Value = serde_json::from_slice(bytes).ok()?;
    if json.get("schema_version")?.as_u64()? != schema_version {
        return None;
    }
    serde_json::from_value(json).ok()
}

/// 初期状態の値を、書き込む JSON にする。`schema_version` を持つことを確かめる。
fn initial_json<T: Serialize>(
    path: &Path,
    value: &T,
    schema_version: u64,
) -> Result<Vec<u8>, StorageError> {
    let json = serde_json::to_value(value).map_err(|source| StorageError::Serialize {
        path: path.to_path_buf(),
        source,
    })?;
    if json.get("schema_version").and_then(Value::as_u64) != Some(schema_version) {
        return Err(StorageError::InitialStateWithoutSchemaVersion {
            path: path.to_path_buf(),
            expected: schema_version,
        });
    }
    to_json_bytes(path, &json)
}

/// 退避先のパス。まだ無い名前を選ぶ。
fn broken_path(path: &Path, now: NaiveDateTime) -> Result<PathBuf, StorageError> {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let stamp = format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}",
        now.year(),
        now.month(),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    );
    let mut number = 1;
    loop {
        let candidate = if number == 1 {
            path.with_file_name(format!("{name}.broken-{stamp}"))
        } else {
            path.with_file_name(format!("{name}.broken-{stamp}-{number}"))
        };
        let taken = candidate
            .try_exists()
            .map_err(|error| StorageError::io(&candidate, error))?;
        if !taken {
            return Ok(candidate);
        }
        number += 1;
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct Progress {
        schema_version: u64,
        lessons: Vec<String>,
    }

    fn initial() -> Progress {
        Progress {
            schema_version: 1,
            lessons: Vec::new(),
        }
    }

    fn at(hour: u32, minute: u32, second: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 11, 23)
            .unwrap()
            .and_hms_opt(hour, minute, second)
            .unwrap()
    }

    #[test]
    fn the_backup_name_has_the_local_date_and_time() {
        let temp = tempfile::tempdir().unwrap();
        let dir = AppDataDir::new(temp.path());
        dir.write_text("progress.json", "{").unwrap();
        let loaded = dir
            .read_versioned_json_at("progress.json", 1, initial, at(9, 5, 7))
            .unwrap();
        assert_eq!(
            loaded.broken_backup,
            Some(temp.path().join("progress.json.broken-20261123T090507"))
        );
    }

    /// 同じ秒に 2 回退避しても、前に退避したファイルを上書きしない。
    #[test]
    fn backups_in_the_same_second_do_not_overwrite_each_other() {
        let temp = tempfile::tempdir().unwrap();
        let dir = AppDataDir::new(temp.path());
        let now = at(14, 25, 0);
        for (index, contents) in ["壊れた 1", "壊れた 2", "壊れた 3"].iter().enumerate() {
            dir.write_text("progress.json", contents).unwrap();
            let loaded = dir
                .read_versioned_json_at("progress.json", 1, initial, now)
                .unwrap();
            let expected = match index {
                0 => "progress.json.broken-20261123T142500".to_string(),
                n => format!("progress.json.broken-20261123T142500-{}", n + 1),
            };
            assert_eq!(loaded.broken_backup, Some(temp.path().join(expected)));
        }
        for (name, contents) in [
            ("progress.json.broken-20261123T142500", "壊れた 1"),
            ("progress.json.broken-20261123T142500-2", "壊れた 2"),
            ("progress.json.broken-20261123T142500-3", "壊れた 3"),
        ] {
            assert_eq!(
                fs::read_to_string(temp.path().join(name)).unwrap(),
                contents
            );
        }
    }

    #[test]
    fn an_optional_file_that_is_missing_is_none_and_is_not_created() {
        let temp = tempfile::tempdir().unwrap();
        let dir = AppDataDir::new(temp.path());
        let loaded: Loaded<Option<Progress>> = dir
            .read_optional_versioned_json("drafts/a.json", 1)
            .unwrap();
        assert_eq!(
            loaded,
            Loaded {
                value: None,
                broken_backup: None
            }
        );
        assert!(!temp.path().join("drafts").exists());
    }

    #[test]
    fn an_optional_file_that_is_readable_is_returned() {
        let temp = tempfile::tempdir().unwrap();
        let dir = AppDataDir::new(temp.path());
        dir.write_json("drafts/a.json", &initial()).unwrap();
        let loaded = dir
            .read_optional_versioned_json::<Progress>("drafts/a.json", 1)
            .unwrap();
        assert_eq!(loaded.value, Some(initial()));
        assert_eq!(loaded.broken_backup, None);
    }

    /// 読み込めないファイルは退避し、作り直さない。
    #[test]
    fn an_unreadable_optional_file_is_moved_aside_and_not_recreated() {
        for contents in [
            "{ 壊れた",
            "{\"lessons\": []}",
            "{\"schema_version\": 2, \"lessons\": []}",
            "{\"schema_version\": 1, \"lessons\": 3}",
        ] {
            let temp = tempfile::tempdir().unwrap();
            let dir = AppDataDir::new(temp.path());
            dir.write_text("drafts/a.json", contents).unwrap();
            let loaded = dir
                .read_optional_versioned_json_at::<Progress>("drafts/a.json", 1, at(14, 25, 0))
                .unwrap();
            let backup = temp.path().join("drafts/a.json.broken-20261123T142500");
            assert_eq!(loaded.value, None, "{contents}");
            assert_eq!(loaded.broken_backup, Some(backup.clone()), "{contents}");
            assert_eq!(fs::read_to_string(&backup).unwrap(), contents);
            assert!(!temp.path().join("drafts/a.json").exists(), "{contents}");
        }
    }

    #[test]
    fn an_initial_state_without_schema_version_is_a_program_error() {
        #[derive(Debug, Serialize, Deserialize)]
        struct NoVersion {
            lessons: Vec<String>,
        }
        let temp = tempfile::tempdir().unwrap();
        let dir = AppDataDir::new(temp.path());
        let error = dir
            .read_versioned_json("progress.json", 1, || NoVersion {
                lessons: Vec::new(),
            })
            .unwrap_err();
        assert!(
            matches!(
                error,
                StorageError::InitialStateWithoutSchemaVersion { expected: 1, .. }
            ),
            "{error:?}"
        );
    }

    #[test]
    fn an_initial_state_with_another_schema_version_is_a_program_error() {
        let temp = tempfile::tempdir().unwrap();
        let dir = AppDataDir::new(temp.path());
        dir.write_text("progress.json", "{").unwrap();
        let error = dir
            .read_versioned_json("progress.json", 2, initial)
            .unwrap_err();
        assert!(
            matches!(
                error,
                StorageError::InitialStateWithoutSchemaVersion { expected: 2, .. }
            ),
            "{error:?}"
        );
        // 作り直せないときは、退避もしない。
        assert_eq!(
            fs::read_to_string(temp.path().join("progress.json")).unwrap(),
            "{"
        );
    }
}
