//! C のビルドの部品のテスト。
//!
//! - 引数・作業ディレクトリ・環境変数・終了コード・制限時間と止める操作は、`clang` の代わりの `/bin/sh` のスクリプト
//!   で確かめる（どの OS でも実行する）。
//! - 実物の `clang` でのビルドは、macOS のときだけ実行する（spec/non-functional/testing.md「CI」、ADR 0276）。

use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    process::Command,
    time::{Duration, Instant},
};

use super::*;
use crate::runner::{
    c_project::assemble_c_project, process::OutputStream, workspace::WorkspaceDir,
};

// ---- 引数と出力のまとめ方 ----

#[test]
fn clang_arguments_follow_the_spec() {
    // c-project-assembly.md「ビルド」の表のコマンドの、`clang` より後ろ。
    assert_eq!(
        CBuildKind::Unchecked.args().join(" "),
        "-std=c17 -g -O0 -Wall -Wextra -fcolor-diagnostics -o main main.c"
    );
    assert_eq!(
        CBuildKind::Checked.args().join(" "),
        "-std=c17 -g -O0 -Wall -Wextra -fcolor-diagnostics -fsanitize=address,undefined \
         -fno-sanitize-recover=all -fno-omit-frame-pointer -o main-checked main.c"
    );
    assert_eq!(
        CBuildKind::LeakCheckLibrary.args().join(" "),
        "-dynamiclib -O2 -o leakcheck/libprogrust_leakcheck.dylib leakcheck/leakcheck.c"
    );
    assert_eq!(CBuildKind::Unchecked.output_file(), "main");
    assert_eq!(CBuildKind::Checked.output_file(), "main-checked");
    assert_eq!(
        CBuildKind::LeakCheckLibrary.output_file(),
        "leakcheck/libprogrust_leakcheck.dylib"
    );
}

#[test]
fn the_default_time_limit_is_the_build_time_limit() {
    let spec = CBuildSpec::new("/usr/bin/clang", "/w/c-project", CBuildKind::Unchecked);
    assert_eq!(spec.time_limit, Duration::from_secs(120));
}

#[test]
fn stdout_and_stderr_are_kept_together_in_the_order_received() {
    let chunk = |stream, text: &str| OutputChunk {
        stream,
        text: text.into(),
    };
    let mut output = Vec::new();
    push_chunks(
        &mut output,
        vec![
            chunk(OutputStream::Stderr, "warning: 1\n"),
            chunk(OutputStream::Stdout, "out\n"),
        ],
    );
    push_chunks(
        &mut output,
        vec![
            chunk(OutputStream::Stderr, ""),
            chunk(OutputStream::Stderr, "error: 2\n"),
        ],
    );
    let outcome = CBuildOutcome {
        status: BuildStatus::Succeeded,
        output,
    };
    assert_eq!(
        outcome.output,
        [CBuildOutputPiece {
            kind: CBuildOutputKind::Clang,
            text: "warning: 1\nout\nerror: 2\n".into(),
        }]
    );
    assert_eq!(outcome.clang_output(), "warning: 1\nout\nerror: 2\n");
}

// ---- clang の代わりのスクリプトで確かめる ----

/// 状態が変わるのを待つ最長の時間。
const WAIT: Duration = Duration::from_secs(30);

/// テストごとの一時ディレクトリの作業領域と、`clang` の代わりのスクリプト。
struct FakeFixture {
    _temp: tempfile::TempDir,
    workspace: WorkspaceDir,
    clang: PathBuf,
}

impl FakeFixture {
    /// `script`（`/bin/sh` のスクリプトの本体）を `clang` の代わりにする。`c-project/` は `main.c` だけで組み立てる。
    fn new(script: &str) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let workspace = WorkspaceDir::new(temp.path().join("workspace"));
        let files = BTreeMap::from([(
            "main.c".to_string(),
            "int main(void) { return 0; }\n".into(),
        )]);
        assemble_c_project(&workspace, &files).unwrap();

        // 書くために開いたファイルを、このプロセスから実行すると、ほかのテストのスレッドが同時に起動した子プロセスが
        // その書き込みの口を持っていて ETXTBSY で失敗することがある。書くのは別のプロセス（cp）に任せる。
        let draft = temp.path().join("fake-clang.txt");
        fs::write(&draft, format!("#!/bin/sh\n{script}\n")).unwrap();
        let clang = temp.path().join("bin/clang");
        fs::create_dir_all(clang.parent().unwrap()).unwrap();
        let copied = Command::new("cp").arg(&draft).arg(&clang).status().unwrap();
        assert!(copied.success());
        fs::set_permissions(&clang, fs::Permissions::from_mode(0o755)).unwrap();
        Self {
            _temp: temp,
            workspace,
            clang,
        }
    }

    fn spec(&self, kind: CBuildKind) -> CBuildSpec {
        CBuildSpec::new(&self.clang, self.workspace.c_project(), kind)
    }
}

/// `pid` のプロセスがあるか。
fn is_alive(pid: libc::pid_t) -> bool {
    // SAFETY: シグナル 0 は送らずに、プロセスがあるかを確かめるだけである。
    unsafe { libc::kill(pid, 0) == 0 }
}

/// 条件が成り立つまで待つ。
fn wait_until(what: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !condition() {
        assert!(Instant::now() < deadline, "{what}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn clang_is_started_in_c_project_with_the_spec_arguments() {
    let fx = FakeFixture::new(
        "printf 'cwd=%s\\n' \"$(pwd -P)\"\nfor arg in \"$@\"; do printf 'arg=%s\\n' \"$arg\"; done",
    );
    for kind in [
        CBuildKind::Unchecked,
        CBuildKind::Checked,
        CBuildKind::LeakCheckLibrary,
    ] {
        let outcome = c_build(fx.spec(kind)).unwrap();
        assert_eq!(outcome.status, BuildStatus::Succeeded, "{kind:?}");
        let c_project = fx.workspace.c_project().canonicalize().unwrap();
        let mut expected = format!("cwd={}\n", c_project.display());
        for arg in kind.args() {
            expected.push_str(&format!("arg={arg}\n"));
        }
        assert_eq!(outcome.clang_output(), expected, "{kind:?}");
    }
}

#[test]
fn environment_variables_are_not_narrowed_for_clang() {
    // `claude` に渡す 6 つのほかの環境変数として、cargo がテストのプロセスに渡す `CARGO_` で始まるもの
    // （`CARGO_MANIFEST_DIR` など）を使う。シェルや OS が書き換えないので、どの OS でも同じに比べられる。
    // 値に改行を含むものは、行ごとに比べられないので除く。
    let others: Vec<(String, String)> = std::env::vars()
        .filter(|(name, value)| name.starts_with("CARGO_") && !value.contains('\n'))
        .collect();
    assert!(
        !others.is_empty(),
        "cargo がテストに渡す環境変数が無く、確かめられない"
    );
    assert!(
        others
            .iter()
            .all(|(name, _)| !crate::claude::ENV_VAR_NAMES.contains(&name.as_str()))
    );
    let fx = FakeFixture::new("env");
    let outcome = c_build(fx.spec(CBuildKind::Unchecked)).unwrap();
    assert_eq!(outcome.status, BuildStatus::Succeeded);
    let output = outcome.clang_output();
    let lines: Vec<&str> = output.lines().collect();
    for (name, value) in &others {
        let line = format!("{name}={value}");
        assert!(
            lines.contains(&line.as_str()),
            "{name} が clang に渡っていない"
        );
    }
}

#[test]
fn only_exit_code_zero_is_success() {
    let fx = FakeFixture::new("echo 'main.c:1:1: error: だめ' >&2\nexit 1");
    let outcome = c_build(fx.spec(CBuildKind::Unchecked)).unwrap();
    assert_eq!(outcome.status, BuildStatus::Failed { exit_code: Some(1) });
    assert_eq!(outcome.clang_output(), "main.c:1:1: error: だめ\n");
    assert!(
        outcome
            .output
            .iter()
            .all(|piece| piece.kind == CBuildOutputKind::Clang)
    );

    let fx = FakeFixture::new("kill -9 $$");
    let outcome = c_build(fx.spec(CBuildKind::Checked)).unwrap();
    assert_eq!(outcome.status, BuildStatus::Failed { exit_code: None });
}

#[test]
fn the_time_limit_kills_the_whole_process_group_and_keeps_the_output() {
    let fx = FakeFixture::new("echo 'warning: 途中' >&2\nsleep 60 &\necho $! > child.pid\nwait");
    let mut spec = fx.spec(CBuildKind::Unchecked);
    spec.time_limit = Duration::from_secs(1);
    let started = Instant::now();
    let outcome = c_build(spec).unwrap();

    assert!(started.elapsed() >= Duration::from_secs(1));
    assert!(started.elapsed() < WAIT, "{:?}", started.elapsed());
    assert_eq!(outcome.status, BuildStatus::TimedOut);
    assert_eq!(
        outcome.output,
        [
            CBuildOutputPiece {
                kind: CBuildOutputKind::Clang,
                text: "warning: 途中\n".into(),
            },
            CBuildOutputPiece {
                kind: CBuildOutputKind::System,
                text: "ビルド時間の制限（1 秒）を超えたため中断しました。".into(),
            },
        ]
    );
    assert_eq!(outcome.clang_output(), "warning: 途中\n");
    let child: libc::pid_t = fs::read_to_string(fx.workspace.c_project().join("child.pid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    wait_until("同じプロセスグループの子が止まらない", || !is_alive(child));
}

#[test]
fn stopping_kills_the_whole_process_group_and_discards_the_output() {
    let fx = FakeFixture::new("echo 'warning: 途中' >&2\nsleep 60 &\necho $! > child.pid\nwait");
    let pid_file = fx.workspace.c_project().join("child.pid");
    let running = start_c_build(fx.spec(CBuildKind::Checked)).unwrap();
    let handle = running.handle();
    wait_until("子が起動しない", || {
        fs::read_to_string(&pid_file).is_ok_and(|text| text.ends_with('\n'))
    });

    handle.stop();
    let outcome = running.wait().unwrap();
    assert_eq!(outcome.status, BuildStatus::Stopped);
    assert!(outcome.output.is_empty(), "{:?}", outcome.output);
    let child: libc::pid_t = fs::read_to_string(&pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    wait_until("同じプロセスグループの子が止まらない", || !is_alive(child));
}

#[test]
fn a_clang_that_cannot_be_started_is_an_error() {
    let temp = tempfile::tempdir().unwrap();
    let spec = CBuildSpec::new(
        temp.path().join("no-clang"),
        temp.path(),
        CBuildKind::Unchecked,
    );
    assert!(c_build(spec).is_err());
}

// ---- 実物の clang でビルドする（macOS のときだけ） ----

#[cfg(target_os = "macos")]
mod macos {
    use super::*;

    use std::path::Path;

    use crate::environment::ClangSearch;
    use crate::runner::c_project::{leakcheck_library, write_leakcheck_source};

    /// テストごとの一時ディレクトリの作業領域。パスに空白と日本語を含める（技術検証 #293「空白と日本語を含むパス」）。
    struct Fixture {
        _temp: tempfile::TempDir,
        workspace: WorkspaceDir,
        clang: PathBuf,
    }

    impl Fixture {
        fn new(main_c: &str) -> Self {
            let temp = tempfile::tempdir().unwrap();
            let workspace = WorkspaceDir::new(
                temp.path()
                    .join("Application Support/日本語 データ/workspace"),
            );
            let files = BTreeMap::from([("main.c".to_string(), main_c.to_string())]);
            assemble_c_project(&workspace, &files).unwrap();
            let clang = ClangSearch::from_process(None)
                .detect()
                .expect("clang が見つからない（Command Line Tools を入れる）")
                .path;
            Self {
                _temp: temp,
                workspace,
                clang,
            }
        }

        fn spec(&self, kind: CBuildKind) -> CBuildSpec {
            CBuildSpec::new(&self.clang, self.workspace.c_project(), kind)
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.workspace.c_project().join(relative)
        }

        /// `c-project/` の中に、`clang` が `#include` すると読み終わらない名前付きパイプを作る。
        fn make_fifo(&self, name: &str) -> PathBuf {
            let path = self.path(name);
            let made = Command::new("mkfifo").arg(&path).status().unwrap();
            assert!(made.success());
            path
        }
    }

    /// 終わったら成功し、標準出力を返す。
    fn run(executable: &Path) -> String {
        let output = Command::new(executable).output().unwrap();
        assert!(output.status.success(), "{output:?}");
        String::from_utf8(output.stdout).unwrap()
    }

    /// 名前付きパイプを読もうとしていたプロセス（`clang`）が、もういないことを確かめる。書く側として、待たずに
    /// 開けない（ENXIO）ことで確かめる。開けたときは、読む側が残っている。そのとき閉じると、読む側に EOF が渡って
    /// `clang` が先に進んでしまうので、やり直さずに 1 回で決める。SIGKILL が届くのを少し待ってから確かめる。
    fn assert_fifo_has_no_reader(path: &Path) {
        use std::os::unix::fs::OpenOptionsExt;
        std::thread::sleep(Duration::from_millis(500));
        let opened = fs::OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(path);
        match opened {
            Err(error) => assert_eq!(error.raw_os_error(), Some(libc::ENXIO), "{error}"),
            Ok(_) => panic!("clang が止まらずに、名前付きパイプを読もうとしている"),
        }
    }

    const HELLO: &str = "#include <stdio.h>\n\nint main(void) {\n    printf(\"こんにちは\\n\");\n    return 0;\n}\n";

    #[test]
    fn the_unchecked_build_makes_main_with_debug_info() {
        let fx = Fixture::new(HELLO);
        let outcome = c_build(fx.spec(CBuildKind::Unchecked)).unwrap();
        assert_eq!(
            outcome.status,
            BuildStatus::Succeeded,
            "{:?}",
            outcome.output
        );
        assert_eq!(outcome.clang_output(), "");
        assert_eq!(run(&fx.path("main")), "こんにちは\n");
        assert!(fx.path("main.dSYM").is_dir());
        assert!(!fx.path("main-checked").exists());
    }

    #[test]
    fn the_checked_build_makes_main_checked_and_its_dsym() {
        let fx = Fixture::new(HELLO);
        let outcome = c_build(fx.spec(CBuildKind::Checked)).unwrap();
        assert_eq!(
            outcome.status,
            BuildStatus::Succeeded,
            "{:?}",
            outcome.output
        );
        assert_eq!(run(&fx.path("main-checked")), "こんにちは\n");
        assert!(fx.path("main-checked.dSYM").is_dir());
        assert!(!fx.path("main").exists());

        // 検査つきなので、範囲外アクセスで異常終了する。
        let fx = Fixture::new(
            "#include <stdlib.h>\n\nint main(void) {\n    int *a = malloc(sizeof(int) * 2);\n    a[2] = 1;\n    free(a);\n    return 0;\n}\n",
        );
        let outcome = c_build(fx.spec(CBuildKind::Checked)).unwrap();
        assert_eq!(
            outcome.status,
            BuildStatus::Succeeded,
            "{:?}",
            outcome.output
        );
        let status = Command::new(fx.path("main-checked"))
            .output()
            .unwrap()
            .status;
        assert!(!status.success(), "{status:?}");
    }

    #[test]
    fn warnings_are_delivered_in_color_and_the_build_succeeds() {
        let fx = Fixture::new("int main(void) {\n    int unused = 1;\n    return 0;\n}\n");
        let outcome = c_build(fx.spec(CBuildKind::Unchecked)).unwrap();
        assert_eq!(outcome.status, BuildStatus::Succeeded);
        let output = outcome.clang_output();
        assert!(output.contains("warning"), "{output}");
        assert!(output.contains("unused"), "{output}");
        assert!(output.contains("main.c:2"), "{output}");
        // `-fcolor-diagnostics` で、パイプでも色（ANSI エスケープ）が付く。
        assert!(output.contains('\u{1b}'), "{output:?}");
        assert!(fx.path("main").is_file());
    }

    #[test]
    fn compile_errors_fail_with_a_nonzero_exit_code_and_the_diagnostics() {
        let fx = Fixture::new("int main(void) {\n    return undefined_name;\n}\n");
        for kind in [CBuildKind::Unchecked, CBuildKind::Checked] {
            let outcome = c_build(fx.spec(kind)).unwrap();
            assert!(
                matches!(outcome.status, BuildStatus::Failed { exit_code: Some(code) } if code != 0),
                "{kind:?}: {:?}",
                outcome.status
            );
            let output = outcome.clang_output();
            assert!(output.contains("error"), "{output}");
            assert!(output.contains("undefined_name"), "{output}");
            assert!(output.contains('\u{1b}'), "{output:?}");
            assert!(!fx.path(kind.output_file()).exists(), "{kind:?}");
        }
    }

    #[test]
    fn the_leak_check_library_is_built_from_the_written_source() {
        let fx = Fixture::new(HELLO);
        write_leakcheck_source(
            &fx.workspace,
            "#include <stdlib.h>\n\nvoid *progrust_test_alloc(size_t size) { return malloc(size); }\n",
        )
        .unwrap();
        let outcome = c_build(fx.spec(CBuildKind::LeakCheckLibrary)).unwrap();
        assert_eq!(
            outcome.status,
            BuildStatus::Succeeded,
            "{:?}",
            outcome.output
        );
        assert!(leakcheck_library(&fx.workspace).is_file());

        // ソースが誤っていれば失敗する。
        write_leakcheck_source(&fx.workspace, "これは C ではない\n").unwrap();
        let outcome = c_build(fx.spec(CBuildKind::LeakCheckLibrary)).unwrap();
        assert!(
            matches!(outcome.status, BuildStatus::Failed { .. }),
            "{:?}",
            outcome.status
        );
    }

    #[test]
    fn the_build_time_limit_stops_clang() {
        let fx = Fixture::new("#include \"hang.h\"\n\nint main(void) {\n    return 0;\n}\n");
        let fifo = fx.make_fifo("hang.h");
        let mut spec = fx.spec(CBuildKind::Unchecked);
        spec.time_limit = Duration::from_secs(2);
        let started = Instant::now();
        let outcome = c_build(spec).unwrap();

        assert!(started.elapsed() >= Duration::from_secs(2));
        assert!(started.elapsed() < WAIT, "{:?}", started.elapsed());
        assert_eq!(outcome.status, BuildStatus::TimedOut);
        assert_eq!(
            outcome.output.last(),
            Some(&CBuildOutputPiece {
                kind: CBuildOutputKind::System,
                text: "ビルド時間の制限（2 秒）を超えたため中断しました。".into(),
            })
        );
        assert_fifo_has_no_reader(&fifo);
        assert!(!fx.path("main").exists());
    }

    #[test]
    fn stopping_stops_clang_and_the_next_build_succeeds() {
        let fx = Fixture::new("#include \"hang.h\"\n\nint main(void) {\n    return 0;\n}\n");
        let fifo = fx.make_fifo("hang.h");
        let running = start_c_build(fx.spec(CBuildKind::Checked)).unwrap();
        let handle = running.handle();
        std::thread::sleep(Duration::from_millis(500));

        let stopped_at = Instant::now();
        handle.stop();
        let outcome = running.wait().unwrap();
        assert!(stopped_at.elapsed() < Duration::from_secs(5));
        assert_eq!(outcome.status, BuildStatus::Stopped);
        assert!(outcome.output.is_empty(), "{:?}", outcome.output);
        assert_fifo_has_no_reader(&fifo);

        // 組み立て直した `c-project/` で、次のビルドが成功する。
        let files = BTreeMap::from([("main.c".to_string(), HELLO.to_string())]);
        assemble_c_project(&fx.workspace, &files).unwrap();
        let outcome = c_build(fx.spec(CBuildKind::Checked)).unwrap();
        assert_eq!(
            outcome.status,
            BuildStatus::Succeeded,
            "{:?}",
            outcome.output
        );
        assert_eq!(run(&fx.path("main-checked")), "こんにちは\n");
    }
}
