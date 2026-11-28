//! `schema_version` を持つ JSON の読み込み（spec/architecture/data-locations.md「書き込みの規則」）。
//!
//! 読み込めないファイルは `{元の名前}.broken-{YYYYMMDDTHHMMSS}`（現地時刻）に退避する。`settings.json` などは
//! 初期状態で作り直し（[`AppDataDir::read_versioned_json`]）、下書き・試行ログは作り直さずに、無いものとして扱う
//! （[`AppDataDir::read_optional_versioned_json`]）。同じ秒に 2 回退避するときは、後のほうの名前の最後に `-2`、`-3`… を付け、前に退避したファイルを上書きしない。
//!
//! 古い版のファイルは、ファイルごとの変換の関数（[`Migration`]）を渡して読むと
//! （[`AppDataDir::read_migrated_json`]）、いまの版に変換して書き直す。変換前のファイルは
//! `{元の名前}.v{旧バージョン}.bak` として残す（spec/non-functional/data-compatibility.md）。同じ名前のファイルが
//! 既にあるときは、退避と同じく、名前の最後に `-2`、`-3`… を付け、既にあるファイルを上書きしない。

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use chrono::{Datelike, Local, NaiveDateTime, Timelike};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::write::{to_json_bytes, write_atomic};
use super::{AppDataDir, StorageError};

/// 古い版の JSON を、1 つ新しい版の形にする変換（spec/non-functional/data-compatibility.md）。
/// ファイルごとに、変換する版の数だけ用意して [`AppDataDir::read_migrated_json`] に渡す。
#[derive(Debug, Clone, Copy)]
pub struct Migration {
    /// 変換する前の版。変換した後の版は `from + 1` である。
    pub from: u64,
    /// 版 `from` の JSON を、版 `from + 1` の形にする。`schema_version` は書き換えなくてよい（読む側が
    /// `from + 1` にする）。形が合わず変換できないときは `None` を返す（読み込めないファイルとして退避する）。
    pub migrate: fn(Value) -> Option<Value>,
}

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
            &[],
            initial,
            Local::now().naive_local(),
        )
    }

    /// [`AppDataDir::read_versioned_json`] と同じく読み、`schema_version` が古い版のファイルは、`migrations` で
    /// いまの版 `schema_version` に変換する。
    ///
    /// - 版 `n` のファイルは、`from` が `n`、`n + 1`、… の変換を順に通して、いまの版にする。変換して `T` の形に
    ///   合ったときは、変換前のファイルを `{元の名前}.v{n}.bak` として残し（既にあるファイルは上書きしない）、
    ///   変換後の内容を書く。変換前のファイルは、書き換える前に残すので、途中で止まっても元の内容は失われない。
    /// - 変換が無い版、変換できなかったとき、変換した後に `T` の形に合わないときは、読み込めないファイルとして
    ///   退避し、初期状態で作り直す（`.v{n}.bak` は残さない）。
    pub fn read_migrated_json<T: Serialize + DeserializeOwned>(
        &self,
        relative: impl AsRef<Path>,
        schema_version: u64,
        migrations: &[Migration],
        initial: impl FnOnce() -> T,
    ) -> Result<Loaded<T>, StorageError> {
        self.read_versioned_json_at(
            relative,
            schema_version,
            migrations,
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

    /// [`AppDataDir::read_migrated_json`] の、退避の名前に付ける日時 `now` を渡せるもの。
    fn read_versioned_json_at<T: Serialize + DeserializeOwned>(
        &self,
        relative: impl AsRef<Path>,
        schema_version: u64,
        migrations: &[Migration],
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
        match parse_or_migrate(&bytes, schema_version, migrations) {
            Parsed::Current(value) => {
                return Ok(Loaded {
                    value,
                    broken_backup: None,
                });
            }
            Parsed::Migrated { value, from } => {
                let migrated_bytes = to_json_bytes(&path, &value)?;
                let backup = free_path(&path, &format!("v{from}.bak"))?;
                write_atomic(&backup, &bytes)?;
                write_atomic(&path, &migrated_bytes)?;
                return Ok(Loaded {
                    value,
                    broken_backup: None,
                });
            }
            Parsed::Unreadable => {}
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

/// [`parse_or_migrate`] の結果。
enum Parsed<T> {
    /// いまの版として読めた。
    Current(T),
    /// 古い版 `from` を、いまの版に変換して読めた。
    Migrated { value: T, from: u64 },
    /// 読み込めない（[`parse_versioned_json`] の条件か、変換できない）。
    Unreadable,
}

/// ファイルの内容 `bytes` を、いまの版 `schema_version` として読む。古い版なら `migrations` で変換してから読む。
fn parse_or_migrate<T: DeserializeOwned>(
    bytes: &[u8],
    schema_version: u64,
    migrations: &[Migration],
) -> Parsed<T> {
    let Ok(mut json) = serde_json::from_slice::<Value>(bytes) else {
        return Parsed::Unreadable;
    };
    let Some(from) = json.get("schema_version").and_then(Value::as_u64) else {
        return Parsed::Unreadable;
    };
    if from > schema_version {
        return Parsed::Unreadable;
    }
    let mut version = from;
    while version < schema_version {
        let Some(migration) = migrations
            .iter()
            .find(|migration| migration.from == version)
        else {
            return Parsed::Unreadable;
        };
        let Some(mut next) = (migration.migrate)(json) else {
            return Parsed::Unreadable;
        };
        version += 1;
        let Some(fields) = next.as_object_mut() else {
            return Parsed::Unreadable;
        };
        fields.insert("schema_version".into(), version.into());
        json = next;
    }
    match serde_json::from_value(json) {
        Ok(value) if from == schema_version => Parsed::Current(value),
        Ok(value) => Parsed::Migrated { value, from },
        Err(_) => Parsed::Unreadable,
    }
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

/// 読み込めないファイルの退避先のパス。まだ無い名前を選ぶ。
fn broken_path(path: &Path, now: NaiveDateTime) -> Result<PathBuf, StorageError> {
    let stamp = format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}",
        now.year(),
        now.month(),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    );
    free_path(path, &format!("broken-{stamp}"))
}

/// `path` の名前の後ろに `.{suffix}` を付けたパス。その名前のファイルが既にあるときは、最後に `-2`、`-3`… を付け、
/// まだ無い名前を選ぶ。
fn free_path(path: &Path, suffix: &str) -> Result<PathBuf, StorageError> {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut number = 1;
    loop {
        let candidate = if number == 1 {
            path.with_file_name(format!("{name}.{suffix}"))
        } else {
            path.with_file_name(format!("{name}.{suffix}-{number}"))
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
            .read_versioned_json_at("progress.json", 1, &[], initial, at(9, 5, 7))
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
                .read_versioned_json_at("progress.json", 1, &[], initial, now)
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

    // ---- 古い版の変換 ----

    /// 版 3 の形。版 1 は `lessons` だけ、版 2 で `done` が、版 3 で `note` が加わったとする。
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Current {
        schema_version: u64,
        lessons: Vec<String>,
        done: bool,
        note: Option<String>,
    }

    fn current_initial() -> Current {
        Current {
            schema_version: 3,
            lessons: Vec::new(),
            done: false,
            note: None,
        }
    }

    fn add_field(name: &'static str, value: Value) -> impl Fn(Value) -> Option<Value> {
        move |mut json| {
            json.as_object_mut()?.insert(name.into(), value.clone());
            Some(json)
        }
    }

    fn add_done(json: Value) -> Option<Value> {
        add_field("done", Value::Bool(false))(json)
    }

    fn add_note(json: Value) -> Option<Value> {
        add_field("note", Value::Null)(json)
    }

    const MIGRATIONS: [Migration; 2] = [
        Migration {
            from: 1,
            migrate: add_done,
        },
        Migration {
            from: 2,
            migrate: add_note,
        },
    ];

    fn read_current(dir: &AppDataDir, migrations: &[Migration]) -> Loaded<Current> {
        dir.read_versioned_json_at(
            "progress.json",
            3,
            migrations,
            current_initial,
            at(14, 25, 0),
        )
        .unwrap()
    }

    /// 古い版のファイルは、変換を順に通していまの版にし、書き直す。変換前のファイルは `.v{版}.bak` に残す。
    #[test]
    fn an_old_version_is_migrated_and_the_original_is_kept_as_a_bak() {
        for (from, contents) in [
            (1, r#"{"schema_version": 1, "lessons": ["a"]}"#),
            (
                2,
                r#"{"schema_version": 2, "lessons": ["a"], "done": false}"#,
            ),
        ] {
            let temp = tempfile::tempdir().unwrap();
            let dir = AppDataDir::new(temp.path());
            dir.write_text("progress.json", contents).unwrap();
            let loaded = read_current(&dir, &MIGRATIONS);
            let expected = Current {
                schema_version: 3,
                lessons: vec!["a".into()],
                done: false,
                note: None,
            };
            assert_eq!(loaded.value, expected, "{contents}");
            assert_eq!(loaded.broken_backup, None, "{contents}");
            let written: Current = serde_json::from_str(
                &fs::read_to_string(temp.path().join("progress.json")).unwrap(),
            )
            .unwrap();
            assert_eq!(written, expected, "{contents}");
            assert_eq!(
                fs::read_to_string(temp.path().join(format!("progress.json.v{from}.bak"))).unwrap(),
                contents
            );
            // 次に読むときは、もう変換しない。
            let again = read_current(&dir, &MIGRATIONS);
            assert_eq!(again.value, expected);
            let names: Vec<_> = fs::read_dir(temp.path())
                .unwrap()
                .map(|entry| entry.unwrap().file_name().into_string().unwrap())
                .collect();
            assert_eq!(names.len(), 2, "{names:?}");
        }
    }

    /// いまの版のファイルは、書き直さず、`.bak` も作らない。
    #[test]
    fn the_current_version_is_read_as_it_is() {
        let temp = tempfile::tempdir().unwrap();
        let dir = AppDataDir::new(temp.path());
        let contents = r#"{"schema_version": 3, "lessons": [], "done": true, "note": "x"}"#;
        dir.write_text("progress.json", contents).unwrap();
        let loaded = read_current(&dir, &MIGRATIONS);
        assert!(loaded.value.done);
        assert_eq!(
            fs::read_to_string(temp.path().join("progress.json")).unwrap(),
            contents
        );
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    }

    /// `.v{版}.bak` が既にあるときは、上書きせずに、名前の最後に `-2`、`-3`… を付ける。
    #[test]
    fn an_existing_bak_is_not_overwritten() {
        let temp = tempfile::tempdir().unwrap();
        let dir = AppDataDir::new(temp.path());
        dir.write_text("progress.json.v1.bak", "前の 1").unwrap();
        dir.write_text("progress.json.v1.bak-2", "前の 2").unwrap();
        let contents = r#"{"schema_version": 1, "lessons": []}"#;
        dir.write_text("progress.json", contents).unwrap();
        read_current(&dir, &MIGRATIONS);
        for (name, expected) in [
            ("progress.json.v1.bak", "前の 1"),
            ("progress.json.v1.bak-2", "前の 2"),
            ("progress.json.v1.bak-3", contents),
        ] {
            assert_eq!(
                fs::read_to_string(temp.path().join(name)).unwrap(),
                expected
            );
        }
    }

    /// 変換が無い版、新しい版、変換できないもの、変換した後に形が合わないものは、読み込めないファイルとして退避し、
    /// 初期状態で作り直す。`.bak` は残さない。
    #[test]
    fn a_file_that_cannot_be_migrated_is_moved_aside() {
        for (migrations, contents) in [
            // 版 1 の変換が無い。
            (&MIGRATIONS[1..], r#"{"schema_version": 1, "lessons": []}"#),
            // 版 0 の変換は無い。
            (&MIGRATIONS[..], r#"{"schema_version": 0, "lessons": []}"#),
            // いまより新しい版。
            (
                &MIGRATIONS[..],
                r#"{"schema_version": 4, "lessons": [], "done": false, "note": null}"#,
            ),
            // 変換した後も、形が合わない。
            (&MIGRATIONS[..], r#"{"schema_version": 1, "lessons": 3}"#),
            (
                &MIGRATIONS[..],
                r#"{"schema_version": 2, "lessons": [], "done": "yes"}"#,
            ),
            (&MIGRATIONS[..], r#"{"schema_version": 1}"#),
        ] {
            let temp = tempfile::tempdir().unwrap();
            let dir = AppDataDir::new(temp.path());
            dir.write_text("progress.json", contents).unwrap();
            let loaded = read_current(&dir, migrations);
            let backup = temp.path().join("progress.json.broken-20261123T142500");
            assert_eq!(loaded.value, current_initial(), "{contents}");
            assert_eq!(loaded.broken_backup, Some(backup.clone()), "{contents}");
            assert_eq!(fs::read_to_string(&backup).unwrap(), contents);
            let names: Vec<_> = fs::read_dir(temp.path())
                .unwrap()
                .map(|entry| entry.unwrap().file_name().into_string().unwrap())
                .collect();
            assert!(
                names.iter().all(|name| !name.contains(".bak")),
                "{contents}: {names:?}"
            );
        }
    }

    /// 変換の関数が `None` を返したときと、オブジェクトでない値を返したときも、読み込めないファイルとして扱う。
    #[test]
    fn a_failed_migration_makes_the_file_unreadable() {
        fn fail(_: Value) -> Option<Value> {
            None
        }
        fn to_array(_: Value) -> Option<Value> {
            Some(Value::Array(Vec::new()))
        }
        for migrate in [fail as fn(Value) -> Option<Value>, to_array] {
            let temp = tempfile::tempdir().unwrap();
            let dir = AppDataDir::new(temp.path());
            dir.write_text(
                "progress.json",
                r#"{"schema_version": 2, "lessons": [], "done": false}"#,
            )
            .unwrap();
            let loaded = read_current(&dir, &[Migration { from: 2, migrate }]);
            assert_eq!(loaded.value, current_initial());
            assert!(loaded.broken_backup.is_some());
            assert!(!temp.path().join("progress.json.v2.bak").exists());
        }
    }
}
