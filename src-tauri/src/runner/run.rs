//! 実行・テスト実行のタスク（spec/exercise-runner/run.md「手順」「テスト実行」「ビルドの出力の表示」「出力の送り方」
//! 「停止」、spec/exercise-runner/attempt-log.md）と、イベント `run_started`・`run_output`・`run_finished` の内容
//! （spec/architecture/commands/run.md「イベント」）。
//!
//! - [`execute`] は、`run_code` が受け付けた後に、別のスレッドで呼ぶ。`project/` を組み立て、ビルドし、
//!   学習者のプログラム（テスト実行では `cargo test`）を動かし、試行ログに `run` イベントを追記してから、
//!   要求を外して `run_finished` を送る。実行（`main`）では、プログラムを起動して標準入力を受け付けるようにしてから、
//!   `run_started` を送る。
//! - C の演習（[`RunToolchain::Clang`]）では、`project/` の組み立てとビルドの代わりに、[`super::c_run`] の手順で
//!   `c-project/` を組み立てて `clang` でビルドする。プログラムの動かし方、試行ログへの追記、`run_finished` を送る
//!   順序は、Rust の演習と同じにする（spec/exercise-runner/c-run.md）。
//! - イベントの送り先と試行ログ・進捗の書き込み先は [`RunEvents`] で受け取る。アプリでは Tauri の `emit` で送り、
//!   学習者データのディレクトリに書く。提出のタスク（[`super::submit`]）も、同じものを使う。
//! - アプリの終了で止めたときは、試行ログに追記せず、`run_finished` も送らない。
//! - ビルドの失敗が、初回準備のやり直しが要る失敗（[`super::lost_crates`]）に当てはまったときは、ビルドの出力を送らずに
//!   捨て、`run_output` の `system` で [`CRATES_LOST_MESSAGE`] を送り、初回準備を始め（[`RunEvents::crates_lost`]）、
//!   結果を `crates_unavailable` にする。試行ログには追記しない。初回準備が終わっても、自動ではやり直さない。

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use serde::Serialize;

use super::RunnerDir;
use super::build::{
    BuildCommand, BuildOutcome, BuildOutputKind, BuildSpec, BuildStatus, start_build,
};
use super::process::{
    self, Limits, OutputChunk, OutputLimit, OutputStream, ProcessSpec, StdinMode, Termination,
    TimeoutOutput,
};
use super::project::{assemble_project, build_env, uses_allowed_crates};
use super::runs::{ActiveRun, RunLimits, Runs, StdinState};
use super::submit::{SubmitJudgement, SubmitProgress};
use super::workspace::WorkspaceDir;
use crate::attempts::{BuildOutput, RunMode, RunRecord, RunStatus, SessionKind, SubmitRecord};
use crate::commands::error::{CommandError, ErrorCode};
use crate::progress::{LessonProgress, now_string};
#[cfg(doc)]
use crate::storage::AppDataDir;
use crate::storage::StorageError;

/// 実行（`main`）で学習者のプログラムを起動したことを送るイベントの名前。
pub const RUN_STARTED: &str = "run_started";
/// 出力を送るイベントの名前。
pub const RUN_OUTPUT: &str = "run_output";
/// 実行・提出が終わったことを送るイベントの名前。
pub const RUN_FINISHED: &str = "run_finished";

/// `run_started` の内容。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunStarted {
    pub request_id: String,
}

/// `run_output` の断片の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStream {
    Stdout,
    Stderr,
    /// アプリが出す知らせ。
    System,
}

/// `run_output` の断片。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunOutputChunk {
    pub stream: RunStream,
    pub text: String,
}

/// `run_output` の内容。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunOutput {
    pub request_id: String,
    pub chunks: Vec<RunOutputChunk>,
}

/// `run_finished` の `kind`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishedKind {
    Run,
    Submit,
}

/// `run_finished` の `run.status`（spec/exercise-runner/run.md「結果の状態」）。試行ログに記録する状態
/// （[`RunStatus`]）に、記録しない `crates_unavailable` を足したもの。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunResultStatus {
    Success,
    CompileError,
    RuntimeError,
    Timeout,
    Stopped,
    /// ビルドの失敗が、初回準備のやり直しが要る失敗に当てはまった。学習者のコードは実行していない。
    CratesUnavailable,
}

impl RunResultStatus {
    /// 試行ログに記録する状態。`crates_unavailable` は記録しないので `None`（ADR 0125）。
    pub fn record_status(self) -> Option<RunStatus> {
        match self {
            Self::Success => Some(RunStatus::Success),
            Self::CompileError => Some(RunStatus::CompileError),
            Self::RuntimeError => Some(RunStatus::RuntimeError),
            Self::Timeout => Some(RunStatus::Timeout),
            Self::Stopped => Some(RunStatus::Stopped),
            Self::CratesUnavailable => None,
        }
    }
}

/// `run_finished` の `run`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunResultSummary {
    pub mode: RunMode,
    pub status: RunResultStatus,
    /// 学習者のプログラム（テスト実行では `cargo test`）の終了コード。起動していないときと、シグナルで終わったとき
    /// （止めた、時間切れ、出力量の上限を含む）は `None`（ADR 0206）。
    pub exit_code: Option<i32>,
}

/// `run_finished` の内容。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunFinished {
    pub request_id: String,
    pub kind: FinishedKind,
    pub finished_at: String,
    /// アプリの側の失敗で、結果の状態を決められずに終わったときのエラー。
    pub error: Option<CommandError>,
    pub run: Option<RunResultSummary>,
    /// 提出の判定結果。実行と、`error` があるときは `None`。
    pub submit: Option<SubmitJudgement>,
    /// 提出で正解してレッスンを完了にしたときの、更新した後の進捗。ほかは `None`。
    pub progress: Option<LessonProgress>,
}

/// イベントの送り先と、試行ログ・進捗の書き込み先。
pub trait RunEvents: Send + Sync + 'static {
    /// `run_started` を送る。
    fn started(&self, started: RunStarted);
    /// `run_output` を送る。
    fn output(&self, output: RunOutput);
    /// `submit_progress` を送る。
    fn submit_progress(&self, progress: SubmitProgress);
    /// `run_finished` を送る。
    fn finished(&self, finished: RunFinished);
    /// 試行ログに `run` イベントを追記する（[`AppDataDir::append_run`]）。
    fn append_run(
        &self,
        lesson_id: &str,
        kind: SessionKind,
        record: RunRecord,
    ) -> Result<(), StorageError>;
    /// 試行ログに `submit` イベントを追記し、追記したセッションの ID を返す（[`AppDataDir::append_submit`]）。
    fn append_submit(
        &self,
        lesson_id: &str,
        kind: SessionKind,
        record: SubmitRecord,
    ) -> Result<String, StorageError>;
    /// 提出で正解したときに、セッションの `passed_at` を書き、レッスンを完了にして、更新した後の進捗を返す
    /// （[`AppDataDir::record_pass`]）。
    fn record_pass(
        &self,
        lesson_id: &str,
        session_id: &str,
        at: &str,
    ) -> Result<Option<LessonProgress>, StorageError>;
    /// ビルドの失敗が、初回準備のやり直しが要る失敗に当てはまった。`prepared.json` の `status` を `failed` にして、
    /// 初回準備を始める（spec/exercise-runner/crates-and-workspace.md「初回準備のやり直しが要る失敗」）。
    /// `run_finished` を送る前に呼ぶ。
    fn crates_lost(&self);
}

/// ビルドの失敗が、初回準備のやり直しが要る失敗に当てはまったときに、`run_output` の `system` で送る知らせ。
pub const CRATES_LOST_MESSAGE: &str = "外部クレートを準備し直しています";

/// 初回準備のやり直しが要る失敗になった実行・提出の後始末。ビルドの出力を送らずに捨て、知らせを `system` で送り、
/// 初回準備を始める（[`RunEvents::crates_lost`]）。捨てたビルドの出力は、原因を後から確かめられるよう、アプリのログ
/// （標準エラー出力）に書く（ADR 0125）。
pub(super) fn handle_lost_crates(events: &dyn RunEvents, request_id: &str, built: &BuildOutcome) {
    let output = built.attempt_output();
    eprintln!(
        "要求 {request_id} のビルドで、外部クレートが失われていた（crates_unavailable）:\n{}{}",
        output.compiler_messages, output.cargo_stderr
    );
    send_output(
        events,
        request_id,
        vec![RunOutputChunk {
            stream: RunStream::System,
            text: CRATES_LOST_MESSAGE.to_string(),
        }],
    );
    events.crates_lost();
}

/// 実行時間の制限で止めたときの知らせ。
pub fn timeout_message(limits: &RunLimits) -> String {
    format!(
        "実行時間の制限（{} 秒）を超えたため停止しました。",
        limits.run_time.as_secs()
    )
}

/// 出力量の上限で止めたときの知らせ。
pub fn output_limit_message(limits: &RunLimits) -> String {
    format!(
        "出力量の上限（{}）を超えたため停止しました。",
        size_text(limits.output)
    )
}

/// バイト数を、MB・KB・バイトで表す（1 MB = 1024 KB）。
pub(super) fn size_text(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = 1024 * KB;
    if bytes >= MB && bytes.is_multiple_of(MB) {
        format!("{} MB", bytes / MB)
    } else if bytes >= KB && bytes.is_multiple_of(KB) {
        format!("{} KB", bytes / KB)
    } else {
        format!("{bytes} バイト")
    }
}

/// 実行・テスト実行の要求と、使うもの。
pub struct RunJob {
    pub active: Arc<ActiveRun>,
    pub runs: Runs,
    /// C の演習では、いつも `main`。
    pub mode: RunMode,
    /// 試行ログに追記するときに、未完了のセッションが無ければ始めるセッションの種類。
    pub session_kind: SessionKind,
    /// エディタ上の全ファイル。
    pub files: BTreeMap<String, String>,
    pub workspace: WorkspaceDir,
    /// ビルドに使うもの。Rust の演習か C の演習かで変わる。
    pub toolchain: RunToolchain,
}

/// 実行のビルドに使うもの。
pub enum RunToolchain {
    /// Rust の演習。`project/` を組み立てて cargo でビルドする（spec/exercise-runner/run.md）。
    Cargo {
        /// `cargo` の絶対パス。
        cargo: PathBuf,
        /// 演習の `dependencies`。
        dependencies: Vec<String>,
        runner: RunnerDir,
    },
    /// C の演習。`c-project/` を組み立てて `clang` でビルドする（spec/exercise-runner/c-run.md）。
    Clang {
        /// 検出した `clang` の絶対パス。
        clang: PathBuf,
    },
}

/// 結果の状態が決まった実行。
pub(super) struct Outcome {
    pub(super) status: RunResultStatus,
    pub(super) exit_code: Option<i32>,
    pub(super) error_codes: Vec<String>,
    pub(super) build: BuildOutput,
    pub(super) stdout: String,
    pub(super) program_stderr: String,
}

impl Outcome {
    /// プログラムを起動していない結果。
    fn without_program(status: RunResultStatus, build: &BuildOutcome) -> Self {
        Self {
            status,
            exit_code: None,
            error_codes: build.error_codes.clone(),
            build: build.attempt_output(),
            stdout: String::new(),
            program_stderr: String::new(),
        }
    }

    /// ビルドを始める前に止めた結果。
    pub(super) fn stopped_before_build() -> Self {
        Self {
            status: RunResultStatus::Stopped,
            exit_code: None,
            error_codes: Vec::new(),
            build: BuildOutput::default(),
            stdout: String::new(),
            program_stderr: String::new(),
        }
    }
}

/// 実行・テスト実行を最後まで行う。終わったら、試行ログに追記し、要求を外してから `run_finished` を送る。
pub fn execute(job: RunJob, events: Arc<dyn RunEvents>) {
    let result = run_steps(&job, &events);
    let finished_at = now_string();
    let request_id = job.active.request_id().to_string();
    let lesson_id = job.active.lesson_id().to_string();

    let finished = match result {
        Ok(outcome) => {
            let summary = RunResultSummary {
                mode: job.mode,
                status: outcome.status,
                exit_code: outcome.exit_code,
            };
            // `crates_unavailable` の実行と、アプリの終了で止めた実行は、記録しない（attempt-log.md）。
            if let Some(status) = outcome.status.record_status()
                && !job.active.stopped_for_app_exit()
            {
                let record = RunRecord {
                    at: finished_at.clone(),
                    mode: job.mode,
                    files: job.files.clone(),
                    status,
                    error_codes: outcome.error_codes,
                    build: outcome.build,
                    stdout: outcome.stdout,
                    program_stderr: outcome.program_stderr,
                    exit_code: outcome.exit_code,
                    stdin: match job.mode {
                        RunMode::Main => job.active.stdin_log(),
                        RunMode::Test => String::new(),
                    },
                };
                // 記録できなくても、結果の状態は決まっているので、run_finished は結果を送る。
                if let Err(error) = events.append_run(&lesson_id, job.session_kind, record) {
                    eprintln!("実行 {request_id} の結果を試行ログに記録できなかった: {error}");
                }
            }
            RunFinished {
                request_id,
                kind: FinishedKind::Run,
                finished_at,
                error: None,
                run: Some(summary),
                submit: None,
                progress: None,
            }
        }
        Err(error) => RunFinished {
            request_id,
            kind: FinishedKind::Run,
            finished_at,
            error: Some(error),
            run: None,
            submit: None,
            progress: None,
        },
    };

    // 要求を外してから送る。受け取った画面が、すぐに次の実行を始められるように。
    // 試行ログへの追記は、要求を外す前に済ませる。`end_session` は、要求が外れるのを待って、止めた実行の追記が
    // 終わったとみなす（Runs::stop_lesson_run_and_wait）。
    job.runs.release(&job.active);
    if !job.active.stopped_for_app_exit() {
        events.finished(finished);
    }
}

/// 組み立てから、プログラムの終わりまで。アプリの側の失敗は `Err` で返す。
fn run_steps(job: &RunJob, events: &Arc<dyn RunEvents>) -> Result<Outcome, CommandError> {
    if job.active.stop_cause().is_some() {
        return Ok(Outcome::stopped_before_build());
    }
    match &job.toolchain {
        RunToolchain::Cargo {
            cargo,
            dependencies,
            runner,
        } => cargo_run_steps(job, cargo, dependencies, runner, events),
        RunToolchain::Clang { clang } => super::c_run::run_steps(job, clang, events),
    }
}

/// Rust の演習の、`project/` の組み立てから、プログラムの終わりまで。
fn cargo_run_steps(
    job: &RunJob,
    cargo: &Path,
    dependencies: &[String],
    runner: &RunnerDir,
    events: &Arc<dyn RunEvents>,
) -> Result<Outcome, CommandError> {
    let limits = job.runs.limits();
    assemble_project(&job.workspace, runner, dependencies, &job.files)?;

    let envs = build_env(&job.workspace, dependencies);
    let project_dir = job.workspace.project();
    let mut build_spec = BuildSpec::new(
        cargo,
        &project_dir,
        match job.mode {
            RunMode::Main => BuildCommand::Build,
            RunMode::Test => BuildCommand::TestNoRun,
        },
        envs.clone(),
    );
    build_spec.time_limit = limits.build_time;
    let running = start_build(build_spec)
        .map_err(|error| app_failure("cargo を起動できませんでした。", error))?;
    job.active.set_building(running.handle());
    let built = running
        .wait()
        .map_err(|error| app_failure("ビルドの結果を受け取れませんでした。", error))?;

    match built.status {
        // 止める操作で止めたときは、ビルドの出力を捨てている（run.md「停止」）。
        BuildStatus::Stopped => {
            return Ok(Outcome::without_program(RunResultStatus::Stopped, &built));
        }
        // 初回準備のやり直しが要る失敗は、ビルドの出力を送らずに捨て、学習者のコードの誤りとして表示しない
        // （run.md「手順」の 2）。
        BuildStatus::Failed { .. }
            if built.needs_crate_preparation(
                &job.workspace
                    .project_target(uses_allowed_crates(dependencies)),
            ) =>
        {
            handle_lost_crates(events.as_ref(), job.active.request_id(), &built);
            return Ok(Outcome::without_program(
                RunResultStatus::CratesUnavailable,
                &built,
            ));
        }
        BuildStatus::Failed { .. } | BuildStatus::TimedOut => {
            send_build_output(job, events, &built);
            return Ok(Outcome::without_program(
                RunResultStatus::CompileError,
                &built,
            ));
        }
        BuildStatus::Succeeded => send_build_output(job, events, &built),
    }

    let mut spec = match job.mode {
        RunMode::Main => {
            let executable = built.main_executable().ok_or_else(|| CommandError {
                code: ErrorCode::Internal,
                message: "実行するプログラムが見つかりませんでした。".into(),
                detail: Some("ビルドの出力に src/main.rs の実行ファイルが無い".into()),
            })?;
            let mut spec = ProcessSpec::new(executable, &project_dir, process_limits(&limits));
            spec.stdin = StdinMode::Interactive;
            spec
        }
        RunMode::Test => {
            let mut spec = ProcessSpec::new(cargo, &project_dir, process_limits(&limits));
            spec.args = ["test", "--offline", "--no-fail-fast"]
                .iter()
                .map(OsString::from)
                .collect();
            spec.envs.push(("CARGO_TERM_COLOR".into(), "always".into()));
            spec.stdin = StdinMode::Closed;
            spec
        }
    };
    spec.envs.extend(
        envs.into_iter()
            .map(|(key, value)| (OsString::from(key), value)),
    );
    spec.on_timeout = TimeoutOutput::Discard;

    // ビルドが終わってからプログラムを起動するまでに止める指示が来ていたら、起動しない。
    if job.active.stop_cause().is_some() {
        return Ok(Outcome::without_program(RunResultStatus::Stopped, &built));
    }

    let program = run_program(job, events, spec)?;
    let ended = program_end(&program.termination, &limits);
    send_notice(job, events, ended.notice);
    Ok(Outcome {
        status: ended.status,
        exit_code: ended.exit_code,
        error_codes: built.error_codes.clone(),
        build: built.attempt_output(),
        stdout: program.stdout,
        program_stderr: program.stderr,
    })
}

/// 動かし終えた学習者のプログラム（テスト実行では `cargo test`）。
pub(super) struct ProgramRun {
    pub(super) termination: Termination,
    /// 受け取った標準出力。
    pub(super) stdout: String,
    /// 受け取った標準エラー出力。
    pub(super) stderr: String,
}

/// 学習者のプログラム（テスト実行では `cargo test`）を `spec` のとおりに起動し、出力を `run_output` で送りながら、
/// 終わるまで待つ。実行（`main`）では、標準入力を受け付けるようにしてから `run_started` を送る。Rust の演習と
/// C の演習で共通である（run.md「手順」の 3〜5、c-run.md「手順」の 3〜5）。
pub(super) fn run_program(
    job: &RunJob,
    events: &Arc<dyn RunEvents>,
    spec: ProcessSpec,
) -> Result<ProgramRun, CommandError> {
    let collected = Arc::new(Mutex::new((String::new(), String::new())));
    let process = {
        let collected = Arc::clone(&collected);
        let events = Arc::clone(events);
        let request_id = job.active.request_id().to_string();
        process::spawn(spec, move |chunks| {
            {
                let mut collected = collected.lock().unwrap_or_else(PoisonError::into_inner);
                for chunk in &chunks {
                    match chunk.stream {
                        OutputStream::Stdout => collected.0.push_str(&chunk.text),
                        OutputStream::Stderr => collected.1.push_str(&chunk.text),
                    }
                }
            }
            events.output(RunOutput {
                request_id: request_id.clone(),
                chunks: chunks.into_iter().map(program_chunk).collect(),
            });
        })
        .map_err(|error| app_failure("プログラムを起動できませんでした。", error))?
    };
    job.active.set_running(
        process.handle(),
        match job.mode {
            RunMode::Main => StdinState::Open,
            RunMode::Test => StdinState::Closed,
        },
    );
    // 標準入力を受け付けるようにした（set_running）後に送る。受け取ってすぐの send_stdin が invalid_state に
    // ならないように。テスト実行は標準入力を使わないので送らない（spec/architecture/commands/run.md「イベント」）。
    if job.mode == RunMode::Main {
        events.started(RunStarted {
            request_id: job.active.request_id().to_string(),
        });
    }
    // 返る前に、プログラムの出力はすべて送り終えている（process::RunningProcess::wait）。
    let termination = process.wait();
    job.active.set_finished();
    let termination = termination
        .map_err(|error| app_failure("プログラムの終わりを受け取れませんでした。", error))?;
    let (stdout, stderr) =
        std::mem::take(&mut *collected.lock().unwrap_or_else(PoisonError::into_inner));
    Ok(ProgramRun {
        termination,
        stdout,
        stderr,
    })
}

/// プログラムの終わり方から決まる、結果の状態。
pub(super) struct ProgramEnd {
    pub(super) status: RunResultStatus,
    pub(super) exit_code: Option<i32>,
    /// `system` で送る知らせ（タイムアウトの旨、出力量の上限で停止した旨）。
    pub(super) notice: Option<String>,
}

/// プログラムの終わり方から、結果の状態（run.md「結果の状態」）と、送る知らせを決める。
pub(super) fn program_end(termination: &Termination, limits: &RunLimits) -> ProgramEnd {
    let (status, exit_code, notice) = match *termination {
        Termination::Exited(0) => (RunResultStatus::Success, Some(0), None),
        Termination::Exited(code) => (RunResultStatus::RuntimeError, Some(code), None),
        Termination::Signaled(_) => (RunResultStatus::RuntimeError, None, None),
        Termination::TimedOut => (
            RunResultStatus::Timeout,
            None,
            Some(timeout_message(limits)),
        ),
        Termination::OutputLimitExceeded => (
            RunResultStatus::RuntimeError,
            None,
            Some(output_limit_message(limits)),
        ),
        Termination::Stopped => (RunResultStatus::Stopped, None, None),
    };
    ProgramEnd {
        status,
        exit_code,
        notice,
    }
}

/// 知らせがあれば、`run_output` の `system` で送る。
pub(super) fn send_notice(job: &RunJob, events: &Arc<dyn RunEvents>, notice: Option<String>) {
    if let Some(text) = notice {
        send(
            job,
            events,
            vec![RunOutputChunk {
                stream: RunStream::System,
                text,
            }],
        );
    }
}

/// 実行・テスト実行の制限値（ビルドを除く）。
pub(super) fn process_limits(limits: &RunLimits) -> Limits {
    Limits {
        time: limits.run_time,
        output: OutputLimit::Total(limits.output),
    }
}

/// ビルドの出力を、受け取った順に 1 つの `run_output` で送る。
fn send_build_output(job: &RunJob, events: &Arc<dyn RunEvents>, built: &BuildOutcome) {
    send(job, events, build_output_chunks(built));
}

/// ビルドの出力を、受け取った順に `run_output` の断片にする。コンパイラの診断と cargo 自身の標準エラー出力は
/// `stderr`、アプリの知らせ（タイムアウトの旨、隠しテストの側でエラーが出たときの案内）は `system` にする。
/// 同じ種類の断片が続く分は 1 つにまとめる。
pub(super) fn build_output_chunks(built: &BuildOutcome) -> Vec<RunOutputChunk> {
    let mut chunks: Vec<RunOutputChunk> = Vec::new();
    for piece in &built.output {
        let stream = match piece.kind {
            BuildOutputKind::CompilerMessage | BuildOutputKind::CargoStderr => RunStream::Stderr,
            BuildOutputKind::System => RunStream::System,
        };
        match chunks.last_mut() {
            Some(last) if last.stream == stream => last.text.push_str(&piece.text),
            _ => chunks.push(RunOutputChunk {
                stream,
                text: piece.text.clone(),
            }),
        }
    }
    chunks
}

/// 断片があれば、`run_output` で送る。
pub(super) fn send(job: &RunJob, events: &Arc<dyn RunEvents>, chunks: Vec<RunOutputChunk>) {
    send_output(events.as_ref(), job.active.request_id(), chunks);
}

/// 断片があれば、要求 `request_id` の `run_output` で送る。
pub(super) fn send_output(events: &dyn RunEvents, request_id: &str, chunks: Vec<RunOutputChunk>) {
    if chunks.is_empty() {
        return;
    }
    events.output(RunOutput {
        request_id: request_id.to_string(),
        chunks,
    });
}

fn program_chunk(chunk: OutputChunk) -> RunOutputChunk {
    RunOutputChunk {
        stream: match chunk.stream {
            OutputStream::Stdout => RunStream::Stdout,
            OutputStream::Stderr => RunStream::Stderr,
        },
        text: chunk.text,
    }
}

/// アプリの側の失敗（`run_finished` の `error`）。
pub(super) fn app_failure(message: &str, error: std::io::Error) -> CommandError {
    CommandError {
        code: ErrorCode::Internal,
        message: message.into(),
        detail: Some(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn limits_follow_the_spec() {
        let limits = RunLimits::default();
        assert_eq!(limits.run_time, Duration::from_secs(60));
        assert_eq!(limits.output, 1024 * 1024);
        assert_eq!(limits.build_time, Duration::from_secs(120));
        assert_eq!(
            timeout_message(&limits),
            "実行時間の制限（60 秒）を超えたため停止しました。"
        );
        assert_eq!(
            output_limit_message(&limits),
            "出力量の上限（1 MB）を超えたため停止しました。"
        );
    }

    #[test]
    fn sizes_are_written_in_mb_kb_or_bytes() {
        assert_eq!(size_text(2 * 1024 * 1024), "2 MB");
        assert_eq!(size_text(64 * 1024), "64 KB");
        assert_eq!(size_text(1000), "1000 バイト");
    }

    #[test]
    fn crates_unavailable_is_sent_but_not_recorded() {
        assert_eq!(
            serde_json::to_value(RunResultStatus::CratesUnavailable).unwrap(),
            serde_json::json!("crates_unavailable")
        );
        assert_eq!(RunResultStatus::CratesUnavailable.record_status(), None);
        for (status, recorded, text) in [
            (RunResultStatus::Success, RunStatus::Success, "success"),
            (
                RunResultStatus::CompileError,
                RunStatus::CompileError,
                "compile_error",
            ),
            (
                RunResultStatus::RuntimeError,
                RunStatus::RuntimeError,
                "runtime_error",
            ),
            (RunResultStatus::Timeout, RunStatus::Timeout, "timeout"),
            (RunResultStatus::Stopped, RunStatus::Stopped, "stopped"),
        ] {
            assert_eq!(status.record_status(), Some(recorded));
            assert_eq!(
                serde_json::to_value(status).unwrap(),
                serde_json::json!(text)
            );
        }
    }

    #[test]
    fn events_serialize_in_the_spec_shape() {
        assert_eq!(
            serde_json::to_value(RunStarted {
                request_id: "r".into()
            })
            .unwrap(),
            serde_json::json!({ "request_id": "r" })
        );

        let output = RunOutput {
            request_id: "r".into(),
            chunks: vec![
                RunOutputChunk {
                    stream: RunStream::Stdout,
                    text: "a".into(),
                },
                RunOutputChunk {
                    stream: RunStream::System,
                    text: "b".into(),
                },
            ],
        };
        assert_eq!(
            serde_json::to_value(&output).unwrap(),
            serde_json::json!({
                "request_id": "r",
                "chunks": [
                    { "stream": "stdout", "text": "a" },
                    { "stream": "system", "text": "b" },
                ],
            })
        );

        let finished = RunFinished {
            request_id: "r".into(),
            kind: FinishedKind::Run,
            finished_at: "2026-11-24T10:00:00+09:00".into(),
            error: None,
            run: Some(RunResultSummary {
                mode: RunMode::Test,
                status: RunResultStatus::RuntimeError,
                exit_code: Some(101),
            }),
            submit: None,
            progress: None,
        };
        assert_eq!(
            serde_json::to_value(&finished).unwrap(),
            serde_json::json!({
                "request_id": "r",
                "kind": "run",
                "finished_at": "2026-11-24T10:00:00+09:00",
                "error": null,
                "run": { "mode": "test", "status": "runtime_error", "exit_code": 101 },
                "submit": null,
                "progress": null,
            })
        );
    }
}
