//! 初回準備の状態の記録 `prepared.json` と、初回準備の「実行する条件」の判定
//! （spec/exercise-runner/crates-and-workspace.md「初回準備」、ADR 0235、ADR 0100）。
//!
//! - `prepared.json` の形は [`PreparedRecord`]。作業領域（[`WorkspaceDir`]）の `prepared.json` を
//!   [`read_prepared`] で読み、[`write_prepared`] で書く。読み込めないファイルは、退避も通知もせずに、
//!   無いものとして扱う。書き込みは、プロセスの中でロックで直列化する。
//! - 演習のビルドで許可クレートが失われていると分かったときに、`status` を `failed` にするのは
//!   [`mark_prepared_failed`]。
//! - 同梱の `allowed-crates.toml` と `Cargo.lock` のハッシュ値は [`crates_hash`]、今の `rustc -vV` の出力は
//!   [`rustc_verbose_version`] で得る。
//! - 初回準備を実行するかは [`needs_preparation`] で、読んだ記録と、今のハッシュ値と `rustc -vV` の出力から
//!   判定する。ファイルもプロセスも使わない。

use std::io;
use std::path::Path;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::RunnerDir;
use super::workspace::WorkspaceDir;
use crate::environment::tool_stdout;
use crate::storage::{StorageError, parse_versioned_json, write_json_file};

/// `prepared.json` の `schema_version`。
pub const SCHEMA_VERSION: u64 = 1;

/// `rustc -vV` が終わるのを待つ時間。過ぎたら止める。
const RUSTC_VERBOSE_TIMEOUT: Duration = Duration::from_secs(10);

/// `prepared.json` の中身。最後に終わった初回準備の結果を表す。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedRecord {
    /// [`SCHEMA_VERSION`]。
    pub schema_version: u64,
    pub status: PreparedStatus,
    /// 初回準備に使った `allowed-crates.toml` と `Cargo.lock` のハッシュ値（[`crates_hash`]）。
    pub crates_hash: String,
    /// 初回準備に使った `rustc` の `rustc -vV` の出力そのまま（[`rustc_verbose_version`]）。
    pub rustc_version: String,
    /// 初回準備が終わった日時（時差を付けた ISO 8601。例: `2026-11-24T19:02:00+09:00`）。
    pub prepared_at: String,
    /// `failed` のときのエラーの内容。`ready` のときは `None`。
    pub error: Option<String>,
}

/// `prepared.json` の `status`。準備中を表す値は持たない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreparedStatus {
    Ready,
    Failed,
}

impl PreparedRecord {
    /// 成功して終わった初回準備の記録。
    pub fn ready(
        crates_hash: impl Into<String>,
        rustc_version: impl Into<String>,
        prepared_at: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            status: PreparedStatus::Ready,
            crates_hash: crates_hash.into(),
            rustc_version: rustc_version.into(),
            prepared_at: prepared_at.into(),
            error: None,
        }
    }

    /// 失敗して終わった初回準備の記録。
    pub fn failed(
        crates_hash: impl Into<String>,
        rustc_version: impl Into<String>,
        prepared_at: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        Self {
            status: PreparedStatus::Failed,
            error: Some(error.into()),
            ..Self::ready(crates_hash, rustc_version, prepared_at)
        }
    }
}

/// `prepared.json` の書き込みを直列化するロック（spec/architecture/data-locations.md「書き込みの規則」）。
/// 実行・提出のビルドと生成問題の検証のビルドは並行して動き、どちらも `status` を `failed` に書くことがある。
/// 同じプロセスの書き込みは同じ一時ファイルの名前を使うので、重なると壊れる。作業領域はふつう 1 つなので、
/// ロックも 1 つでよい。
static WRITE_LOCK: Mutex<()> = Mutex::new(());

/// 作業領域の `prepared.json` を読む。
///
/// - 無いときと、読み込めないとき（JSON として読めない、[`PreparedRecord`] の形に合わない、`schema_version` が
///   [`SCHEMA_VERSION`] でない）は `Ok(None)` を返す。読み込めないファイルは、退避も通知もしない（ADR 0235）。
/// - ファイルの読み込みそのものに失敗したとき（権限が無いなど）は、[`StorageError::Io`] を返す
///   （spec/architecture/data-locations.md「書き込みの規則」）。
pub fn read_prepared(workspace: &WorkspaceDir) -> Result<Option<PreparedRecord>, StorageError> {
    let path = workspace.prepared_json();
    match std::fs::read(&path) {
        Ok(bytes) => Ok(parse_versioned_json(&bytes, SCHEMA_VERSION)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(StorageError::Io { path, source }),
    }
}

/// 作業領域の `prepared.json` に `record` を、一時ファイルとリネームで書く。作業領域のディレクトリが無ければ作る。
/// 同じプロセスの中の書き込みは、ロックで直列化する。
pub fn write_prepared(
    workspace: &WorkspaceDir,
    record: &PreparedRecord,
) -> Result<(), StorageError> {
    // ロックが守るのはファイルへの操作の順番だけなので、ほかのスレッドがパニックしていても使う。
    let _guard = WRITE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    write_json_file(&workspace.prepared_json(), record)
}

/// 演習のビルドで許可クレートが失われていると分かったときに、`prepared.json` に書くエラーの内容。
pub const LOST_CRATES_ERROR: &str =
    "演習のビルドで、外部クレートのソースかビルド済みの依存が失われていたため、準備し直します。";

/// 作業領域の `prepared.json` の `status` を `failed` にし、`error` を `error` にする（spec/exercise-runner/
/// crates-and-workspace.md「初回準備のやり直しが要る失敗」）。ほかの項目は変えない。書いたら `true`。
///
/// `prepared.json` が無いとき（読み込めないときを含む）は、書かずに `false` を返す。無いことが、すでに
/// 「実行する条件」に当てはまるからである。読み込みと書き込みの間に、ほかの書き込みが入らないよう、
/// [`write_prepared`] と同じロックを持つ。
pub fn mark_prepared_failed(workspace: &WorkspaceDir, error: &str) -> Result<bool, StorageError> {
    let _guard = WRITE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(mut record) = read_prepared(workspace)? else {
        return Ok(false);
    };
    record.status = PreparedStatus::Failed;
    record.error = Some(error.to_string());
    write_json_file(&workspace.prepared_json(), &record)?;
    Ok(true)
}

/// 同梱の `allowed-crates.toml` と `Cargo.lock` のハッシュ値（[`crates_hash_of`]）。
/// どちらかのファイルを読めないときは、そのファイルのパスを含むエラーを返す。
pub fn crates_hash(runner: &RunnerDir) -> io::Result<String> {
    let read = |path: &Path| {
        std::fs::read(path)
            .map_err(|error| io::Error::new(error.kind(), format!("{}: {error}", path.display())))
    };
    let allowed_crates = read(&runner.allowed_crates())?;
    let cargo_lock = read(&runner.cargo_lock())?;
    Ok(crates_hash_of(&allowed_crates, &cargo_lock))
}

/// `allowed-crates.toml` のバイト列に `Cargo.lock` のバイト列を続けたものの SHA-256 を、小文字の 16 進数で返す。
pub fn crates_hash_of(allowed_crates: &[u8], cargo_lock: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(allowed_crates);
    hasher.update(cargo_lock);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// `rustc` を `-vV` で起動して、その出力をそのまま返す。起動できないとき、失敗したとき、
/// 10 秒の間に終わらないとき（止める）は `None`。
pub fn rustc_verbose_version(rustc: &Path) -> Option<String> {
    tool_stdout(rustc, &["-vV"], RUSTC_VERBOSE_TIMEOUT)
}

/// 初回準備を実行する条件（spec/exercise-runner/crates-and-workspace.md「初回準備」の「実行する条件」）に
/// 当てはまるか。
///
/// `recorded` は読んだ `prepared.json`（無い・読み込めないときは `None`）、`crates_hash` は同梱のファイルの
/// 今のハッシュ値、`rustc_version` は今の `rustc -vV` の出力。記録が無い、`status` が `failed`、
/// `crates_hash` が異なる、`rustc_version` が異なる、のどれかなら `true`。
pub fn needs_preparation(
    recorded: Option<&PreparedRecord>,
    crates_hash: &str,
    rustc_version: &str,
) -> bool {
    let Some(recorded) = recorded else {
        return true;
    };
    recorded.status == PreparedStatus::Failed
        || recorded.crates_hash != crates_hash
        || recorded.rustc_version != rustc_version
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    const HASH: &str = "3f5a";
    const RUSTC: &str = "rustc 1.100.0 (0ef1bc386 2026-11-09)\nbinary: rustc\nrelease: 1.100.0\n";

    fn ready() -> PreparedRecord {
        PreparedRecord::ready(HASH, RUSTC, "2026-11-24T19:02:00+09:00")
    }

    #[test]
    fn preparation_is_needed_without_a_record() {
        assert!(needs_preparation(None, HASH, RUSTC));
    }

    #[test]
    fn preparation_is_not_needed_when_ready_and_everything_matches() {
        assert!(!needs_preparation(Some(&ready()), HASH, RUSTC));
    }

    #[test]
    fn preparation_is_needed_when_the_last_one_failed() {
        let failed = PreparedRecord::failed(HASH, RUSTC, "2026-11-24T19:02:00+09:00", "失敗");
        assert!(needs_preparation(Some(&failed), HASH, RUSTC));
    }

    #[test]
    fn preparation_is_needed_when_the_crates_hash_differs() {
        assert!(needs_preparation(Some(&ready()), "9b2c", RUSTC));
    }

    /// `rustc` の版が変わった（`rustup update` など）ときは、初回準備をやり直す（ADR 0100）。
    #[test]
    fn preparation_is_needed_when_the_rustc_output_differs() {
        let updated = RUSTC.replace("1.100.0", "1.101.0");
        assert!(needs_preparation(Some(&ready()), HASH, &updated));
        // 版の行が同じでも、ほかの行（commit-hash、host など）が違えば、別の rustc とみなす。
        let other_host = format!("{RUSTC}host: x86_64-apple-darwin\n");
        assert!(needs_preparation(Some(&ready()), HASH, &other_host));
    }

    #[test]
    fn a_written_record_reads_back() {
        let temp = tempfile::tempdir().unwrap();
        // 作業領域のディレクトリがまだ無くても書ける。
        let workspace = WorkspaceDir::new(temp.path().join("workspace"));
        for record in [
            ready(),
            PreparedRecord::failed(
                HASH,
                RUSTC,
                "2026-11-24T19:02:00+09:00",
                "ネットワークに接続できない",
            ),
        ] {
            write_prepared(&workspace, &record).unwrap();
            assert_eq!(read_prepared(&workspace).unwrap(), Some(record));
        }
        // 一時ファイルは残らない。
        let names: Vec<_> = fs::read_dir(workspace.root())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, ["prepared.json"]);
    }

    #[test]
    fn the_written_file_has_the_specified_shape() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = WorkspaceDir::new(temp.path());
        write_prepared(&workspace, &ready()).unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&fs::read(workspace.prepared_json()).unwrap()).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "schema_version": 1,
                "status": "ready",
                "crates_hash": HASH,
                "rustc_version": RUSTC,
                "prepared_at": "2026-11-24T19:02:00+09:00",
                "error": null,
            })
        );
    }

    #[test]
    fn marking_failed_changes_only_the_status_and_the_error() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = WorkspaceDir::new(temp.path());
        write_prepared(&workspace, &ready()).unwrap();
        assert!(mark_prepared_failed(&workspace, LOST_CRATES_ERROR).unwrap());
        assert_eq!(
            read_prepared(&workspace).unwrap(),
            Some(PreparedRecord::failed(
                HASH,
                RUSTC,
                "2026-11-24T19:02:00+09:00",
                LOST_CRATES_ERROR
            ))
        );
        assert!(needs_preparation(
            read_prepared(&workspace).unwrap().as_ref(),
            HASH,
            RUSTC
        ));
    }

    #[test]
    fn marking_failed_without_a_record_writes_nothing() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = WorkspaceDir::new(temp.path().join("workspace"));
        assert!(!mark_prepared_failed(&workspace, LOST_CRATES_ERROR).unwrap());
        assert!(!workspace.prepared_json().exists());
    }

    #[test]
    fn a_missing_file_reads_as_none() {
        let temp = tempfile::tempdir().unwrap();
        assert_eq!(
            read_prepared(&WorkspaceDir::new(temp.path())).unwrap(),
            None
        );
    }

    /// ファイルの読み込みそのものの失敗は、無いものとせず、失敗として返す。
    #[test]
    fn a_file_that_cannot_be_read_is_an_io_error() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = WorkspaceDir::new(temp.path());
        // `prepared.json` をディレクトリにしておくと、ファイルとして読めない。
        fs::create_dir(workspace.prepared_json()).unwrap();
        let error = read_prepared(&workspace).unwrap_err();
        assert!(matches!(error, StorageError::Io { .. }), "{error:?}");
    }

    /// 並行して書いても、ファイルは書いた記録のどれか 1 つのとおりで、一時ファイルも残らない。
    #[test]
    fn concurrent_writes_are_serialized() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = WorkspaceDir::new(temp.path());
        let records: Vec<PreparedRecord> = (0..8)
            .map(|index| {
                PreparedRecord::failed(
                    HASH,
                    RUSTC,
                    "2026-11-24T19:02:00+09:00",
                    format!("失敗 {index}").repeat(1000),
                )
            })
            .collect();
        std::thread::scope(|scope| {
            for record in &records {
                let workspace = &workspace;
                scope.spawn(move || {
                    for _ in 0..20 {
                        write_prepared(workspace, record).unwrap();
                    }
                });
            }
        });
        let read = read_prepared(&workspace).unwrap().unwrap();
        assert!(records.contains(&read));
        let names: Vec<_> = fs::read_dir(temp.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, ["prepared.json"]);
    }

    /// 読み込めない `prepared.json` は、無いものとして扱い、退避しない。
    #[test]
    fn an_unreadable_file_reads_as_none_and_is_left_alone() {
        let valid = serde_json::to_value(ready()).unwrap();
        let with = |key: &str, value: serde_json::Value| {
            let mut json = valid.clone();
            json[key] = value;
            json.to_string()
        };
        let without_version = {
            let mut json = valid.clone();
            json.as_object_mut().unwrap().remove("schema_version");
            json.to_string()
        };
        for contents in [
            "{ 壊れた".to_string(),
            "[]".to_string(),
            "{\"schema_version\": 1}".to_string(),
            with("status", serde_json::json!("preparing")),
            with("crates_hash", serde_json::json!(3)),
            with("schema_version", serde_json::json!(2)),
            without_version,
        ] {
            let temp = tempfile::tempdir().unwrap();
            let workspace = WorkspaceDir::new(temp.path());
            fs::write(workspace.prepared_json(), &contents).unwrap();
            assert_eq!(read_prepared(&workspace).unwrap(), None, "{contents}");
            assert!(needs_preparation(
                read_prepared(&workspace).unwrap().as_ref(),
                HASH,
                RUSTC
            ));
            // 退避せず、そのまま残す。
            let names: Vec<_> = fs::read_dir(temp.path())
                .unwrap()
                .map(|entry| entry.unwrap().file_name())
                .collect();
            assert_eq!(names, ["prepared.json"], "{contents}");
            assert_eq!(
                fs::read_to_string(workspace.prepared_json()).unwrap(),
                contents
            );
        }
    }

    #[test]
    fn the_hash_is_the_lowercase_sha256_of_both_files_in_order() {
        // SHA-256("abc")（FIPS 180-2 の例）。
        let abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(crates_hash_of(b"ab", b"c"), abc);
        assert_eq!(crates_hash_of(b"abc", b""), abc);
        assert_eq!(crates_hash_of(b"", b"abc"), abc);
    }

    #[test]
    fn the_hash_changes_with_the_file_contents() {
        let temp = tempfile::tempdir().unwrap();
        let runner = RunnerDir::new(temp.path());
        fs::write(
            runner.allowed_crates(),
            "[dependencies]\nrand = \"=0.9.1\"\n",
        )
        .unwrap();
        fs::write(runner.cargo_lock(), "version = 4\n").unwrap();
        let first = crates_hash(&runner).unwrap();
        assert_eq!(first.len(), 64);
        assert!(
            first
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
            "{first}"
        );
        assert_eq!(crates_hash(&runner).unwrap(), first);

        fs::write(
            runner.allowed_crates(),
            "[dependencies]\nrand = \"=0.9.2\"\n",
        )
        .unwrap();
        let second = crates_hash(&runner).unwrap();
        assert_ne!(second, first);

        fs::write(runner.cargo_lock(), "version = 4\n\n").unwrap();
        let third = crates_hash(&runner).unwrap();
        assert_ne!(third, first);
        assert_ne!(third, second);
    }

    #[test]
    fn the_hash_of_missing_files_is_an_error_naming_the_file() {
        let temp = tempfile::tempdir().unwrap();
        let runner = RunnerDir::new(temp.path());
        let error = crates_hash(&runner).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert!(error.to_string().contains("allowed-crates.toml"), "{error}");

        fs::write(runner.allowed_crates(), "[dependencies]\n").unwrap();
        let error = crates_hash(&runner).unwrap_err();
        assert!(error.to_string().contains("Cargo.lock"), "{error}");
    }

    /// 同梱の `runner/` のファイルからハッシュ値を求められる。
    #[test]
    fn the_hash_of_the_bundled_files_is_computed() {
        let runner = RunnerDir::new(Path::new(env!("CARGO_MANIFEST_DIR")).join("../runner"));
        let expected = crates_hash_of(
            &fs::read(runner.allowed_crates()).unwrap(),
            &fs::read(runner.cargo_lock()).unwrap(),
        );
        assert_eq!(crates_hash(&runner).unwrap(), expected);
    }
}
