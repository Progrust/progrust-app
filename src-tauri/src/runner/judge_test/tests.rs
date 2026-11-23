//! `test` 方式の隠しテストの判定の部品のテスト。出力の読み方と結果の決め方は作った値で、判定の全体は実物の `cargo`
//! （このテストを動かしている cargo）でビルドしたテスト実行ファイルで確かめる。

use super::*;
use crate::runner::{
    RunnerDir,
    build::{BuildCommand, BuildSpec, BuildStatus, build},
    judge::JudgeStatus,
    project::{assemble_project, build_env},
    workspace::WorkspaceDir,
};

fn hidden(name: &str, label: &str) -> HiddenTest {
    HiddenTest {
        name: name.into(),
        label: label.into(),
    }
}

fn collected(termination: Termination, stdout: &str, stderr: &str) -> CollectedOutput {
    CollectedOutput {
        termination,
        stdout: stdout.into(),
        stderr: stderr.into(),
    }
}

// ---- 制限値と組み込む内容 ----

#[test]
fn limits_and_the_declaration_follow_the_spec() {
    assert_eq!(TESTS_TIME_LIMIT, Duration::from_secs(10));
    let limits = TestLimits::default();
    assert_eq!(limits.time, Duration::from_secs(10));
    assert_eq!(
        limits.parallelism,
        thread::available_parallelism().unwrap_or(NonZeroUsize::MIN)
    );
    assert_eq!(
        HIDDEN_TESTS_DECLARATION,
        "\n#[cfg(test)]\n#[path = \"../progrust_judge_tests.rs\"]\nmod progrust_judge;\n"
    );
    assert_eq!(HIDDEN_TESTS_FILE, "progrust_judge_tests.rs");
    assert_eq!(
        HIDDEN_TEST_ERROR_HINT,
        "問題文で指定された関数名・型・シグネチャを確認してください。"
    );
}

#[test]
fn hidden_tests_are_placed_and_declared_at_the_end_of_the_target_copy() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path();
    fs::create_dir_all(project.join("src")).unwrap();
    // 最後の行に改行が無くても、足す宣言はその行に続かない。
    fs::write(project.join("src/lib.rs"), "pub fn f() {} // 最後の行").unwrap();
    fs::write(project.join("src/main.rs"), "fn main() {}\n").unwrap();

    embed_hidden_tests(project, "src/lib.rs", "use super::*;\n").unwrap();

    assert_eq!(
        fs::read_to_string(project.join("progrust_judge_tests.rs")).unwrap(),
        "use super::*;\n"
    );
    assert_eq!(
        fs::read_to_string(project.join("src/lib.rs")).unwrap(),
        "pub fn f() {} // 最後の行\n\
         #[cfg(test)]\n\
         #[path = \"../progrust_judge_tests.rs\"]\n\
         mod progrust_judge;\n"
    );
    assert_eq!(
        fs::read_to_string(project.join("src/main.rs")).unwrap(),
        "fn main() {}\n"
    );
}

#[test]
fn a_missing_target_gets_only_the_declaration_and_bad_targets_are_refused() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(project.join("src")).unwrap();

    embed_hidden_tests(&project, "src/lib.rs", "").unwrap();
    assert_eq!(
        fs::read_to_string(project.join("src/lib.rs")).unwrap(),
        HIDDEN_TESTS_DECLARATION
    );

    let error = embed_hidden_tests(&project, "../outside.rs", "").unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    assert!(!temp.path().join("outside.rs").exists());
}

// ---- --list の出力を読む ----

#[test]
fn hidden_test_names_are_read_from_the_list_output_in_order() {
    let stdout = "progrust_judge::adds: test\n\
                  progrust_judge::nested::deep: test\n\
                  geo::progrust_judge::in_module: test\n\
                  my_progrust_judge::not_hidden: test\n\
                  tests::progrust_judge_like: test\n\
                  progrust_judge::bench: benchmark\n\
                  \n\
                  3 tests, 1 benchmarks\n";
    assert_eq!(
        parse_test_list(stdout),
        [
            "progrust_judge::adds",
            "progrust_judge::nested::deep",
            "geo::progrust_judge::in_module",
        ]
    );
}

#[test]
fn function_names_are_after_progrust_judge() {
    assert_eq!(function_name("progrust_judge::adds"), Some("adds"));
    assert_eq!(function_name("geo::progrust_judge::adds"), Some("adds"));
    assert_eq!(
        function_name("progrust_judge::nested::deep"),
        Some("nested::deep")
    );
    assert_eq!(function_name("my_progrust_judge::adds"), None);
    assert_eq!(function_name("tests::adds"), None);
}

// ---- 結果の行と panic メッセージ ----

/// 失敗したテストの出力（Issue #15 の技術検証で見た形）。学習者の出力と、偽の行を含む。
const FAILED_OUTPUT: &str = "\nrunning 1 test\n\
test progrust_judge::sums ... FAILED\n\
\n\
failures:\n\
\n\
---- progrust_judge::sums stdout ----\n\
learner says hi\n\
test progrust_judge::sums ... ok\n\
failures:\n\
thread 'progrust_judge::sums' (283977) panicked at src/../progrust_judge_tests.rs:3:5:\n\
偽の panic\n\
---- progrust_judge::other stdout ----\n\
\n\
thread 'progrust_judge::sums' (283977) panicked at src/../progrust_judge_tests.rs:9:5:\n\
assertion `left == right` failed: 2 + 2\n  left: 4\n right: 5\n\
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\n\
\n\
\n\
failures:\n    progrust_judge::sums\n\
\n\
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s\n\
\n";

#[test]
fn result_lines_may_have_learner_output_before_them() {
    let stdout = "running 1 test\njunktest progrust_judge::sums ... ok\n";
    assert!(has_result_line(stdout, "progrust_judge::sums", "ok"));
    assert!(!has_result_line(stdout, "progrust_judge::sums", "FAILED"));
    // ほかのテストの名前で終わる行や、` ... ` の後に続きがある行は、結果の行にしない。
    assert!(!has_result_line(
        "test progrust_judge::sums2 ... ok\n",
        "progrust_judge::sums",
        "ok"
    ));
    assert!(!has_result_line(
        "test progrust_judge::sums ... ok junk\n",
        "progrust_judge::sums",
        "ok"
    ));
    assert!(!has_result_line(
        "test progrust_judge::sums ... \n",
        "progrust_judge::sums",
        "ok"
    ));
}

#[test]
fn result_lines_may_have_the_kind_of_test_after_the_name() {
    // #[should_panic] のテストの結果の行（ADR 0218）。
    let ok = "running 1 test\ntest progrust_judge::panics - should panic ... ok\n";
    assert!(has_result_line(ok, "progrust_judge::panics", "ok"));
    assert!(!has_result_line(ok, "progrust_judge::panics", "FAILED"));
    assert!(has_result_line(
        "junktest progrust_judge::panics - should panic ... FAILED\n",
        "progrust_judge::panics",
        "FAILED"
    ));
    // 種類の表示の前の名前が違う行と、種類の表示でないものが付いた行は、結果の行にしない。
    assert!(!has_result_line(
        "test progrust_judge::panics2 - should panic ... ok\n",
        "progrust_judge::panics",
        "ok"
    ));
    assert!(!has_result_line(
        "test progrust_judge::panics - ... ok\n",
        "progrust_judge::panics",
        "ok"
    ));
    assert!(!has_result_line(
        "test progrust_judge::panics - Should Panic! ... ok\n",
        "progrust_judge::panics",
        "ok"
    ));
}

#[test]
fn should_panic_failures_have_the_libtest_explanation() {
    // panic の内容が expected と違った（rustc stable の libtest の出力の形）。
    let wrong_message = "\nrunning 1 test\n\
test progrust_judge::panics - should panic ... FAILED\n\
\n\
failures:\n\
\n\
---- progrust_judge::panics stdout ----\n\
\n\
thread 'progrust_judge::panics' (1410024) panicked at src/main.rs:3:9:\n\
負の数です: -1\n\
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\n\
note: panic did not contain expected string\n      \
panic message: \"負の数です: -1\",\n \
expected substring: \"期待する文\"\n\
\n\
failures:\n    progrust_judge::panics\n\
\n\
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s\n";
    assert_eq!(
        panic_message(wrong_message, "progrust_judge::panics"),
        "負の数です: -1\n\
         note: panic did not contain expected string\n      \
         panic message: \"負の数です: -1\",\n \
         expected substring: \"期待する文\""
    );

    // panic しなかった。学習者の出力の後に改行なしで出ても、説明から取り出す。
    let did_not_panic = "\nrunning 1 test\n\
test progrust_judge::panics - should panic ... FAILED\n\
\n\
failures:\n\
\n\
---- progrust_judge::panics stdout ----\n\
途中まで書いたnote: test did not panic as expected at src/../progrust_judge_tests.rs:21:8\n\
\n\
failures:\n    progrust_judge::panics\n";
    assert_eq!(
        panic_message(did_not_panic, "progrust_judge::panics"),
        "note: test did not panic as expected at src/../progrust_judge_tests.rs:21:8"
    );
}

#[test]
fn the_panic_message_is_the_last_panic_after_failures_without_the_note() {
    assert_eq!(
        panic_message(FAILED_OUTPUT, "progrust_judge::sums"),
        "assertion `left == right` failed: 2 + 2\n  left: 4\n right: 5"
    );
}

#[test]
fn the_panic_message_ends_at_the_next_section_without_the_note() {
    let stdout = "test progrust_judge::sums ... FAILED\n\
                  \n\
                  failures:\n\
                  \n\
                  ---- progrust_judge::sums stdout ----\n\
                  thread 'progrust_judge::sums' (1) panicked at src/../progrust_judge_tests.rs:3:5:\n\
                  1 行目\n\
                  \n\
                  3 行目\n\
                  \n\
                  \n\
                  failures:\n    progrust_judge::sums\n";
    assert_eq!(
        panic_message(stdout, "progrust_judge::sums"),
        "1 行目\n\n3 行目"
    );
    // 学習者の出力の後に改行なしで panic しても、panic の行を見つける。
    let stdout = "failures:\n\
                  ---- progrust_judge::sums stdout ----\n\
                  途中まで書いたthread 'progrust_judge::sums' (1) panicked at src/main.rs:1:1:\n\
                  ゼロで割った\n";
    assert_eq!(
        panic_message(stdout, "progrust_judge::sums"),
        "ゼロで割った"
    );
}

#[test]
fn a_failed_test_without_a_panic_line_has_an_empty_message() {
    let stdout = "test progrust_judge::sums ... FAILED\n\nfailures:\n\
                  ---- progrust_judge::sums stdout ----\n\
                  Error: \"Err を返した\"\n";
    assert_eq!(panic_message(stdout, "progrust_judge::sums"), "");
    assert_eq!(panic_message("出力なし\n", "progrust_judge::sums"), "");
}

// ---- テストケースの結果の決め方 ----

#[test]
fn case_results_follow_the_table_in_the_spec() {
    let test = hidden("progrust_judge::sums", "合計");
    let ok = "running 1 test\ntest progrust_judge::sums ... ok\n";
    let started = "running 1 test\ntest progrust_judge::sums ... ";

    let pass = case_result(&test, &collected(Termination::Exited(0), ok, "")).unwrap();
    assert_eq!(
        pass,
        CaseResult {
            name: "合計".into(),
            status: CaseStatus::Pass,
            detail: None,
        }
    );

    let fail = case_result(
        &test,
        &collected(Termination::Exited(101), FAILED_OUTPUT, ""),
    )
    .unwrap();
    assert_eq!(fail.status, CaseStatus::Fail);
    assert_eq!(
        fail.detail,
        Some(CaseDetail::PanicMessage(
            "assertion `left == right` failed: 2 + 2\n  left: 4\n right: 5".into()
        ))
    );

    // 結果の行がないまま終了コード 0 で終わった（process::exit(0)）。
    let exited = case_result(&test, &collected(Termination::Exited(0), started, "")).unwrap();
    assert_eq!(exited.status, CaseStatus::RuntimeError);
    assert_eq!(exited.detail, Some(CaseDetail::Stderr(String::new())));

    // ok の行があっても、終了コードが 0 でなければ通過にしない。
    let nonzero = case_result(&test, &collected(Termination::Exited(3), ok, "")).unwrap();
    assert_eq!(nonzero.status, CaseStatus::RuntimeError);

    let signaled = case_result(
        &test,
        &collected(
            Termination::Signaled(6),
            started,
            "thread 'progrust_judge::sums' has overflowed its stack\n",
        ),
    )
    .unwrap();
    assert_eq!(signaled.status, CaseStatus::RuntimeError);
    assert_eq!(
        signaled.detail,
        Some(CaseDetail::Stderr(
            "thread 'progrust_judge::sums' has overflowed its stack\n".into()
        ))
    );

    // 制限時間で止めたものは、ok の行があっても時間切れ。
    let timed_out =
        case_result(&test, &collected(Termination::TimedOut, ok, "眠ります\n")).unwrap();
    assert_eq!(timed_out.status, CaseStatus::Timeout);
    assert_eq!(
        timed_out.detail,
        Some(CaseDetail::Stderr("眠ります\n".into()))
    );

    assert_eq!(
        case_result(&test, &collected(Termination::Stopped, ok, "")),
        None
    );
}

#[test]
fn the_panic_message_detail_serializes_as_a_string() {
    let result = JudgeResult::from_cases(vec![CaseResult {
        name: "合計".into(),
        status: CaseStatus::Fail,
        detail: Some(CaseDetail::PanicMessage("left: 4".into())),
    }]);
    assert_eq!(
        serde_json::to_value(&result).unwrap()["cases"][0],
        serde_json::json!({ "name": "合計", "status": "fail", "detail": "left: 4" })
    );
}

// ---- 実物の cargo でビルドしたテスト実行ファイルで判定する ----

/// テストごとの一時ディレクトリの作業領域に、組み立てて隠しテストを組み込み、`cargo test --no-run` でビルドしたもの。
struct Built {
    _temp: tempfile::TempDir,
    workspace: WorkspaceDir,
    outcome: BuildOutcome,
}

/// `files`（エディタ上のファイル）で `project/` を組み立て、`judge_tests` を `target` に組み込んでビルドする。
fn build_with_hidden_tests(
    files: &BTreeMap<String, String>,
    target: &str,
    judge_tests: &str,
) -> Built {
    let temp = tempfile::tempdir().unwrap();
    let workspace = WorkspaceDir::new(temp.path().join("workspace"));
    let runner = RunnerDir::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../runner"));
    assemble_project(&workspace, &runner, &[], files).unwrap();
    embed_hidden_tests(&workspace.project(), target, judge_tests).unwrap();
    let outcome = build(BuildSpec::new(
        env!("CARGO"),
        workspace.project(),
        BuildCommand::TestNoRun,
        build_env(&workspace, &[]),
    ))
    .unwrap();
    Built {
        _temp: temp,
        workspace,
        outcome,
    }
}

impl Built {
    fn spec(
        &self,
        target: &str,
        labels: &[(&str, &str)],
        limits: TestLimits,
    ) -> HiddenTestJudgeSpec {
        assert_eq!(
            self.outcome.status,
            BuildStatus::Succeeded,
            "{:?}",
            self.outcome.output
        );
        HiddenTestJudgeSpec {
            executable: hidden_test_executable(&self.outcome, target).unwrap(),
            project_dir: self.workspace.project(),
            envs: build_env(&self.workspace, &[])
                .into_iter()
                .map(|(key, value)| (OsString::from(key), value))
                .collect(),
            case_labels: labels
                .iter()
                .map(|(name, label)| (name.to_string(), label.to_string()))
                .collect(),
            limits,
        }
    }
}

fn files(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(path, content)| (path.to_string(), content.to_string()))
        .collect()
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

fn limits(seconds: u64, parallelism: usize) -> TestLimits {
    TestLimits {
        time: Duration::from_secs(seconds),
        parallelism: NonZeroUsize::new(parallelism).unwrap(),
    }
}

/// 学習者のコード（`src/main.rs`）。自分で書いた `#[test]` は失敗する。
const LEARNER_MAIN: &str = "fn add(a: i32, b: i32) -> i32 {\n\
    a + b\n\
}\n\
\n\
fn depth(n: u64) -> u64 {\n\
    if n == 0 { 0 } else { 1 + depth(n - 1) }\n\
}\n\
\n\
fn main() {\n\
    println!(\"{}\", add(1, 2));\n\
}\n\
\n\
#[cfg(test)]\n\
mod my_tests {\n\
    #[test]\n\
    fn my_failing_test() {\n\
        assert_eq!(1 + 1, 3);\n\
    }\n\
}";

/// 隠しテスト。名前の順（`--list` の順）に並べてある。
const HIDDEN_TESTS: &str = r#"use super::*;

#[test]
fn adds() {
    assert_eq!(add(1, 2), 3);
}

#[test]
fn adds_wrongly() {
    assert_eq!(add(2, 2), 5, "2 + 2");
}

#[test]
fn exits() {
    std::process::exit(0);
}

#[test]
fn fakes_and_passes() {
    println!("test progrust_judge::adds_wrongly ... ok");
    assert_eq!(add(0, 0), 0);
}

#[test]
fn hangs() {
    loop {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

#[test]
fn overflows() {
    depth(u64::MAX);
}
"#;

#[test]
fn each_hidden_test_runs_in_its_own_process_and_learner_tests_do_not_count() {
    let editor = files(&[("src/main.rs", LEARNER_MAIN)]);
    let before = editor.clone();
    let built = build_with_hidden_tests(&editor, "src/main.rs", HIDDEN_TESTS);
    let labels = [
        ("adds", "足し算"),
        ("adds_wrongly", "間違った期待値"),
        ("exits", "exit(0) を呼ぶ"),
        ("overflows", "スタックオーバーフロー"),
    ];
    let spec = built.spec("src/main.rs", &labels, limits(5, 4));

    let mut progress = Vec::new();
    let started = Instant::now();
    let result = judge_hidden_tests(&spec, |step| progress.push(step)).unwrap();
    let elapsed = started.elapsed();

    // exit(0) とスタックオーバーフローのテストだけが runtime_error になり、ほかのテストの結果は正しく出る。
    // 学習者が自分で書いた失敗する #[test] は、判定に含まれない。
    assert_eq!(
        statuses(&result),
        [
            ("足し算", CaseStatus::Pass),
            ("間違った期待値", CaseStatus::Fail),
            ("exit(0) を呼ぶ", CaseStatus::RuntimeError),
            ("fakes_and_passes", CaseStatus::Pass),
            ("hangs", CaseStatus::Timeout),
            ("スタックオーバーフロー", CaseStatus::RuntimeError),
        ]
    );
    assert_eq!(result.status, JudgeStatus::Wrong);
    assert_eq!((result.passed, result.total), (2, 6));
    assert_eq!(
        progress,
        (1..=6)
            .map(|completed| JudgeProgress {
                completed,
                total: 6
            })
            .collect::<Vec<_>>()
    );
    // 終わらないテストは、テスト全体の制限時間（ここでは 5 秒）で止めた。
    assert!(
        elapsed >= Duration::from_secs(5) && elapsed < Duration::from_secs(25),
        "{elapsed:?}"
    );

    // assert_eq! の失敗では、左辺・右辺を含む panic メッセージが detail に入る。
    let CaseDetail::PanicMessage(message) = detail_of(&result, "間違った期待値") else {
        panic!("fail の detail が panic メッセージでない");
    };
    assert!(message.contains("2 + 2"), "{message}");
    assert!(message.contains("left: 4"), "{message}");
    assert!(message.contains("right: 5"), "{message}");
    assert!(!message.contains("RUST_BACKTRACE"), "{message}");
    let CaseDetail::Stderr(overflow) = detail_of(&result, "スタックオーバーフロー")
    else {
        panic!("runtime_error の detail が標準エラー出力でない");
    };
    assert!(overflow.contains("overflowed its stack"), "{overflow}");

    // エディタ上の学習者のファイル（渡したもの）は変わらない。変えたのは project/ の写しだけ。
    assert_eq!(editor, before);
    let copy = fs::read_to_string(built.workspace.project().join("src/main.rs")).unwrap();
    assert_eq!(copy, format!("{LEARNER_MAIN}{HIDDEN_TESTS_DECLARATION}"));
}

#[test]
fn should_panic_hidden_tests_pass_when_the_answer_panics_and_fail_otherwise() {
    // ch09-01 の演習の形（spec/curriculum/exercise-constraints.md、ADR 0218）。学習者のコードは、負の数で panic するが、
    // 0 では panic しない。
    let built = build_with_hidden_tests(
        &files(&[(
            "src/main.rs",
            "fn check(n: i32) {\n    if n < 0 {\n        panic!(\"負の数です: {n}\");\n    }\n}\n\nfn main() {\n    check(1);\n}\n",
        )]),
        "src/main.rs",
        r#"use super::*;

#[test]
#[should_panic]
fn a_panics_on_negative() {
    check(-1);
}

#[test]
#[should_panic(expected = "負の数")]
fn b_panics_with_message() {
    check(-1);
}

#[test]
#[should_panic]
fn c_panics_on_zero() {
    println!("学習者の出力");
    check(0);
}

#[test]
#[should_panic(expected = "0 以上の数を渡してください")]
fn d_panics_with_other_message() {
    check(-1);
}
"#,
    );
    let spec = built.spec("src/main.rs", &[], limits(10, 2));
    let result = judge_hidden_tests(&spec, |_| {}).unwrap();
    assert_eq!(
        statuses(&result),
        [
            ("a_panics_on_negative", CaseStatus::Pass),
            ("b_panics_with_message", CaseStatus::Pass),
            ("c_panics_on_zero", CaseStatus::Fail),
            ("d_panics_with_other_message", CaseStatus::Fail),
        ]
    );
    assert_eq!((result.passed, result.total), (2, 4));

    // panic しなかったときは、libtest の説明が detail に入る。
    let CaseDetail::PanicMessage(did_not_panic) = detail_of(&result, "c_panics_on_zero") else {
        panic!("fail の detail が panic メッセージでない");
    };
    assert!(
        did_not_panic.starts_with("note: test did not panic as expected"),
        "{did_not_panic}"
    );
    // panic の内容が expected と違ったときは、panic メッセージと、期待した内容が detail に入る。
    let CaseDetail::PanicMessage(other_message) = detail_of(&result, "d_panics_with_other_message")
    else {
        panic!("fail の detail が panic メッセージでない");
    };
    assert!(other_message.contains("負の数です: -1"), "{other_message}");
    assert!(
        other_message.contains("0 以上の数を渡してください"),
        "{other_message}"
    );
    assert!(!other_message.contains("RUST_BACKTRACE"), "{other_message}");
}

#[test]
fn tests_not_started_before_the_time_limit_are_timeouts() {
    let built = build_with_hidden_tests(
        &files(&[("src/main.rs", "fn main() {}\n")]),
        "src/main.rs",
        "#[test]\n\
         fn a_hangs() {\n    loop { std::thread::sleep(std::time::Duration::from_millis(50)); }\n}\n\
         #[test]\n\
         fn b_never_starts() {}\n",
    );
    let spec = built.spec("src/main.rs", &[], limits(2, 1));
    let mut progress = Vec::new();
    let result = judge_hidden_tests(&spec, |step| progress.push(step)).unwrap();
    assert_eq!(
        statuses(&result),
        [
            ("a_hangs", CaseStatus::Timeout),
            ("b_never_starts", CaseStatus::Timeout),
        ]
    );
    assert_eq!(
        detail_of(&result, "b_never_starts"),
        &CaseDetail::Stderr(String::new())
    );
    assert_eq!(progress.len(), 2);
}

#[test]
fn with_main_and_lib_the_hidden_tests_run_from_the_lib_test_executable() {
    // main.rs と lib.rs に同じ名前で違う値を返す関数を置き、どちらのテスト実行ファイルで動いたかを結果で見分ける。
    let built = build_with_hidden_tests(
        &files(&[
            (
                "src/main.rs",
                "fn answer() -> i32 {\n    1\n}\n\nfn main() {\n    println!(\"{}\", answer());\n}\n",
            ),
            (
                "src/lib.rs",
                "pub fn answer() -> i32 {\n    2\n}\n\n\
                 #[cfg(test)]\n\
                 mod tests {\n    #[test]\n    fn learner_fails() {\n        panic!(\"学習者のテスト\");\n    }\n}\n",
            ),
            (
                "tests/it.rs",
                "#[test]\nfn integration_fails() {\n    assert_eq!(progrust_exercise::answer(), 0);\n}\n",
            ),
        ]),
        "src/lib.rs",
        "use super::*;\n\n#[test]\nfn answer_is_two() {\n    assert_eq!(answer(), 2);\n}\n",
    );
    let executable = hidden_test_executable(&built.outcome, "src/lib.rs").unwrap();
    assert_eq!(
        built.outcome.test_executable(Path::new("src/lib.rs")),
        Some(executable.as_path())
    );
    assert_ne!(
        built.outcome.test_executable(Path::new("src/main.rs")),
        Some(executable.as_path())
    );

    let spec = built.spec(
        "src/lib.rs",
        &[("answer_is_two", "答えは 2")],
        limits(10, 2),
    );
    let result = judge_hidden_tests(&spec, |_| {}).unwrap();
    assert_eq!(statuses(&result), [("答えは 2", CaseStatus::Pass)]);
    assert_eq!(result.status, JudgeStatus::Passed);
    assert_eq!((result.passed, result.total), (1, 1));
}

#[test]
fn errors_in_the_hidden_tests_add_the_hint_and_learner_errors_do_not() {
    // 問題文と違う名前の関数を書いた（隠しテストの側でエラーが出る）。作業領域は、診断のパスをたどるので残しておく。
    let mut wrong_name = build_with_hidden_tests(
        &files(&[(
            "src/main.rs",
            "fn plus(a: i32, b: i32) -> i32 {\n    a + b\n}\n\nfn main() {}\n",
        )]),
        "src/main.rs",
        "use super::*;\n\n#[test]\nfn adds() {\n    assert_eq!(add(1, 2), 3);\n}\n",
    );
    let outcome = &mut wrong_name.outcome;
    assert!(matches!(outcome.status, BuildStatus::Failed { .. }));
    assert!(
        hidden_tests_have_errors(outcome),
        "{:?}",
        outcome.error_files
    );
    assert!(add_hidden_test_error_hint(outcome));
    assert_eq!(
        outcome.output.last(),
        Some(&BuildOutputPiece {
            kind: BuildOutputKind::System,
            text: HIDDEN_TEST_ERROR_HINT.into(),
        })
    );
    // 案内は、試行ログに入れるビルドの出力には入らない。
    let attempt = outcome.attempt_output();
    assert!(!attempt.compiler_messages.contains(HIDDEN_TEST_ERROR_HINT));
    assert!(
        attempt
            .compiler_messages
            .contains("progrust_judge_tests.rs")
    );

    // 問題文と違う型を返す関数を書いた。エラーは assert_eq! の展開の中で出る。
    let mut wrong_type = build_with_hidden_tests(
        &files(&[(
            "src/main.rs",
            "fn add(a: i32, b: i32) -> String {\n    format!(\"{}\", a + b)\n}\n\nfn main() {}\n",
        )]),
        "src/main.rs",
        "use super::*;\n\n#[test]\nfn adds() {\n    assert_eq!(add(1, 2), 3);\n}\n",
    );
    let outcome = &mut wrong_type.outcome;
    assert!(matches!(outcome.status, BuildStatus::Failed { .. }));
    assert!(
        hidden_tests_have_errors(outcome),
        "{:?}",
        outcome.error_files
    );
    assert!(add_hidden_test_error_hint(outcome));

    // 学習者のコードだけにエラーがある。
    let mut learner_error = build_with_hidden_tests(
        &files(&[(
            "src/main.rs",
            "fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n\nfn main() {\n    let x: i32 = \"a\";\n}\n",
        )]),
        "src/main.rs",
        "use super::*;\n\n#[test]\nfn adds() {\n    assert_eq!(add(1, 2), 3);\n}\n",
    );
    let outcome = &mut learner_error.outcome;
    assert!(matches!(outcome.status, BuildStatus::Failed { .. }));
    assert!(
        outcome.has_error_in(Path::new("src/main.rs")),
        "{:?}",
        outcome.error_files
    );
    assert!(!hidden_tests_have_errors(outcome));
    let output = outcome.output.clone();
    assert!(!add_hidden_test_error_hint(outcome));
    assert_eq!(outcome.output, output);
}

#[test]
fn a_missing_test_executable_or_no_hidden_tests_is_an_error() {
    let built = build_with_hidden_tests(
        &files(&[("src/main.rs", "fn main() {}\n")]),
        "src/main.rs",
        "// 隠しテストが無い\n",
    );
    assert!(hidden_test_executable(&built.outcome, "src/lib.rs").is_err());

    let spec = built.spec("src/main.rs", &[], limits(10, 2));
    let mut progress = Vec::new();
    assert!(judge_hidden_tests(&spec, |step| progress.push(step)).is_err());
    assert!(progress.is_empty());

    let missing = HiddenTestJudgeSpec {
        executable: built.workspace.project().join("無い"),
        ..spec
    };
    assert!(judge_hidden_tests(&missing, |_| {}).is_err());
}
