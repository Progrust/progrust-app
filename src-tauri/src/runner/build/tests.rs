//! ビルドの部品のテスト。cargo の JSON を読む部分は作った行で、ビルドは実物の `cargo`（このテストを動かしている
//! cargo）で確かめる。実物の cargo のテストは、許可クレートを使わない演習（依存の無いプロジェクト）にする。

use std::{
    collections::BTreeMap,
    process::Command,
    thread,
    time::{Duration, Instant},
};

use super::*;
use crate::runner::{
    RunnerDir,
    project::{assemble_project, build_env},
    workspace::WorkspaceDir,
};

// ---- cargo の JSON を読む ----

fn stdout(text: &str) -> OutputChunk {
    OutputChunk {
        stream: OutputStream::Stdout,
        text: text.into(),
    }
}

fn stderr(text: &str) -> OutputChunk {
    OutputChunk {
        stream: OutputStream::Stderr,
        text: text.into(),
    }
}

fn diagnostic_line(level: &str, code: Option<&str>, rendered: Option<&str>) -> String {
    let message = serde_json::json!({
        "$message_type": "diagnostic",
        "message": "診断",
        "code": code.map(|code| serde_json::json!({ "code": code, "explanation": null })),
        "level": level,
        "spans": [],
        "children": [],
        "rendered": rendered,
    });
    let line = serde_json::json!({
        "reason": "compiler-message",
        "package_id": "path+file:///p#progrust_exercise@0.1.0",
        "target": { "kind": ["bin"], "name": "progrust_exercise", "src_path": "/p/src/main.rs" },
        "message": message,
    });
    format!("{line}\n")
}

fn artifact_line(kind: &str, src_path: &str, test: bool, executable: Option<&str>) -> String {
    let line = serde_json::json!({
        "reason": "compiler-artifact",
        "package_id": "path+file:///p#progrust_exercise@0.1.0",
        "manifest_path": "/p/Cargo.toml",
        "target": {
            "kind": [kind],
            "crate_types": [kind],
            "name": "progrust_exercise",
            "src_path": src_path,
            "edition": "2024",
            "doc": true,
            "doctest": false,
            "test": true,
        },
        "profile": { "opt_level": "0", "debuginfo": 2, "test": test },
        "features": [],
        "filenames": [],
        "executable": executable,
        "fresh": false,
    });
    format!("{line}\n")
}

fn collect(chunks: Vec<OutputChunk>) -> Collected {
    let mut collector = Collector::default();
    collector.push_chunks(chunks);
    collector.finish()
}

fn piece(kind: BuildOutputKind, text: &str) -> BuildOutputPiece {
    BuildOutputPiece {
        kind,
        text: text.into(),
    }
}

#[test]
fn diagnostics_and_cargo_stderr_are_kept_in_the_order_received() {
    let collected = collect(vec![
        stdout(&diagnostic_line(
            "warning",
            Some("unused_variables"),
            Some("警告1\n"),
        )),
        stderr("warning: cargo の lint\n"),
        stdout(&diagnostic_line("error", Some("E0382"), Some("エラー\n"))),
        stdout(&diagnostic_line(
            "failure-note",
            None,
            Some("For more information\n"),
        )),
        stderr("error: could not compile `progrust_exercise`\n"),
    ]);
    assert_eq!(
        collected.output,
        [
            piece(BuildOutputKind::CompilerMessage, "警告1\n"),
            piece(BuildOutputKind::CargoStderr, "warning: cargo の lint\n"),
            piece(
                BuildOutputKind::CompilerMessage,
                "エラー\nFor more information\n"
            ),
            piece(
                BuildOutputKind::CargoStderr,
                "error: could not compile `progrust_exercise`\n"
            ),
        ]
    );
}

#[test]
fn json_lines_split_across_chunks_are_read_once_complete() {
    let line = diagnostic_line("error", Some("E0308"), Some("型が違う\n"));
    let (head, tail) = line.split_at(line.len() / 2);
    let (middle, tail) = tail.split_at(tail.len() / 2);
    let collected = collect(vec![
        stdout(head),
        stderr("途中の cargo の出力\n"),
        stdout(middle),
        stdout(tail),
    ]);
    // 診断は、行が揃った時点（cargo の出力の後）に並ぶ。
    assert_eq!(
        collected.output,
        [
            piece(BuildOutputKind::CargoStderr, "途中の cargo の出力\n"),
            piece(BuildOutputKind::CompilerMessage, "型が違う\n"),
        ]
    );
    assert_eq!(collected.error_codes, ["E0308"]);
}

#[test]
fn a_last_line_without_newline_is_read_and_a_broken_one_is_ignored() {
    let line = diagnostic_line("error", Some("E0425"), Some("見つからない\n"));
    let collected = collect(vec![stdout(line.trim_end())]);
    assert_eq!(collected.error_codes, ["E0425"]);

    // 止めたときなどに途中で切れた行は、JSON として読めないので無視する。
    let collected = collect(vec![stdout(&line[..line.len() / 2])]);
    assert!(collected.output.is_empty());
    assert!(collected.error_codes.is_empty());
}

#[test]
fn error_codes_are_from_error_level_only_unique_and_in_order() {
    let collected = collect(vec![stdout(
        &[
            diagnostic_line("warning", Some("unused_variables"), Some("w\n")),
            diagnostic_line("error", Some("E0382"), Some("a\n")),
            diagnostic_line("error", None, Some("aborting\n")),
            diagnostic_line("error", Some("E0308"), Some("b\n")),
            diagnostic_line("error", Some("E0382"), Some("c\n")),
            diagnostic_line("failure-note", None, Some("note\n")),
            diagnostic_line("error", Some("dead_code"), Some("deny した lint\n")),
        ]
        .concat(),
    )]);
    assert_eq!(collected.error_codes, ["E0382", "E0308", "dead_code"]);
}

#[test]
fn diagnostics_without_rendered_text_and_non_json_lines_are_skipped() {
    let collected = collect(vec![stdout(
        &[
            "これは JSON ではない\n".to_string(),
            "{\"reason\":\"build-script-executed\",\"package_id\":\"x\"}\n".to_string(),
            diagnostic_line("error", Some("E0001"), None),
            "{\"reason\":\"build-finished\",\"success\":false}\n".to_string(),
        ]
        .concat(),
    )]);
    assert!(collected.output.is_empty());
    assert_eq!(collected.error_codes, ["E0001"]);
    assert!(collected.artifacts.is_empty());
}

#[test]
fn artifacts_without_executables_are_skipped() {
    let collected = collect(vec![stdout(
        &[
            artifact_line("lib", "/p/src/lib.rs", false, None),
            artifact_line("bin", "/p/src/main.rs", false, Some("/t/progrust_exercise")),
            artifact_line("lib", "/p/src/lib.rs", true, Some("/t/lib-test")),
        ]
        .concat(),
    )]);
    assert_eq!(
        collected.artifacts,
        [
            BuildArtifact {
                src_path: "/p/src/main.rs".into(),
                kinds: vec!["bin".into()],
                test: false,
                executable: "/t/progrust_exercise".into(),
            },
            BuildArtifact {
                src_path: "/p/src/lib.rs".into(),
                kinds: vec!["lib".into()],
                test: true,
                executable: "/t/lib-test".into(),
            },
        ]
    );
}

#[test]
fn executables_are_chosen_by_root_file_and_test_profile() {
    let collected = collect(vec![stdout(
        &[
            artifact_line("bin", "/p/src/bin/other.rs", false, Some("/t/other")),
            artifact_line("bin", "/p/src/main.rs", true, Some("/t/main-test")),
            artifact_line("bin", "/p/src/main.rs", false, Some("/t/main")),
            artifact_line("lib", "/p/src/lib.rs", true, Some("/t/lib-test")),
            artifact_line("test", "/p/tests/it.rs", true, Some("/t/it-test")),
        ]
        .concat(),
    )]);
    let outcome = BuildOutcome {
        status: BuildStatus::Succeeded,
        output: collected.output,
        error_codes: collected.error_codes,
        artifacts: collected.artifacts,
        project_dir: "/p".into(),
    };
    assert_eq!(outcome.main_executable(), Some(Path::new("/t/main")));
    assert_eq!(
        outcome.test_executable(Path::new("src/main.rs")),
        Some(Path::new("/t/main-test"))
    );
    assert_eq!(
        outcome.test_executable(Path::new("src/lib.rs")),
        Some(Path::new("/t/lib-test"))
    );
    assert_eq!(
        outcome.test_executable(Path::new("/p/tests/it.rs")),
        Some(Path::new("/t/it-test"))
    );
    assert_eq!(outcome.test_executable(Path::new("src/shapes.rs")), None);
}

#[test]
fn attempt_output_splits_diagnostics_and_cargo_stderr_without_system_messages() {
    let outcome = BuildOutcome {
        status: BuildStatus::TimedOut,
        output: vec![
            piece(BuildOutputKind::CompilerMessage, "診断1\n"),
            piece(BuildOutputKind::CargoStderr, "cargo1\n"),
            piece(BuildOutputKind::CompilerMessage, "診断2\n"),
            piece(BuildOutputKind::CargoStderr, "cargo2\n"),
            piece(BuildOutputKind::System, &timeout_message(BUILD_TIME_LIMIT)),
        ],
        error_codes: Vec::new(),
        artifacts: Vec::new(),
        project_dir: "/p".into(),
    };
    assert_eq!(
        outcome.attempt_output(),
        BuildOutput {
            compiler_messages: "診断1\n診断2\n".into(),
            cargo_stderr: "cargo1\ncargo2\n".into(),
        }
    );
}

#[test]
fn cargo_arguments_follow_the_spec() {
    assert_eq!(
        BuildCommand::Build.args(),
        [
            "build",
            "--offline",
            "--quiet",
            "--message-format=json-diagnostic-rendered-ansi"
        ]
    );
    assert_eq!(
        BuildCommand::TestNoRun.args(),
        [
            "test",
            "--offline",
            "--quiet",
            "--no-run",
            "--message-format=json-diagnostic-rendered-ansi"
        ]
    );
    assert_eq!(BUILD_TIME_LIMIT, Duration::from_secs(120));
    assert_eq!(
        timeout_message(BUILD_TIME_LIMIT),
        "ビルド時間の制限（120 秒）を超えたため中断しました。"
    );
}

// ---- 実物の cargo でビルドする ----

/// 状態が変わるのを待つ最長の時間。
const WAIT: Duration = Duration::from_secs(30);

/// コンパイルが終わらないコード（定数の評価が終わらない）。
const ENDLESS_COMPILE: &str = "#![allow(long_running_const_eval)]\n\
     const X: u64 = {\n\
         let mut i: u64 = 0;\n\
         loop {\n\
             i = i.wrapping_add(1);\n\
         }\n\
     };\n\
     fn main() {\n\
         println!(\"{X}\");\n\
     }\n";

/// テストごとの一時ディレクトリの作業領域。
struct Fixture {
    temp: tempfile::TempDir,
    workspace: WorkspaceDir,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let workspace = WorkspaceDir::new(temp.path().join("workspace"));
        Self { temp, workspace }
    }

    /// 同梱の `runner/` で、許可クレートを使わない演習の `project/` を組み立てる。
    fn assemble(&self, files: &[(&str, &str)]) {
        let runner = RunnerDir::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../runner"));
        let files: BTreeMap<String, String> = files
            .iter()
            .map(|(path, content)| (path.to_string(), content.to_string()))
            .collect();
        assemble_project(&self.workspace, &runner, &[], &files).unwrap();
    }

    /// プロジェクトの組み立ての環境変数で、`project/` をビルドする指定。
    fn spec(&self, command: BuildCommand) -> BuildSpec {
        BuildSpec::new(
            env!("CARGO"),
            self.workspace.project(),
            command,
            build_env(&self.workspace, &[]),
        )
    }

    /// このテストの一時ディレクトリの名前。ビルドの rustc の引数（出力先）に入る。
    fn unique_name(&self) -> String {
        self.temp
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned()
    }

    /// 引数にこのテストの一時ディレクトリを含むプロセス（ビルドの rustc）があるか。
    fn has_build_process(&self) -> bool {
        Command::new("pgrep")
            .args(["-f", &self.unique_name()])
            .output()
            .unwrap()
            .status
            .success()
    }
}

/// 終わったら成功し、出力を返す。
fn run_executable(executable: &Path, args: &[&str]) -> String {
    let output = Command::new(executable).args(args).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn compile_errors_fail_with_diagnostics_and_error_codes() {
    let fx = Fixture::new();
    fx.assemble(&[(
        "src/main.rs",
        "fn main() {\n\
             let s = String::from(\"a\");\n\
             let t = s;\n\
             println!(\"{s} {t}\");\n\
             let u = String::from(\"b\");\n\
             let v = u;\n\
             println!(\"{u} {v}\");\n\
         }\n",
    )]);
    let outcome = build(fx.spec(BuildCommand::Build)).unwrap();

    assert_eq!(
        outcome.status,
        BuildStatus::Failed {
            exit_code: Some(101)
        }
    );
    assert_eq!(outcome.error_codes, ["E0382"]);
    let output = outcome.attempt_output();
    assert!(
        output.compiler_messages.contains("E0382"),
        "{}",
        output.compiler_messages
    );
    // 色の指定（ANSI エスケープ）が付いたまま受け取る。
    assert!(output.compiler_messages.contains('\u{1b}'));
    assert!(
        output.cargo_stderr.contains("could not compile"),
        "{}",
        output.cargo_stderr
    );
    assert!(
        outcome
            .output
            .iter()
            .all(|piece| piece.kind != BuildOutputKind::System)
    );
    assert_eq!(outcome.main_executable(), None);
}

#[test]
fn warnings_only_build_succeeds_with_warning_diagnostics_and_an_executable() {
    let fx = Fixture::new();
    fx.assemble(&[(
        "src/main.rs",
        "fn main() {\n    let unused = 1;\n    println!(\"hello\");\n}\n",
    )]);
    let outcome = build(fx.spec(BuildCommand::Build)).unwrap();

    assert_eq!(outcome.status, BuildStatus::Succeeded);
    assert!(outcome.error_codes.is_empty());
    let output = outcome.attempt_output();
    assert!(
        output.compiler_messages.contains("unused variable"),
        "{}",
        output.compiler_messages
    );
    let executable = outcome.main_executable().expect("実行ファイルが無い");
    assert_eq!(run_executable(executable, &[]), "hello\n");
}

#[test]
fn the_executable_of_src_main_rs_is_chosen_over_src_bin() {
    let fx = Fixture::new();
    fx.assemble(&[
        ("src/main.rs", "fn main() {\n    println!(\"main\");\n}\n"),
        (
            "src/bin/other.rs",
            "fn main() {\n    println!(\"other\");\n}\n",
        ),
    ]);
    let outcome = build(fx.spec(BuildCommand::Build)).unwrap();
    assert_eq!(outcome.status, BuildStatus::Succeeded);
    let executable = outcome.main_executable().expect("実行ファイルが無い");
    assert_eq!(run_executable(executable, &[]), "main\n");
}

#[test]
fn test_executables_of_main_lib_and_integration_tests_are_told_apart() {
    let fx = Fixture::new();
    fx.assemble(&[
        (
            "src/main.rs",
            "fn main() {}\n\
             #[cfg(test)]\n\
             mod tests {\n    #[test]\n    fn in_main() {}\n}\n",
        ),
        (
            "src/lib.rs",
            "pub fn f() {}\n\
             #[cfg(test)]\n\
             mod tests {\n    #[test]\n    fn in_lib() {}\n}\n",
        ),
        ("tests/it.rs", "#[test]\nfn in_it() {}\n"),
    ]);
    let outcome = build(fx.spec(BuildCommand::TestNoRun)).unwrap();
    assert_eq!(outcome.status, BuildStatus::Succeeded);

    let main = outcome
        .test_executable(Path::new("src/main.rs"))
        .expect("src/main.rs のテスト実行ファイルが無い");
    let lib = outcome
        .test_executable(Path::new("src/lib.rs"))
        .expect("src/lib.rs のテスト実行ファイルが無い");
    let it = outcome
        .test_executable(Path::new("tests/it.rs"))
        .expect("tests/it.rs のテスト実行ファイルが無い");
    assert_eq!(
        outcome.test_executable(&fx.workspace.project().join("src/lib.rs")),
        Some(lib)
    );

    assert_eq!(
        run_executable(main, &["--list", "--format=terse"]),
        "tests::in_main: test\n"
    );
    assert_eq!(
        run_executable(lib, &["--list", "--format=terse"]),
        "tests::in_lib: test\n"
    );
    assert_eq!(
        run_executable(it, &["--list", "--format=terse"]),
        "in_it: test\n"
    );
}

/// 条件が成り立つまで待つ。
fn wait_until(what: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !condition() {
        assert!(Instant::now() < deadline, "{what}");
        thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn stopping_during_the_build_stops_rustc_and_the_next_build_succeeds() {
    let fx = Fixture::new();
    fx.assemble(&[("src/main.rs", ENDLESS_COMPILE)]);
    let running = start_build(fx.spec(BuildCommand::Build)).unwrap();
    let handle = running.handle();
    wait_until("ビルドの rustc が起動しない", || {
        fx.has_build_process()
    });

    let stopped_at = Instant::now();
    handle.stop();
    let outcome = running.wait().unwrap();
    assert!(
        stopped_at.elapsed() < Duration::from_secs(5),
        "止めるのに時間がかかった: {:?}",
        stopped_at.elapsed()
    );
    assert_eq!(outcome.status, BuildStatus::Stopped);
    assert!(outcome.output.is_empty(), "{:?}", outcome.output);
    assert!(outcome.error_codes.is_empty());
    assert!(outcome.artifacts.is_empty());
    wait_until("ビルドの rustc が止まらない", || {
        !fx.has_build_process()
    });

    // 同じ作業領域（同じビルド先）で、次のビルドが成功する。
    fx.assemble(&[("src/main.rs", "fn main() {\n    println!(\"next\");\n}\n")]);
    let outcome = build(fx.spec(BuildCommand::Build)).unwrap();
    assert_eq!(
        outcome.status,
        BuildStatus::Succeeded,
        "{:?}",
        outcome.output
    );
    let executable = outcome.main_executable().expect("実行ファイルが無い");
    assert_eq!(run_executable(executable, &[]), "next\n");
}

#[test]
fn the_build_time_limit_stops_the_build_and_adds_the_timeout_message() {
    let fx = Fixture::new();
    fx.assemble(&[("src/main.rs", ENDLESS_COMPILE)]);
    let mut spec = fx.spec(BuildCommand::Build);
    spec.time_limit = Duration::from_secs(3);
    let started = Instant::now();
    let outcome = build(spec).unwrap();

    assert!(started.elapsed() >= Duration::from_secs(3));
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(outcome.status, BuildStatus::TimedOut);
    assert_eq!(
        outcome.output.last(),
        Some(&piece(
            BuildOutputKind::System,
            "ビルド時間の制限（3 秒）を超えたため中断しました。"
        ))
    );
    wait_until("ビルドの rustc が止まらない", || {
        !fx.has_build_process()
    });
}
