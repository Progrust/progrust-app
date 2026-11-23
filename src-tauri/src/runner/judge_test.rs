//! `test` 方式の判定の部品（spec/exercise-runner/submit.md「test 方式」「学習者のテストの判定」「判定結果」、
//! spec/exercise-runner/limits.md のテスト全体の実行時間と学習者のテスト全体の実行時間、ADR 0009、ADR 0101、
//! ADR 0113、ADR 0114、ADR 0205）。
//!
//! 判定は、次の順に、呼び出し側が部品をつないで行う。ビルドを止める操作と、ビルドが終わったときの知らせを
//! 呼び出し側が扱えるように、ビルドはこの部品の中で行わない。
//!
//! 1. [`super::project::assemble_project`] で組み立てた `project/` に、[`embed_hidden_tests`] で隠しテストを組み込む。
//!    変えるのは `project/` の中の写しだけで、エディタ上の学習者のファイル（呼び出し側が持つもの）は変えない。
//! 2. ビルドの部品（[`super::build`]）で、[`super::build::BuildCommand::TestNoRun`] でビルドする。失敗したときは、
//!    [`add_hidden_test_error_hint`] で、隠しテストの側でエラーが出たときの案内をビルドの出力に足す。
//! 3. [`hidden_test_executable`] で、ビルドの結果から隠しテストの入ったテスト実行ファイルを決める。
//! 4. [`judge_tests`] で、隠しテストの名前を得て、名前ごとに別のプロセスで実行する。`min_learner_tests` のある演習
//!    （[`LearnerTestsSpec`]）では、続けて、学習者が書いたテストを `cargo test` でまとめて実行し、テストケース 1 件
//!    「自分で書いたテスト」として判定する。判定結果を返す。テストケースが 1 件終わるたびに進み具合
//!    （[`JudgeProgress`]）を知らせる。
//!
//! 判定結果は、`stdout` 方式（[`super::judge`]）と同じ [`JudgeResult`] にする。隠しテストの `fail` の `detail` は
//! panic メッセージ（[`CaseDetail::PanicMessage`]）、学習者のテストの `fail` の `detail` は通ったテストの件数と
//! 必要な件数と失敗したテストの名前（[`CaseDetail::LearnerTests`]）、`timeout`・`runtime_error` の `detail` は
//! 標準エラー出力の先頭にする。
//!
//! Tauri の実行時に頼らない（判定だけを行うコマンドラインのプログラムからも使う。ADR 0208）。

use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs::{self, OpenOptions},
    io::{self, Write},
    num::{NonZeroU32, NonZeroUsize},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use super::build::{BuildOutcome, BuildOutputKind, BuildOutputPiece};
use super::file_path::validate_file_path;
use super::judge::{CaseDetail, CaseResult, CaseStatus, JudgeProgress, JudgeResult};
use super::process::{
    self, CollectedOutput, Limits, OutputLimit, ProcessSpec, Termination, TimeoutOutput,
};

/// 隠しテストを置くファイルの名前。`project/` の直下に置く。
pub const HIDDEN_TESTS_FILE: &str = "progrust_judge_tests.rs";
/// 隠しテストのモジュールの名前。
pub const HIDDEN_TESTS_MODULE: &str = "progrust_judge";
/// `judge.target` のファイルの末尾に足す内容（spec/exercise-runner/submit.md「test 方式」の 2）。
/// 学習者のファイルの最後の行に改行が無くても、その行に続かないように、改行から始める。
/// 末尾に足すので、学習者のコードの行番号は変わらない（ADR 0009）。
pub const HIDDEN_TESTS_DECLARATION: &str =
    "\n#[cfg(test)]\n#[path = \"../progrust_judge_tests.rs\"]\nmod progrust_judge;\n";
/// 隠しテストの側でエラーが出たときに、ビルドの出力に併せて示す案内（spec/exercise-runner/submit.md「test 方式」の 3）。
pub const HIDDEN_TEST_ERROR_HINT: &str =
    "問題文で指定された関数名・型・シグネチャを確認してください。";
/// テスト全体の実行時間の制限（spec/exercise-runner/limits.md）。
pub const TESTS_TIME_LIMIT: Duration = Duration::from_secs(10);
/// 学習者のテスト全体の実行時間の制限（spec/exercise-runner/limits.md）。
pub const LEARNER_TESTS_TIME_LIMIT: Duration = Duration::from_secs(10);

/// `project_dir`（組み立てた `project/`）に、隠しテスト `judge_tests`（`judge_tests.rs` の内容）を
/// `progrust_judge_tests.rs` として置き、`target`（`judge.target`。`src/main.rs` など）のファイルの末尾に
/// [`HIDDEN_TESTS_DECLARATION`] を足す。
///
/// `target` のファイルが無いとき（学習者が消したときなど）は、その宣言だけのファイルを作る。隠しテストが
/// 学習者のコードを見つけられないので、ビルドが隠しテストの側のエラーで失敗する。
pub fn embed_hidden_tests(project_dir: &Path, target: &str, judge_tests: &str) -> io::Result<()> {
    validate_file_path(target)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    fs::write(project_dir.join(HIDDEN_TESTS_FILE), judge_tests)?;
    let target = target
        .split('/')
        .fold(project_dir.to_path_buf(), |dir, segment| dir.join(segment));
    OpenOptions::new()
        .append(true)
        .create(true)
        .open(target)?
        .write_all(HIDDEN_TESTS_DECLARATION.as_bytes())
}

/// ビルドで、隠しテストのファイルを指すエラーが出たか。
pub fn hidden_tests_have_errors(build: &BuildOutcome) -> bool {
    build.has_error_in(Path::new(HIDDEN_TESTS_FILE))
}

/// 隠しテストの側でエラーが出たときに、ビルドの出力の最後に、アプリの知らせとして [`HIDDEN_TEST_ERROR_HINT`] を足す。
/// 足したかどうかを返す。
pub fn add_hidden_test_error_hint(build: &mut BuildOutcome) -> bool {
    if !hidden_tests_have_errors(build) {
        return false;
    }
    build.output.push(BuildOutputPiece {
        kind: BuildOutputKind::System,
        text: HIDDEN_TEST_ERROR_HINT.to_string(),
    });
    true
}

/// ビルドの結果から、`target`（`judge.target`）をクレートの根にするテスト実行ファイルを決める（ADR 0205）。
/// 見つからないときは、アプリの側の失敗として `Err` を返す。
pub fn hidden_test_executable(build: &BuildOutcome, target: &str) -> io::Result<PathBuf> {
    build
        .test_executable(Path::new(target))
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            io::Error::other(format!(
                "{target} の隠しテストの入ったテスト実行ファイルが、ビルドの出力に無い"
            ))
        })
}

/// `test` 方式の判定の制限。アプリでは [`TestLimits::default`] を使い、テストでは変えられる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TestLimits {
    /// テスト全体の実行時間の制限。隠しテストの名前を得るところから数える。
    pub time: Duration,
    /// 同時に起動するプロセスの数の上限。
    pub parallelism: NonZeroUsize,
    /// 学習者のテスト全体の実行時間の制限。学習者のテストを実行する cargo を起動したときから数える。
    pub learner_time: Duration,
}

impl Default for TestLimits {
    /// 制限時間は [`TESTS_TIME_LIMIT`]、同時に起動する数は CPU の論理コア数（spec/exercise-runner/submit.md
    /// 「test 方式」の 5）。コア数が分からないときは 1 にする。学習者のテストの制限時間は
    /// [`LEARNER_TESTS_TIME_LIMIT`]。
    fn default() -> Self {
        Self {
            time: TESTS_TIME_LIMIT,
            parallelism: thread::available_parallelism().unwrap_or(NonZeroUsize::MIN),
            learner_time: LEARNER_TESTS_TIME_LIMIT,
        }
    }
}

/// 学習者のテストの判定（spec/exercise-runner/submit.md「学習者のテストの判定」）で使うもの。
#[derive(Debug, Clone)]
pub struct LearnerTestsSpec {
    /// `cargo` の絶対パス。
    pub cargo: PathBuf,
    /// `lesson.json` の `exercise.judge.min_learner_tests`。通る必要のある、学習者のテストの件数。
    pub min_tests: NonZeroU32,
}

/// `test` 方式で判定するもの。
#[derive(Debug, Clone)]
pub struct TestJudgeSpec {
    /// 隠しテストの入ったテスト実行ファイル（[`hidden_test_executable`]）。
    pub executable: PathBuf,
    /// `project/`。テスト実行ファイルの作業ディレクトリにする。
    pub project_dir: PathBuf,
    /// アプリの環境変数に加えて設定する環境変数。
    pub envs: Vec<(OsString, OsString)>,
    /// `lesson.json` の `exercise.case_labels`。テスト関数名から表示名への対応。
    pub case_labels: BTreeMap<String, String>,
    /// 学習者のテストの判定。`min_learner_tests` の無い演習では `None` にし、学習者が書いたテストを判定に含めない。
    pub learner_tests: Option<LearnerTestsSpec>,
    pub limits: TestLimits,
}

/// 隠しテスト 1 件。
#[derive(Debug, Clone, PartialEq, Eq)]
struct HiddenTest {
    /// `--list` で得た名前（`progrust_judge::adds` など）。
    name: String,
    /// 表示名。`case_labels` に無いときは関数名。
    label: String,
}

/// 隠しテストの名前を得て、名前ごとに別のプロセスで実行する。`spec.learner_tests` があるときは、続けて学習者の
/// テストを判定する。判定結果を返す。テストケースが 1 件終わるたびに `on_progress` を呼ぶ。
/// テストケースは、隠しテストを `--list` の出力の順に並べ、学習者のテストのテストケースを最後に置く。
/// 進み具合の全テストケースの数には、学習者のテストのテストケースも数える。
///
/// 次のときは、アプリの側の失敗として `Err` を返す。
/// - テスト実行ファイルか cargo を起動できなかった。
/// - 隠しテストの名前を得る起動（`--list`）が、終了コード 0 で終わらなかった。
/// - 隠しテストが 1 件も無かった（教材の誤り。判定しないまま正解にしない）。
pub fn judge_tests<F>(spec: &TestJudgeSpec, mut on_progress: F) -> io::Result<JudgeResult>
where
    F: FnMut(JudgeProgress),
{
    let deadline = Instant::now() + spec.limits.time;
    let tests = list_hidden_tests(spec, deadline)?;
    let hidden_total = tests.len();
    let total = hidden_total + usize::from(spec.learner_tests.is_some());
    let workers = spec.limits.parallelism.get().min(hidden_total);
    let next = AtomicUsize::new(0);
    let abort = AtomicBool::new(false);
    let mut results: Vec<Option<CaseResult>> = vec![None; hidden_total];

    let stopped = thread::scope(|scope| -> io::Result<bool> {
        let (sender, receiver) = mpsc::channel();
        for _ in 0..workers {
            let sender = sender.clone();
            let (tests, next, abort) = (&tests, &next, &abort);
            scope.spawn(move || {
                while !abort.load(Ordering::SeqCst) {
                    let index = next.fetch_add(1, Ordering::SeqCst);
                    let Some(test) = tests.get(index) else { break };
                    if sender
                        .send((index, run_test(spec, test, deadline)))
                        .is_err()
                    {
                        break;
                    }
                }
            });
        }
        drop(sender);

        for (done, (index, outcome)) in receiver.into_iter().enumerate() {
            let result = match outcome {
                Ok(Some(result)) => result,
                Ok(None) => {
                    abort.store(true, Ordering::SeqCst);
                    return Ok(true);
                }
                Err(error) => {
                    abort.store(true, Ordering::SeqCst);
                    return Err(error);
                }
            };
            results[index] = Some(result);
            on_progress(JudgeProgress {
                completed: done + 1,
                total,
            });
        }
        Ok(false)
    })?;
    if stopped {
        return Ok(JudgeResult::stopped());
    }
    let mut cases = results
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| io::Error::other("結果の決まらなかった隠しテストがある"))?;
    if let Some(learner) = &spec.learner_tests {
        let Some(case) = judge_learner_tests(spec, learner)? else {
            return Ok(JudgeResult::stopped());
        };
        cases.push(case);
        on_progress(JudgeProgress {
            completed: total,
            total,
        });
    }
    Ok(JudgeResult::from_cases(cases))
}

/// テスト実行ファイルを `--list progrust_judge::` で起動し、隠しテストを得る。
fn list_hidden_tests(spec: &TestJudgeSpec, deadline: Instant) -> io::Result<Vec<HiddenTest>> {
    let collected = process::collect_output(test_process(
        spec,
        &["--list", "progrust_judge::"],
        deadline.saturating_duration_since(Instant::now()),
    ))?;
    if collected.termination != Termination::Exited(0) {
        return Err(io::Error::other(format!(
            "隠しテストの名前を得られなかった（{:?}）: {}",
            collected.termination, collected.stderr
        )));
    }
    let tests: Vec<HiddenTest> = parse_test_list(&collected.stdout)
        .into_iter()
        .map(|name| {
            let function = function_name(&name).unwrap_or(&name);
            let label = spec
                .case_labels
                .get(function)
                .cloned()
                .unwrap_or_else(|| function.to_string());
            HiddenTest { name, label }
        })
        .collect();
    if tests.is_empty() {
        return Err(io::Error::other("隠しテストが 1 件も無い"));
    }
    Ok(tests)
}

/// `--list` の出力の `{名前}: test` の行から、隠しテストの名前を、出てきた順に得る。名前に `progrust_judge` の
/// モジュールが無いもの（学習者のテストの名前に、文字列として `progrust_judge::` が入っているだけのもの）は除く。
fn parse_test_list(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter_map(|line| line.strip_suffix(": test"))
        .filter(|name| function_name(name).is_some())
        .map(str::to_string)
        .collect()
}

/// 隠しテストの名前の、`progrust_judge::` より後の部分（テストケースの関数名）。隠しテストの名前でないときは `None`。
fn function_name(name: &str) -> Option<&str> {
    let prefix = format!("{HIDDEN_TESTS_MODULE}::");
    if let Some(function) = name.strip_prefix(&prefix) {
        return Some(function);
    }
    name.split_once(&format!("::{prefix}"))
        .map(|(_, function)| function)
}

/// 隠しテスト 1 件を、別のプロセスで実行して結果を決める。制限時間に達していたら、起動せずに `timeout` にする。
/// 止める操作で止めたときは `None`。
fn run_test(
    spec: &TestJudgeSpec,
    test: &HiddenTest,
    deadline: Instant,
) -> io::Result<Option<CaseResult>> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Ok(Some(CaseResult {
            name: test.label.clone(),
            status: CaseStatus::Timeout,
            detail: Some(CaseDetail::Stderr(String::new())),
        }));
    }
    let collected = process::collect_output(test_process(
        spec,
        &["--exact", &test.name, "--test-threads=1"],
        remaining,
    ))?;
    Ok(case_result(test, &collected))
}

/// テスト実行ファイルを起動するプロセス。`--nocapture` は付けない（ADR 0101）。
fn test_process(spec: &TestJudgeSpec, args: &[&str], time: Duration) -> ProcessSpec {
    let mut process = ProcessSpec::new(
        &spec.executable,
        &spec.project_dir,
        Limits {
            time,
            // 制限値（spec/exercise-runner/limits.md）にある test 方式の制限は時間だけなので、標準出力の量は数えない。
            // 標準エラー出力は、`detail` に使う先頭だけを受け取る。
            output: OutputLimit::Stdout(usize::MAX),
        },
    );
    process.args = args.iter().map(OsString::from).collect();
    process.envs = spec.envs.clone();
    // 時間切れの `detail` に、止めるまでに受け取った標準エラー出力を使う。
    process.on_timeout = TimeoutOutput::Deliver;
    process
}

/// テストケースの結果を、spec/exercise-runner/submit.md「test 方式」の 6 の表のとおりに決める（ADR 0101）。
/// 止める操作で止めたときは `None`。
fn case_result(test: &HiddenTest, collected: &CollectedOutput) -> Option<CaseResult> {
    let stderr_detail = || Some(CaseDetail::Stderr(collected.stderr.clone()));
    let (status, detail) = match collected.termination {
        Termination::Stopped => return None,
        Termination::TimedOut => (CaseStatus::Timeout, stderr_detail()),
        termination => {
            if termination == Termination::Exited(0)
                && has_result_line(&collected.stdout, &test.name, "ok")
            {
                (CaseStatus::Pass, None)
            } else if has_result_line(&collected.stdout, &test.name, "FAILED") {
                (
                    CaseStatus::Fail,
                    Some(CaseDetail::PanicMessage(panic_message(
                        &collected.stdout,
                        &test.name,
                    ))),
                )
            } else {
                (CaseStatus::RuntimeError, stderr_detail())
            }
        }
    };
    Some(CaseResult {
        name: test.label.clone(),
        status,
        detail,
    })
}

/// `stdout` に、`name` の結果の行があるか（spec/exercise-runner/submit.md「test 方式」の 6、ADR 0218）。結果の行は、
/// 行末が ` ... {result}` で、その直前に `test {name}` か、`test {name} - {テストの種類}`（`#[should_panic]` のテストの
/// `test {name} - should panic` など）がある行とする。行頭に学習者の出力が付いていてもよい。
fn has_result_line(stdout: &str, name: &str, result: &str) -> bool {
    let test = format!("test {name}");
    let suffix = format!(" ... {result}");
    stdout
        .lines()
        .filter_map(|line| line.strip_suffix(&suffix))
        .any(|head| {
            head.ends_with(&test)
                || head
                    .rsplit_once(" - ")
                    .is_some_and(|(before, mode)| before.ends_with(&test) && is_test_mode(mode))
        })
}

/// libtest が名前の後に付けるテストの種類の表示（`should panic` など）か。小文字の英語と空白・`_` だけでできている。
fn is_test_mode(mode: &str) -> bool {
    !mode.is_empty()
        && mode
            .chars()
            .all(|c| c.is_ascii_lowercase() || c == ' ' || c == '_')
}

/// `#[should_panic]` のテストが panic しなかったときに、libtest が出す説明の始まり。
const DID_NOT_PANIC: &str = "note: test did not panic as expected";

/// `fail` の panic メッセージを取り出す（spec/exercise-runner/submit.md「test 方式」の 7、ADR 0218）。
///
/// - `failures:` の行より後で最後の `thread '{name}' ... panicked at` の行の、次の行から、次の節の見出しの前までとし、
///   `note: run with` で始まる行を除く。末尾の空行も除く。`#[should_panic(expected = ...)]` のテストで、panic の内容が
///   `expected` と違ったときの libtest の説明も、これに含まれる。
/// - `panicked at` の行が無いときは、`failures:` の行より後の、[`DID_NOT_PANIC`] から行末までとする
///   （`#[should_panic]` のテストが panic しなかったとき）。
/// - どちらも見つからないときは、空の文字列にする。
fn panic_message(stdout: &str, name: &str) -> String {
    let lines: Vec<&str> = stdout.lines().collect();
    let Some(failures) = lines.iter().position(|line| *line == "failures:") else {
        return String::new();
    };
    let after = &lines[failures + 1..];
    let thread = format!("thread '{name}'");
    let Some(panicked) = after
        .iter()
        .rposition(|line| line.contains(&thread) && line.contains("panicked at"))
    else {
        return after
            .iter()
            .find_map(|line| line.find(DID_NOT_PANIC).map(|start| &line[start..]))
            .unwrap_or_default()
            .to_string();
    };
    let mut message: Vec<&str> = after[panicked + 1..]
        .iter()
        .copied()
        .take_while(|line| !is_section_heading(line))
        .filter(|line| !line.starts_with("note: run with"))
        .collect();
    while message.last().is_some_and(|line| line.trim().is_empty()) {
        message.pop();
    }
    message.join("\n")
}

/// libtest の出力の節の見出し（`failures:`、`successes:`、`---- {名前} stdout ----`、`test result: …`）か。
fn is_section_heading(line: &str) -> bool {
    matches!(line, "failures:" | "successes:")
        || (line.starts_with("---- ") && line.ends_with(" ----"))
        || line.starts_with("test result: ")
}

// ---- 学習者のテストの判定 ----

/// 学習者のテストのテストケースの表示名（spec/exercise-runner/submit.md「学習者のテストの判定」）。
pub fn learner_tests_label(min_tests: NonZeroU32) -> String {
    format!("自分で書いたテスト（{min_tests} 件以上）")
}

/// 学習者のテストを実行する cargo の引数。隠しテストは `--skip` で除く（spec/exercise-runner/submit.md
/// 「学習者のテストの判定」の 1、ADR 0113）。
fn learner_tests_args() -> Vec<OsString> {
    [
        "test".to_string(),
        "--offline".to_string(),
        "--no-fail-fast".to_string(),
        "--".to_string(),
        "--skip".to_string(),
        format!("{HIDDEN_TESTS_MODULE}::"),
    ]
    .into_iter()
    .map(OsString::from)
    .collect()
}

/// 学習者のテストを、隠しテストの判定でビルドした `project/` で `cargo test` を新しいプロセスグループで起動して
/// まとめて実行し、テストケース 1 件の結果を決める。止める操作で止めたときは `None`。
fn judge_learner_tests(
    spec: &TestJudgeSpec,
    learner: &LearnerTestsSpec,
) -> io::Result<Option<CaseResult>> {
    let mut process = ProcessSpec::new(
        &learner.cargo,
        &spec.project_dir,
        Limits {
            time: spec.limits.learner_time,
            // 制限値（spec/exercise-runner/limits.md）にある学習者のテストの制限は時間だけなので、標準出力の量は
            // 数えない。標準エラー出力は、`detail` に使う先頭だけを受け取る。
            output: OutputLimit::Stdout(usize::MAX),
        },
    );
    process.args = learner_tests_args();
    process.envs = spec.envs.clone();
    // `detail` に使う cargo の標準エラー出力に、アプリの環境変数によって色（ANSI エスケープ）が付かないようにする。
    process
        .envs
        .push((OsString::from("CARGO_TERM_COLOR"), OsString::from("never")));
    // 時間切れの `detail` に、止めるまでに受け取った標準エラー出力を使う。
    process.on_timeout = TimeoutOutput::Deliver;
    let collected = process::collect_output(process)?;
    Ok(learner_case_result(learner.min_tests, &collected))
}

/// 学習者のテストの結果の行を読んだもの。
#[derive(Debug, Default, PartialEq, Eq)]
struct LearnerTestLines {
    /// ` ... ok` の行の数。
    passed: usize,
    /// ` ... FAILED` の行のテストの名前。出てきた順に並ぶ。
    failed: Vec<String>,
}

/// 標準出力から、行頭が `test ` で、行末が ` ... ok` か ` ... FAILED` の行を、学習者のテストの結果の行として読む
/// （spec/exercise-runner/submit.md「学習者のテストの判定」の 2）。` ... ignored` の行は数えない。
fn read_learner_test_lines(stdout: &str) -> LearnerTestLines {
    let mut lines = LearnerTestLines::default();
    for line in stdout.lines() {
        let Some(rest) = line.strip_prefix("test ") else {
            continue;
        };
        if rest.ends_with(" ... ok") {
            lines.passed += 1;
        } else if let Some(name) = rest.strip_suffix(" ... FAILED") {
            lines.failed.push(name.to_string());
        }
    }
    lines
}

/// 学習者のテストのテストケースの結果を、spec/exercise-runner/submit.md「学習者のテストの判定」の 3 の表のとおりに
/// 決める。止める操作で止めたときは `None`。
fn learner_case_result(min_tests: NonZeroU32, collected: &CollectedOutput) -> Option<CaseResult> {
    let stderr_detail = || Some(CaseDetail::Stderr(collected.stderr.clone()));
    let (status, detail) = match collected.termination {
        Termination::Stopped => return None,
        Termination::TimedOut => (CaseStatus::Timeout, stderr_detail()),
        termination => {
            let lines = read_learner_test_lines(&collected.stdout);
            let enough = u32::try_from(lines.passed).unwrap_or(u32::MAX) >= min_tests.get();
            // 学習者のテストが `process::exit(0)` を呼ぶと、同じ実行ファイルのほかのテストが FAILED でも cargo が
            // 終了コード 0 で終わるので、FAILED の行があれば pass にしない（ADR 0219）。
            if termination == Termination::Exited(0) && lines.failed.is_empty() && enough {
                (CaseStatus::Pass, None)
            } else if !lines.failed.is_empty() || !enough {
                (
                    CaseStatus::Fail,
                    Some(CaseDetail::LearnerTests(learner_fail_detail(
                        min_tests, &lines,
                    ))),
                )
            } else {
                (CaseStatus::RuntimeError, stderr_detail())
            }
        }
    };
    Some(CaseResult {
        name: learner_tests_label(min_tests),
        status,
        detail,
    })
}

/// 学習者のテストの `fail` の `detail`。通ったテストの件数と必要な件数、失敗したテストの名前を入れる。
fn learner_fail_detail(min_tests: NonZeroU32, lines: &LearnerTestLines) -> String {
    let mut detail = format!(
        "通ったテスト: {} 件（{min_tests} 件以上必要）",
        lines.passed
    );
    if !lines.failed.is_empty() {
        detail.push_str("\n失敗したテスト:");
        for name in &lines.failed {
            detail.push_str("\n  ");
            detail.push_str(name);
        }
    }
    detail
}

#[cfg(test)]
mod tests;
