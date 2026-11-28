//! 実行のコマンド `run_code`・`send_stdin`・`close_stdin`・`stop_run`・`submit_code`
//! （spec/architecture/commands/run.md）。
//!
//! 実行・テスト実行のタスクは [`crate::runner::run`]、提出のタスクは [`crate::runner::submit`]、同時に 1 つだけ行う
//! 状態は [`crate::runner::runs`] にある。ここでは、受け付ける前に確かめること（ID、下書きの保存、Rust の演習では
//! `cargo` と初回準備の状態、C の演習では `mode` と `clang`）と、イベントの送り先・試行ログと進捗の書き込み先を扱う。
//!
//! C の演習（レッスンの言語が `c`）では、`cargo` と初回準備の状態を確かめない
//! （spec/exercise-runner/c-project-assembly.md「作業領域」）。C の演習の提出は、まだ作っていない（`submit_code` は
//! `internal` を返す）。

use std::collections::BTreeMap;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use super::error::{CommandError, ErrorCode};
use crate::attempts::{RunMode, RunRecord, SessionKind, SubmitRecord};
use crate::content::{ContentDir, Language, has_exercise};
use crate::environment::Environment;
use crate::environment::preparation::restart_crate_preparation;
use crate::progress::LessonProgress;
use crate::runner::RunnerDir;
use crate::runner::run::{
    self, RUN_FINISHED, RUN_OUTPUT, RUN_STARTED, RunEvents, RunFinished, RunJob, RunOutput,
    RunStarted, RunToolchain,
};
use crate::runner::runs::{ActiveRun, RunKind, Runs, not_running};
use crate::runner::submit::{self, SUBMIT_PROGRESS, SubmitJob, SubmitProgress};
use crate::runner::workspace::WorkspaceDir;
use crate::storage::{AppDataDir, StorageError};

/// 演習画面の「実行」（`mode` が `main`）と「テスト」（`test`）。下書きを保存し、`cargo` と初回準備の状態
/// （C の演習では、`mode` が `main` であることと `clang`）を確かめてから、別のスレッドで実行を始めて返す。
/// 実行（`main`）では、プログラムを起動したことを `run_started` で送る。出力は `run_output`、結果は `run_finished`
/// で送る。
///
/// 確かめる間に `cargo` の検出が終わるのを待つことがあるので、async のコマンドにして、別のスレッドで行う。
#[tauri::command(rename_all = "snake_case")]
pub async fn run_code<R: Runtime>(
    app: AppHandle<R>,
    request_id: String,
    lesson_id: String,
    mode: RunMode,
    files: BTreeMap<String, String>,
) -> Result<(), CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let request = Request {
            request_id: &request_id,
            lesson_id: &lesson_id,
            kind: RunKind::Run,
            mode: Some(mode),
            files,
        };
        start(&app, request, |prepared| {
            let job = RunJob {
                active: prepared.active,
                runs: prepared.runs,
                mode,
                session_kind: prepared.session_kind,
                files: prepared.files,
                workspace: prepared.workspace,
                toolchain: prepared.toolchain,
            };
            Ok(Box::new(move |events| run::execute(job, events)))
        })
    })
    .await
    .map_err(|error| CommandError {
        code: ErrorCode::Internal,
        message: "実行を始められませんでした。".into(),
        detail: Some(error.to_string()),
    })?
}

/// 演習画面の「提出」。下書きを保存し、`cargo` と初回準備の状態を確かめ、演習の判定方式を読んでから、
/// 別のスレッドで提出を始めて返す。ビルドの出力は `run_output`、判定の進み具合は `submit_progress`、
/// 判定結果は `run_finished` で送る。
#[tauri::command(rename_all = "snake_case")]
pub async fn submit_code<R: Runtime>(
    app: AppHandle<R>,
    request_id: String,
    lesson_id: String,
    files: BTreeMap<String, String>,
) -> Result<(), CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let request = Request {
            request_id: &request_id,
            lesson_id: &lesson_id,
            kind: RunKind::Submit,
            mode: None,
            files,
        };
        start(&app, request, |prepared| {
            let RunToolchain::Cargo {
                cargo,
                dependencies,
                runner,
            } = prepared.toolchain
            else {
                // C の演習の提出（spec/exercise-runner/c-submit.md）は、まだ作っていない。
                return Err(CommandError {
                    code: ErrorCode::Internal,
                    message: "C の演習の提出には、まだ対応していません。".into(),
                    detail: Some(format!(
                        "レッスン {} は C の演習",
                        prepared.active.lesson_id()
                    )),
                });
            };
            let judge = app
                .state::<ContentDir>()
                .exercise_judge(prepared.active.lesson_id())?;
            let job = SubmitJob {
                active: prepared.active,
                runs: prepared.runs,
                session_kind: prepared.session_kind,
                files: prepared.files,
                dependencies,
                judge,
                cargo,
                workspace: prepared.workspace,
                runner,
            };
            Ok(Box::new(move |events| submit::execute(job, events)))
        })
    })
    .await
    .map_err(|error| CommandError {
        code: ErrorCode::Internal,
        message: "提出を始められませんでした。".into(),
        detail: Some(error.to_string()),
    })?
}

/// 実行・提出の要求。
struct Request<'a> {
    request_id: &'a str,
    lesson_id: &'a str,
    kind: RunKind,
    /// `run_code` の `mode`。提出では `None`。
    mode: Option<RunMode>,
    /// エディタ上の全ファイル。
    files: BTreeMap<String, String>,
}

/// 別のスレッドで動かすタスク。
type Task = Box<dyn FnOnce(Arc<dyn RunEvents>) + Send>;

/// 受け付ける前に確かめた結果と、実行・提出のタスクに渡すもの。
struct Prepared {
    active: Arc<ActiveRun>,
    runs: Runs,
    session_kind: SessionKind,
    files: BTreeMap<String, String>,
    workspace: WorkspaceDir,
    toolchain: RunToolchain,
}

/// 要求を始め、確かめることを確かめ、`make_task` でタスクを作ってから、タスクのスレッドを起動する。
/// 確かめられなかったときと、タスクを作れなかったときは、要求を外して返す。
fn start<R: Runtime>(
    app: &AppHandle<R>,
    request: Request<'_>,
    make_task: impl FnOnce(Prepared) -> Result<Task, CommandError>,
) -> Result<(), CommandError> {
    let runs = app.state::<Runs>().inner().clone();
    let active = runs.begin(request.request_id, request.lesson_id, request.kind)?;
    match prepare(app, &active, &runs, request.mode, request.files).and_then(make_task) {
        Ok(task) => {
            let events: Arc<dyn RunEvents> = Arc::new(AppEvents(app.clone()));
            std::thread::spawn(move || task(events));
            Ok(())
        }
        Err(error) => {
            runs.release(&active);
            Err(error)
        }
    }
}

/// Rust の演習は、下書きの保存 → `cargo` → 初回準備の状態の順に確かめる。C の演習は、`mode` → 下書きの保存
/// → `clang` の順に確かめる。ファイルパスと `mode` の誤り（`invalid_argument`）は、`clang_unavailable` より先に返す
/// （spec/architecture/commands/run.md「run_code」）。
fn prepare<R: Runtime>(
    app: &AppHandle<R>,
    active: &Arc<ActiveRun>,
    runs: &Runs,
    mode: Option<RunMode>,
    files: BTreeMap<String, String>,
) -> Result<Prepared, CommandError> {
    let lesson_id = active.lesson_id();
    let content = app.state::<ContentDir>();
    let data_dir = app.state::<AppDataDir>();
    let environment = app.state::<Environment>();

    let kind = content.lesson_kind(lesson_id)?;
    let session_kind = SessionKind::for_lesson_kind(&kind)
        .filter(|_| has_exercise(&kind))
        .ok_or_else(|| CommandError {
            code: ErrorCode::InvalidArgument,
            message: "このレッスンには演習がありません。".into(),
            detail: Some(format!("レッスン {lesson_id} は演習の無いレッスン")),
        })?;
    let language = content.lesson_language(lesson_id)?;
    if language == Language::C && mode == Some(RunMode::Test) {
        // C の演習には、テスト実行が無い（spec/exercise-runner/c-run.md「テスト実行が無いこと」）。
        return Err(CommandError {
            code: ErrorCode::InvalidArgument,
            message: "C の演習には、テスト実行がありません。".into(),
            detail: Some(format!("レッスン {lesson_id} は C の演習で、mode が test")),
        });
    }
    // ファイルパスが条件を満たさないときは、ここで、何も書かずに invalid_argument になる。
    data_dir.save_draft(lesson_id, language, files.clone())?;
    let toolchain = match language {
        Language::Rust => {
            let tools = environment.rust_tools()?;
            let dependencies = content.exercise_dependencies(lesson_id)?;
            if !dependencies.is_empty() {
                environment.ensure_crates_ready()?;
            }
            RunToolchain::Cargo {
                cargo: tools.cargo,
                dependencies,
                runner: app.state::<RunnerDir>().inner().clone(),
            }
        }
        // C の演習は、cargo の有無と初回準備の状態に左右されない。
        Language::C => RunToolchain::Clang {
            clang: environment.clang()?.path,
        },
    };
    Ok(Prepared {
        active: Arc::clone(active),
        runs: runs.clone(),
        session_kind,
        files,
        workspace: WorkspaceDir::in_app_data(&data_dir),
        toolchain,
    })
}

/// 実行中のプログラムの標準入力に `text`（入力欄の 1 行に改行を付けたもの）を書く。
#[tauri::command(rename_all = "snake_case")]
pub fn send_stdin(
    runs: State<'_, Runs>,
    request_id: String,
    text: String,
) -> Result<(), CommandError> {
    runs.find(&request_id)
        .ok_or_else(not_running)?
        .write_stdin(&text)
}

/// 実行中のプログラムの標準入力を閉じる（「入力終了」）。
#[tauri::command(rename_all = "snake_case")]
pub fn close_stdin(runs: State<'_, Runs>, request_id: String) -> Result<(), CommandError> {
    runs.find(&request_id)
        .ok_or_else(not_running)?
        .close_stdin()
}

/// 実行・テスト実行・提出を止める。動いていない ID のときは、何もしない。止めた結果は `run_finished` で送る
/// （提出では `submit.status` が `stopped` になり、試行ログに追記しない）。
#[tauri::command(rename_all = "snake_case")]
pub fn stop_run(runs: State<'_, Runs>, request_id: String) -> Result<(), CommandError> {
    runs.stop(&request_id);
    Ok(())
}

/// アプリを終了するときに、動いている実行・テスト実行・提出を止める。`run_finished` を送らず、記録しない。
pub fn stop_runs_on_exit<R: Runtime>(app: &AppHandle<R>) {
    if let Some(runs) = app.try_state::<Runs>() {
        runs.stop_for_app_exit();
    }
}

/// イベントを、アプリのウィンドウに `emit` で送る。試行ログと進捗は、アプリの状態の [`AppDataDir`] に書く。
/// ビルドの失敗が初回準備のやり直しが要る失敗に当てはまったときは、`prepared.json` を `failed` にして初回準備を始める。
struct AppEvents<R: Runtime>(AppHandle<R>);

impl<R: Runtime> RunEvents for AppEvents<R> {
    fn started(&self, started: RunStarted) {
        let _ = self.0.emit(RUN_STARTED, started);
    }

    fn output(&self, output: RunOutput) {
        // 送れなくても（ウィンドウが無いなど）、実行は続ける。
        let _ = self.0.emit(RUN_OUTPUT, output);
    }

    fn submit_progress(&self, progress: SubmitProgress) {
        let _ = self.0.emit(SUBMIT_PROGRESS, progress);
    }

    fn finished(&self, finished: RunFinished) {
        let _ = self.0.emit(RUN_FINISHED, finished);
    }

    fn append_run(
        &self,
        lesson_id: &str,
        kind: SessionKind,
        record: RunRecord,
    ) -> Result<(), StorageError> {
        self.0
            .state::<AppDataDir>()
            .append_run(lesson_id, kind, record)
            .map(|_| ())
    }

    fn append_submit(
        &self,
        lesson_id: &str,
        kind: SessionKind,
        record: SubmitRecord,
    ) -> Result<String, StorageError> {
        self.0
            .state::<AppDataDir>()
            .append_submit(lesson_id, kind, record)
    }

    fn record_pass(
        &self,
        lesson_id: &str,
        session_id: &str,
        at: &str,
    ) -> Result<Option<LessonProgress>, StorageError> {
        self.0
            .state::<AppDataDir>()
            .record_pass(lesson_id, session_id, at)
    }

    fn crates_lost(&self) {
        restart_crate_preparation(&self.0);
    }
}
