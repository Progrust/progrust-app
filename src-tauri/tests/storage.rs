//! 学習者データのファイルの読み書き（spec/architecture/data-locations.md「書き込みの規則」）。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Barrier;
use std::thread;
use std::time::{Duration, Instant};

use progrust_lib::commands::error::{CommandError, ErrorCode};
use progrust_lib::storage::{AppDataDir, Loaded, StorageError};
use serde::{Deserialize, Serialize};

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

/// 中身の大きい `progress.json` の値。書き込みに時間がかかり、途中で止まった状態や重なった状態を作りやすい。
fn large_progress(label: usize) -> Progress {
    Progress {
        schema_version: 1,
        lessons: (0..20_000).map(|i| format!("{label}-{i:05}")).collect(),
    }
}

/// `dir` の中の、ファイルの名前の一覧（並べたもの）。
fn file_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

// ---- 書き込み中にプロセスを強制終了しても、ファイルが壊れない ----

/// 子のプロセスに書き込みを続けさせるときの、学習者データのディレクトリを渡す環境変数。
const CRASH_DIR_ENV: &str = "PROGRUST_TEST_STORAGE_CRASH_DIR";

/// 強制終了のテストで書き込む `progress.json` の内容。書き込みに時間がかかるように、数 MB にする。
fn crash_test_contents(label: usize) -> String {
    let lessons: Vec<_> = (0..200_000).map(|i| format!("{label}-{i:06}")).collect();
    serde_json::to_string(&Progress {
        schema_version: 1,
        lessons,
    })
    .unwrap()
}

/// 子のプロセスとして動く。`progress.json` に、大きい内容を書き込み続ける。
/// `writes_survive_killing_the_process` が、このテストを子のプロセスで実行し、書き込みの途中で強制終了する。
/// 内容は先に作っておき、時間のほとんどを書き込みに使うようにする。
#[test]
#[ignore = "writes_survive_killing_the_process が子のプロセスで実行する"]
fn child_keeps_writing_progress() {
    let Some(root) = std::env::var_os(CRASH_DIR_ENV) else {
        return;
    };
    let dir = AppDataDir::new(PathBuf::from(root));
    let contents = [crash_test_contents(0), crash_test_contents(1)];
    for contents in contents.iter().cycle() {
        dir.write_text("progress.json", contents).unwrap();
    }
}

#[test]
fn writes_survive_killing_the_process() {
    let expected = [crash_test_contents(0), crash_test_contents(1)];
    for wait_ms in [10, 30, 50, 80, 120, 170, 230, 300] {
        let wait = Duration::from_millis(wait_ms);
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("progress.json");
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "child_keeps_writing_progress", "--ignored"])
            .env(CRASH_DIR_ENV, temp.path())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        // 最初の書き込みが済むのを待ってから、書き込みを続けている途中で強制終了する（SIGKILL）。
        let started = Instant::now();
        while !path.exists() {
            assert!(
                started.elapsed() < Duration::from_secs(30),
                "子のプロセスが書き込まない"
            );
            thread::sleep(Duration::from_millis(5));
        }
        thread::sleep(wait);
        child.kill().unwrap();
        let status = child.wait().unwrap();
        assert!(!status.success(), "子のプロセスは強制終了で止まる");

        // 書き込んでいたファイルは、どれか 1 つの書き込みの内容のとおりで、壊れていない。
        let contents = fs::read_to_string(&path).unwrap();
        assert!(
            expected.contains(&contents),
            "{wait_ms} ms で止めたとき、壊れていない内容が残る（{} バイト）",
            contents.len()
        );
        let dir = AppDataDir::new(temp.path());
        let loaded = dir
            .read_versioned_json("progress.json", 1, initial)
            .unwrap();
        assert_eq!(loaded.broken_backup, None, "退避されない");
    }
}

// ---- 同じファイルへの書き込みを、複数のスレッドから同時に行う ----

#[test]
fn concurrent_writes_to_the_same_file_leave_one_of_the_writes() {
    const THREADS: usize = 8;
    const WRITES: usize = 5;
    let temp = tempfile::tempdir().unwrap();
    let dir = AppDataDir::new(temp.path());
    let values: Vec<_> = (0..THREADS).map(large_progress).collect();
    let expected_bytes: Vec<_> = values
        .iter()
        .map(|value| {
            let mut bytes = serde_json::to_vec_pretty(value).unwrap();
            bytes.push(b'\n');
            bytes
        })
        .collect();
    let path = temp.path().join("progress.json");
    let barrier = Barrier::new(THREADS + 1);

    thread::scope(|scope| {
        for value in &values {
            scope.spawn(|| {
                barrier.wait();
                for _ in 0..WRITES {
                    dir.write_json("progress.json", value).unwrap();
                }
            });
        }
        // 書き込みの最中に読んでも、いつも、どれか 1 つの書き込みの内容のとおりである。
        scope.spawn(|| {
            barrier.wait();
            for _ in 0..THREADS * WRITES {
                if let Ok(bytes) = fs::read(&path) {
                    assert!(
                        expected_bytes.contains(&bytes),
                        "書き込みの途中の内容が見えた"
                    );
                }
            }
        });
    });

    let bytes = fs::read(&path).unwrap();
    assert!(
        expected_bytes.contains(&bytes),
        "どれか 1 つの書き込みの内容のとおり"
    );
    assert_eq!(
        file_names(temp.path()),
        ["progress.json"],
        "一時ファイルが残らない"
    );
}

#[test]
fn concurrent_reads_of_a_broken_file_back_it_up_once() {
    const THREADS: usize = 8;
    let temp = tempfile::tempdir().unwrap();
    let dir = AppDataDir::new(temp.path());
    dir.write_text("progress.json", "{ 壊れている").unwrap();
    let barrier = Barrier::new(THREADS);

    let backups: Vec<_> = thread::scope(|scope| {
        let handles: Vec<_> = (0..THREADS)
            .map(|_| {
                scope.spawn(|| {
                    barrier.wait();
                    dir.read_versioned_json("progress.json", 1, initial)
                        .unwrap()
                })
            })
            .collect();
        handles
            .into_iter()
            .filter_map(|handle| handle.join().unwrap().broken_backup)
            .collect()
    });

    assert_eq!(backups.len(), 1, "退避するのは 1 回だけ");
    assert_eq!(fs::read_to_string(&backups[0]).unwrap(), "{ 壊れている");
    assert_eq!(
        file_names(temp.path()).len(),
        2,
        "progress.json と退避したファイル"
    );
}

// ---- schema_version を持つ JSON を読む ----

#[test]
fn reads_a_valid_file() {
    let temp = tempfile::tempdir().unwrap();
    let dir = AppDataDir::new(temp.path());
    let value = Progress {
        schema_version: 1,
        lessons: vec!["ch01-01".into()],
    };
    dir.write_json("progress.json", &value).unwrap();
    assert_eq!(
        dir.read_versioned_json("progress.json", 1, initial)
            .unwrap(),
        Loaded {
            value,
            broken_backup: None
        }
    );
}

#[test]
fn a_missing_file_gives_the_initial_state_without_creating_the_file() {
    let temp = tempfile::tempdir().unwrap();
    let dir = AppDataDir::new(temp.path());
    assert_eq!(
        dir.read_versioned_json("progress.json", 1, initial)
            .unwrap(),
        Loaded {
            value: initial(),
            broken_backup: None
        }
    );
    assert!(file_names(temp.path()).is_empty());
}

/// 読み込めない JSON は `.broken-{日時}` を付けて退避され、初期状態で作り直される。
#[test]
fn unreadable_files_are_backed_up_and_recreated() {
    let cases: [(&str, &[u8]); 6] = [
        ("JSON として読めない", b"{ \"schema_version\": 1, "),
        ("UTF-8 でない", b"\xff\xfe\x00"),
        (
            "形が合わない",
            br#"{ "schema_version": 1, "lessons": "ch01-01" }"#,
        ),
        ("schema_version が無い", br#"{ "lessons": [] }"#),
        (
            "schema_version が数でない",
            br#"{ "schema_version": "1", "lessons": [] }"#,
        ),
        (
            "知らない schema_version",
            br#"{ "schema_version": 2, "lessons": [] }"#,
        ),
    ];
    for (case, contents) in cases {
        let temp = tempfile::tempdir().unwrap();
        let dir = AppDataDir::new(temp.path());
        fs::write(temp.path().join("progress.json"), contents).unwrap();

        let loaded = dir
            .read_versioned_json("progress.json", 1, initial)
            .unwrap();

        assert_eq!(loaded.value, initial(), "{case}: 初期状態を返す");
        let backup = loaded
            .broken_backup
            .unwrap_or_else(|| panic!("{case}: 退避したことが分かる"));
        assert_eq!(backup.parent(), Some(temp.path()), "{case}");
        let backup_name = backup.file_name().unwrap().to_str().unwrap();
        let stamp = backup_name
            .strip_prefix("progress.json.broken-")
            .unwrap_or_else(|| panic!("{case}: 退避の名前 {backup_name}"));
        assert!(
            chrono::NaiveDateTime::parse_from_str(stamp, "%Y%m%dT%H%M%S").is_ok(),
            "{case}: 退避の名前の日時 {stamp}"
        );
        assert_eq!(
            fs::read(&backup).unwrap(),
            contents,
            "{case}: 元の内容を退避する"
        );

        // 作り直したファイルは、初期状態として読める。
        let reloaded = dir
            .read_versioned_json("progress.json", 1, initial)
            .unwrap();
        assert_eq!(
            reloaded,
            Loaded {
                value: initial(),
                broken_backup: None
            },
            "{case}: 作り直したファイルを読める"
        );
        assert_eq!(file_names(temp.path()).len(), 2, "{case}");
    }
}

#[test]
fn read_failures_are_io_errors_and_nothing_is_backed_up() {
    let temp = tempfile::tempdir().unwrap();
    let dir = AppDataDir::new(temp.path());
    // ディレクトリはファイルとして読めない。
    fs::create_dir(temp.path().join("progress.json")).unwrap();

    let error = dir
        .read_versioned_json("progress.json", 1, initial)
        .unwrap_err();
    assert!(matches!(error, StorageError::Io { .. }), "{error:?}");
    let command_error = CommandError::from(error);
    assert_eq!(command_error.code, ErrorCode::IoError);
    assert!(command_error.detail.is_some());
    assert_eq!(file_names(temp.path()), ["progress.json"]);
}

// ---- 学習者データのディレクトリ ----

/// アプリでは、Tauri の `app_data_dir()`（アプリの識別子の名前のディレクトリ）を使う。
/// パスを求めるだけで、ディレクトリを読み書きしない。
#[test]
fn the_app_uses_tauri_app_data_dir() {
    let app = progrust_lib::app_builder(tauri::test::mock_builder())
        .build(progrust_lib::app_context())
        .expect("モックのアプリを作れない");
    let dir = AppDataDir::from_app(app.handle()).unwrap();
    assert!(
        dir.root().ends_with("com.progrust.desktop"),
        "{}",
        dir.root().display()
    );
}
