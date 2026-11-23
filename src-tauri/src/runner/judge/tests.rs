//! `stdout` 方式の判定の部品のテスト。結果の決め方と正規化は作った値で、判定の全体は実物の `cargo`（このテストを
//! 動かしている cargo）でビルドしたプログラムで確かめる。

use std::{collections::BTreeMap, time::Instant};

use super::*;
use crate::runner::{
    RunnerDir,
    build::{BuildCommand, BuildSpec, BuildStatus, build},
    project::{assemble_project, build_env},
    workspace::WorkspaceDir,
};

fn case(name: &str, stdin: &str, expected_stdout: &str) -> StdoutCase {
    StdoutCase {
        name: name.into(),
        stdin: stdin.into(),
        expected_stdout: expected_stdout.into(),
    }
}

fn collected(termination: Termination, stdout: &str, stderr: &str) -> CollectedOutput {
    CollectedOutput {
        termination,
        stdout: stdout.into(),
        stderr: stderr.into(),
    }
}

fn output_detail(input: &str, expected: &str, actual: &str) -> Option<CaseDetail> {
    Some(CaseDetail::Output {
        input: input.into(),
        expected: expected.into(),
        actual: actual.into(),
    })
}

fn stderr_detail(text: &str) -> Option<CaseDetail> {
    Some(CaseDetail::Stderr(text.into()))
}

// ---- 制限値と cases.json ----

#[test]
fn limits_follow_the_spec() {
    assert_eq!(CASE_TIME_LIMIT, Duration::from_secs(5));
    assert_eq!(CASE_OUTPUT_LIMIT, 64 * 1024);
    assert_eq!(
        CaseLimits::default(),
        CaseLimits {
            time: Duration::from_secs(5),
            stdout: 64 * 1024,
        }
    );
}

#[test]
fn cases_json_in_the_spec_shape_is_read() {
    let cases = parse_cases(
        r#"[
          { "name": "基本の例", "stdin": "3\n1 2 3\n", "expected_stdout": "6\n" },
          { "name": "空の入力", "stdin": "0\n\n", "expected_stdout": "0\n" }
        ]"#,
    )
    .unwrap();
    assert_eq!(
        cases,
        [
            case("基本の例", "3\n1 2 3\n", "6\n"),
            case("空の入力", "0\n\n", "0\n"),
        ]
    );
}

// ---- 正規化 ----

#[test]
fn normalization_unifies_crlf_and_drops_trailing_spaces_and_blank_lines() {
    assert_eq!(normalize("a\r\nb\r\n"), "a\nb");
    assert_eq!(normalize("a  \nb\t\n"), "a\nb");
    assert_eq!(normalize("a\n\n  \n\r\n"), "a");
    assert_eq!(normalize("a"), "a");
    assert_eq!(normalize("a\n"), "a");
    assert_eq!(normalize(""), "");
    assert_eq!(normalize("\n\n"), "");
    // 空白は Unicode の空白（全角の空白を含む）とする。
    assert_eq!(normalize("あ\u{3000}\n"), "あ");
}

#[test]
fn normalization_keeps_leading_spaces_and_inner_or_leading_blank_lines() {
    assert_eq!(normalize("  a\n"), "  a");
    assert_eq!(normalize("a\n\nb\n"), "a\n\nb");
    assert_eq!(normalize("\na\n"), "\na");
    assert_eq!(normalize("a b\n"), "a b");
}

// ---- テストケースの結果の決め方 ----

#[test]
fn output_that_matches_after_normalization_passes() {
    let case = case("例", "1\n", "6\n");
    let result = case_result(&case, &collected(Termination::Exited(0), "6  \r\n\n", "")).unwrap();
    assert_eq!(
        result,
        CaseResult {
            name: "例".into(),
            status: CaseStatus::Pass,
            detail: None,
        }
    );
}

#[test]
fn different_output_fails_with_the_input_and_both_outputs_before_normalization() {
    let case = case("例", "1\n", "6\n");
    let result = case_result(&case, &collected(Termination::Exited(0), "7 \n", "")).unwrap();
    assert_eq!(result.status, CaseStatus::Fail);
    assert_eq!(result.detail, output_detail("1\n", "6\n", "7 \n"));
}

#[test]
fn nonzero_exit_or_signal_is_a_runtime_error_even_if_the_output_matches() {
    let case = case("例", "", "6\n");
    let exited = case_result(
        &case,
        &collected(Termination::Exited(101), "6\n", "panicked at src/main.rs\n"),
    )
    .unwrap();
    assert_eq!(exited.status, CaseStatus::RuntimeError);
    assert_eq!(exited.detail, stderr_detail("panicked at src/main.rs\n"));

    let signaled = case_result(&case, &collected(Termination::Signaled(6), "6\n", "")).unwrap();
    assert_eq!(signaled.status, CaseStatus::RuntimeError);
    assert_eq!(signaled.detail, stderr_detail(""));
}

#[test]
fn timeout_and_output_limit_come_before_other_results() {
    let case = case("例", "1\n", "6\n");
    // 止めたプログラムは、出力が期待値と一致していても、止めた理由で決まる。
    let timed_out = case_result(
        &case,
        &collected(Termination::TimedOut, "6\n", "眠ります\n"),
    )
    .unwrap();
    assert_eq!(timed_out.status, CaseStatus::Timeout);
    assert_eq!(timed_out.detail, stderr_detail("眠ります\n"));

    let too_much = case_result(
        &case,
        &collected(Termination::OutputLimitExceeded, "6\n", "エラー\n"),
    )
    .unwrap();
    assert_eq!(too_much.status, CaseStatus::Fail);
    assert_eq!(too_much.detail, output_detail("1\n", "6\n", "6\n"));
}

#[test]
fn a_stopped_case_has_no_result() {
    let case = case("例", "", "6\n");
    assert_eq!(
        case_result(&case, &collected(Termination::Stopped, "6\n", "")),
        None
    );
}

// ---- 判定結果 ----

fn result_with(name: &str, status: CaseStatus) -> CaseResult {
    CaseResult {
        name: name.into(),
        status,
        detail: None,
    }
}

#[test]
fn the_result_is_passed_only_when_every_case_passes() {
    let passed = JudgeResult::from_cases(vec![
        result_with("a", CaseStatus::Pass),
        result_with("b", CaseStatus::Pass),
    ]);
    assert_eq!(passed.status, JudgeStatus::Passed);
    assert_eq!((passed.passed, passed.total), (2, 2));

    let wrong = JudgeResult::from_cases(vec![
        result_with("a", CaseStatus::Pass),
        result_with("b", CaseStatus::Timeout),
        result_with("c", CaseStatus::Fail),
    ]);
    assert_eq!(wrong.status, JudgeStatus::Wrong);
    assert_eq!((wrong.passed, wrong.total), (1, 3));
    assert_eq!(
        wrong
            .cases
            .iter()
            .map(|case| case.name.as_str())
            .collect::<Vec<_>>(),
        ["a", "b", "c"]
    );
}

#[test]
fn a_stopped_result_has_no_cases() {
    assert_eq!(
        JudgeResult::stopped(),
        JudgeResult {
            status: JudgeStatus::Stopped,
            passed: 0,
            total: 0,
            cases: Vec::new(),
        }
    );
}

#[test]
fn results_serialize_in_the_spec_shape() {
    let result = JudgeResult::from_cases(vec![
        result_with("基本の例", CaseStatus::Pass),
        CaseResult {
            name: "違う出力".into(),
            status: CaseStatus::Fail,
            detail: output_detail("1\n", "6\n", "7\n"),
        },
        CaseResult {
            name: "panic".into(),
            status: CaseStatus::RuntimeError,
            detail: stderr_detail("panicked\n"),
        },
        CaseResult {
            name: "時間切れ".into(),
            status: CaseStatus::Timeout,
            detail: stderr_detail(""),
        },
    ]);
    assert_eq!(
        serde_json::to_value(&result).unwrap(),
        serde_json::json!({
            "status": "wrong",
            "passed": 1,
            "total": 4,
            "cases": [
                { "name": "基本の例", "status": "pass", "detail": null },
                {
                    "name": "違う出力",
                    "status": "fail",
                    "detail": { "input": "1\n", "expected": "6\n", "actual": "7\n" },
                },
                { "name": "panic", "status": "runtime_error", "detail": "panicked\n" },
                { "name": "時間切れ", "status": "timeout", "detail": "" },
            ],
        })
    );
    assert_eq!(
        serde_json::to_value(JudgeResult::from_cases(vec![result_with(
            "a",
            CaseStatus::Pass
        )]))
        .unwrap()["status"],
        "passed"
    );
    assert_eq!(
        serde_json::to_value(JudgeResult::stopped()).unwrap(),
        serde_json::json!({ "status": "stopped", "passed": 0, "total": 0, "cases": [] })
    );
}

// ---- 実物の cargo でビルドしたプログラムを判定する ----

/// 標準入力の 1 行目で、することを選ぶプログラム。
const PROGRAM: &str = r#"use std::io::{self, Read, Write};

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut out = io::stdout();
    match input.lines().next().unwrap_or("") {
        "sum" => {
            let numbers = input.lines().nth(1).unwrap_or("");
            let sum: i64 = numbers.split_whitespace().map(|n| n.parse::<i64>().unwrap()).sum();
            println!("{sum}");
        }
        "trailing" => print!("6  \r\n\t\n\n"),
        "wrong" => println!("7"),
        "panic" => {
            println!("6");
            panic!("最後に panic しました");
        }
        "exit" => {
            println!("6");
            eprintln!("終了コード 3 で終わります");
            std::process::exit(3);
        }
        "sleep" => {
            eprintln!("眠ります");
            std::thread::sleep(std::time::Duration::from_secs(30));
            println!("6");
        }
        "exact" => {
            out.write_all(&vec![b'a'; 64 * 1024 - 1]).unwrap();
            out.write_all(b"\n").unwrap();
        }
        "over" => {
            out.write_all(&vec![b'a'; 64 * 1024]).unwrap();
            out.write_all(b"\n").unwrap();
        }
        "noisy" => {
            let line = vec![b'e'; 1023];
            let mut err = io::stderr();
            for _ in 0..1024 {
                err.write_all(&line).unwrap();
                err.write_all(b"\n").unwrap();
            }
            println!("6");
        }
        other => panic!("知らない指示: {other}"),
    }
}
"#;

/// テストごとの一時ディレクトリの作業領域で、[`PROGRAM`] をビルドしたもの。
struct Built {
    _temp: tempfile::TempDir,
    workspace: WorkspaceDir,
    executable: PathBuf,
}

fn build_program() -> Built {
    let temp = tempfile::tempdir().unwrap();
    let workspace = WorkspaceDir::new(temp.path().join("workspace"));
    let runner = RunnerDir::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../runner"));
    let files = BTreeMap::from([("src/main.rs".to_string(), PROGRAM.to_string())]);
    assemble_project(&workspace, &runner, &[], &files).unwrap();
    let built = build(BuildSpec::new(
        env!("CARGO"),
        workspace.project(),
        BuildCommand::Build,
        build_env(&workspace, &[]),
    ))
    .unwrap();
    assert_eq!(built.status, BuildStatus::Succeeded, "{:?}", built.output);
    let executable = built.main_executable().unwrap().to_path_buf();
    Built {
        _temp: temp,
        workspace,
        executable,
    }
}

impl Built {
    fn spec(&self, cases: Vec<StdoutCase>) -> StdoutJudgeSpec {
        StdoutJudgeSpec {
            executable: self.executable.clone(),
            project_dir: self.workspace.project(),
            envs: build_env(&self.workspace, &[])
                .into_iter()
                .map(|(key, value)| (OsString::from(key), value))
                .collect(),
            cases,
            limits: CaseLimits::default(),
        }
    }
}

fn statuses(result: &JudgeResult) -> Vec<(&str, CaseStatus)> {
    result
        .cases
        .iter()
        .map(|case| (case.name.as_str(), case.status))
        .collect()
}

fn detail_of<'a>(result: &'a JudgeResult, name: &str) -> &'a CaseDetail {
    result
        .cases
        .iter()
        .find(|case| case.name == name)
        .and_then(|case| case.detail.as_ref())
        .unwrap_or_else(|| panic!("{name} の detail が無い"))
}

#[test]
fn every_case_is_judged_with_the_spec_limits_and_progress_is_reported_per_case() {
    let built = build_program();
    let exact = format!("{}\n", "a".repeat(64 * 1024 - 1));
    let over = format!("{}\n", "a".repeat(64 * 1024));
    let spec = built.spec(vec![
        case("合計", "sum\n1 2 3\n", "6\n"),
        case("行末の空白", "trailing\n", "6\n"),
        case("違う出力", "wrong\n", "6\n"),
        case("最後に panic", "panic\n", "6\n"),
        case("終了コード 3", "exit\n", "6\n"),
        case("時間切れ", "sleep\n", "6\n"),
        case("ちょうど 64 KB", "exact\n", &exact),
        case("64 KB を超える", "over\n", &over),
        case("標準エラー出力が多い", "noisy\n", "6\n"),
    ]);

    let mut progress = Vec::new();
    let started = Instant::now();
    let result = judge_stdout(&spec, |step| progress.push(step)).unwrap();
    let elapsed = started.elapsed();

    assert_eq!(
        statuses(&result),
        [
            ("合計", CaseStatus::Pass),
            ("行末の空白", CaseStatus::Pass),
            ("違う出力", CaseStatus::Fail),
            ("最後に panic", CaseStatus::RuntimeError),
            ("終了コード 3", CaseStatus::RuntimeError),
            ("時間切れ", CaseStatus::Timeout),
            ("ちょうど 64 KB", CaseStatus::Pass),
            ("64 KB を超える", CaseStatus::Fail),
            ("標準エラー出力が多い", CaseStatus::Pass),
        ]
    );
    assert_eq!(result.status, JudgeStatus::Wrong);
    assert_eq!((result.passed, result.total), (4, 9));
    assert_eq!(
        progress,
        (1..=9)
            .map(|completed| JudgeProgress {
                completed,
                total: 9
            })
            .collect::<Vec<_>>()
    );
    // 時間切れのテストケースは、5 秒の制限で止めた（プログラムは 30 秒眠る）。
    assert!(
        elapsed >= Duration::from_secs(5) && elapsed < Duration::from_secs(25),
        "{elapsed:?}"
    );

    assert_eq!(
        detail_of(&result, "違う出力"),
        &CaseDetail::Output {
            input: "wrong\n".into(),
            expected: "6\n".into(),
            actual: "7\n".into(),
        }
    );
    let CaseDetail::Stderr(panic) = detail_of(&result, "最後に panic") else {
        panic!("panic の detail が標準エラー出力でない");
    };
    assert!(panic.contains("最後に panic しました"), "{panic}");
    assert_eq!(
        detail_of(&result, "終了コード 3"),
        &CaseDetail::Stderr("終了コード 3 で終わります\n".into())
    );
    assert_eq!(
        detail_of(&result, "時間切れ"),
        &CaseDetail::Stderr("眠ります\n".into())
    );
    let CaseDetail::Output { actual, .. } = detail_of(&result, "64 KB を超える") else {
        panic!("出力過多の detail が出力でない");
    };
    assert_eq!(actual.len(), 64 * 1024);
}

#[test]
fn a_correct_program_passes_every_case() {
    let built = build_program();
    let spec = built.spec(vec![
        case("基本の例", "sum\n1 2 3\n", "6\n"),
        case("負の数", "sum\n-1 -2\n", "-3\n"),
        case("行末の空白", "trailing\n", "6"),
    ]);
    let result = judge_stdout(&spec, |_| {}).unwrap();
    assert_eq!(result.status, JudgeStatus::Passed);
    assert_eq!((result.passed, result.total), (3, 3));
    assert!(result.cases.iter().all(|case| case.detail.is_none()));
}

#[test]
fn a_missing_executable_is_an_error() {
    let temp = tempfile::tempdir().unwrap();
    let spec = StdoutJudgeSpec {
        executable: temp.path().join("無い"),
        project_dir: temp.path().to_path_buf(),
        envs: Vec::new(),
        cases: vec![case("例", "", "")],
        limits: CaseLimits::default(),
    };
    let mut progress = Vec::new();
    assert!(judge_stdout(&spec, |step| progress.push(step)).is_err());
    assert!(progress.is_empty());
}
