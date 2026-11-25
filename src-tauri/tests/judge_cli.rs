//! 判定だけを行うコマンドラインのプログラム（`src/bin/progrust-judge.rs`。spec/content-guidelines/check-tool.md、
//! ADR 0208、ADR 0242）を、実物の `cargo` で動かして確かめる。テストごとに、一時ディレクトリにレッスンと作業領域を作る。
//!
//! 許可クレートを使う演習のテスト（名前が `network_` で始まるもの）は、作業領域の初回準備でクレートをダウンロード
//! するので、ネットワークが要る（ダウンロード済みなら要らない）。初回準備の失敗は、偽の `cargo` で確かめる。

#[path = "fake_tools/rust.rs"]
mod fake_rust;

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use fake_rust::FakeRust;
use progrust_lib::runner::RunnerDir;
use progrust_lib::runner::prepared::{
    LOST_CRATES_ERROR, PreparedRecord, crates_hash, rustc_verbose_version, write_prepared,
};
use progrust_lib::runner::workspace::WorkspaceDir;
use serde_json::{Value, json};

const STDOUT_LESSON: &str = r#"{
  "id": "ch01-01",
  "chapter": "ch01",
  "kind": "lesson",
  "title": "t",
  "exercise": {
    "layout": "single",
    "judge": { "mode": "stdout", "target": "src/main.rs" },
    "dependencies": [],
    "readonly_files": [],
    "case_labels": {}
  }
}"#;

const CASES: &str = r#"[
  { "name": "基本", "stdin": "2 3\n", "expected_stdout": "5\n" },
  { "name": "負の数", "stdin": "-1 1\n", "expected_stdout": "0\n" }
]"#;

/// 標準入力の 2 つの整数から `expr` を計算して出力するプログラム。
fn program(expr: &str) -> String {
    format!(
        "use std::io::Read;\n\
         fn main() {{\n\
         \x20   let mut input = String::new();\n\
         \x20   std::io::stdin().read_to_string(&mut input).unwrap();\n\
         \x20   let n: Vec<i64> = input.split_whitespace().map(|s| s.parse().unwrap()).collect();\n\
         \x20   let (a, b) = (n[0], n[1]);\n\
         \x20   println!(\"{{}}\", {expr});\n\
         }}\n"
    )
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// `files` を、レッスンのディレクトリからの相対パスと内容の組で書く。
fn lesson(dir: &Path, lesson_json: &str, files: &[(&str, &str)]) {
    write(&dir.join("lesson.json"), lesson_json);
    for (path, content) in files {
        write(&dir.join(path), content);
    }
}

fn judge(lesson_dir: &Path, files: &str, workspace: &Path) -> Output {
    judge_with(lesson_dir, files, workspace, Path::new(env!("CARGO")))
}

/// `cargo` を使って判定する。
fn judge_with(lesson_dir: &Path, files: &str, workspace: &Path, cargo: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_progrust-judge"))
        .arg("--lesson")
        .arg(lesson_dir)
        .args(["--files", files, "--workspace"])
        .arg(workspace)
        .env("CARGO", cargo)
        .output()
        .unwrap()
}

/// 許可クレートの `rand` を使い、標準入力の 2 つの整数から `expr` を計算して出力するプログラム。
/// `rand` の値は 0 しか出ないので、出力は `expr` の値になる。
fn rand_program(expr: &str) -> String {
    format!(
        "use rand::Rng;\n\
         use std::io::Read;\n\
         fn main() {{\n\
         \x20   let mut input = String::new();\n\
         \x20   std::io::stdin().read_to_string(&mut input).unwrap();\n\
         \x20   let n: Vec<i64> = input.split_whitespace().map(|s| s.parse().unwrap()).collect();\n\
         \x20   let (a, b) = (n[0], n[1]);\n\
         \x20   let zero: i64 = rand::rng().random_range(0..1);\n\
         \x20   println!(\"{{}}\", zero + {expr});\n\
         }}\n"
    )
}

/// 許可クレートの `rand` を使う stdout 方式のレッスン。
fn crates_lesson(dir: &Path, solution: &str, starter: &str) {
    lesson(
        dir,
        &STDOUT_LESSON.replace(r#""dependencies": []"#, r#""dependencies": ["rand"]"#),
        &[
            ("exercise/cases.json", CASES),
            ("exercise/solution/src/main.rs", solution),
            ("exercise/starter/src/main.rs", starter),
        ],
    );
}

/// 終了コード 0 で終わり、標準出力の JSON を返す。
fn judgement(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn returns_the_stdout_judgement_of_the_solution_and_the_starter_as_json() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("lessons/ch01-01");
    lesson(
        &dir,
        STDOUT_LESSON,
        &[
            ("exercise/cases.json", CASES),
            ("exercise/solution/src/main.rs", &program("a + b")),
            ("exercise/starter/src/main.rs", &program("a * b")),
        ],
    );
    let workspace = temp.path().join("workspace");

    assert_eq!(
        judgement(&judge(&dir, "solution", &workspace)),
        json!({
            "status": "passed",
            "passed": 2,
            "total": 2,
            "cases": [
                { "name": "基本", "status": "pass", "detail": null },
                { "name": "負の数", "status": "pass", "detail": null },
            ],
            "build_output": "",
        })
    );
    assert_eq!(
        judgement(&judge(&dir, "starter", &workspace)),
        json!({
            "status": "wrong",
            "passed": 0,
            "total": 2,
            "cases": [
                {
                    "name": "基本",
                    "status": "fail",
                    "detail": { "input": "2 3\n", "expected": "5\n", "actual": "6\n" },
                },
                {
                    "name": "負の数",
                    "status": "fail",
                    "detail": { "input": "-1 1\n", "expected": "0\n", "actual": "-1\n" },
                },
            ],
            "build_output": "",
        })
    );
    // 作業領域は、渡したディレクトリの下に、アプリと同じ分け方で作る。
    assert!(workspace.join("project/src/main.rs").is_file());
    assert!(workspace.join("target-std").is_dir());
}

#[test]
fn returns_the_test_judgement_with_the_learner_tests() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("lessons/ch01-02");
    let solution = "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n\
                    fn main() {}\n\
                    #[cfg(test)]\nmod tests {\n    #[test]\n    fn mine() {\n        assert_eq!(super::add(2, 2), 4);\n    }\n}\n";
    lesson(
        &dir,
        r#"{
          "id": "ch01-02", "chapter": "ch01", "kind": "lesson", "title": "t",
          "exercise": {
            "layout": "single",
            "judge": { "mode": "test", "target": "src/main.rs", "min_learner_tests": 1 },
            "dependencies": [],
            "readonly_files": [],
            "case_labels": { "adds": "足し算" }
          }
        }"#,
        &[
            (
                "exercise/judge_tests.rs",
                "use super::*;\n#[test]\nfn adds() {\n    assert_eq!(add(1, 2), 3);\n}\n",
            ),
            ("exercise/solution/src/main.rs", solution),
            (
                "exercise/starter/src/main.rs",
                "pub fn add(_a: i32, _b: i32) -> i32 {\n    0\n}\nfn main() {}\n",
            ),
        ],
    );
    let workspace = temp.path().join("workspace");

    let passed = judgement(&judge(&dir, "solution", &workspace));
    assert_eq!(passed["status"], "passed");
    assert_eq!(
        (&passed["passed"], &passed["total"]),
        (&json!(2), &json!(2))
    );
    assert_eq!(passed["cases"][0]["name"], "足し算");
    assert_eq!(passed["cases"][1]["name"], "自分で書いたテスト（1 件以上）");

    let wrong = judgement(&judge(&dir, "starter", &workspace));
    assert_eq!(wrong["status"], "wrong");
    assert_eq!((&wrong["passed"], &wrong["total"]), (&json!(0), &json!(2)));
}

#[test]
fn compile_errors_come_with_the_build_output() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("lessons/ch01-03");
    lesson(
        &dir,
        STDOUT_LESSON,
        &[
            ("exercise/cases.json", CASES),
            (
                "exercise/solution/src/main.rs",
                &program("a + undefined_value"),
            ),
        ],
    );
    let result = judgement(&judge(&dir, "solution", &temp.path().join("workspace")));
    assert_eq!(result["status"], "compile_error");
    assert_eq!(result["cases"], json!([]));
    let build_output = result["build_output"].as_str().unwrap();
    assert!(build_output.contains("undefined_value"), "{build_output}");
    // ANSI エスケープを除く。
    assert!(!build_output.contains('\u{1b}'), "{build_output:?}");
}

#[test]
fn lessons_that_cannot_be_judged_fail_with_a_message() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");

    let video_only = temp.path().join("lessons/video");
    lesson(
        &video_only,
        r#"{ "id": "video", "chapter": "ch01", "kind": "video_only", "title": "t" }"#,
        &[],
    );
    let missing_cases = temp.path().join("lessons/missing");
    lesson(
        &missing_cases,
        STDOUT_LESSON,
        &[("exercise/solution/src/main.rs", &program("a + b"))],
    );

    for (dir, expected) in [
        (&video_only, "演習の無いレッスン"),
        (&missing_cases, "cases.json"),
    ] {
        let output = judge(dir, "solution", &workspace);
        assert_eq!(output.status.code(), Some(1), "{}", dir.display());
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(expected), "{stderr}");
    }
    // 判定しなかったので、作業領域を作っていない。
    assert!(!workspace.exists());
}

/// 許可クレートを使う演習は、判定の前に作業領域で初回準備をしてから、アプリと同じく判定する（ネットワークが要る）。
#[test]
fn network_judges_lessons_that_use_allowed_crates_after_preparing_the_workspace() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("lessons/ch01-04");
    crates_lesson(&dir, &rand_program("a + b"), &rand_program("a * b"));
    let workspace = temp.path().join("workspace");

    let passed = judgement(&judge(&dir, "solution", &workspace));
    assert_eq!(passed["status"], "passed", "{passed}");
    assert_eq!(
        (&passed["passed"], &passed["total"]),
        (&json!(2), &json!(2))
    );
    // 初回準備をして、prepared.json に ready を書いた。target-validate/ への複製（手順の 4）はしない。
    let prepared: Value =
        serde_json::from_slice(&fs::read(workspace.join("prepared.json")).unwrap()).unwrap();
    assert_eq!(prepared["status"], "ready", "{prepared}");
    assert!(workspace.join("warmup/Cargo.toml").is_file());
    assert!(workspace.join("target").is_dir());
    assert!(!workspace.join("target-validate").exists());
    let prepared_at = prepared["prepared_at"].clone();

    let wrong = judgement(&judge(&dir, "starter", &workspace));
    assert_eq!(wrong["status"], "wrong", "{wrong}");
    // 「実行する条件」に当てはまらないので、2 回目は初回準備をしない。
    let prepared: Value =
        serde_json::from_slice(&fs::read(workspace.join("prepared.json")).unwrap()).unwrap();
    assert_eq!(prepared["prepared_at"], prepared_at);
}

/// 初回準備に失敗したら、判定結果を返さずに 0 以外の終了コードで終わり、理由を標準エラー出力に書く。
#[test]
fn a_failed_preparation_exits_without_a_judgement() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("lessons/ch01-05");
    crates_lesson(&dir, &rand_program("a + b"), &rand_program("a * b"));
    let workspace = temp.path().join("workspace");
    let fake = FakeRust::new("1.100.0");
    // 偽の cargo の `cargo build` を失敗させる（tests/fake_tools/cargo の説明）。
    fs::write(fake.cargo().with_file_name("cargo-build"), "fail").unwrap();

    let output = judge_with(&dir, "solution", &workspace, fake.cargo());
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("初回準備に失敗"), "{stderr}");
    assert!(stderr.contains("`cargo build`が失敗しました"), "{stderr}");
    // 初回準備の cargo build だけを動かし、判定のビルドはしていない。
    assert_eq!(
        fs::read_to_string(fake.cargo().with_file_name("cargo-calls")).unwrap(),
        "build --quiet\n"
    );
    // 失敗を prepared.json に書いた。rustc は、cargo と同じディレクトリのものを使った。
    let prepared: Value =
        serde_json::from_slice(&fs::read(workspace.join("prepared.json")).unwrap()).unwrap();
    assert_eq!(prepared["status"], "failed", "{prepared}");
    let rustc = Command::new(fake.rustc()).arg("-vV").output().unwrap();
    assert_eq!(
        prepared["rustc_version"],
        String::from_utf8(rustc.stdout).unwrap()
    );
}

/// 判定のビルドが、初回準備のやり直しが要る失敗（許可クレートが失われている）になったときは、`prepared.json` の
/// `status` を `failed` にして、初回準備をやり直す。ここでは、cargo をオフラインにして、やり直しの初回準備を失敗させ、
/// 判定結果を返さずに 0 以外の終了コードで終わることを確かめる（ネットワークは使わない）。
#[test]
fn lost_crates_prepare_the_workspace_again() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("lessons/ch01-07");
    crates_lesson(&dir, &rand_program("a + b"), &rand_program("a * b"));
    let workspace = temp.path().join("workspace");
    // 初回準備が済んだことにして、クレートのダウンロード先（CARGO_HOME）を空にする。
    let cargo = Path::new(env!("CARGO"));
    let runner = RunnerDir::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../runner"));
    write_prepared(
        &WorkspaceDir::new(&workspace),
        &PreparedRecord::ready(
            crates_hash(&runner).unwrap(),
            rustc_verbose_version(&cargo.with_file_name("rustc")).unwrap(),
            "2026-11-24T19:02:00+09:00",
        ),
    )
    .unwrap();
    // 初回準備の手順の 1 で作る allcrates/（spec/exercise-runner/crates-and-workspace.md「作業領域」）。
    write(
        &workspace.join("allcrates/Cargo.toml"),
        &format!(
            "[package]\nname = \"progrust_allowed_crates\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n{}",
            fs::read_to_string(runner.allowed_crates()).unwrap()
        ),
    );
    write(&workspace.join("allcrates/src/lib.rs"), "");
    let cargo_home = temp.path().join("cargo-home");
    fs::create_dir(&cargo_home).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_progrust-judge"))
        .arg("--lesson")
        .arg(&dir)
        .args(["--files", "solution", "--workspace"])
        .arg(&workspace)
        .env("CARGO", cargo)
        .env("CARGO_HOME", &cargo_home)
        .env("CARGO_NET_OFFLINE", "true")
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("初回準備に失敗"), "{stderr}");
    assert!(stderr.contains("`cargo build`が失敗しました"), "{stderr}");
    // やり直した初回準備の失敗を、prepared.json に書いた。
    let prepared: Value =
        serde_json::from_slice(&fs::read(workspace.join("prepared.json")).unwrap()).unwrap();
    assert_eq!(prepared["status"], "failed", "{prepared}");
    assert_ne!(prepared["error"], LOST_CRATES_ERROR, "{prepared}");
    assert!(workspace.join("warmup/Cargo.toml").is_file());
}

/// 許可クレートを使わない演習は、初回準備をしない。
#[test]
fn lessons_without_allowed_crates_do_not_prepare_the_workspace() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("lessons/ch01-06");
    lesson(
        &dir,
        STDOUT_LESSON,
        &[
            ("exercise/cases.json", CASES),
            ("exercise/solution/src/main.rs", &program("a + b")),
        ],
    );
    let workspace = temp.path().join("workspace");
    assert_eq!(
        judgement(&judge(&dir, "solution", &workspace))["status"],
        "passed"
    );
    assert!(!workspace.join("prepared.json").exists());
    assert!(!workspace.join("warmup").exists());
}

#[test]
fn wrong_arguments_exit_with_usage() {
    let output = Command::new(env!("CARGO_BIN_EXE_progrust-judge"))
        .args(["--lesson", "x", "--files", "answer"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("使い方"));
}
