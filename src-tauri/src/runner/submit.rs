//! 提出のタスク（spec/exercise-runner/submit.md、spec/exercise-runner/attempt-log.md、spec/lesson-flow/completion.md）と、
//! イベント `submit_progress` と `run_finished` の `submit` の内容（spec/architecture/commands/run.md「イベント」）。
//!
//! - [`execute`] は、`submit_code` が受け付けた後に、別のスレッドで呼ぶ。`project/` を組み立て（`test` 方式では
//!   隠しテストを組み込み）、ビルドし、`judge.mode` に従って `stdout` 方式（[`super::judge`]）か `test` 方式
//!   （[`super::judge_test`]）で判定する。終わったら、試行ログに `submit` イベントを追記し、正解ならレッスンを完了にして
//!   から、要求を外して `run_finished` を送る。
//! - ビルドの出力は、ビルドが終わってから `run_output`（`stderr`。`test` 方式の隠しテストの側のエラーの案内は
//!   `system`）で送る。判定で動かしたプログラムの出力は送らない。
//! - 判定の進み具合は、段階が変わったとき（`build` → `judge`）と、テストケースが 1 件終わるたびに `submit_progress` で送る。
//! - 止める操作（`stop_run`）で止めた提出は `stopped` にし、試行ログに追記せず、レッスンを完了にしない。アプリの終了で
//!   止めたときは、`run_finished` も送らない（spec/exercise-runner/submit.md「途中で止めた提出」）。
//! - ビルドの失敗が、初回準備のやり直しが要る失敗（[`super::lost_crates`]）に当てはまったときは、ビルドの出力を送らずに
//!   捨て、`run_output` の `system` で「外部クレートを準備し直しています」を送り、初回準備を始め
//!   （[`RunEvents::crates_lost`]）、判定結果を `crates_unavailable` にする。試行ログには追記せず、レッスンを完了にしない
//!   （spec/exercise-runner/submit.md「判定をしなかった提出」）。
//! - 組み立てから判定までの手順は [`judge_submission`] にまとめ、判定だけを行うコマンドラインのプログラム
//!   （[`super::lesson_judge`]。ADR 0208）も同じ手順・制限値で判定する。止める操作とイベントの送信は、
//!   [`JudgeHooks`] でつなぐ。

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;

use super::RunnerDir;
use super::build::{BuildCommand, BuildHandle, BuildOutcome, BuildSpec, BuildStatus, start_build};
use super::judge::{
    CaseDetail, CaseLimits, CaseResult, CaseStatus, JudgeProgress, JudgeResult, JudgeStatus,
    StdoutJudgeSpec, judge_stdout,
};
use super::judge_test::{
    LearnerTestsSpec, TestJudgeSpec, TestLimits, add_hidden_test_error_hint, embed_hidden_tests,
    hidden_test_executable, judge_tests,
};
use super::process::Stopper;
use super::project::{assemble_project, build_env, uses_allowed_crates};
use super::run::{
    FinishedKind, RunEvents, RunFinished, app_failure, build_output_chunks, handle_lost_crates,
    send_output,
};
use super::runs::{ActiveRun, Runs};
use super::workspace::WorkspaceDir;
use crate::attempts::{BuildOutput, SessionKind, SubmitRecord, SubmitStatus};
use crate::commands::error::{CommandError, ErrorCode};
use crate::content::ExerciseJudge;
use crate::progress::now_string;

/// 判定の進み具合を送るイベントの名前。
pub const SUBMIT_PROGRESS: &str = "submit_progress";

/// `submit_progress` の `phase`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmitPhase {
    /// ビルド中。
    Build,
    /// テストケースの実行中。
    Judge,
}

/// `submit_progress` の内容。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SubmitProgress {
    pub request_id: String,
    pub phase: SubmitPhase,
    /// 結果が決まったテストケースの数。`build` では 0。
    pub completed: usize,
    /// 全テストケースの数。決まる前（`build` と、`test` 方式で隠しテストの名前を得る前）は `None`（JSON では `null`）。
    pub total: Option<usize>,
}

/// `run_finished` の `submit.status`（spec/exercise-runner/submit.md「判定結果」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmitResultStatus {
    /// 全テストケースが通過した。
    Passed,
    /// 1 件以上のテストケースが失敗した。
    Wrong,
    /// ビルドに失敗した（ビルド時間の制限を超えたときを含む）。
    CompileError,
    /// 判定をしていない。ビルドの失敗が、初回準備のやり直しが要る失敗に当てはまった。
    CratesUnavailable,
    /// 途中で止めた。
    Stopped,
}

/// `run_finished` の `submit`（spec/exercise-runner/submit.md「判定結果」）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SubmitJudgement {
    pub status: SubmitResultStatus,
    /// 通過したテストケースの数。
    pub passed: u32,
    /// 全テストケースの数。
    pub total: u32,
    /// テストケースごとの結果。
    pub cases: Vec<CaseResult>,
}

impl SubmitJudgement {
    /// テストケースを 1 件も実行しなかった結果（`compile_error`・`crates_unavailable`・`stopped`）。`passed`・`total` は 0、`cases` は空にする。
    pub(crate) fn without_cases(status: SubmitResultStatus) -> Self {
        Self {
            status,
            passed: 0,
            total: 0,
            cases: Vec::new(),
        }
    }

    /// 判定の部品の結果から作る。
    pub(crate) fn judged(result: JudgeResult) -> Self {
        Self {
            status: match result.status {
                JudgeStatus::Passed => SubmitResultStatus::Passed,
                JudgeStatus::Wrong => SubmitResultStatus::Wrong,
                JudgeStatus::Stopped => SubmitResultStatus::Stopped,
            },
            passed: result.passed,
            total: result.total,
            cases: result.cases,
        }
    }

    /// 試行ログに記録する `status`。`crates_unavailable` の提出と、途中で止めた提出は記録しないので `None`。
    fn record_status(&self) -> Option<SubmitStatus> {
        match self.status {
            SubmitResultStatus::Passed => Some(SubmitStatus::Passed),
            SubmitResultStatus::Wrong => Some(SubmitStatus::Wrong),
            SubmitResultStatus::CompileError => Some(SubmitStatus::CompileError),
            SubmitResultStatus::CratesUnavailable | SubmitResultStatus::Stopped => None,
        }
    }

    /// 失敗したテストケース（`pass` でないもの）の表示名。テストケースの順に並べる。
    fn failed_cases(&self) -> Vec<String> {
        self.cases
            .iter()
            .filter(|case| case.status != CaseStatus::Pass)
            .map(|case| case.name.clone())
            .collect()
    }

    /// 試行ログの `stderr` に入れる、判定で動かしたプログラムの標準エラー出力。`timeout`・`runtime_error` の
    /// テストケースの `detail`（標準エラー出力の先頭）を、テストケースの順に、改行で区切って連結する（ADR 0221）。
    fn program_stderr(&self) -> String {
        let mut stderr = String::new();
        for case in &self.cases {
            if let Some(CaseDetail::Stderr(text)) = &case.detail
                && !text.is_empty()
            {
                stderr.push_str(text);
                if !text.ends_with('\n') {
                    stderr.push('\n');
                }
            }
        }
        stderr
    }
}

/// 提出の要求と、使うもの。
pub struct SubmitJob {
    pub active: Arc<ActiveRun>,
    pub runs: Runs,
    /// 試行ログに追記するときに、未完了のセッションが無ければ始めるセッションの種類。
    pub session_kind: SessionKind,
    /// エディタ上の全ファイル。試行ログには、隠しテストを組み込む前のこのファイルを記録する。
    pub files: BTreeMap<String, String>,
    /// 演習の `dependencies`。
    pub dependencies: Vec<String>,
    /// 演習の判定方式と、判定に使うもの。
    pub judge: ExerciseJudge,
    /// `cargo` の絶対パス。
    pub cargo: PathBuf,
    pub workspace: WorkspaceDir,
    pub runner: RunnerDir,
}

/// 判定結果が決まった提出。
struct Outcome {
    judgement: SubmitJudgement,
    error_codes: Vec<String>,
    build: BuildOutput,
}

impl Outcome {
    /// 判定をしなかった結果（途中で止めた、`crates_unavailable`）。記録しないので、ビルドの出力は持たない。
    fn not_judged(status: SubmitResultStatus) -> Self {
        Self {
            judgement: SubmitJudgement::without_cases(status),
            error_codes: Vec::new(),
            build: BuildOutput::default(),
        }
    }
}

/// 提出を最後まで行う。終わったら、試行ログに追記し、正解ならレッスンを完了にし、要求を外してから `run_finished` を送る。
pub fn execute(job: SubmitJob, events: Arc<dyn RunEvents>) {
    let result = submit_steps(&job, events.as_ref());
    let finished_at = now_string();
    let request_id = job.active.request_id().to_string();
    let lesson_id = job.active.lesson_id().to_string();

    let finished = match result {
        Ok(outcome) => {
            let mut progress = None;
            if let Some(status) = outcome.judgement.record_status()
                && !job.active.stopped_for_app_exit()
            {
                let record = SubmitRecord {
                    at: finished_at.clone(),
                    files: job.files.clone(),
                    status,
                    error_codes: outcome.error_codes,
                    passed: outcome.judgement.passed,
                    total: outcome.judgement.total,
                    failed_cases: outcome.judgement.failed_cases(),
                    build: outcome.build,
                    program_stderr: outcome.judgement.program_stderr(),
                };
                // 記録できなくても、判定結果は決まっているので、run_finished は判定結果を送る。
                match events.append_submit(&lesson_id, job.session_kind, record) {
                    Ok(session_id) if status == SubmitStatus::Passed => {
                        match events.record_pass(&lesson_id, &session_id, &finished_at) {
                            Ok(updated) => progress = updated,
                            Err(error) => eprintln!(
                                "提出 {request_id} の正解を記録できなかった（レッスンを完了にできなかった）: {error}"
                            ),
                        }
                    }
                    Ok(_) => {}
                    Err(error) => {
                        eprintln!("提出 {request_id} の結果を試行ログに記録できなかった: {error}");
                    }
                }
            }
            RunFinished {
                request_id,
                kind: FinishedKind::Submit,
                finished_at,
                error: None,
                run: None,
                submit: Some(outcome.judgement),
                progress,
            }
        }
        Err(error) => RunFinished {
            request_id,
            kind: FinishedKind::Submit,
            finished_at,
            error: Some(error),
            run: None,
            submit: None,
            progress: None,
        },
    };

    // 要求を外してから送る。受け取った画面が、すぐに次の実行・提出を始められるように。
    job.runs.release(&job.active);
    if !job.active.stopped_for_app_exit() {
        events.finished(finished);
    }
}

/// `project/` の組み立てから、判定の終わりまで。アプリの側の失敗は `Err` で返す。
fn submit_steps(job: &SubmitJob, events: &dyn RunEvents) -> Result<Outcome, CommandError> {
    let request = JudgeRequest {
        cargo: &job.cargo,
        workspace: &job.workspace,
        runner: &job.runner,
        dependencies: &job.dependencies,
        files: &job.files,
        judge: &job.judge,
        build_time: job.runs.limits().build_time,
    };
    let hooks = AppHooks { job, events };
    Ok(match judge_submission(&request, &hooks)? {
        Judged::Stopped => Outcome::not_judged(SubmitResultStatus::Stopped),
        Judged::CratesUnavailable(built) => {
            handle_lost_crates(events, job.active.request_id(), &built);
            Outcome::not_judged(SubmitResultStatus::CratesUnavailable)
        }
        Judged::CompileError(built) => compile_error(&built),
        Judged::Done { result, built } => Outcome {
            judgement: SubmitJudgement::judged(result),
            error_codes: built.error_codes.clone(),
            build: built.attempt_output(),
        },
    })
}

/// 提出の要求（[`ActiveRun`]）とイベントの送り先を、判定の手順（[`judge_submission`]）につなぐ。
struct AppHooks<'a> {
    job: &'a SubmitJob,
    events: &'a dyn RunEvents,
}

impl JudgeHooks for AppHooks<'_> {
    fn stop_requested(&self) -> bool {
        self.job.active.stop_cause().is_some()
    }

    fn building(&self, build: BuildHandle) {
        self.job.active.set_building(build);
    }

    fn built(&self, built: &BuildOutcome) {
        send_output(
            self.events,
            self.job.active.request_id(),
            build_output_chunks(built),
        );
    }

    fn judging(&self, stopper: Stopper) {
        self.job.active.set_judging(stopper);
    }

    fn judged(&self) {
        self.job.active.set_finished();
    }

    fn progress(&self, phase: SubmitPhase, completed: usize, total: Option<usize>) {
        send_progress(self.job, self.events, phase, completed, total);
    }
}

/// 判定するもの（[`judge_submission`]）。
pub struct JudgeRequest<'a> {
    /// `cargo` の絶対パス。
    pub cargo: &'a Path,
    pub workspace: &'a WorkspaceDir,
    pub runner: &'a RunnerDir,
    /// 演習の `dependencies`。
    pub dependencies: &'a [String],
    /// 判定するファイル。キーは cargo パッケージのルートからの相対パス（`/` 区切り）。
    pub files: &'a BTreeMap<String, String>,
    /// 演習の判定方式と、判定に使うもの。
    pub judge: &'a ExerciseJudge,
    /// ビルド時間の制限。
    pub build_time: Duration,
}

/// 判定の手順の途中で呼ぶもの。アプリの提出は、止める操作の受け付けと、イベントの送信に使う。
/// 判定だけを行うコマンドラインのプログラムは、何もしない既定の実装を使う。
pub trait JudgeHooks {
    /// 止める指示があったか。
    fn stop_requested(&self) -> bool {
        false
    }
    /// ビルドを始めた。
    fn building(&self, _build: BuildHandle) {}
    /// ビルドが終わった（成功と失敗。止めたときと、初回準備のやり直しが要る失敗のときは呼ばない）。`test` 方式の隠しテストの側のエラーの案内は、付けた後。
    fn built(&self, _built: &BuildOutcome) {}
    /// テストケースの実行を始めた。
    fn judging(&self, _stopper: Stopper) {}
    /// テストケースの実行が終わった。
    fn judged(&self) {}
    /// 判定の進み具合が変わった。
    fn progress(&self, _phase: SubmitPhase, _completed: usize, _total: Option<usize>) {}
}

/// 何もしない [`JudgeHooks`]。
pub struct NoHooks;

impl JudgeHooks for NoHooks {}

/// 判定の手順の結果。
#[derive(Debug)]
pub enum Judged {
    /// 途中で止めた。
    Stopped,
    /// ビルドに失敗した（ビルド時間の制限を超えたときを含む）。テストケースは 1 件も実行していない。
    CompileError(BuildOutcome),
    /// ビルドの失敗が、初回準備のやり直しが要る失敗に当てはまった。テストケースは 1 件も実行していない。
    /// ビルドの出力は、まだ送っていない（[`JudgeHooks::built`] を呼んでいない）。
    CratesUnavailable(BuildOutcome),
    /// テストケースを実行した。
    Done {
        result: JudgeResult,
        built: BuildOutcome,
    },
}

/// 提出の判定の手順（spec/exercise-runner/submit.md）。`project/` を組み立て（`test` 方式では隠しテストを組み込み）、
/// ビルドし、`judge.mode` に従って `stdout` 方式か `test` 方式で判定する。アプリの提出と、判定だけを行うコマンドラインの
/// プログラム（ADR 0208）が、同じ手順・制限値で判定するために使う。アプリの側の失敗は `Err` で返す。
pub fn judge_submission(
    request: &JudgeRequest<'_>,
    hooks: &dyn JudgeHooks,
) -> Result<Judged, CommandError> {
    if hooks.stop_requested() {
        return Ok(Judged::Stopped);
    }
    hooks.progress(SubmitPhase::Build, 0, None);
    assemble_project(
        request.workspace,
        request.runner,
        request.dependencies,
        request.files,
    )?;
    let project_dir = request.workspace.project();
    let command = match request.judge {
        ExerciseJudge::Stdout { .. } => BuildCommand::Build,
        ExerciseJudge::Test {
            target,
            judge_tests,
            ..
        } => {
            // 変えるのは project/ の中の写しだけで、エディタ上の学習者のファイル（request.files）は変えない。
            embed_hidden_tests(&project_dir, target, judge_tests)
                .map_err(|error| app_failure("隠しテストを組み込めませんでした。", error))?;
            BuildCommand::TestNoRun
        }
    };

    let envs = build_env(request.workspace, request.dependencies);
    let mut build_spec = BuildSpec::new(request.cargo, &project_dir, command, envs.clone());
    build_spec.time_limit = request.build_time;
    let running = start_build(build_spec)
        .map_err(|error| app_failure("cargo を起動できませんでした。", error))?;
    hooks.building(running.handle());
    let mut built = running
        .wait()
        .map_err(|error| app_failure("ビルドの結果を受け取れませんでした。", error))?;
    // ビルドが終わる直前に止める指示が来たときも、ビルドの出力を送らず、記録もしない。
    if hooks.stop_requested() {
        return Ok(Judged::Stopped);
    }

    match built.status {
        // 止める操作で止めたときは、ビルドの出力を捨てている。
        BuildStatus::Stopped => return Ok(Judged::Stopped),
        // 初回準備のやり直しが要る失敗は、ビルドの出力を送らず、隠しテストの側のエラーの案内も付けない。
        BuildStatus::Failed { .. }
            if built.needs_crate_preparation(
                &request
                    .workspace
                    .project_target(uses_allowed_crates(request.dependencies)),
            ) =>
        {
            return Ok(Judged::CratesUnavailable(built));
        }
        BuildStatus::Failed { .. } | BuildStatus::TimedOut => {
            if matches!(request.judge, ExerciseJudge::Test { .. }) {
                add_hidden_test_error_hint(&mut built);
            }
            hooks.built(&built);
            return Ok(Judged::CompileError(built));
        }
        BuildStatus::Succeeded => hooks.built(&built),
    }

    // ビルドが終わってから判定を始めるまでに止める指示が来ていたら、判定しない。
    if hooks.stop_requested() {
        return Ok(Judged::Stopped);
    }
    let envs: Vec<(OsString, OsString)> = envs
        .into_iter()
        .map(|(key, value)| (OsString::from(key), value))
        .collect();
    let stopper = Stopper::new();
    hooks.judging(stopper.clone());
    let on_progress = |step: JudgeProgress| {
        hooks.progress(SubmitPhase::Judge, step.completed, Some(step.total));
    };
    let result = match request.judge {
        ExerciseJudge::Stdout { cases } => {
            let executable = built.main_executable().ok_or_else(|| CommandError {
                code: ErrorCode::Internal,
                message: "判定するプログラムが見つかりませんでした。".into(),
                detail: Some("ビルドの出力に src/main.rs の実行ファイルが無い".into()),
            })?;
            let spec = StdoutJudgeSpec {
                executable: executable.to_path_buf(),
                project_dir,
                envs,
                cases: cases.clone(),
                limits: CaseLimits::default(),
            };
            hooks.progress(SubmitPhase::Judge, 0, Some(cases.len()));
            judge_stdout(&spec, &stopper, on_progress)
        }
        ExerciseJudge::Test {
            target,
            case_labels,
            min_learner_tests,
            ..
        } => {
            let executable = hidden_test_executable(&built, target).map_err(|error| {
                app_failure(
                    "隠しテストの入ったテスト実行ファイルが見つかりませんでした。",
                    error,
                )
            })?;
            let spec = TestJudgeSpec {
                executable,
                project_dir,
                envs,
                case_labels: case_labels.clone(),
                learner_tests: min_learner_tests.map(|min_tests| LearnerTestsSpec {
                    cargo: request.cargo.to_path_buf(),
                    min_tests,
                }),
                limits: TestLimits::default(),
            };
            // 隠しテストの名前を得る前なので、全テストケースの数はまだ決まらない。
            hooks.progress(SubmitPhase::Judge, 0, None);
            judge_tests(&spec, &stopper, on_progress)
        }
    };
    hooks.judged();

    // 判定の途中か、最後のテストケースが終わった直後に止める指示が来たときは、判定の結果にかかわらず止めた提出にする。
    // 止めた提出を正解として記録すると、正解の画面を見ないままレッスンが完了になる（ADR 0202）。
    if hooks.stop_requested() {
        return Ok(Judged::Stopped);
    }
    let result = result.map_err(|error| app_failure("判定できませんでした。", error))?;
    if result.status == JudgeStatus::Stopped {
        return Ok(Judged::Stopped);
    }
    Ok(Judged::Done { result, built })
}

/// ビルドに失敗した結果。テストケースは 1 件も実行していない（ADR 0220）。
fn compile_error(built: &BuildOutcome) -> Outcome {
    Outcome {
        judgement: SubmitJudgement::without_cases(SubmitResultStatus::CompileError),
        error_codes: built.error_codes.clone(),
        build: built.attempt_output(),
    }
}

/// `submit_progress` を送る。
fn send_progress(
    job: &SubmitJob,
    events: &dyn RunEvents,
    phase: SubmitPhase,
    completed: usize,
    total: Option<usize>,
) {
    events.submit_progress(SubmitProgress {
        request_id: job.active.request_id().to_string(),
        phase,
        completed,
        total,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case(name: &str, status: CaseStatus, detail: Option<CaseDetail>) -> CaseResult {
        CaseResult {
            name: name.into(),
            status,
            detail,
        }
    }

    #[test]
    fn submit_progress_serializes_in_the_spec_shape() {
        let progress = SubmitProgress {
            request_id: "s".into(),
            phase: SubmitPhase::Build,
            completed: 0,
            total: None,
        };
        assert_eq!(
            serde_json::to_value(&progress).unwrap(),
            serde_json::json!({ "request_id": "s", "phase": "build", "completed": 0, "total": null })
        );
        let progress = SubmitProgress {
            request_id: "s".into(),
            phase: SubmitPhase::Judge,
            completed: 2,
            total: Some(3),
        };
        assert_eq!(
            serde_json::to_value(&progress).unwrap(),
            serde_json::json!({ "request_id": "s", "phase": "judge", "completed": 2, "total": 3 })
        );
    }

    #[test]
    fn run_finished_of_a_submit_serializes_in_the_spec_shape() {
        let finished = RunFinished {
            request_id: "s".into(),
            kind: FinishedKind::Submit,
            finished_at: "2026-11-24T10:00:00+09:00".into(),
            error: None,
            run: None,
            submit: Some(SubmitJudgement::judged(JudgeResult::from_cases(vec![
                case("基本", CaseStatus::Pass, None),
                case(
                    "空の入力",
                    CaseStatus::Fail,
                    Some(CaseDetail::Output {
                        input: "0\n".into(),
                        expected: "0\n".into(),
                        actual: "1\n".into(),
                    }),
                ),
            ]))),
            progress: None,
        };
        assert_eq!(
            serde_json::to_value(&finished).unwrap(),
            serde_json::json!({
                "request_id": "s",
                "kind": "submit",
                "finished_at": "2026-11-24T10:00:00+09:00",
                "error": null,
                "run": null,
                "submit": {
                    "status": "wrong",
                    "passed": 1,
                    "total": 2,
                    "cases": [
                        { "name": "基本", "status": "pass", "detail": null },
                        {
                            "name": "空の入力",
                            "status": "fail",
                            "detail": { "input": "0\n", "expected": "0\n", "actual": "1\n" },
                        },
                    ],
                },
                "progress": null,
            })
        );
        for (status, text) in [
            (SubmitResultStatus::CompileError, "compile_error"),
            (SubmitResultStatus::CratesUnavailable, "crates_unavailable"),
            (SubmitResultStatus::Stopped, "stopped"),
        ] {
            assert_eq!(
                serde_json::to_value(SubmitJudgement::without_cases(status)).unwrap(),
                serde_json::json!({ "status": text, "passed": 0, "total": 0, "cases": [] })
            );
        }
    }

    #[test]
    fn stopped_submits_are_not_recorded() {
        assert_eq!(
            SubmitJudgement::without_cases(SubmitResultStatus::Stopped).record_status(),
            None
        );
        assert_eq!(
            SubmitJudgement::without_cases(SubmitResultStatus::CratesUnavailable).record_status(),
            None
        );
        assert_eq!(
            SubmitJudgement::without_cases(SubmitResultStatus::CompileError).record_status(),
            Some(SubmitStatus::CompileError)
        );
        assert_eq!(
            SubmitJudgement::judged(JudgeResult::stopped()).status,
            SubmitResultStatus::Stopped
        );
    }

    #[test]
    fn failed_cases_and_program_stderr_come_from_the_cases_in_order() {
        let judgement = SubmitJudgement::judged(JudgeResult::from_cases(vec![
            case("a", CaseStatus::Pass, None),
            case(
                "b",
                CaseStatus::RuntimeError,
                Some(CaseDetail::Stderr("panicked at b".into())),
            ),
            case(
                "c",
                CaseStatus::Fail,
                Some(CaseDetail::PanicMessage("left: 1\nright: 2".into())),
            ),
            case(
                "d",
                CaseStatus::Timeout,
                Some(CaseDetail::Stderr(String::new())),
            ),
            case(
                "e",
                CaseStatus::Timeout,
                Some(CaseDetail::Stderr("waiting\n".into())),
            ),
        ]));
        assert_eq!(judgement.failed_cases(), ["b", "c", "d", "e"]);
        assert_eq!(judgement.program_stderr(), "panicked at b\nwaiting\n");
        assert_eq!(
            SubmitJudgement::without_cases(SubmitResultStatus::CompileError).program_stderr(),
            ""
        );
    }
}
