//! 判定だけを行うコマンドラインのプログラム（`src/bin/progrust-judge.rs`。spec/content-guidelines/check-tool.md、
//! ADR 0208）を、実物の `cargo` で動かして確かめる。テストごとに、一時ディレクトリにレッスンと作業領域を作る。

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

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
    Command::new(env!("CARGO_BIN_EXE_progrust-judge"))
        .arg("--lesson")
        .arg(lesson_dir)
        .args(["--files", files, "--workspace"])
        .arg(workspace)
        .env("CARGO", env!("CARGO"))
        .output()
        .unwrap()
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
    let crates = temp.path().join("lessons/crates");
    lesson(
        &crates,
        &STDOUT_LESSON.replace(r#""dependencies": []"#, r#""dependencies": ["rand"]"#),
        &[
            ("exercise/cases.json", CASES),
            ("exercise/solution/src/main.rs", &program("a + b")),
        ],
    );
    let missing_cases = temp.path().join("lessons/missing");
    lesson(
        &missing_cases,
        STDOUT_LESSON,
        &[("exercise/solution/src/main.rs", &program("a + b"))],
    );

    for (dir, expected) in [
        (&video_only, "演習の無いレッスン"),
        (&crates, "許可クレートを使う演習"),
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

#[test]
fn wrong_arguments_exit_with_usage() {
    let output = Command::new(env!("CARGO_BIN_EXE_progrust-judge"))
        .args(["--lesson", "x", "--files", "answer"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("使い方"));
}
