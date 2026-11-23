//! 実行・テスト実行のタスク（spec/exercise-runner/run.md「手順」「テスト実行」「ビルドの出力の表示」「出力の送り方」
//! 「停止」、spec/exercise-runner/attempt-log.md）と、イベント `run_output`・`run_finished` の内容
//! （spec/architecture/commands/run.md「イベント」）。
//!
//! - [`execute`] は、`run_code` が受け付けた後に、別のスレッドで呼ぶ。`project/` を組み立て、ビルドし、
//!   学習者のプログラム（テスト実行では `cargo test`）を動かし、試行ログに `run` イベントを追記してから、
//!   要求を外して `run_finished` を送る。
//! - イベントの送り先は [`RunEvents`] で受け取る。アプリでは Tauri の `emit` で送り、テストでは差し替えられる。
//! - アプリの終了で止めたときは、試行ログに追記せず、`run_finished` も送らない。

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;
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
use super::project::{assemble_project, build_env};
use super::runs::{ActiveRun, RunLimits, Runs, StdinState};
use super::workspace::WorkspaceDir;
use crate::attempts::{BuildOutput, RunMode, RunRecord, RunStatus, SessionKind};
use crate::commands::error::{CommandError, ErrorCode};
use crate::progress::{LessonProgress, now_string};
#[cfg(doc)]
use crate::storage::AppDataDir;
use crate::storage::StorageError;

/// 出力を送るイベントの名前。
pub const RUN_OUTPUT: &str = "run_output";
/// 実行・提出が終わったことを送るイベントの名前。
pub const RUN_FINISHED: &str = "run_finished";

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

/// `run_finished` の `run`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunResultSummary {
    pub mode: RunMode,
    pub status: RunStatus,
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
    /// 提出の判定結果。提出は別のタスクで作るので、実行ではいつも `None`。
    pub submit: Option<serde_json::Value>,
    pub progress: Option<LessonProgress>,
}

/// イベントの送り先と、試行ログの書き込み先。
pub trait RunEvents: Send + Sync + 'static {
    /// `run_output` を送る。
    fn output(&self, output: RunOutput);
    /// `run_finished` を送る。
    fn finished(&self, finished: RunFinished);
    /// 試行ログに `run` イベントを追記する（[`AppDataDir::append_run`]）。
    fn append_run(
        &self,
        lesson_id: &str,
        kind: SessionKind,
        record: RunRecord,
    ) -> Result<(), StorageError>;
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
fn size_text(bytes: usize) -> String {
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
    pub mode: RunMode,
    /// 試行ログに追記するときに、未完了のセッションが無ければ始めるセッションの種類。
    pub session_kind: SessionKind,
    /// エディタ上の全ファイル。
    pub files: BTreeMap<String, String>,
    /// 演習の `dependencies`。
    pub dependencies: Vec<String>,
    /// `cargo` の絶対パス。
    pub cargo: PathBuf,
    pub workspace: WorkspaceDir,
    pub runner: RunnerDir,
}

/// 結果の状態が決まった実行。
struct Outcome {
    status: RunStatus,
    exit_code: Option<i32>,
    error_codes: Vec<String>,
    build: BuildOutput,
    stdout: String,
    program_stderr: String,
}

impl Outcome {
    /// プログラムを起動していない結果。
    fn without_program(status: RunStatus, build: &BuildOutcome) -> Self {
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
    fn stopped_before_build() -> Self {
        Self {
            status: RunStatus::Stopped,
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
            if !job.active.stopped_for_app_exit() {
                let record = RunRecord {
                    at: finished_at.clone(),
                    mode: job.mode,
                    files: job.files.clone(),
                    status: outcome.status,
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
    job.runs.release(&job.active);
    if !job.active.stopped_for_app_exit() {
        events.finished(finished);
    }
}

/// `project/` の組み立てから、プログラムの終わりまで。アプリの側の失敗は `Err` で返す。
fn run_steps(job: &RunJob, events: &Arc<dyn RunEvents>) -> Result<Outcome, CommandError> {
    let limits = job.runs.limits();
    if job.active.stop_cause().is_some() {
        return Ok(Outcome::stopped_before_build());
    }
    assemble_project(&job.workspace, &job.runner, &job.dependencies, &job.files)?;

    let envs = build_env(&job.workspace, &job.dependencies);
    let project_dir = job.workspace.project();
    let mut build_spec = BuildSpec::new(
        &job.cargo,
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
        BuildStatus::Stopped => return Ok(Outcome::without_program(RunStatus::Stopped, &built)),
        BuildStatus::Failed { .. } | BuildStatus::TimedOut => {
            send_build_output(job, events, &built);
            return Ok(Outcome::without_program(RunStatus::CompileError, &built));
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
            let mut spec = ProcessSpec::new(&job.cargo, &project_dir, process_limits(&limits));
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
        return Ok(Outcome::without_program(RunStatus::Stopped, &built));
    }

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
    let termination = process.wait();
    job.active.set_finished();
    let termination = termination
        .map_err(|error| app_failure("プログラムの終わりを受け取れませんでした。", error))?;

    let (status, exit_code, notice) = match termination {
        Termination::Exited(0) => (RunStatus::Success, Some(0), None),
        Termination::Exited(code) => (RunStatus::RuntimeError, Some(code), None),
        Termination::Signaled(_) => (RunStatus::RuntimeError, None, None),
        Termination::TimedOut => (RunStatus::Timeout, None, Some(timeout_message(&limits))),
        Termination::OutputLimitExceeded => (
            RunStatus::RuntimeError,
            None,
            Some(output_limit_message(&limits)),
        ),
        Termination::Stopped => (RunStatus::Stopped, None, None),
    };
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

    let (stdout, program_stderr) =
        std::mem::take(&mut *collected.lock().unwrap_or_else(PoisonError::into_inner));
    Ok(Outcome {
        status,
        exit_code,
        error_codes: built.error_codes.clone(),
        build: built.attempt_output(),
        stdout,
        program_stderr,
    })
}

/// 実行・テスト実行の制限値（ビルドを除く）。
fn process_limits(limits: &RunLimits) -> Limits {
    Limits {
        time: limits.run_time,
        output: OutputLimit::Total(limits.output),
    }
}

/// ビルドの出力を、受け取った順に 1 つの `run_output` で送る。コンパイラの診断と cargo 自身の標準エラー出力は
/// `stderr`、タイムアウトの旨は `system` にする。
fn send_build_output(job: &RunJob, events: &Arc<dyn RunEvents>, built: &BuildOutcome) {
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
    send(job, events, chunks);
}

/// 断片があれば、`run_output` で送る。
fn send(job: &RunJob, events: &Arc<dyn RunEvents>, chunks: Vec<RunOutputChunk>) {
    if chunks.is_empty() {
        return;
    }
    events.output(RunOutput {
        request_id: job.active.request_id().to_string(),
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
fn app_failure(message: &str, error: std::io::Error) -> CommandError {
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
    fn events_serialize_in_the_spec_shape() {
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
                status: RunStatus::RuntimeError,
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
